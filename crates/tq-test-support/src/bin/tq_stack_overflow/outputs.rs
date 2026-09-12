//! Exact output captures and token comparisons, independent of timed samples.

use super::report::ScenarioRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Write as _, path::Path, time::Duration};
use tiktoken_rs::{CoreBPE, cl100k_base, o200k_base};
use tq_test_support::{
    benchmark::{BenchmarkCampaignReport, BenchmarkTool},
    compatibility::{
        Invocation, ProcessOutcome, ProcessStatus, ToolIdentity, normalize_jq,
        run_process_with_environment, toon_values_match,
    },
};

type Error = Box<dyn std::error::Error>;

#[derive(Serialize)]
pub struct SavedReport {
    #[serde(flatten)]
    pub benchmark: BenchmarkCampaignReport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_comparison: Option<OutputCampaign>,
}

impl SavedReport {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        // Deserialize benchmark u128 metrics directly; serde flatten's deferred
        // content deserializer does not support u128 values.
        let benchmark = serde_json::from_slice(bytes)?;
        let mut value: Value = serde_json::from_slice(bytes)?;
        let output_comparison = value
            .as_object_mut()
            .and_then(|object| object.remove("output_comparison"))
            .filter(|value| !value.is_null())
            .map(serde_json::from_value)
            .transpose()?;
        Ok(Self {
            benchmark,
            output_comparison,
        })
    }
}

/// Preserve the original JSON numbers rather than reserializing f64 summaries.
pub fn attach_captures(bytes: &[u8], captures: &OutputCampaign) -> Result<Value, Error> {
    let mut original: Value = serde_json::from_slice(bytes)?;
    original
        .as_object_mut()
        .ok_or("saved benchmark is not an object")?
        .insert("output_comparison".into(), serde_json::to_value(captures)?);
    Ok(original)
}

#[derive(Deserialize, Serialize)]
pub struct OutputCampaign {
    pub captured_at: String,
    pub tools: Vec<ToolIdentity>,
    pub cases: BTreeMap<String, OutputCase>,
}

#[derive(Deserialize, Serialize)]
pub struct OutputCase {
    pub query: String,
    pub input_sha256: String,
    pub compact: Capture,
    pub expanded: Capture,
    pub toon: Capture,
}

