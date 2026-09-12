//! Reproducible JSON-equivalence and output-size report for manual examples.

use serde_json::Value;
use std::{env, fmt::Write as _, fs, path::PathBuf, process::ExitCode, time::Duration};
use tq_test_support::compatibility::{
    CompatibilityCatalog, ExecutableConfig, ManualReferencePin, ReviewedDisparity, ToolKind,
    compare_manual_with_disparities, discover_tool, load_catalog, manual_host_target,
    read_gap_inventory, read_manual_review_case_ids, summarize_manual_comparison,
    validate_completion_manual_report, validate_manual_reference, validate_manual_source_checkout,
    validate_manual_source_inventory, validate_strict_manual_report,
};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("tq-manual-compare: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = env::args().skip(1);
    let mut argument = None;
    let mut approval_path = None;
    let mut markdown_path = None;
    let mut source_root = None;
    let mut render_only = false;
    while let Some(value) = args.next() {
        match value.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: tq-manual-compare [--completion APPROVALS.toon] [--markdown-dir DIRECTORY] [--source-root COMPANION] [--render-only] [REPORT.toon]\nWrites TOON evidence and appends generated Results sections to the jq manual Markdown pages. Default exact mode fails on every difference. Completion mode accepts only explicit reviewed disparities from the approval file, retaining separate counts. Skips, missing evidence, timeouts, crashes, and unresolved failures always fail. --markdown-dir selects the manual document directory, defaulting to docs/tests/jq-manual. --render-only renders saved REPORT.toon observations without executing tools. --source-root additionally verifies the pinned companion manual files; fixture execution needs only the committed inventory."
                );
                return Ok(ExitCode::SUCCESS);
            }
            "--completion" => {
                if approval_path.is_some() {
                    return Err("--completion may be supplied only once".into());
                }
                approval_path = Some(PathBuf::from(
                    args.next()
                        .ok_or("--completion requires an approval file")?,
                ));
            }
            "--source-root" => {
                if source_root.is_some() {
                    return Err("--source-root may be supplied only once".into());
                }
                source_root = Some(PathBuf::from(
                    args.next()
                        .ok_or("--source-root requires a companion path")?,
                ));
            }
            "--render-only" => render_only = true,
            "--markdown-dir" => {
                if markdown_path.is_some() {
                    return Err("--markdown-dir may be supplied only once".into());
                }
                markdown_path = Some(PathBuf::from(
                    args.next()
                        .ok_or("--markdown-dir requires an output directory")?,
                ));
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}").into());
            }
            _ if argument.is_some() => return Err("expected at most one report path".into()),
            _ => argument = Some(value),
        }
    }
    let approvals: Vec<ReviewedDisparity> = approval_path
        .as_ref()
        .map(|path| tq_test_support::fixture_data::read(path))
        .transpose()?
        .unwrap_or_default();
    let destination = report_destination(argument, &root)?;
    let markdown_destination = markdown_path.unwrap_or_else(|| root.join("docs/tests/jq-manual"));
    let reviews = root.join("tests/compatibility/reviews/jq-manual");
    if render_only {
        let report: Value = tq_test_support::fixture_data::read(&destination)?;
        write_markdown_sections(&markdown_destination, &report, &reviews)?;
        return Ok(ExitCode::SUCCESS);
    }
    let config = ExecutableConfig::from_env();
    let pin = verify_reference_inputs(&root, &config, source_root.as_deref())?;
    let catalog = load_catalog(&root.join("tests/compatibility/cases"))?;
    let mut report = compare_manual_with_disparities(
        &catalog,
        &config,
        &root,
        Duration::from_secs(5),
        &approvals,
    )?;
    report["reference_pin"] = serde_json::json!({
        "source_inventory_sha256": pin.source_inventory_sha256,
        "target": manual_host_target(),
        "source_checkout_verified": source_root.is_some(),
    });
    write_reports(&destination, &report)?;
    write_markdown_sections(&markdown_destination, &report, &reviews)?;
    println!("{}", serde_json::to_string_pretty(&report["summary"])?);
    validate_written_report(
        &root,
        &report,
        &catalog,
        &approvals,
        approval_path.is_some(),
    )
}

