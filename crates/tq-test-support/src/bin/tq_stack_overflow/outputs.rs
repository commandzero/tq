//! Exact output captures and token comparisons, independent of timed samples.

use super::report::OutputMode;
use super::report::ScenarioRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
    time::Duration,
};
use tiktoken_rs::{CoreBPE, cl100k_base, o200k_base};
use tq_test_support::{
    benchmark::{BenchmarkCampaignReport, BenchmarkTool},
    compatibility::{
        Invocation, OutputLimit, ProcessOutcome, ProcessStatus, ToolIdentity, ToolKind,
        normalize_jq, run_process_with_environment_bounded, toon_values_match,
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
    /// Output profile used for this capture. Missing means legacy structured output.
    #[serde(default)]
    pub output_mode: OutputMode,
    /// Exact fixture and option provenance for non-legacy captures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<OutputProvenance>,
    /// Legacy structured capture layout retained for old reports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compact: Option<Capture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded: Option<Capture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toon: Option<Capture>,
    /// Explicit per-tool captures for raw and color profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_captures: Option<BTreeMap<BenchmarkTool, Capture>>,
}

#[derive(Deserialize, Serialize)]
pub struct OutputProvenance {
    pub fixture: String,
    pub input_bytes: u64,
    pub output_mode: OutputMode,
    pub environment: BTreeMap<String, String>,
    pub contract_sha256: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Capture {
    pub command: Vec<String>,
    pub status: Option<ProcessStatus>,
    pub exit_code: Option<i32>,
    /// Set when stdout or stderr exceeded the capture limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_limit: Option<OutputLimit>,
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
        let mode = record.scenario.benchmark.output_mode;
        let (compact, expanded, toon, scenario_captures) = match mode {
            OutputMode::Structured => {
                let compact = execute(
                    tools.get(&BenchmarkTool::Jq),
                    profile_args(mode, BenchmarkTool::Jq, query),
                    record,
                    root,
                    mode,
                )?;
                let expanded = execute(
                    tools.get(&BenchmarkTool::Jq),
                    vec![query.clone()],
                    record,
                    root,
                    mode,
                )?;
                let toon = execute(
                    tools.get(&BenchmarkTool::Tq),
                    profile_args(mode, BenchmarkTool::Tq, query),
                    record,
                    root,
                    mode,
                )?;
                (Some(compact), Some(expanded), Some(toon), None)
            }
            OutputMode::Raw | OutputMode::Color => {
                let captures = capture_plan(mode, query)
                    .into_iter()
                    .map(|(tool, args)| {
                        execute(tools.get(&tool), args, record, root, mode)
                            .map(|capture| (tool, capture))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?;
                (None, None, None, Some(captures))
            }
        };
        let environment = profile_environment(mode);
        let digest_captures = match mode {
            OutputMode::Structured => vec![
                compact.as_ref().expect("compact capture"),
                expanded.as_ref().expect("expanded capture"),
                toon.as_ref().expect("TOON capture"),
            ],
            OutputMode::Raw | OutputMode::Color => scenario_captures
                .as_ref()
                .expect("per-tool captures")
                .values()
                .collect(),
        };
        let contract_sha256 =
            contract_digest(mode, query, &record.input, digest_captures, &environment);
        cases.insert(
            record.scenario.id.clone(),
            OutputCase {
                query: query.clone(),
                input_sha256: super::sha256_hex(&record.input),
                output_mode: mode,
                provenance: Some(OutputProvenance {
                    fixture: fixture_identity(&record.path),
                    input_bytes: u64::try_from(record.input.len()).unwrap_or(u64::MAX),
                    output_mode: mode,
                    environment,
                    contract_sha256,
                }),
                compact,
                expanded,
                toon,
                scenario_captures,
            },
        );
    }
    Ok(OutputCampaign {
        captured_at: jiff::Timestamp::now().to_string(),
        tools: tools.values().cloned().collect(),
        cases,
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "capture setup keeps the executable, environment, and raw process result together"
)]
fn execute(
    tool: Option<&ToolIdentity>,
    args: Vec<String>,
    record: &ScenarioRecord,
    root: &Path,
    mode: OutputMode,
) -> Result<Capture, Error> {
    let Some(tool) = tool else {
        return Ok(Capture {
            command: args,
            status: None,
            exit_code: None,
            output_limit: None,
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
    let result = run_process_with_environment_bounded(
        &invocation,
        &profile_environment(mode),
        super::OUTPUT_LIMIT,
    )?;
    Ok(Capture {
        command: result.outcome.recorded_command,
        status: Some(result.outcome.status),
        exit_code: result.outcome.exit_code,
        output_limit: result.output_limit,
        stdout: result.outcome.stdout,
        stderr: result.outcome.stderr,
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "validation checks schema, commands, profiles, and provenance atomically"
)]
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
        let mode = record.scenario.benchmark.output_mode;
        if case.output_mode != mode {
            return Err(format!("stale output profile for {}", record.scenario.id));
        }
        let captures =
            match mode {
                OutputMode::Structured => {
                    let compact = case.compact.as_ref().ok_or_else(|| {
                        format!("missing compact capture for {}", record.scenario.id)
                    })?;
                    let expanded = case.expanded.as_ref().ok_or_else(|| {
                        format!("missing expanded capture for {}", record.scenario.id)
                    })?;
                    let toon = case.toon.as_ref().ok_or_else(|| {
                        format!("missing TOON capture for {}", record.scenario.id)
                    })?;
                    if case.scenario_captures.is_some()
                        || !command_matches(
                            campaign,
                            BenchmarkTool::Jq,
                            compact,
                            &profile_args(mode, BenchmarkTool::Jq, &case.query),
                        )
                        || !command_matches(
                            campaign,
                            BenchmarkTool::Jq,
                            expanded,
                            std::slice::from_ref(&case.query),
                        )
                        || !(command_matches(
                            campaign,
                            BenchmarkTool::Tq,
                            toon,
                            &profile_args(mode, BenchmarkTool::Tq, &case.query),
                        ) || (case.provenance.is_none()
                            && command_matches(
                                campaign,
                                BenchmarkTool::Tq,
                                toon,
                                &legacy_toon_args(&case.query),
                            )))
                    {
                        return Err(format!(
                            "output command/options mismatch for {}",
                            record.scenario.id
                        ));
                    }
                    vec![compact, expanded, toon]
                }
                OutputMode::Raw | OutputMode::Color => {
                    if case.compact.is_some() || case.expanded.is_some() || case.toon.is_some() {
                        return Err(format!(
                            "raw/color capture uses named structured fields for {}",
                            record.scenario.id
                        ));
                    }
                    let scenario_captures = case.scenario_captures.as_ref().ok_or_else(|| {
                        format!("missing per-tool captures for {}", record.scenario.id)
                    })?;
                    let expected_tools = [BenchmarkTool::Jq, BenchmarkTool::Yq, BenchmarkTool::Tq];
                    if scenario_captures.len() != expected_tools.len() {
                        return Err(format!(
                            "incomplete per-tool captures for {}",
                            record.scenario.id
                        ));
                    }
                    for tool in expected_tools {
                        let capture = scenario_captures.get(&tool).ok_or_else(|| {
                            format!("missing {tool:?} capture for {}", record.scenario.id)
                        })?;
                        if !command_matches(
                            campaign,
                            tool,
                            capture,
                            &profile_args(mode, tool, &case.query),
                        ) {
                            return Err(format!(
                                "output command/options mismatch for {}",
                                record.scenario.id
                            ));
                        }
                    }
                    expected_tools
                        .into_iter()
                        .map(|tool| &scenario_captures[&tool])
                        .collect()
                }
            };
        match &case.provenance {
            None if mode == OutputMode::Structured => {}
            None => {
                return Err(format!(
                    "non-structured output capture for {} has no provenance",
                    record.scenario.id
                ));
            }
            Some(provenance) => {
                let environment = profile_environment(mode);
                if provenance.fixture != fixture_identity(&record.path)
                    || provenance.input_bytes
                        != u64::try_from(record.input.len()).unwrap_or(u64::MAX)
                    || provenance.output_mode != mode
                    || provenance.environment != environment
                    || provenance.contract_sha256
                        != contract_digest(mode, &case.query, &record.input, captures, &environment)
                {
                    return Err(format!(
                        "stale output provenance for {}",
                        record.scenario.id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn command_matches(
    campaign: &OutputCampaign,
    tool: BenchmarkTool,
    capture: &Capture,
    args: &[String],
) -> bool {
    if capture.status.is_none() {
        return capture.command == args;
    }
    let Some(identity) = campaign.tools.iter().find(|identity| {
        matches!(
            (tool, identity.tool),
            (BenchmarkTool::Jq, ToolKind::Jq)
                | (BenchmarkTool::Yq, ToolKind::Yq)
                | (BenchmarkTool::Tq, ToolKind::Tq)
        )
    }) else {
        return false;
    };
    let mut expected = vec![identity.path.display().to_string()];
    expected.extend(args.iter().cloned());
    capture.command == expected
}

fn capture_plan(mode: OutputMode, query: &str) -> Vec<(BenchmarkTool, Vec<String>)> {
    match mode {
        OutputMode::Structured => vec![(
            BenchmarkTool::Tq,
            profile_args(mode, BenchmarkTool::Tq, query),
        )],
        OutputMode::Raw | OutputMode::Color => vec![
            (
                BenchmarkTool::Jq,
                profile_args(mode, BenchmarkTool::Jq, query),
            ),
            (
                BenchmarkTool::Yq,
                profile_args(mode, BenchmarkTool::Yq, query),
            ),
            (
                BenchmarkTool::Tq,
                profile_args(mode, BenchmarkTool::Tq, query),
            ),
        ],
    }
}

pub(crate) fn legacy_toon_args(query: &str) -> Vec<String> {
    vec![
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "toon".to_owned(),
        query.to_owned(),
    ]
}

fn fixture_identity(path: &Path) -> String {
    let mut components = path.components();
    let mut found_tests = false;
    let mut identity = PathBuf::new();
    for component in components.by_ref() {
        if found_tests {
            identity.push(component.as_os_str());
        } else if component.as_os_str() == "tests" {
            found_tests = true;
            identity.push(component.as_os_str());
        }
    }
    if found_tests {
        identity.display().to_string()
    } else {
        path.display().to_string()
    }
}

pub(crate) fn profile_args(mode: OutputMode, tool: BenchmarkTool, query: &str) -> Vec<String> {
    let mut args = match (mode, tool) {
        (OutputMode::Structured, BenchmarkTool::Jq) => vec!["-c".to_owned()],
        (OutputMode::Structured, BenchmarkTool::Yq) => vec![
            "-p=json".to_owned(),
            "-o=json".to_owned(),
            "-I=0".to_owned(),
        ],
        (OutputMode::Structured, BenchmarkTool::Tq) => {
            vec!["--input-format".to_owned(), "json".to_owned()]
        }
        (OutputMode::Raw, BenchmarkTool::Jq) => vec!["-r".to_owned()],
        (OutputMode::Raw, BenchmarkTool::Yq) => vec![
            "-p=json".to_owned(),
            "-o=json".to_owned(),
            "-I=0".to_owned(),
            "-r".to_owned(),
        ],
        (OutputMode::Raw, BenchmarkTool::Tq) => {
            vec![
                "--input-format".to_owned(),
                "json".to_owned(),
                "-r".to_owned(),
            ]
        }
        (OutputMode::Color, BenchmarkTool::Jq) => vec!["-C".to_owned()],
        (OutputMode::Color, BenchmarkTool::Yq) => vec![
            "-p=json".to_owned(),
            "-o=json".to_owned(),
            "-I=0".to_owned(),
            "-C".to_owned(),
        ],
        (OutputMode::Color, BenchmarkTool::Tq) => vec![
            "--input-format".to_owned(),
            "json".to_owned(),
            "--output-format".to_owned(),
            "json".to_owned(),
            "-C".to_owned(),
        ],
    };
    args.push(query.to_owned());
    args
}

pub(crate) fn profile_environment(mode: OutputMode) -> BTreeMap<String, String> {
    match mode {
        OutputMode::Structured | OutputMode::Raw => {
            BTreeMap::from([("NO_COLOR".to_owned(), "1".to_owned())])
        }
        OutputMode::Color => BTreeMap::new(),
    }
}

fn contract_digest(
    mode: OutputMode,
    query: &str,
    input: &[u8],
    captures: Vec<&Capture>,
    environment: &BTreeMap<String, String>,
) -> String {
    let commands = captures
        .into_iter()
        .map(|capture| &capture.command)
        .collect::<Vec<_>>();
    let canonical = serde_json::json!({
        "output_mode": mode,
        "query": query,
        "input_sha256": super::sha256_hex(input),
        "input_bytes": input.len(),
        "commands": commands,
        "environment": environment,
    });
    super::sha256_hex(&serde_json::to_vec(&canonical).expect("provenance serializes"))
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
    match case.output_mode {
        OutputMode::Structured => structured_eligible(case),
        OutputMode::Raw => raw_eligible(case),
        OutputMode::Color => color_eligible(case),
    }
}

fn successful_capture(name: &str, capture: &Capture) -> Result<(), String> {
    if capture.output_limit.is_some() {
        return Err(format!("{name} exceeded the bounded output limit"));
    }
    if capture.status != Some(ProcessStatus::Exited) || capture.exit_code != Some(0) {
        return Err(format!("{name} did not exit successfully"));
    }
    if std::str::from_utf8(&capture.stdout).is_err() {
        return Err(format!("{name} stdout is not UTF-8"));
    }
    Ok(())
}

fn structured_eligible(case: &OutputCase) -> Result<(), String> {
    let compact = case
        .compact
        .as_ref()
        .ok_or("Compact JSON capture is missing")?;
    let expanded = case
        .expanded
        .as_ref()
        .ok_or("Expanded JSON capture is missing")?;
    let toon = case.toon.as_ref().ok_or("TOON capture is missing")?;
    for (name, capture) in [
        ("Compact JSON", compact),
        ("Expanded JSON", expanded),
        ("TOON", toon),
    ] {
        successful_capture(name, capture)?;
    }
    if toon.stdout.is_empty() {
        return Err("TOON stdout is empty".into());
    }
    strict_json(&compact.stdout).map_err(|reason| format!("Compact JSON: {reason}"))?;
    strict_json(&expanded.stdout).map_err(|reason| format!("Expanded JSON: {reason}"))?;
    let compact = normalize_jq(&compact.outcome()).map_err(|error| error.to_string())?;
    let expanded = normalize_jq(&expanded.outcome()).map_err(|error| error.to_string())?;
    if compact.results != expanded.results || !toon_values_match(&toon.stdout, &compact.results) {
        return Err("JSON and TOON outputs are not semantically equivalent".into());
    }
    Ok(())
}

fn raw_eligible(case: &OutputCase) -> Result<(), String> {
    let captures = case
        .scenario_captures
        .as_ref()
        .ok_or("raw output captures are missing")?;
    if captures.len() != 3 {
        return Err("raw output captures are incomplete".into());
    }
    let mut values = captures.values();
    let first = values.next().ok_or("raw output captures are empty")?;
    successful_capture("raw reference", first)?;
    for capture in values {
        successful_capture("raw candidate", capture)?;
        if capture.stdout != first.stdout {
            return Err("raw output bytes differ".into());
        }
    }
    Ok(())
}

fn color_eligible(case: &OutputCase) -> Result<(), String> {
    let captures = case
        .scenario_captures
        .as_ref()
        .ok_or("color output captures are missing")?;
    if captures.len() != 3 {
        return Err("color output captures are incomplete".into());
    }
    let mut results = None;
    for (tool, capture) in captures {
        successful_capture("color output", capture)?;
        let stripped = strip_sgr(&capture.stdout)
            .map_err(|error| format!("{tool:?} color output: {error}"))?;
        strict_json(&stripped).map_err(|reason| format!("{tool:?} color JSON: {reason}"))?;
        let normalized = normalize_jq(&ProcessOutcome {
            status: ProcessStatus::Exited,
            exit_code: Some(0),
            signal: None,
            stdout: stripped,
            stderr: Vec::new(),
            wall_time_micros: 0,
            recorded_command: capture.command.clone(),
        })
        .map_err(|error| format!("{tool:?} color JSON: {error}"))?;
        if let Some(expected) = &results {
            if expected != &normalized.results {
                return Err("color outputs are not semantically equivalent".into());
            }
        } else {
            results = Some(normalized.results);
        }
    }
    Ok(())
}

fn strip_sgr(bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut stripped = Vec::with_capacity(bytes.len());
    let mut saw_sgr = false;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            stripped.push(bytes[index]);
            index += 1;
            continue;
        }
        if bytes.get(index + 1) != Some(&b'[') {
            return Err("contains a non-SGR ANSI escape");
        }
        let mut end = index + 2;
        while let Some(byte) = bytes.get(end) {
            if *byte == b'm' {
                break;
            }
            if !byte.is_ascii_digit() && *byte != b';' {
                return Err("contains a malformed SGR escape");
            }
            end += 1;
        }
        if bytes.get(end) != Some(&b'm') {
            return Err("contains an unterminated SGR escape");
        }
        saw_sgr = true;
        index = end + 1;
    }
    saw_sgr
        .then_some(stripped)
        .ok_or("does not contain an SGR escape")
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
        "\n\n### Output and tokens\n\nCaptured separately from timed samples.\nOutput profile: `{}`.\n",
        output_mode_name(case.output_mode)
    );
    if case.output_mode == OutputMode::Color {
        output.push_str(
            "ANSI control bytes are escaped for display; saved stdout retains the exact raw bytes.\n",
        );
    }
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
    let rendered_captures = match case.output_mode {
        OutputMode::Structured => vec![
            (
                "Compact JSON (`jq -c`)",
                case.compact.as_ref().expect("validated compact capture"),
            ),
            (
                "Expanded JSON (`jq`)",
                case.expanded.as_ref().expect("validated expanded capture"),
            ),
            (
                "TOON (`tq` default)",
                case.toon.as_ref().expect("validated TOON capture"),
            ),
        ],
        OutputMode::Raw | OutputMode::Color => case
            .scenario_captures
            .as_ref()
            .expect("validated per-tool captures")
            .iter()
            .map(|(tool, capture)| {
                (
                    match tool {
                        BenchmarkTool::Jq => "jq",
                        BenchmarkTool::Yq => "yq",
                        BenchmarkTool::Tq => "tq",
                    },
                    capture,
                )
            })
            .collect(),
    };
    for (label, capture) in rendered_captures {
        let _ = writeln!(
            output,
            "\n#### {label}\n\nStatus: `{}`.\nCommand: `{}`\n",
            capture_status(capture),
            display_command(&capture.command),
        );
        write_stdout(&mut output, &capture.stdout);
    }
    if case.output_mode != OutputMode::Structured {
        match eligible(case) {
            Ok(()) => output.push_str(
                "\nToken comparison is not applicable to raw or color output profiles.\n",
            ),
            Err(reason) => {
                let _ = writeln!(output, "\nOutput gate failed: {reason}.\n");
            }
        }
        return output;
    }
    if let Err(reason) = eligible(case) {
        let _ = writeln!(output, "\nToken comparison excluded: {reason}.\n");
        return output;
    }
    let compact = case.compact.as_ref().expect("validated compact capture");
    let expanded = case.expanded.as_ref().expect("validated expanded capture");
    let toon = case.toon.as_ref().expect("validated TOON capture");
    output.push_str("\nCounts include complete stdout and its trailing newline. Diff is TOON minus JSON; negative values are savings.\n\n| Tokenizer | JSON layout | JSON tokens | TOON tokens | Diff | % |\n| --- | --- | ---: | ---: | ---: | ---: |\n");
    for (name, tokenizer) in &tokenizers.0 {
        let toon = token_count(tokenizer, &toon.stdout);
        for (layout, capture) in [("Compact", compact), ("Expanded", expanded)] {
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
        Ok(_text) if bytes.contains(&0x1b) => bytes.escape_ascii().to_string(),
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

fn output_mode_name(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::Structured => "structured",
        OutputMode::Raw => "raw",
        OutputMode::Color => "color",
    }
}

fn capture_status(capture: &Capture) -> &'static str {
    if capture.output_limit.is_some() {
        return "output-limit";
    }
    match capture.status {
        None => "unavailable",
        Some(ProcessStatus::Exited) if capture.exit_code == Some(0) => "exited-0",
        Some(ProcessStatus::Exited) => "exited-nonzero",
        Some(ProcessStatus::TimedOut) => "timed-out",
        Some(ProcessStatus::Signaled) => "signaled",
    }
}

fn display_command(command: &[String]) -> String {
    let mut arguments = command.iter();
    let executable = arguments
        .next()
        .map(|path| {
            Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
        })
        .unwrap_or_default();
    std::iter::once(executable)
        .chain(arguments.map(String::as_str))
        .map(shell_quote)
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(argument: &str) -> String {
    if !argument.is_empty()
        && argument
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_./:=+-".contains(&byte))
    {
        return argument.to_owned();
    }
    format!("'{}'", argument.replace('\'', "'\\''"))
}

pub fn render_summary(campaign: Option<&OutputCampaign>) -> String {
    let Some(campaign) = campaign else {
        return String::new();
    };
    let included = campaign
        .cases
        .values()
        .filter(|case| case.output_mode == OutputMode::Structured && eligible(case).is_ok())
        .count();
    let structured = campaign
        .cases
        .values()
        .filter(|case| case.output_mode == OutputMode::Structured)
        .count();
    format!(
        "\n\n### Output token coverage\n\n{included} of {structured} structured scenarios have successful, equivalent JSON and TOON output, including ordered result streams. Raw and color profiles use separate output gates and do not contribute token savings. Each eligible structured page compares exact TOON stdout with both compact and expanded jq JSON using `o200k_base` and `cl100k_base`. Diff and % are negative for savings. Output captures are separate from timing samples; their exact capture timestamp is preserved in the saved output artifact.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::super::report::{BenchmarkInput, Scenario, Source};
    use super::*;
    use std::path::PathBuf;
    use tq_test_support::compatibility::{ToolIdentity, ToolKind};
    use tq_test_support::corpus::ArtifactIdentity;

    #[test]
    fn output_profiles_build_stable_native_arguments() {
        assert_eq!(
            profile_args(OutputMode::Structured, BenchmarkTool::Jq, "."),
            ["-c", "."]
        );
        assert_eq!(
            profile_args(OutputMode::Raw, BenchmarkTool::Jq, ".name"),
            ["-r", ".name"]
        );
        assert_eq!(
            profile_args(OutputMode::Raw, BenchmarkTool::Yq, ".name"),
            ["-p=json", "-o=json", "-I=0", "-r", ".name"]
        );
        assert_eq!(
            profile_args(OutputMode::Color, BenchmarkTool::Tq, "."),
            [
                "--input-format",
                "json",
                "--output-format",
                "json",
                "-C",
                "."
            ]
        );
        assert_eq!(
            profile_environment(OutputMode::Structured),
            BTreeMap::from([(String::from("NO_COLOR"), String::from("1"))])
        );
        assert_eq!(profile_environment(OutputMode::Color), BTreeMap::new());
    }

    #[test]
    fn raw_and_color_profiles_have_independent_output_gates() {
        let mut raw = example();
        raw.output_mode = OutputMode::Raw;
        raw.compact = None;
        raw.expanded = None;
        raw.toon = None;
        raw.scenario_captures = Some(BTreeMap::from([
            (BenchmarkTool::Jq, observed("raw\n")),
            (BenchmarkTool::Yq, observed("raw\n")),
            (BenchmarkTool::Tq, observed("raw\n")),
        ]));
        assert!(raw_eligible(&raw).is_ok());
        raw.scenario_captures
            .as_mut()
            .unwrap()
            .get_mut(&BenchmarkTool::Tq)
            .unwrap()
            .stdout = b"different\n".to_vec();
        assert!(raw_eligible(&raw).is_err());

        let mut color = example();
        color.output_mode = OutputMode::Color;
        color.compact = None;
        color.expanded = None;
        color.toon = None;
        let ansi = b"\x1b[31m[65,233]\x1b[0m\n".to_vec();
        color.scenario_captures = Some(BTreeMap::from([
            (
                BenchmarkTool::Jq,
                observed(std::str::from_utf8(&ansi).unwrap()),
            ),
            (
                BenchmarkTool::Yq,
                observed(std::str::from_utf8(&ansi).unwrap()),
            ),
            (
                BenchmarkTool::Tq,
                observed(std::str::from_utf8(&ansi).unwrap()),
            ),
        ]));
        assert!(color_eligible(&color).is_ok());
        color
            .scenario_captures
            .as_mut()
            .unwrap()
            .get_mut(&BenchmarkTool::Tq)
            .unwrap()
            .stdout = b"\x1b[31m[65,234]\x1b[0m\n".to_vec();
        assert!(color_eligible(&color).is_err());
        color
            .scenario_captures
            .as_mut()
            .unwrap()
            .get_mut(&BenchmarkTool::Tq)
            .unwrap()
            .stdout = b"[65,233]\n".to_vec();
        assert!(color_eligible(&color).is_err());
    }

    #[test]
    fn profiled_capture_provenance_survives_roundtrip_and_worktree_move() {
        for mode in [OutputMode::Structured, OutputMode::Raw, OutputMode::Color] {
            let (campaign, mut scenario) = validation_campaign(mode);
            let restored: OutputCampaign =
                serde_json::from_slice(&serde_json::to_vec(&campaign).unwrap()).unwrap();
            assert!(validate(&restored, std::slice::from_ref(&scenario)).is_ok());
            scenario.path = PathBuf::from("/another/worktree").join(&scenario.path);
            assert!(validate(&restored, std::slice::from_ref(&scenario)).is_ok());
        }
    }

    #[test]
    fn output_validation_accepts_exact_legacy_toon_and_rejects_extra_options() {
        let (mut legacy, scenario) = validation_campaign(OutputMode::Structured);
        let case = legacy.cases.get_mut("case").expect("case");
        case.provenance = None;
        case.toon.as_mut().expect("TOON").command = {
            let mut command = vec!["/tools/tq".to_owned()];
            command.extend(legacy_toon_args(".name"));
            command
        };
        assert!(validate(&legacy, std::slice::from_ref(&scenario)).is_ok());

        let (mut stale, scenario) = validation_campaign(OutputMode::Structured);
        stale
            .cases
            .get_mut("case")
            .expect("case")
            .compact
            .as_mut()
            .expect("compact")
            .command
            .insert(0, "--extra".to_owned());
        assert!(
            validate(&stale, std::slice::from_ref(&scenario))
                .expect_err("extra leading option")
                .contains("command/options")
        );
    }

    #[test]
    fn output_validation_requires_fresh_nonstructured_provenance_and_mode() {
        let (mut missing, scenario) = validation_campaign(OutputMode::Raw);
        missing.cases.get_mut("case").expect("case").provenance = None;
        assert!(
            validate(&missing, std::slice::from_ref(&scenario))
                .expect_err("missing provenance")
                .contains("no provenance")
        );

        let (mut stale, scenario) = validation_campaign(OutputMode::Raw);
        stale
            .cases
            .get_mut("case")
            .expect("case")
            .provenance
            .as_mut()
            .expect("provenance")
            .fixture = "tests/stack-overflow/other.toon".to_owned();
        assert!(
            validate(&stale, std::slice::from_ref(&scenario))
                .expect_err("stale provenance")
                .contains("stale output provenance")
        );

        let (mut changed_mode, scenario) = validation_campaign(OutputMode::Raw);
        changed_mode
            .cases
            .get_mut("case")
            .expect("case")
            .output_mode = OutputMode::Color;
        assert!(
            validate(&changed_mode, std::slice::from_ref(&scenario))
                .expect_err("stale mode")
                .contains("stale output profile")
        );
    }

    type ValidationTool = (BenchmarkTool, &'static str, ToolKind);
    type ValidationCaptureSet = (
        Option<Capture>,
        Option<Capture>,
        Option<Capture>,
        Option<BTreeMap<BenchmarkTool, Capture>>,
    );

    fn validation_campaign(mode: OutputMode) -> (OutputCampaign, ScenarioRecord) {
        let scenario = validation_scenario(mode);
        let tools = validation_tools();
        let identities = validation_identities(&tools);
        let (compact, expanded, toon, scenario_captures) = validation_captures(mode, &tools);
        let input_bytes = &scenario.input;
        let environment = profile_environment(mode);
        let case = OutputCase {
            query: ".name".to_owned(),
            input_sha256: super::super::sha256_hex(input_bytes),
            output_mode: mode,
            provenance: Some(OutputProvenance {
                fixture: "tests/stack-overflow/01-case.toon".to_owned(),
                input_bytes: input_bytes.len() as u64,
                output_mode: mode,
                environment: environment.clone(),
                contract_sha256: contract_digest(
                    mode,
                    ".name",
                    input_bytes,
                    validation_capture_refs(
                        mode,
                        compact.as_ref(),
                        expanded.as_ref(),
                        toon.as_ref(),
                        scenario_captures.as_ref(),
                    ),
                    &environment,
                ),
            }),
            compact,
            expanded,
            toon,
            scenario_captures,
        };
        (
            OutputCampaign {
                captured_at: "today".to_owned(),
                tools: identities,
                cases: BTreeMap::from([("case".to_owned(), case)]),
            },
            scenario,
        )
    }

    fn validation_scenario(mode: OutputMode) -> ScenarioRecord {
        let input = serde_json::json!({"name": "lambda"});
        ScenarioRecord {
            scenario: Scenario {
                id: "case".to_owned(),
                rank: 1,
                source: Source {
                    title: "case".to_owned(),
                    url: None,
                    score: None,
                    retrieved_from: None,
                },
                answer: None,
                benchmark: BenchmarkInput {
                    output_mode: mode,
                    query: ".name".to_owned(),
                    input: input.clone(),
                },
            },
            path: PathBuf::from("tests/stack-overflow/01-case.toon"),
            input: serde_json::to_vec(&input).expect("input"),
        }
    }

    fn validation_tools() -> [ValidationTool; 3] {
        [
            (BenchmarkTool::Jq, "/tools/jq", ToolKind::Jq),
            (BenchmarkTool::Yq, "/tools/yq", ToolKind::Yq),
            (BenchmarkTool::Tq, "/tools/tq", ToolKind::Tq),
        ]
    }

    fn validation_identities(tools: &[ValidationTool; 3]) -> Vec<ToolIdentity> {
        tools
            .iter()
            .map(|(_, path, kind)| ToolIdentity {
                tool: *kind,
                path: PathBuf::from(path),
                version: "test".to_owned(),
                executable: ArtifactIdentity {
                    path: (*path).to_owned(),
                    bytes: 1,
                    sha256: "x".repeat(64),
                },
                build_features: Vec::new(),
                runtime_libraries: Vec::new(),
            })
            .collect()
    }

    fn validation_captures(mode: OutputMode, tools: &[ValidationTool; 3]) -> ValidationCaptureSet {
        match mode {
            OutputMode::Structured => (
                Some(validation_capture(
                    tools,
                    BenchmarkTool::Jq,
                    profile_args(mode, BenchmarkTool::Jq, ".name"),
                )),
                Some(validation_capture(
                    tools,
                    BenchmarkTool::Jq,
                    vec![".name".to_owned()],
                )),
                Some(validation_capture(
                    tools,
                    BenchmarkTool::Tq,
                    profile_args(mode, BenchmarkTool::Tq, ".name"),
                )),
                None,
            ),
            OutputMode::Raw | OutputMode::Color => (
                None,
                None,
                None,
                Some(
                    tools
                        .iter()
                        .map(|(tool, _, _)| {
                            (
                                *tool,
                                validation_capture(
                                    tools,
                                    *tool,
                                    profile_args(mode, *tool, ".name"),
                                ),
                            )
                        })
                        .collect(),
                ),
            ),
        }
    }

    fn validation_capture(
        tools: &[ValidationTool; 3],
        tool: BenchmarkTool,
        args: Vec<String>,
    ) -> Capture {
        let path = tools
            .iter()
            .find(|(candidate, _, _)| *candidate == tool)
            .expect("tool")
            .1;
        let mut command = vec![path.to_owned()];
        command.extend(args);
        Capture {
            command,
            status: Some(ProcessStatus::Exited),
            exit_code: Some(0),
            output_limit: None,
            stdout: b"1\n".to_vec(),
            stderr: Vec::new(),
        }
    }

    fn validation_capture_refs<'a>(
        mode: OutputMode,
        compact: Option<&'a Capture>,
        expanded: Option<&'a Capture>,
        toon: Option<&'a Capture>,
        scenario_captures: Option<&'a BTreeMap<BenchmarkTool, Capture>>,
    ) -> Vec<&'a Capture> {
        match mode {
            OutputMode::Structured => vec![
                compact.expect("compact"),
                expanded.expect("expanded"),
                toon.expect("TOON"),
            ],
            OutputMode::Raw | OutputMode::Color => scenario_captures
                .expect("per-tool captures")
                .values()
                .collect(),
        }
    }

    fn observed(text: &str) -> Capture {
        Capture {
            command: vec![".".into()],
            status: Some(ProcessStatus::Exited),
            exit_code: Some(0),
            output_limit: None,
            stdout: text.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }
    fn example() -> OutputCase {
        OutputCase {
            query: ".".into(),
            input_sha256: String::new(),
            output_mode: OutputMode::Structured,
            provenance: None,
            compact: Some(observed("[65,233]\n")),
            expanded: Some(observed("[\n  65,\n  233\n]\n")),
            toon: Some(observed("[2]: 65,233\n")),
            scenario_captures: None,
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
    fn legacy_capture_without_output_limit_deserializes_as_complete() {
        let capture: Capture = serde_json::from_str(
            r#"{"command":["tq"],"status":"exited","exit_code":0,"stdout":[49,10],"stderr":[]}"#,
        )
        .expect("legacy capture");
        assert_eq!(capture.output_limit, None);
        assert_eq!(capture_status(&capture), "exited-0");
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
        case.toon = Some(observed("[1]: 65\n"));
        assert!(eligible(&case).is_err());
        case.toon = Some(observed("[2]: 65,233\n"));
        case.toon.as_mut().unwrap().exit_code = Some(5);
        assert!(eligible(&case).is_err());
        case.compact = Some(observed("{}\n"));
        case.expanded = Some(observed("{}\n"));
        case.toon = Some(observed(""));
        assert!(eligible(&case).is_err());
        case.toon = Some(observed("a: 1\n"));
        case.toon.as_mut().unwrap().output_limit = Some(OutputLimit {
            stream: tq_test_support::compatibility::OutputStream::Stdout,
            limit: 32,
            observed_bytes: 33,
        });
        assert_eq!(capture_status(case.toon.as_ref().unwrap()), "output-limit");
        assert!(eligible(&case).is_err());
        let campaign = OutputCampaign {
            captured_at: "today".to_owned(),
            tools: Vec::new(),
            cases: BTreeMap::from([("case".to_owned(), case)]),
        };
        let rendered = render_case(
            Some(&campaign),
            "case",
            &Tokenizers::new().expect("tokenizers"),
        );
        assert!(rendered.contains("Status: `output-limit`."));
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
            case.compact = Some(observed(json));
            case.expanded = Some(observed(json));
            case.toon = Some(observed(toon));
            assert!(eligible(&case).is_ok(), "{json}");
        }
        let mut case = example();
        case.compact = Some(observed("2\n4\n"));
        case.expanded = Some(observed("2\n4\n"));
        for toon in ["4\n2\n", "2\n", "2\n4\n6\n", "2\n4", "2\n4\ngarbage"] {
            case.toon = Some(observed(toon));
            assert!(eligible(&case).is_err(), "{toon}");
        }
        case.toon = Some(observed("2\n4\n"));
        case.expanded = Some(observed("4\n2\n"));
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
    fn display_commands_quote_queries_without_leaking_executable_paths() {
        assert_eq!(
            display_command(&[
                "/private/tmp/tq-benchmarks/bin/tq".into(),
                "--input-format".into(),
                "json".into(),
                ".items[] | select(.name == \"a b\")".into(),
            ]),
            "tq --input-format json '.items[] | select(.name == \"a b\")'"
        );
    }

    #[test]
    fn color_rendering_escapes_controls_but_keeps_raw_capture() {
        let mut case = example();
        case.output_mode = OutputMode::Color;
        case.compact = None;
        case.expanded = None;
        case.toon = None;
        case.scenario_captures = Some(BTreeMap::from([
            (BenchmarkTool::Jq, observed("\u{1b}[31m[1]\u{1b}[0m\n")),
            (BenchmarkTool::Yq, observed("\u{1b}[31m[1]\u{1b}[0m\n")),
            (BenchmarkTool::Tq, observed("\u{1b}[31m[1]\u{1b}[0m\n")),
        ]));
        let campaign = OutputCampaign {
            captured_at: "today".into(),
            tools: Vec::new(),
            cases: BTreeMap::from([("case".into(), case)]),
        };
        let text = render_case(Some(&campaign), "case", &Tokenizers::new().unwrap());
        assert!(text.contains("ANSI control bytes are escaped for display"));
        assert!(text.contains("\\x1b[31m[1]\\x1b[0m"));
        assert!(!text.contains('\u{1b}'));
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