#[derive(Deserialize, Serialize)]
pub struct Capture {
    pub command: Vec<String>,
    pub status: Option<ProcessStatus>,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Capture {
    fn outcome(&self) -> ProcessOutcome {
        ProcessOutcome {
            status: self.status.unwrap_or(ProcessStatus::Signaled),
            exit_code: self.exit_code,
            signal: None,
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            wall_time_micros: 0,
            recorded_command: self.command.clone(),
        }
    }
}

pub fn capture(
    scenarios: &[ScenarioRecord],
    tools: &BTreeMap<BenchmarkTool, ToolIdentity>,
    root: &Path,
) -> Result<OutputCampaign, Error> {
    let mut cases = BTreeMap::new();
    for record in scenarios {
        let query = &record.scenario.benchmark.query;
        let compact = execute(
            tools.get(&BenchmarkTool::Jq),
            vec!["-c".into(), query.clone()],
            record,
            root,
        )?;
        let expanded = execute(
            tools.get(&BenchmarkTool::Jq),
            vec![query.clone()],
            record,
            root,
        )?;
        let toon = execute(
            tools.get(&BenchmarkTool::Tq),
            vec![
                "--input-format".into(),
                "json".into(),
                "--output-format".into(),
                "toon".into(),
                query.clone(),
            ],
            record,
            root,
        )?;
        cases.insert(
            record.scenario.id.clone(),
            OutputCase {
                query: query.clone(),
                input_sha256: super::sha256_hex(&record.input),
                compact,
                expanded,
                toon,
            },
        );
    }
    Ok(OutputCampaign {
        captured_at: jiff::Timestamp::now().to_string(),
        tools: tools
            .values()
            .filter(|tool| {
                matches!(
                    tool.tool,
                    tq_test_support::compatibility::ToolKind::Jq
                        | tq_test_support::compatibility::ToolKind::Tq
                )
            })
            .cloned()
            .collect(),
        cases,
    })
}

fn execute(
    tool: Option<&ToolIdentity>,
    args: Vec<String>,
    record: &ScenarioRecord,
    root: &Path,
) -> Result<Capture, Error> {
    let Some(tool) = tool else {
        return Ok(Capture {
            command: args,
            status: None,
            exit_code: None,
            stdout: Vec::new(),
            stderr: b"executable unavailable".to_vec(),
        });
    };
    let invocation = Invocation {
        executable: tool.path.clone(),
        args,
        stdin: record.input.clone(),
        timeout: Duration::from_secs(super::TIMEOUT_SECONDS),
        current_dir: Some(root.to_owned()),
        environment: BTreeMap::new(),
    };
    let result = run_process_with_environment(
        &invocation,
        &BTreeMap::from([("NO_COLOR".to_owned(), "1".to_owned())]),
    )?;
    Ok(Capture {
        command: result.recorded_command,
        status: Some(result.status),
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    })
}

pub fn validate(campaign: &OutputCampaign, scenarios: &[ScenarioRecord]) -> Result<(), String> {
    if campaign.cases.len() != scenarios.len() {
        return Err("output captures do not match the scenario count".into());
    }
    for record in scenarios {
        let case = campaign
            .cases
            .get(&record.scenario.id)
            .ok_or_else(|| format!("missing output capture for {}", record.scenario.id))?;
        if case.query != record.scenario.benchmark.query
            || case.input_sha256 != super::sha256_hex(&record.input)
        {
            return Err(format!("stale output capture for {}", record.scenario.id));
        }
        for capture in [&case.compact, &case.expanded, &case.toon] {
            if capture.command.last() != Some(&case.query) {
                return Err(format!(
                    "output command/query mismatch for {}",
                    record.scenario.id
                ));
            }
        }
    }
    Ok(())
}

fn strict_json(bytes: &[u8]) -> Result<Vec<Value>, &'static str> {
    let values = serde_json::Deserializer::from_slice(bytes)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "output contains raw text or invalid JSON")?;
    if values.is_empty() {
        return Err("empty output contains no JSON value");
    }
    Ok(values)
}

fn eligible(case: &OutputCase) -> Result<(), String> {
    for (name, capture) in [
        ("Compact JSON", &case.compact),
        ("Expanded JSON", &case.expanded),
        ("TOON", &case.toon),
    ] {
        if capture.status != Some(ProcessStatus::Exited) || capture.exit_code != Some(0) {
            return Err(format!("{name} did not exit successfully"));
        }
        if std::str::from_utf8(&capture.stdout).is_err() {
            return Err(format!("{name} stdout is not UTF-8"));
        }
    }
    if case.toon.stdout.is_empty() {
        return Err("TOON stdout is empty".into());
    }
    strict_json(&case.compact.stdout).map_err(|reason| format!("Compact JSON: {reason}"))?;
    strict_json(&case.expanded.stdout).map_err(|reason| format!("Expanded JSON: {reason}"))?;
    let compact = normalize_jq(&case.compact.outcome()).map_err(|error| error.to_string())?;
    let expanded = normalize_jq(&case.expanded.outcome()).map_err(|error| error.to_string())?;
    if compact.results != expanded.results
        || !toon_values_match(&case.toon.stdout, &compact.results)
    {
        return Err("JSON and TOON outputs are not semantically equivalent".into());
    }
    Ok(())
}

