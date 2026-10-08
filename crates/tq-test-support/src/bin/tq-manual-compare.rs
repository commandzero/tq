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
    report["generated_at"] = Value::String(jiff::Timestamp::now().to_string());
    report["reference_pin"] = serde_json::json!({
        "source_inventory_sha256": pin.source_inventory_sha256,
        "target": manual_host_target(),
        "source_checkout_verified": source_root.is_some(),
        "execution": report["reference_execution"],
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
    let generated_at = report["generated_at"].as_str();
    validate_tool_metadata(report)?;

    let mut authored = Vec::new();
    for name in sections.keys() {
        let path = destination.join(format!("{name}.md"));
        let document = match read_optional(&path)? {
            Some(document) => document,
            None => format!(
                "{}# jq manual: {name} token comparison\n",
                report_metadata(
                    &format!("jq manual: {name} token comparison"),
                    generated_at,
                    report,
                )?
            ),
        };
        authored.push((path, bind_benchmark_runs(&document, report)?));
    }
    let index_path = destination.join("index.md");
    let index =
        read_optional(&index_path)?.unwrap_or_else(|| "# jq manual test reviews\n".to_owned());
    if index.starts_with("---\n") {
        return Err("index.md navigation must not have frontmatter".into());
    }
    replace_generated(&index, "index", "")?;
    let overview_path = destination.join("overview.md");
    let overview_document = match read_optional(&overview_path)? {
        Some(document) => document,
        None => format!(
            "{}# jq manual comparison overview\n",
            report_metadata("jq manual comparison overview", generated_at, report)?
        ),
    };
    let overview_document = bind_benchmark_runs(&overview_document, report)?;
    replace_generated(&overview_document, "overview", "")?;
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
            "{}. [{name}]({name}.md): {} case{}",
            position + 1,
            section["summary"]["cases"],
            plural_suffix(&section["summary"]["cases"])
        )?;
    }
    overview.push_str("\n### Regenerate\n\nRun from the repository root. The comparison binary runs separately from the default test suite.\n\n    cargo run -p tq-test-support --bin tq-manual-compare -- target/manual-comparison.toon\n\nTo render the saved observations without running the tools again:\n\n    cargo run -p tq-test-support --bin tq-manual-compare -- --render-only target/manual-comparison.toon\n");
    let overview = replace_generated(&overview_document, "overview", overview.trim_end())?;
    let index = if index.contains("(overview.md)") {
        index
    } else {
        format!(
            "{}\n\n[Measured comparison overview](overview.md).\n",
            index.trim_end()
        )
    };

    // Prepare and validate every document before writing any page so malformed
    // frontmatter or markers cannot leave the bundle partially updated.
    let mut pending = Vec::new();
    for ((name, section), (path, document)) in sections.iter().zip(authored) {
        let generated = generated_results(section, name, destination, reviews)?;
        pending.push((path, replace_generated(&document, name, &generated)?));
    }
    pending.push((index_path, index));
    pending.push((overview_path, overview));
    fs::create_dir_all(destination)?;
    for (path, document) in pending {
        fs::write(path, document)?;
    }
    Ok(())
}