fn validate_written_report(
    root: &std::path::Path,
    report: &Value,
    catalog: &CompatibilityCatalog,
    approvals: &[ReviewedDisparity],
    completion_requested: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let inventory =
        read_gap_inventory(&root.join("tests/compatibility/reviews/jq-manual/gap-inventory.toon"))?;
    let review_case_ids =
        read_manual_review_case_ids(&root.join("tests/compatibility/reviews/jq-manual"))?;
    let validation = if completion_requested {
        validate_completion_manual_report(report, catalog, &inventory, &review_case_ids, approvals)
    } else {
        validate_strict_manual_report(report, catalog, &inventory, &review_case_ids)
    };
    if let Err(error) = validation {
        eprintln!("{error}");
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

fn write_reports(
    destination: &std::path::Path,
    report: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let path = destination;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
    }
    let report_bytes = if destination.extension().is_some_and(|ext| ext == "toon") {
        tq_test_support::fixture_data::to_toon(report)?
    } else {
        format!("{}\n", serde_json::to_string_pretty(report)?)
    };
    fs::write(destination, report_bytes)?;
    Ok(())
}

fn verify_reference_inputs(
    root: &std::path::Path,
    config: &ExecutableConfig,
    source_root: Option<&std::path::Path>,
) -> Result<ManualReferencePin, Box<dyn std::error::Error>> {
    let pin: ManualReferencePin = tq_test_support::fixture_data::read(
        &root.join("tests/compatibility/reviews/jq-manual/reference-pin.toon"),
    )?;
    let bytes = fs::read(root.join("tests/compatibility/reviews/jq-manual/source-examples.toon"))?;
    validate_manual_source_inventory(&pin, &bytes)?;
    if let Some(source_root) = source_root {
        validate_manual_source_checkout(&pin, source_root)?;
    }
    let reference = discover_tool(ToolKind::Jq, config, root)?.ok_or("jq is unavailable")?;
    validate_manual_reference(&pin, &reference, &manual_host_target())?;
    Ok(pin)
}

fn report_destination(
    argument: Option<String>,
    root: &std::path::Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let destination = argument.map_or_else(|| root.join("target/comparison.toon"), PathBuf::from);
    if destination
        .extension()
        .is_none_or(|extension| extension != "json" && extension != "toon")
    {
        return Err("report path must end in .toon or .json".into());
    }
    Ok(destination)
}

fn section_reports(
    report: &Value,
    reviews: &std::path::Path,
) -> Result<std::collections::BTreeMap<String, Value>, Box<dyn std::error::Error>> {
    use std::collections::{BTreeMap, BTreeSet};
    let rows = report["cases"].as_array().ok_or("missing cases")?;
    let mut sections = BTreeMap::new();
    let mut covered = BTreeSet::new();
    let completeness =
        tq_test_support::compatibility::read_manual_ledger(&reviews.join("completeness.toon"))?;
    for entry in fs::read_dir(reviews)? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "toon") {
            continue;
        }
        let ledger = tq_test_support::compatibility::read_manual_ledger(&path)?;
        let mut ids = BTreeSet::new();
        for field in ["examples", "coverage_notes", "coverage_evidence"] {
            for row in ledger[field].as_array().into_iter().flatten() {
                for key in ["case_id", "evidence_case_id"] {
                    if let Some(id) = row[key].as_str() {
                        ids.insert(id.to_owned());
                    }
                }
            }
        }
        for requirement in completeness["requirements"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if requirement["evidence_kind"] == "catalog-case"
                && requirement["behavior_ledger"]
                    .as_str()
                    .is_some_and(|name| Some(std::ffi::OsStr::new(name)) == path.file_name())
                && let Some(id) = requirement["evidence_ref"].as_str()
            {
                ids.insert(id.to_owned());
            }
        }
        if ids.is_empty() {
            continue;
        }
        let selected = rows
            .iter()
            .filter(|row| row["id"].as_str().is_some_and(|id| ids.contains(id)))
            .cloned()
            .collect::<Vec<_>>();
        covered.extend(
            selected
                .iter()
                .filter_map(|row| row["id"].as_str())
                .map(str::to_owned),
        );
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or("invalid section name")?;
        let mut section = report.clone();
        section["cases"] = Value::Array(selected);
        summarize_manual_comparison(&mut section)?;
        sections.insert(name.to_owned(), section);
    }
    let missing = rows
        .iter()
        .filter_map(|row| row["id"].as_str())
        .filter(|id| !covered.contains(*id))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "report cases lack a section mapping: {}",
            missing.join(", ")
        )
        .into());
    }
    Ok(sections)
}