pub struct Tokenizers(Vec<(&'static str, CoreBPE)>);
impl Tokenizers {
    pub fn new() -> Result<Self, Error> {
        Ok(Self(vec![
            ("o200k_base", o200k_base()?),
            ("cl100k_base", cl100k_base()?),
        ]))
    }
}

pub fn render_case(campaign: Option<&OutputCampaign>, id: &str, tokenizers: &Tokenizers) -> String {
    let Some(campaign) = campaign else {
        return "\n\n### Output and tokens\n\nNot captured in this saved report. Run the `outputs` command to add output evidence.\n".into();
    };
    let case = &campaign.cases[id];
    let mut output = format!(
        "\n\n### Output and tokens\n\nCaptured separately on {}. Timing samples are unchanged.\n",
        campaign.captured_at
    );
    let _ = writeln!(
        output,
        "Output tools: {}.\n",
        campaign
            .tools
            .iter()
            .map(|tool| tool.version.replace(['\r', '\n'], " "))
            .collect::<Vec<_>>()
            .join("; ")
    );
    for (label, capture) in [
        ("Compact JSON (`jq -c`)", &case.compact),
        ("Expanded JSON (`jq`)", &case.expanded),
        ("TOON (`tq -o toon`)", &case.toon),
    ] {
        let _ = writeln!(output, "\n#### {label}\n");
        write_stdout(&mut output, &capture.stdout);
    }
    if let Err(reason) = eligible(case) {
        let _ = writeln!(output, "\nToken comparison excluded: {reason}.\n");
        return output;
    }
    output.push_str("\nCounts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.\n\n| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |\n| --- | --- | ---: | ---: | ---: | ---: |\n");
    for (name, tokenizer) in &tokenizers.0 {
        let toon = token_count(tokenizer, &case.toon.stdout);
        for (layout, capture) in [("Compact", &case.compact), ("Expanded", &case.expanded)] {
            let json = token_count(tokenizer, &capture.stdout);
            let _ = writeln!(
                output,
                "| `{name}` | {layout} | {json} | {toon} | {:+} | {} |",
                i128::from(toon) - i128::from(json),
                percent(toon, json)
            );
        }
    }
    output
}

fn token_count(tokenizer: &CoreBPE, bytes: &[u8]) -> u64 {
    tokenizer
        .encode_ordinary(std::str::from_utf8(bytes).expect("eligible UTF-8 stdout"))
        .len() as u64
}

#[allow(
    clippy::cast_precision_loss,
    reason = "human-readable percentage of token counts"
)]
fn percent(toon: u64, json: u64) -> String {
    if json == 0 {
        return "n/a".into();
    }
    format!("{:+.2}%", (toon as f64 / json as f64 - 1.0) * 100.0)
}

fn write_stdout(output: &mut String, bytes: &[u8]) {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes.escape_ascii().to_string(),
    };
    let longest = text.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(3.max(longest + 1));
    let _ = writeln!(output, "{fence}text");
    output.push_str(&text);
    if !text.ends_with('\n') {
        output.push('\n');
    }
    let _ = writeln!(output, "{fence}");
    if bytes.is_empty() {
        output.push_str("\nNo stdout bytes.\n");
    } else if !bytes.ends_with(b"\n") {
        output.push_str("\nNo trailing newline in captured stdout.\n");
    }
}