fn validate_tool_metadata(report: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let tools = report["tools"]
        .as_array()
        .filter(|tools| !tools.is_empty())
        .ok_or("report lacks measured tools")?;
    for tool in tools {
        if tool["tool"].as_str().is_none_or(str::is_empty) {
            return Err("measured tool lacks name".into());
        }
        if tool["version"]
            .as_str()
            .is_none_or(|version| version.trim().is_empty())
        {
            return Err("measured tool lacks version".into());
        }
        if tool["executable"]["sha256"].as_str().is_none_or(|sha256| {
            sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err("measured tool lacks valid SHA-256 executable identity".into());
        }
    }
    Ok(())
}

fn bind_benchmark_runs(
    document: &str,
    report: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    validate_tool_metadata(report)?;
    let generated_at = report["generated_at"].as_str();
    let (mut value, original_body) = if document.starts_with("---\n") {
        (parse_frontmatter(document)?, None)
    } else {
        let initial = report_metadata("jq manual test reviews", generated_at, report)?;
        let frontmatter = initial
            .strip_prefix("---\n")
            .ok_or("missing generated frontmatter start")?
            .split_once("\n---\n")
            .ok_or("missing generated frontmatter end")?
            .0;
        (yaml_serde::from_str::<Value>(frontmatter)?, Some(document))
    };
    let mut binaries = serde_json::Map::new();
    for name in ["tq", "jq", "yq", "helper"] {
        binaries.insert(
            name.to_owned(),
            serde_json::json!({
                "version": null,
                "sha256": null,
                "identity_status": "not-recorded"
            }),
        );
    }
    for tool in report["tools"].as_array().expect("validated tools") {
        let name = tool["tool"].as_str().expect("validated name");
        binaries.insert(
            name.to_owned(),
            serde_json::json!({
                "version": tool["version"],
                "sha256": tool["executable"]["sha256"],
                "identity_status": "measured"
            }),
        );
    }
    let tools = report["tools"].as_array().expect("validated tools");
    let runs = value
        .as_object_mut()
        .ok_or("frontmatter must be a mapping")?
        .entry("benchmark_runs")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or("benchmark_runs must be a sequence")?;
    let existing = runs.iter_mut().find(|run| {
        run["campaign_id"] == report["campaign_id"]
            && run["captured_at"].as_str() == generated_at
            && tools.iter().all(|tool| {
                let name = tool["tool"].as_str().expect("validated name");
                run["binaries"][name]["version"] == tool["version"]
                    && run["binaries"][name]["sha256"] == tool["executable"]["sha256"]
            })
    });
    if let Some(existing) = existing {
        let existing_binaries = existing["binaries"]
            .as_object_mut()
            .ok_or("benchmark binaries must be a mapping")?;
        for (name, binary) in binaries {
            if binary["identity_status"] == "measured" {
                if let Some(Value::Object(fields)) = existing_binaries.get_mut(&name) {
                    let Value::Object(current) = binary else {
                        unreachable!("constructed binary mapping")
                    };
                    fields.extend(current);
                } else {
                    existing_binaries.insert(name, binary);
                }
            } else {
                existing_binaries.entry(name).or_insert(binary);
            }
        }
    } else {
        runs.push(serde_json::json!({
            "campaign_id": report["campaign_id"],
            "captured_at": generated_at,
            "binaries": binaries
        }));
    }
    if let Some(body) = original_body {
        let mut output = String::from("---\n");
        for (key, item) in value.as_object().ok_or("frontmatter must be a mapping")? {
            writeln!(
                output,
                "{}: {}",
                serde_json::to_string(key)?,
                serde_json::to_string(item)?
            )?;
        }
        output.push_str("---\n");
        output.push_str(body);
        Ok(output)
    } else {
        serialize_frontmatter(&value, document)
    }
}

fn generated_results(
    report: &Value,
    section: &str,
    destination: &std::path::Path,
    reviews: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut markdown = render(report)?;
    let results_start = markdown
        .find("## Results\n")
        .ok_or("missing results heading")?;
    markdown.drain(..results_start);
    markdown = nest_generated_results(&markdown);
    let collection = relative_markdown_link(destination, &reviews.join(format!("{section}.toon")))?;
    Ok(format!(
        "## Results\n\n[Case collection]({collection})\n\n{}",
        markdown
            .strip_prefix("## Results\n")
            .ok_or("generated results missing heading")?,
    ))
}

fn relative_markdown_link(
    from_directory: &std::path::Path,
    target: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let from = fs::canonicalize(from_directory)?;
    let target = fs::canonicalize(target)?;
    let from_components = from.components().collect::<Vec<_>>();
    let target_components = target.components().collect::<Vec<_>>();
    let common = from_components
        .iter()
        .zip(&target_components)
        .take_while(|(from, target)| from == target)
        .count();

    if common == 0 {
        return Ok(markdown_path(&target));
    }

    let mut relative = PathBuf::new();
    for _ in from_components.iter().skip(common) {
        relative.push("..");
    }
    for component in target_components.iter().skip(common) {
        relative.push(component.as_os_str());
    }
    Ok(markdown_path(&relative))
}

fn markdown_path(path: &std::path::Path) -> String {
    let path = {
        #[cfg(windows)]
        {
            path.to_string_lossy().replace('\\', "/")
        }
        #[cfg(not(windows))]
        {
            path.to_string_lossy().into_owned()
        }
    };
    let mut escaped = String::with_capacity(path.len());
    for character in path.chars() {
        match character {
            '%' => escaped.push_str("%25"),
            '#' => escaped.push_str("%23"),
            '?' => escaped.push_str("%3F"),
            ' ' => escaped.push_str("%20"),
            '\t' => escaped.push_str("%09"),
            '\n' => escaped.push_str("%0A"),
            '\r' => escaped.push_str("%0D"),
            '(' => escaped.push_str("%28"),
            ')' => escaped.push_str("%29"),
            '<' => escaped.push_str("%3C"),
            '>' => escaped.push_str("%3E"),
            '\\' => escaped.push_str("%5C"),
            character if character.is_whitespace() => {
                for byte in character.to_string().as_bytes() {
                    write!(escaped, "%{byte:02X}").expect("write URL escape");
                }
            }
            character => escaped.push(character),
        }
    }
    escaped
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

fn report_metadata(
    title: &str,
    generated_at: Option<&str>,
    report: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let actor = format!("tq-manual-compare/{}", env!("CARGO_PKG_VERSION"));
    let generated = match generated_at {
        Some(timestamp) => format!(
            "{{ by: {}, at: {} }}",
            serde_json::to_string(&actor)?,
            serde_json::to_string(timestamp)?
        ),
        None => format!("{{ by: {} }}", serde_json::to_string(&actor)?),
    };
    let base = format!(
        "---\ntype: Report\ntitle: {}\ndescription: Generated jq and tq output comparisons and token counts.\ngenerated: {generated}\n---\n\n",
        serde_json::to_string(title)?,
    );
    bind_benchmark_runs(&base, report)
}

fn parse_frontmatter(document: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let document = document
        .strip_prefix("---\n")
        .ok_or("missing frontmatter start")?;
    let (frontmatter, _) = document
        .split_once("\n---\n")
        .ok_or("missing frontmatter end")?;
    let metadata: Value = yaml_serde::from_str(frontmatter)?;
    if !metadata.is_object() {
        return Err("frontmatter must be a mapping".into());
    }
    Ok(metadata)
}

fn serialize_frontmatter(
    metadata: &Value,
    document: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let (_, body) = document
        .strip_prefix("---\n")
        .ok_or("missing frontmatter start")?
        .split_once("\n---\n")
        .ok_or("missing frontmatter end")?;
    let mut output = String::from("---\n");
    for (key, item) in metadata
        .as_object()
        .ok_or("frontmatter must be a mapping")?
    {
        let key = serde_json::to_string(key)?;
        let scalar = match item {
            Value::String(text) => serde_json::to_string(text)?,
            _ => serde_json::to_string(item)?,
        };
        writeln!(output, "{key}: {scalar}")?;
    }
    output.push_str("---\n");
    output.push_str(body);
    Ok(output)
}

fn plural_suffix(count: &Value) -> &'static str {
    if count.as_u64() == Some(1) { "" } else { "s" }
}

fn render(report: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let summary = &report["summary"];
    let sizes = summary["tokens"]
        .as_object()
        .ok_or("missing token totals")?;
    let mut output = String::from("# jq manual JSON compatibility and output size\n\n");
    writeln!(
        output,
        "{} case{}. {}\n",
        summary["cases"],
        plural_suffix(&summary["cases"]),
        report["method"].as_str().ok_or("missing method")?
    )?;
    output.push_str("## Results\n\n| Verdict | Cases |\n| --- | ---: |\n");
    for (verdict, count) in summary["verdicts"].as_object().ok_or("missing verdicts")? {
        let label = match verdict.as_str() {
            "match" => "Exact match",
            "failure" => "Differences",
            other => other,
        };
        writeln!(output, "| {label} | {count} |")?;
    }
    if let Some(campaigns) = summary["encoding_campaigns"].as_object() {
        output.push_str("\nIndependent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.\n\n| Output campaign | Exact matches | Cases |\n| --- | ---: | ---: |\n");
        for (name, counts) in campaigns {
            writeln!(
                output,
                "| {name} | {} | {} |",
                counts["matches"], counts["cases"]
            )?;
        }
    }
    output.push_str("\nAn exact match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from exact matches. Historical expected-difference labels do not pass either gate.\n\nDifferences include missing features and unaccepted mismatches; these still fail the strict and completion gates. Reference discrepancies describe errors in the imported manual, not successful compatibility.\n\n");
    output.push_str("JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.\n\n");
    output.push_str("## Output size\n\n");
    writeln!(
        output,
        "{} eligible example{}. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; TOON sequence captures, when available, are shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.\n\n| Tokenizer | JSON tokens | TOON tokens | Diff | % |\n| --- | ---: | ---: | ---: | ---: |",
        summary["size_samples"],
        plural_suffix(&summary["size_samples"])
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
    render_differences(&mut output, report)?;
    output.push_str("\n## Cases\n\nEach case shows the original jq invocation and the available jq, tq JSON, and tq TOON output captures. Missing captures are marked `<not run>`; these placeholders are not execution evidence. Control bytes use `\\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.\n\n");
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

fn render_differences(
    output: &mut String,
    report: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    output.push_str("## Differences\n\n");
    let mut differences = report["cases"]
        .as_array()
        .ok_or("missing cases")?
        .iter()
        .filter(|row| row["verdict"] != "match")
        .peekable();
    if differences.peek().is_none() {
        output.push_str("No differences.\n");
    }
    for (position, row) in differences.enumerate() {
        writeln!(
            output,
            "{}. `{}`: {}",
            position + 1,
            row["id"].as_str().ok_or("missing ID")?,
            difference_summary(row).ok_or("missing reason")?
        )?;
    }
    Ok(())
}

fn difference_summary(row: &Value) -> Option<&str> {
    // The producer verifies presentation notes against the captured observations.
    if let Some(note) = row["presentation_note"].as_str() {
        return Some(
            if note.contains("JSON data and process behavior agree; ANSI styling differs") {
                "Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged."
            } else {
                "Expected presentation difference: ANSI styling differs."
            },
        );
    }
    if clean_observation(&row["jq"]) && clean_observation(&row["tq"]) {
        let known_math = match row["id"].as_str() {
            Some("manual.audit.math.erfc-ulp") => Some((
                "0.0046777349810472645",
                "0.004677734981047266",
                "Expected rounding difference: tq is 2 ULP higher for erfc(2).",
            )),
            Some("manual.audit.math.tgamma-ulp") => Some((
                "1.772453850905516",
                "1.7724538509055159",
                "Expected rounding difference: tq is 1 ULP lower for tgamma(0.5).",
            )),
            _ => None,
        };
        if let Some((jq, tq, summary)) = known_math
            && single_number_is(&row["jq"]["results"], jq)
            && single_number_is(&row["tq"]["results"], tq)
        {
            return Some(summary);
        }
        if row["id"] == "manual.invoking.run-tests"
            && let Some(jq) = row["jq"]["stdout_hex"].as_str().and_then(decode_hex)
            && let Some(tq) = row["tq"]["stdout_hex"].as_str().and_then(decode_hex)
            && jq.starts_with(&tq)
            && tq.starts_with(
                b"Test #1: '.' at line number 2\n1 of 1 tests passed (0 malformed, 0 skipped)\n",
            )
            && (jq[tq.len()..].starts_with(b"Test jq_") || jq[tq.len()..].starts_with(b"Test jq "))
        {
            return Some(
                "jq prints extra internal self-test messages; both tools pass the supplied test and exit 0.",
            );
        }
    }
    row["reason"].as_str()
}

fn clean_observation(observation: &Value) -> bool {
    observation["state"] == "executed"
        && observation["process_status"] == "exited"
        && observation["exit_code"] == 0
        && observation["error_class"].is_null()
        && (observation["stderr_hex"].is_null() || observation["stderr_hex"] == "")
}

fn single_number_is(results: &Value, expected: &str) -> bool {
    results.as_array().is_some_and(|values| {
        values.len() == 1
            && values[0]
                .as_number()
                .is_some_and(|number| number.to_string() == expected)
    })
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
        let mut report = tiny_report();
        report["generated_at"] = json!(timestamp);
        let document = super::report_metadata(title, Some(timestamp), &report).expect("metadata");
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
    fn legacy_report_without_timestamp_keeps_capture_time_unknown() {
        let mut report = tiny_report();
        report.as_object_mut().unwrap().remove("generated_at");
        let document = super::bind_benchmark_runs("# authored body\n", &report).unwrap();
        let (frontmatter, body) = document
            .strip_prefix("---\n")
            .unwrap()
            .split_once("\n---\n")
            .unwrap();
        let metadata: serde_json::Value = yaml_serde::from_str(frontmatter).unwrap();

        assert_eq!(metadata["generated"]["at"], serde_json::Value::Null);
        assert_eq!(
            metadata["benchmark_runs"][0]["captured_at"],
            serde_json::Value::Null
        );
        assert_eq!(body, "# authored body\n");
    }

    #[test]
    fn regeneration_preserves_other_captures_and_authored_provenance() {
        let report = tiny_report();
        let body = "# Synthetic authored content\n";
        let initial = super::bind_benchmark_runs(body, &report).unwrap();
        let mut metadata = super::parse_frontmatter(&initial).unwrap();
        let matched = &mut metadata["benchmark_runs"][0];
        matched["source_role"] = json!("authored");
        matched["binaries"]["tq"]["capture_note"] = json!("retained");
        matched["binaries"]["tq"]["identity_status"] = json!("not-recorded");
        let mut other = matched.clone();
        other["binaries"]["tq"]["sha256"] =
            json!("3333333333333333333333333333333333333333333333333333333333333333");
        metadata["benchmark_runs"]
            .as_array_mut()
            .unwrap()
            .push(other.clone());
        let source = format!(
            "---\ncustom: {{owner: authored}}\nbenchmark_runs: {}\n---\n{body}",
            metadata["benchmark_runs"]
        );
        let rendered = super::bind_benchmark_runs(&source, &report).unwrap();
        let actual = super::parse_frontmatter(&rendered).unwrap();
        let runs = actual["benchmark_runs"].as_array().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1], other);
        assert_eq!(
            runs[0]["source_role"],
            metadata["benchmark_runs"][0]["source_role"]
        );
        assert_eq!(
            runs[0]["binaries"]["tq"]["capture_note"],
            metadata["benchmark_runs"][0]["binaries"]["tq"]["capture_note"]
        );
        assert_eq!(runs[0]["binaries"]["tq"]["identity_status"], "measured");
        assert_eq!(
            actual["custom"],
            super::parse_frontmatter(&source).unwrap()["custom"]
        );
        assert!(rendered.ends_with(body));
        assert_eq!(
            super::bind_benchmark_runs(&rendered, &report).unwrap(),
            rendered
        );
        let mut distinct = report;
        distinct["generated_at"] = json!("2026-09-10T03:30:01Z");
        let appended = super::bind_benchmark_runs(&rendered, &distinct).unwrap();
        let appended = super::parse_frontmatter(&appended).unwrap();
        assert_eq!(appended["benchmark_runs"].as_array().unwrap().len(), 3);
        assert_eq!(appended["benchmark_runs"][0], runs[0]);
        assert_eq!(appended["benchmark_runs"][1], other);
    }

    #[test]
    fn malformed_captured_versions_and_digests_cannot_generate_provenance() {
        for (field, value) in [
            ("version", json!("")),
            ("version", json!(" \n")),
            ("executable", json!({})),
            ("executable", json!({"sha256": ""})),
            ("executable", json!({"sha256": "not-a-digest"})),
        ] {
            let mut report = tiny_report();
            report["tools"][0][field] = value;
            assert!(super::bind_benchmark_runs("# authored\n", &report).is_err());
        }
    }

    #[test]
    fn temporary_sections_preserve_authored_content_and_render_idempotently() {
        let mut report = tiny_report();
        report["cases"][1]["verdict"] = json!("failure");
        report["cases"][1]["reason"] = json!("CLI output differs.");
        tq_test_support::compatibility::summarize_manual_comparison(&mut report).unwrap();
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
        std::fs::write(
            output.path().join("sample.md"),
            "---\ntitle: authored sample\ncustom: preserved\n---\n# authored sample\n\n",
        )
        .unwrap();
        std::fs::write(output.path().join("index.md"), "# authored index\n\n").unwrap();
        std::fs::write(
            output.path().join("overview.md"),
            "---\ntype: Report\ntitle: \"Authored overview\"\ncustom: retained\n---\n# Authored overview\n\n",
        )
        .unwrap();

        super::write_markdown_sections(output.path(), &report, reviews.path()).unwrap();
        let first = read_documents(output.path());
        let sample = String::from_utf8_lossy(first.get("sample.md").unwrap());
        assert_eq!(
            super::parse_frontmatter(&sample).unwrap()["custom"],
            "preserved"
        );
        assert!(sample.contains("# authored sample\n\n"));
        let collection_link = sample
            .lines()
            .find_map(|line| line.strip_prefix("[Case collection]("))
            .and_then(|line| line.strip_suffix(')'))
            .expect("case collection link");
        assert_eq!(
            std::fs::canonicalize(output.path().join(collection_link)).unwrap(),
            std::fs::canonicalize(reviews.path().join("sample.toon")).unwrap(),
            "the case collection link must resolve from an arbitrary markdown directory"
        );

        let index = String::from_utf8_lossy(first.get("index.md").unwrap());
        assert!(index.contains("# authored index\n\n"));
        assert!(!index.starts_with("---\n"));
        let overview = String::from_utf8_lossy(first.get("overview.md").unwrap());
        assert!(overview.contains("# Authored overview\n\n"));
        let metadata = yaml_serde::from_str::<serde_json::Value>(
            overview
                .strip_prefix("---\n")
                .unwrap()
                .split_once("\n---\n")
                .unwrap()
                .0,
        )
        .unwrap();
        assert_eq!(metadata["custom"], "retained");
        assert_eq!(metadata["type"], "Report");
        assert_eq!(metadata["title"], "Authored overview");
        let run = &metadata["benchmark_runs"][0];
        assert_eq!(run["campaign_id"], serde_json::Value::Null);
        assert_eq!(run["captured_at"], report["generated_at"]);
        assert_eq!(run["binaries"]["yq"]["identity_status"], "not-recorded");
        let mut malformed = report.clone();
        malformed["tools"] = json!({});
        assert!(super::write_markdown_sections(output.path(), &malformed, reviews.path()).is_err());
        assert_eq!(
            first,
            read_documents(output.path()),
            "malformed metadata must not write pages"
        );
        let valid_overview = first.get("overview.md").unwrap();
        std::fs::write(output.path().join("overview.md"), "---\ntype: [broken\n").unwrap();
        let malformed_overview = read_documents(output.path());
        assert!(super::write_markdown_sections(output.path(), &report, reviews.path()).is_err());
        assert_eq!(
            malformed_overview,
            read_documents(output.path()),
            "malformed overview frontmatter must not write section pages"
        );
        std::fs::write(output.path().join("overview.md"), valid_overview).unwrap();

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

    #[test]
    fn missing_toon_capture_qualifies_prose_without_changing_tool_labels() {
        let mut report = tiny_report();
        let row = &mut report["cases"][0];
        row["input"] = json!("jq --seq .");
        row["tq_toon"] = serde_json::Value::Null;
        let rendered = super::render(&report).unwrap();
        assert!(rendered.contains("TOON sequence captures, when available"));
        assert!(rendered.contains("Missing captures are marked `<not run>`"));
        assert!(rendered.contains("# tq\n<not run>"));
        assert!(rendered.contains("| tq | Diff |"));
        assert!(!rendered.contains("TOON output not captured"));
    }

    #[test]
    fn human_verdict_labels_preserve_machine_verdicts_and_comparison_criteria() {
        let mut report = tiny_report();
        report["cases"][1]["verdict"] = json!("failure");
        tq_test_support::compatibility::summarize_manual_comparison(&mut report).unwrap();
        let original = report.clone();

        let rendered = super::render(&report).unwrap();

        assert!(rendered.contains("| Exact match | 1 |"));
        assert!(rendered.contains("| Differences | 1 |"));
        assert!(rendered.contains("| Output campaign | Exact matches | Cases |"));
        assert!(
            rendered
                .contains("An exact match requires equivalent JSON results and process behavior")
        );
        assert!(rendered.contains("Compact JSON compares exact stdout bytes and process behavior"));
        assert!(rendered.contains("JSON equivalence ignores whitespace and object key order"));
        assert!(!rendered.contains("| failure |"));
        assert!(!rendered.contains("| match |"));
        assert!(!rendered.contains("| Matches |"));
        assert_eq!(
            report, original,
            "human labels must not alter machine evidence"
        );
        assert_eq!(report["summary"]["verdicts"]["failure"], 1);
        assert_eq!(report["summary"]["verdicts"]["match"], 1);
    }

    #[test]
    fn differences_list_includes_every_non_match_with_its_report_reason() {
        let mut report = tiny_report();
        let mut rows = vec![report["cases"][0].clone()];
        for (id, verdict, reason) in [
            (
                "sample.unaccepted",
                "failure",
                "Ordered JSON results differ.",
            ),
            (
                "sample.reviewed",
                "disparity",
                "Observed CLI output differs.",
            ),
            (
                "sample.historical",
                "expected-difference",
                "Historical expectation differs.",
            ),
            (
                "sample.reference",
                "reference-discrepancy",
                "Imported manual expectation differs.",
            ),
            (
                "sample.unexecuted",
                "skip",
                "The executable was unavailable.",
            ),
        ] {
            let mut row = report["cases"][1].clone();
            row["id"] = json!(id);
            row["verdict"] = json!(verdict);
            row["reason"] = json!(reason);
            rows.push(row);
        }
        report["cases"] = json!(rows);
        tq_test_support::compatibility::summarize_manual_comparison(&mut report).unwrap();

        let rendered = super::render(&report).unwrap();
        let expected = "## Differences\n\n1. `sample.unaccepted`: Ordered JSON results differ.\n2. `sample.reviewed`: Observed CLI output differs.\n3. `sample.historical`: Historical expectation differs.\n4. `sample.reference`: Imported manual expectation differs.\n5. `sample.unexecuted`: The executable was unavailable.\n";
        assert!(
            rendered.contains(expected),
            "missing complete numbered differences list"
        );
        assert!(!rendered.contains("Reviewed disparities and historical differences"));
        assert!(!rendered.contains("| Case | Reason |"));
        assert!(!rendered.contains("1. `sample.result`"));
        assert!(
            !rendered.contains("](#sample"),
            "IDs must also work on the index without case headings"
        );
    }

    #[test]
    fn expected_presentation_note_is_rendered_without_relabeling_raw_failure() {
        let mut report = tiny_report();
        let row = &mut report["cases"][1];
        row["verdict"] = json!("failure");
        row["reason"] = json!("Raw byte mismatch has not been accepted as expected.");
        row["presentation_note"] = json!(
            "Expected presentation difference: Structural quotes and ANSI resets differ. JSON data and process behavior agree; ANSI styling differs"
        );
        tq_test_support::compatibility::summarize_manual_comparison(&mut report).unwrap();
        let original = report.clone();
        let rendered = super::render(&report).unwrap();
        assert!(rendered.contains("Expected presentation difference: quote styling and ANSI escapes differ; JSON data is unchanged."));
        assert!(!rendered.contains("Structural quotes and ANSI resets differ."));
        assert!(!rendered.contains("Raw byte mismatch has not been accepted as expected."));
        assert!(rendered.contains("| Differences | 1 |"));
        assert_eq!(report, original);
        assert_eq!(report["summary"]["failures"], 1);
    }

    #[test]
    fn presentation_summary_does_not_invent_a_data_guarantee() {
        let mut report = tiny_report();
        report["cases"][1]["verdict"] = json!("failure");
        report["cases"][1]["presentation_note"] =
            json!("Expected presentation difference: ANSI styling differs");
        let rendered = super::render(&report).unwrap();
        assert!(rendered.contains("Expected presentation difference: ANSI styling differs."));
        assert!(!rendered.contains("JSON data is unchanged."));
    }

    fn math_difference(id: &str, jq: &str, tq: &str) -> serde_json::Value {
        let mut row = tiny_report()["cases"][0].clone();
        row["id"] = json!(id);
        row["verdict"] = json!("failure");
        row["reason"] = json!("Original math mismatch.");
        row["jq"]["results"] = serde_json::from_str(jq).unwrap();
        row["tq"]["results"] = serde_json::from_str(tq).unwrap();
        row
    }

    fn difference_text(row: &serde_json::Value) -> String {
        let mut output = String::new();
        super::render_differences(&mut output, &json!({"cases": [row]})).unwrap();
        output
    }

    #[test]
    fn known_math_pairs_have_concise_expected_labels_without_changing_evidence() {
        for (id, jq, tq, label) in [
            (
                "manual.audit.math.erfc-ulp",
                "[0.0046777349810472645]",
                "[0.004677734981047266]",
                "Expected rounding difference: tq is 2 ULP higher for erfc(2).",
            ),
            (
                "manual.audit.math.tgamma-ulp",
                "[1.772453850905516]",
                "[1.7724538509055159]",
                "Expected rounding difference: tq is 1 ULP lower for tgamma(0.5).",
            ),
        ] {
            let row = math_difference(id, jq, tq);
            let original = row.clone();
            assert!(difference_text(&row).contains(label));
            assert_eq!(row, original);
            assert_eq!(row["verdict"], "failure");
        }
    }

    #[test]
    fn math_summary_rejects_changed_values_ids_and_result_sequences() {
        let row = math_difference(
            "manual.audit.math.erfc-ulp",
            "[0.0046777349810472645]",
            "[0.004677734981047266]",
        );
        for (tool, results) in [
            ("jq", json!([0])),
            ("tq", json!([0])),
            ("tq", json!([])),
            ("tq", json!([0.004_677_734_981_047_266, 1])),
            ("tq", json!(["0.004677734981047266"])),
        ] {
            let mut changed = row.clone();
            changed[tool]["results"] = results;
            assert!(difference_text(&changed).contains("Original math mismatch."));
        }
        let mut changed = row;
        changed["id"] = json!("manual.audit.math.other");
        assert!(difference_text(&changed).contains("Original math mismatch."));
    }

    fn run_tests_difference() -> serde_json::Value {
        let mut row = tiny_report()["cases"][1].clone();
        row["id"] = json!("manual.invoking.run-tests");
        row["verdict"] = json!("failure");
        row["reason"] = json!("Original CLI mismatch.");
        let prefix = "Test #1: '.' at line number 2\n1 of 1 tests passed (0 malformed, 0 skipped)\nTest jq_state: .[]\nTest jq_state: .[] | if .%2 == 0 then halt_error else . end\n";
        for (tool, stdout) in [
            (
                "jq",
                format!("{prefix}Test jq_compile_args with array args\n"),
            ),
            ("tq", prefix.to_owned()),
        ] {
            let hex = tq_test_support::compatibility::encode_hex(stdout.as_bytes());
            row[tool]["stdout_hex"] = json!(hex);
            row[tool]["raw_stdout_hex"] = json!(hex);
        }
        row
    }

    #[test]
    fn run_tests_summary_requires_verified_prefix_and_supplied_test_pass() {
        let row = run_tests_difference();
        let original = row.clone();
        let text = difference_text(&row);
        assert!(text.contains("jq prints extra internal self-test messages; both tools pass the supplied test and exit 0."));
        assert!(!text.contains("Expected"));
        assert_eq!(row, original);
        assert_eq!(row["verdict"], "failure");
        for stdout in [
            "unrelated output\n",
            "Test #1: '.' at line number 2\n",
            "1 of 1 tests passed (0 malformed, 0 skipped)\n",
            "Test #1: '.' at line number 2\n0 of 1 tests passed (0 malformed, 0 skipped)\n",
        ] {
            let mut changed = row.clone();
            changed["tq"]["stdout_hex"] = json!(tq_test_support::compatibility::encode_hex(
                stdout.as_bytes()
            ));
            assert!(difference_text(&changed).contains("Original CLI mismatch."));
        }
        let mut equal = row.clone();
        equal["jq"]["stdout_hex"] = equal["tq"]["stdout_hex"].clone();
        assert!(difference_text(&equal).contains("Original CLI mismatch."));
        let mut invalid_hex = row;
        invalid_hex["jq"]["stdout_hex"] = json!("zz");
        assert!(difference_text(&invalid_hex).contains("Original CLI mismatch."));
    }

    #[test]
    fn observation_errors_keep_original_math_and_run_tests_reasons() {
        for row in [
            math_difference(
                "manual.audit.math.erfc-ulp",
                "[0.0046777349810472645]",
                "[0.004677734981047266]",
            ),
            math_difference(
                "manual.audit.math.tgamma-ulp",
                "[1.772453850905516]",
                "[1.7724538509055159]",
            ),
            run_tests_difference(),
        ] {
            for tool in ["jq", "tq"] {
                for (field, value) in [
                    ("state", json!("not-run")),
                    ("process_status", json!("signaled")),
                    ("exit_code", json!(1)),
                    ("exit_code", serde_json::Value::Null),
                    ("stderr_hex", json!("6572726f72")),
                    ("error_class", json!("runtime")),
                ] {
                    let mut changed = row.clone();
                    changed[tool][field] = value;
                    assert!(
                        difference_text(&changed).contains(row["reason"].as_str().unwrap()),
                        "{tool}.{field} must keep the original reason"
                    );
                }
            }
        }
    }

    #[test]
    fn exact_match_report_has_an_explicit_empty_differences_section() {
        let rendered = super::render(&tiny_report()).unwrap();
        assert!(rendered.contains("## Differences\n\nNo differences.\n\n## Cases\n"));
        assert!(!rendered.contains("| Case | Reason |"));
    }

    #[test]
    fn report_counts_use_singular_only_for_one() {
        for count in [0, 1, 2] {
            let mut report = tiny_report();
            report["summary"]["cases"] = json!(count);
            report["summary"]["size_samples"] = json!(count);
            let rendered = super::render(&report).unwrap();
            let suffix = if count == 1 { "" } else { "s" };
            assert!(rendered.contains(&format!("{count} case{suffix}.")));
            assert!(rendered.contains(&format!("{count} eligible example{suffix}.")));
        }
    }

    #[test]
    fn markdown_paths_escape_url_delimiters_and_spaces() {
        assert_eq!(
            super::markdown_path(std::path::Path::new("../reviews # ? space.toon")),
            "../reviews%20%23%20%3F%20space.toon"
        );
    }

    #[test]
    fn relative_markdown_links_escape_special_directory_names() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("markdown # space");
        let reviews = root.path().join("reviews # space");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        let target = reviews.join("sample # %.toon");
        std::fs::write(&target, "fixture").unwrap();

        assert_eq!(
            super::relative_markdown_link(&output, &target).unwrap(),
            "../reviews%20%23%20space/sample%20%23%20%25.toon"
        );
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
            "generated_at": "2026-09-10T03:30:00Z",
            "tools": [
                {"tool": "jq", "version": "jq measured", "executable": {"sha256": "1111111111111111111111111111111111111111111111111111111111111111"}},
                {"tool": "tq", "version": "tq measured", "executable": {"sha256": "2222222222222222222222222222222222222222222222222222222222222222"}}
            ],
            "cases": [result, raw],
        });
        tq_test_support::compatibility::summarize_manual_comparison(&mut report).unwrap();
        report
    }
}