const GENERATED_START: &str = "<!-- tq-manual-compare:begin";
const GENERATED_END: &str = "<!-- tq-manual-compare:end -->";

fn write_markdown_sections(
    destination: &std::path::Path,
    report: &Value,
    reviews: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let sections = section_reports(report, reviews)?;
    let generated_at = jiff::Timestamp::now().to_string();
    fs::create_dir_all(destination)?;

    for (name, section) in &sections {
        let path = destination.join(format!("{name}.md"));
        let document = read_optional(&path)?.unwrap_or_else(|| {
            let metadata = report_metadata(
                &format!("jq manual: {name} token comparison"),
                &generated_at,
            )
            .expect("report metadata");
            format!("{metadata}# jq manual: {name} token comparison\n")
        });
        let generated = generated_results(section, name)?;
        fs::write(path, replace_generated(&document, name, &generated)?)?;
    }

    let mut overview = render(report)?;
    let results_start = overview
        .find("## Results\n")
        .ok_or("missing results heading")?;
    overview.drain(..results_start);
    let cases_start = overview
        .find("\n## Cases\n")
        .ok_or("missing cases heading")?;
    overview.truncate(cases_start);
    overview = nest_generated_results(&overview);
    overview.push_str("\n### Sections\n\nEach page uses the matching case collection in tests/compatibility/reviews/jq-manual, including witnesses assigned to that collection by completeness.toon. Cases linked by multiple collections appear in each page; the totals above count each case once. Inventory and execution metadata do not create case pages.\n\n");
    for (position, (name, section)) in sections.iter().enumerate() {
        writeln!(
            overview,
            "{}. [{name}]({name}.md): {} cases",
            position + 1,
            section["summary"]["cases"]
        )?;
    }
    overview.push_str("\n### Regenerate\n\nRun from the repository root. The comparison binary runs separately from the default test suite.\n\n    cargo run -p tq-test-support --bin tq-manual-compare -- target/manual-comparison.toon\n\nTo render the saved observations without running the tools again:\n\n    cargo run -p tq-test-support --bin tq-manual-compare -- --render-only target/manual-comparison.toon\n");

    let index_path = destination.join("index.md");
    let index =
        read_optional(&index_path)?.unwrap_or_else(|| "# jq manual test reviews\n".to_owned());
    fs::write(
        index_path,
        replace_generated(&index, "index", overview.trim_end())?,
    )?;
    Ok(())
}

fn generated_results(report: &Value, section: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut markdown = render(report)?;
    let results_start = markdown
        .find("## Results\n")
        .ok_or("missing results heading")?;
    markdown.drain(..results_start);
    markdown = nest_generated_results(&markdown);
    Ok(format!(
        "## Results\n\n[Case collection](../../../tests/compatibility/reviews/jq-manual/{section}.toon)\n\n{}",
        markdown
            .strip_prefix("## Results\n")
            .ok_or("generated results missing heading")?,
    ))
}