pub fn render_summary(campaign: Option<&OutputCampaign>) -> String {
    let Some(campaign) = campaign else {
        return String::new();
    };
    let included = campaign
        .cases
        .values()
        .filter(|case| eligible(case).is_ok())
        .count();
    format!(
        "\n\n### Output token coverage\n\n{included} of {} scenarios have successful, equivalent JSON and TOON output, including ordered result streams. The remaining scenarios show an exclusion reason. Each eligible page compares exact TOON stdout with both compact and expanded jq JSON using `o200k_base` and `cl100k_base`. Diff and % are negative for savings. Output captures are separate from timing samples and were collected on {}.\n",
        campaign.cases.len(),
        campaign.captured_at
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observed(text: &str) -> Capture {
        Capture {
            command: vec![".".into()],
            status: Some(ProcessStatus::Exited),
            exit_code: Some(0),
            stdout: text.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }
    fn example() -> OutputCase {
        OutputCase {
            query: ".".into(),
            input_sha256: String::new(),
            compact: observed("[65,233]\n"),
            expanded: observed("[\n  65,\n  233\n]\n"),
            toon: observed("[2]: 65,233\n"),
        }
    }
    #[test]
    fn attaching_outputs_preserves_exact_original_number_lexemes() {
        let bytes = br#"{"ratio":0.00042614858145155325,"large":18446744073709551616}"#;
        let captures = OutputCampaign {
            captured_at: "today".into(),
            tools: Vec::new(),
            cases: BTreeMap::new(),
        };
        let mut value = attach_captures(bytes, &captures).unwrap();
        value.as_object_mut().unwrap().remove("output_comparison");
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            std::str::from_utf8(bytes).unwrap()
        );
    }

    #[test]
    fn strict_json_includes_values_and_streams_but_excludes_raw_and_empty() {
        for text in [
            "42\n",
            "true\n",
            "null\n",
            "\"abcé\"\n",
            "{}\n",
            "[]\n",
            "2\n4\n",
        ] {
            assert!(strict_json(text.as_bytes()).is_ok(), "{text}");
        }
        for text in ["", "text\n", "{} garbage", "\u{1e}{}\n"] {
            assert!(strict_json(text.as_bytes()).is_err(), "{text}");
        }
    }
    #[test]
    fn errors_and_semantic_mismatches_are_excluded() {
        let mut case = example();
        assert!(eligible(&case).is_ok());
        case.toon = observed("[1]: 65\n");
        assert!(eligible(&case).is_err());
        case.toon = observed("[2]: 65,233\n");
        case.toon.exit_code = Some(5);
        assert!(eligible(&case).is_err());
        case.compact = observed("{}\n");
        case.expanded = observed("{}\n");
        case.toon = observed("");
        assert!(eligible(&case).is_err());
    }

    #[test]
    fn streams_preserve_values_order_cardinality_and_complete_stdout() {
        for (json, toon) in [
            ("2\n4\n", "2\n4\n"),
            ("\"a\"\n\"b\"\n", "a\nb\n"),
            ("{\"a\":1}\n{\"b\":2}\n", "a: 1\nb: 2\n"),
            ("[1,2]\n[3]\n", "[2]: 1,2\n[1]: 3\n"),
        ] {
            let mut case = example();
            case.compact = observed(json);
            case.expanded = observed(json);
            case.toon = observed(toon);
            assert!(eligible(&case).is_ok(), "{json}");
        }
        let mut case = example();
        case.compact = observed("2\n4\n");
        case.expanded = observed("2\n4\n");
        for toon in ["4\n2\n", "2\n", "2\n4\n6\n", "2\n4", "2\n4\ngarbage"] {
            case.toon = observed(toon);
            assert!(eligible(&case).is_err(), "{toon}");
        }
        case.toon = observed("2\n4\n");
        case.expanded = observed("4\n2\n");
        assert!(eligible(&case).is_err());
    }

    #[test]
    fn exact_stdout_counts_include_the_terminating_newline() {
        for (_, tokenizer) in Tokenizers::new().unwrap().0 {
            assert_eq!(token_count(&tokenizer, b"lambda"), 1);
            assert_eq!(token_count(&tokenizer, b"lambda\n"), 2);
            assert_eq!(token_count(&tokenizer, b"\"lambda\"\n"), 3);
        }
    }
    #[test]
    fn percentage_signs_follow_toon_minus_json() {
        assert_eq!(percent(5, 10), "-50.00%");
        assert_eq!(percent(12, 10), "+20.00%");
        assert_eq!(percent(10, 10), "+0.00%");
    }
    #[test]
    fn tables_compare_both_layouts_and_show_exact_toon() {
        let campaign = OutputCampaign {
            captured_at: "today".into(),
            tools: Vec::new(),
            cases: BTreeMap::from([("case".into(), example())]),
        };
        let text = render_case(Some(&campaign), "case", &Tokenizers::new().unwrap());
        assert!(text.contains("[2]: 65,233\n"));
        for tokenizer in ["o200k_base", "cl100k_base"] {
            for layout in ["Compact", "Expanded"] {
                assert!(text.contains(&format!("| `{tokenizer}` | {layout} |")));
            }
        }
        assert!(!text.contains("excluded"));
        assert!(
            serde_json::from_str::<OutputCampaign>(&serde_json::to_string(&campaign).unwrap())
                .is_ok()
        );
    }
    #[test]
    fn code_fences_do_not_hide_backticks_or_add_counted_newlines() {
        let mut output = String::new();
        write_stdout(&mut output, b"```\n");
        assert!(output.starts_with("````text\n```\n````\n"));
        let mut output = String::new();
        write_stdout(&mut output, b"abc");
        assert!(output.contains("No trailing newline"));
    }
}