fn nest_generated_results(markdown: &str) -> String {
    let mut output = String::with_capacity(markdown.len());
    let mut in_fence = false;
    let mut saw_results = false;
    for line in markdown.lines() {
        let transformed = if line.starts_with("```") {
            in_fence = !in_fence;
            line.to_owned()
        } else if !in_fence && line == "## Results" && !saw_results {
            saw_results = true;
            line.to_owned()
        } else if !in_fence && (line.starts_with("## ") || line.starts_with("### ")) {
            format!("#{line}")
        } else {
            line.to_owned()
        };
        output.push_str(&transformed);
        output.push('\n');
    }
    output
}

fn read_optional(path: &std::path::Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match fs::read_to_string(path) {
        Ok(document) => Ok(Some(document)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn replace_generated(
    document: &str,
    section: &str,
    generated: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let starts = document.matches(GENERATED_START).count();
    let ends = document.matches(GENERATED_END).count();
    if starts > 1 || ends > 1 || starts != ends {
        return Err(format!("malformed generated markers in {section}.md").into());
    }
    let block = format!("{GENERATED_START} section={section} -->\n{generated}\n{GENERATED_END}");
    let document = document.trim_end();
    let (prefix, suffix) = if let Some(start) = document.find(GENERATED_START) {
        let end = start
            + document[start..]
                .find(GENERATED_END)
                .ok_or("generated end marker not found")?
            + GENERATED_END.len();
        (&document[..start], &document[end..])
    } else {
        (document, "")
    };
    let prefix = prefix.trim_end();
    let suffix = if suffix.is_empty() {
        "\n".to_owned()
    } else if suffix.starts_with('\n') {
        suffix.to_owned()
    } else {
        format!("\n\n{suffix}")
    };
    Ok(format!("{prefix}\n\n{block}{suffix}"))
}

fn report_metadata(title: &str, generated_at: &str) -> Result<String, serde_json::Error> {
    let actor = format!("tq-manual-compare/{}", env!("CARGO_PKG_VERSION"));
    Ok(format!(
        "---\ntype: Report\ntitle: {}\ndescription: Generated jq and tq output comparisons and token counts.\ngenerated: {{ by: {}, at: {} }}\n---\n\n",
        serde_json::to_string(title)?,
        serde_json::to_string(&actor)?,
        serde_json::to_string(generated_at)?,
    ))
}

fn render(report: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let summary = &report["summary"];
    let sizes = summary["tokens"]
        .as_object()
        .ok_or("missing token totals")?;
    let mut output = String::from("# jq manual JSON compatibility and output size\n\n");
    writeln!(
        output,
        "{} cases. {}\n",
        summary["cases"],
        report["method"].as_str().ok_or("missing method")?
    )?;
    output.push_str("## Results\n\n| Verdict | Cases |\n| --- | ---: |\n");
    for (verdict, count) in summary["verdicts"].as_object().ok_or("missing verdicts")? {
        writeln!(output, "| {verdict} | {count} |")?;
    }
    if let Some(campaigns) = summary["encoding_campaigns"].as_object() {
        output.push_str("\nIndependent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.\n\n| Output campaign | Matches | Cases |\n| --- | ---: | ---: |\n");
        for (name, counts) in campaigns {
            writeln!(
                output,
                "| {name} | {} | {} |",
                counts["matches"], counts["cases"]
            )?;
        }
    }
    output.push_str("\nA match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.\n\nMissing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.\n\n");
    output.push_str("JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.\n\n");
    output.push_str("## Output size\n\n");
    writeln!(
        output,
        "{} eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.\n\n| Tokenizer | JSON tokens | TOON tokens | Diff | % |\n| --- | ---: | ---: | ---: | ---: |",
        summary["size_samples"]
    )?;
    for encoding in ["o200k_base", "cl100k_base"] {
        let counts = sizes.get(encoding).ok_or("missing tokenizer totals")?;
        writeln!(
            output,
            "| `{encoding}` | {} | {} | {} | {} |",
            counts["json"],
            counts["toon"],
            counts["diff"],
            format_percent(counts["diff_percent"].as_f64())
        )?;
    }
    output.push_str("\nOnly successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.\n\n");
    output.push_str(
        "## Reviewed disparities and historical differences\n\n| Case | Reason |\n| --- | --- |\n",
    );
    for row in report["cases"]
        .as_array()
        .ok_or("missing cases")?
        .iter()
        .filter(|row| row["verdict"] == "expected-difference" || row["verdict"] == "disparity")
    {
        writeln!(
            output,
            "| `{}` | {} |",
            row["id"].as_str().ok_or("missing ID")?,
            row["reason"].as_str().ok_or("missing reason")?
        )?;
    }
    output.push_str("\n## Cases\n\nEach case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.\n\n");
    for row in report["cases"].as_array().ok_or("missing cases")? {
        let id = row["id"].as_str().ok_or("missing ID")?;
        writeln!(output, "### {id}\n")?;
        writeln!(output, "{}\n", markdown_code_block(&markdown_case(row)))?;
        let jq_label = output_label(row, "jq", "jq", "jq --seq");
        let json_label = output_label(row, "tq", "tq -o json", "tq -o json --seq");
        let toon_label = output_label(row, "tq_toon", "tq", "tq --seq");
        writeln!(
            output,
            "| Tokenizer | {jq_label} | {json_label} | {toon_label} | Diff | % |\n| --- | ---: | ---: | ---: | ---: | ---: |"
        )?;
        for encoding in ["o200k_base", "cl100k_base"] {
            let counts = row["tokens"].get(encoding);
            let jq = counts.and_then(|value| value["jq"].as_u64());
            let json = counts.and_then(|value| value["json"].as_u64());
            let toon = counts.and_then(|value| value["toon"].as_u64());
            let diff = toon
                .zip(json)
                .map(|(toon, json)| i128::from(toon) - i128::from(json));
            let diff_percent = counts.and_then(|value| value["diff_percent"].as_f64());
            writeln!(
                output,
                "| `{encoding}` | {} | {} | {} | {} | {} |",
                format_count(jq),
                format_count(json),
                format_count(toon),
                format_diff(diff),
                format_percent(diff_percent)
            )?;
        }
        output.push('\n');
    }
    output.pop();
    Ok(output)
}

fn markdown_case(row: &Value) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "# input\n{}\n",
        row["input"].as_str().unwrap_or("jq")
    )
    .expect("write input invocation");
    output.push_str(&markdown_outputs(row));
    output
}

fn markdown_outputs(row: &Value) -> String {
    let mut output = String::new();
    let outputs = [
        (
            if output_is_sequence(row, "jq") {
                "jq --seq"
            } else {
                "jq"
            },
            "jq",
        ),
        (
            if output_is_sequence(row, "tq") {
                "tq -o json --seq"
            } else {
                "tq -o json"
            },
            "tq",
        ),
        (
            if output_is_sequence(row, "tq_toon") {
                "tq --seq"
            } else {
                "tq"
            },
            "tq_toon",
        ),
    ];
    for (index, (label, key)) in outputs.into_iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        writeln!(output, "# {label}").expect("write output label");
        output.push_str(&render_output(row, key));
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }
    output
}

fn output_is_sequence(row: &Value, key: &str) -> bool {
    row[key]["stdout_hex"]
        .as_str()
        .is_some_and(|hex| hex.starts_with("1e"))
}

fn output_label<'a>(row: &Value, key: &str, plain: &'a str, sequence: &'a str) -> &'a str {
    if output_is_sequence(row, key) {
        sequence
    } else {
        plain
    }
}

fn render_output(row: &Value, key: &str) -> String {
    let observation = &row[key];
    let Some(object) = observation.as_object() else {
        return "<not run>".to_owned();
    };
    let mut output = object
        .get("stdout_hex")
        .and_then(Value::as_str)
        .and_then(decode_hex)
        .map_or_else(String::new, |bytes| display_bytes(&bytes));
    if let Some(stderr) = object
        .get("stderr_hex")
        .filter(|value| !value.is_null())
        .and_then(Value::as_str)
        .and_then(decode_hex)
    {
        output.push_str("\n\n[stderr]\n");
        output.push_str(&display_bytes(&stderr));
    }
    output
}

fn markdown_code_block(text: &str) -> String {
    let fence_length = text
        .lines()
        .map(|line| {
            line.chars()
                .take_while(|character| *character == '`')
                .count()
        })
        .max()
        .unwrap_or(0)
        .max(2)
        + 1;
    let fence = "`".repeat(fence_length);
    let separator = if text.ends_with('\n') { "" } else { "\n" };
    format!("{fence}\n{text}{separator}{fence}")
}

fn format_count(count: Option<u64>) -> String {
    count.map_or_else(|| "n/a".to_owned(), |count| count.to_string())
}

fn format_diff(diff: Option<i128>) -> String {
    diff.map_or_else(
        || "n/a".to_owned(),
        |diff| {
            if diff > 0 {
                format!("+{diff}")
            } else {
                diff.to_string()
            }
        },
    )
}

fn format_percent(percent: Option<f64>) -> String {
    let Some(percent) = percent else {
        return "n/a".to_owned();
    };
    let rounded = if percent.abs() < 0.005 { 0.0 } else { percent };
    let formatted = format!("{rounded:+.2}");
    format!("{}%", formatted.trim_end_matches('0').trim_end_matches('.'))
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(hex.get(index..index + 2)?, 16).ok())
        .collect()
}

fn display_bytes(bytes: &[u8]) -> String {
    let Ok(text) = String::from_utf8(bytes.to_vec()) else {
        return bytes.iter().fold(
            String::with_capacity(bytes.len() * 4),
            |mut output, byte| {
                write!(output, "\\x{byte:02x}").expect("write byte escape");
                output
            },
        );
    };
    let mut output = String::new();
    for character in text.chars() {
        match character {
            '\n' | '\r' | '\t' => output.push(character),
            character if character.is_control() => {
                write!(output, "\\x{:02x}", character as u32).expect("write control byte");
            }
            character => output.push(character),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[derive(serde::Deserialize)]
    struct GeneratedMetadata {
        by: String,
        at: Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct ReportMetadata {
        title: String,
        generated: GeneratedMetadata,
    }

    fn parse_frontmatter(document: &str) -> (ReportMetadata, &str) {
        let document = document.strip_prefix("---\n").expect("frontmatter start");
        let (frontmatter, body) = document.split_once("\n---\n").expect("frontmatter end");
        (
            yaml_serde::from_str(frontmatter).expect("valid frontmatter"),
            body,
        )
    }

    #[test]
    fn report_metadata_escapes_title_and_records_versioned_timestamp() {
        let title = r#"section: \"quoted\" \\ escaped"#;
        let timestamp = "2026-09-10T03:30:00Z";
        let document = super::report_metadata(title, timestamp).expect("metadata");
        let (metadata, body) = parse_frontmatter(&document);

        assert_eq!(metadata.title, title);
        assert_eq!(
            metadata.generated.by,
            format!("tq-manual-compare/{}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(metadata.generated.at.as_deref(), Some(timestamp));
        assert!(timestamp.parse::<jiff::Timestamp>().is_ok());
        assert_eq!(body, "\n");
    }

    #[test]
    fn temporary_sections_preserve_authored_content_and_render_idempotently() {
        let report = tiny_report();
        let reviews = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let section = json!({
            "examples": [{"case_id": "sample.result"}, {"case_id": "sample.raw"}],
            "coverage_notes": [],
            "coverage_evidence": [],
        });
        let completeness = json!({"requirements": []});
        std::fs::write(
            reviews.path().join("sample.toon"),
            tq_test_support::fixture_data::to_toon(&section).unwrap(),
        )
        .unwrap();
        std::fs::write(
            reviews.path().join("completeness.toon"),
            tq_test_support::fixture_data::to_toon(&completeness).unwrap(),
        )
        .unwrap();
        std::fs::write(output.path().join("sample.md"), "# authored sample\n\n").unwrap();
        std::fs::write(output.path().join("index.md"), "# authored index\n\n").unwrap();

        super::write_markdown_sections(output.path(), &report, reviews.path()).unwrap();
        let first = read_documents(output.path());
        assert!(
            String::from_utf8_lossy(first.get("sample.md").unwrap())
                .starts_with("# authored sample\n\n")
        );
        let sample = String::from_utf8_lossy(first.get("sample.md").unwrap());
        assert!(sample.contains("### sample.result"));
        assert!(sample.contains("### sample.raw"));
        assert!(sample.contains("1\n"));
        assert!(sample.contains("raw\n"));

        let index = String::from_utf8_lossy(first.get("index.md").unwrap());
        assert!(index.starts_with("# authored index\n\n"));
        assert!(index.contains("1. [sample](sample.md): 2 cases"));
        assert!(index.contains("target/manual-comparison.toon"));
        assert!(index.contains("| Tokenizer | JSON tokens | TOON tokens | Diff | % |"));
        assert!(!index.contains("tokens/"));
        assert_eq!(first.len(), 2, "render one section and its index");
        assert!(!output.path().join("tokens").exists());
        assert!(index.contains("<!-- tq-manual-compare:begin section=index -->"));
        assert!(index.ends_with("<!-- tq-manual-compare:end -->\n"));
        assert_eq!(super::format_percent(Some(10.0)), "+10%");
        assert_eq!(super::format_percent(Some(-20.0)), "-20%");

        super::write_markdown_sections(output.path(), &report, reviews.path()).unwrap();
        assert_eq!(
            first,
            read_documents(output.path()),
            "rendering is not idempotent"
        );
    }

    fn read_documents(path: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(path)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    std::fs::read(entry.path()).unwrap(),
                )
            })
            .collect()
    }

    fn tiny_report() -> serde_json::Value {
        let observation = |tool: &str, stdout: &str, raw: bool, results: serde_json::Value| {
            json!({
                "tool": tool,
                "input_format": "json",
                "state": "executed",
                "results": results,
                "stdout_hex": tq_test_support::compatibility::encode_hex(stdout.as_bytes()),
                "raw_stdout_hex": raw.then(|| tq_test_support::compatibility::encode_hex(stdout.as_bytes())),
                "stderr_hex": null,
                "process_status": "exited",
                "exit_code": 0,
                "error_class": null,
            })
        };
        let reference = observation("jq", "1\n", false, json!([1]));
        let candidate = observation("tq", "1\n", false, json!([1]));
        let result = json!({
            "id": "sample.result",
            "input": "jq .",
            "contract": "result-sequence",
            "verdict": "match",
            "reason": "synthetic",
            "json_equivalent": true,
            "toon_sequence": false,
            "toon_equivalent": true,
            "tokens": {"o200k_base": {"json": 1, "toon": 1}, "cl100k_base": {"json": 1, "toon": 1}},
            "toon_contract_match": true,
            "compact": {"exact": true, "jq": reference, "tq": candidate, "differences": []},
            "differences": [],
            "jq": reference,
            "tq": candidate,
            "tq_toon": observation("tq", "1\n", false, json!([1])),
        });
        let reference = observation("jq", "raw\n", true, json!([]));
        let candidate = observation("tq", "raw\n", true, json!([]));
        let raw = json!({
            "id": "sample.raw",
            "input": "jq --version",
            "contract": "raw-bytes",
            "verdict": "match",
            "reason": "synthetic",
            "json_equivalent": false,
            "toon_sequence": false,
            "toon_equivalent": false,
            "tokens": null,
            "toon_contract_match": null,
            "compact": null,
            "differences": [],
            "jq": reference,
            "tq": candidate,
            "tq_toon": null,
        });
        let mut report = json!({
            "schema_version": 1,
            "method": "synthetic",
            "cases": [result, raw],
        });
        tq_test_support::compatibility::summarize_manual_comparison(&mut report).unwrap();
        report
    }
}
