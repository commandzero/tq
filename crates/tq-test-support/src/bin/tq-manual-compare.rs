//! Reproducible JSON-equivalence and output-size report for manual examples.

use serde_json::Value;
use std::{env, fmt::Write as _, fs, path::PathBuf, time::Duration};
use tq_test_support::compatibility::{ExecutableConfig, compare_manual, load_catalog};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = env::args().skip(1);
    let argument = args.next();
    if argument
        .as_deref()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "Usage: tq-manual-compare [REPORT.toon]\nWrites TOON evidence and a matching Markdown report. Compatibility failures are reported, not used as this audit command's exit status."
        );
        return Ok(());
    }
    if args.next().is_some() {
        return Err("expected at most one report path".into());
    }
    let destination =
        argument.map_or_else(|| root.join("target/manual-comparison.toon"), PathBuf::from);
    if destination
        .extension()
        .is_none_or(|extension| extension != "json" && extension != "toon")
    {
        return Err("report path must end in .toon or .json".into());
    }
    let catalog = load_catalog(&root.join("tests/compatibility/cases"))?;
    let report = compare_manual(
        &catalog,
        &ExecutableConfig::from_env(),
        &root,
        Duration::from_secs(5),
    )?;
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &destination,
        if destination.extension().is_some_and(|ext| ext == "toon") {
            tq_test_support::fixture_data::to_toon(&report)?
        } else {
            format!("{}\n", serde_json::to_string_pretty(&report)?)
        },
    )?;
    fs::write(destination.with_extension("md"), render(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report["summary"])?);
    Ok(())
}

fn render(report: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let summary = &report["summary"];
    let sizes = &summary["characters"];
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
    output.push_str("\nA match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Expected differences cite a documented policy and check the observed behavior.\n\nMissing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.\n\n");
    output.push_str("JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.\n\n");
    writeln!(
        output,
        "## Output size\n\n{} eligible examples. These are default `-o json` and `-o toon` outputs, not compact JSON or tokenizer counts.\n\n| JSON characters | TOON characters | Saved | Saved % |\n| ---: | ---: | ---: | ---: |\n| {} | {} | {} | {:.2} |\n",
        summary["size_samples"],
        sizes["json"],
        sizes["toon"],
        sizes["saved"],
        sizes["saved_percent"].as_f64().unwrap_or(0.0)
    )?;
    output.push_str("Counts include Unicode scalar values, record separators, and trailing newlines. Negative savings mean TOON is longer. Only successful jq/JSON/TOON-equivalent results enter the total, which is weighted by characters rather than averaging case percentages. The manual is a correctness corpus, not a representative workload benchmark.\n\n");
    output.push_str("## Expected differences\n\n| Case | Reason |\n| --- | --- |\n");
    for row in report["cases"]
        .as_array()
        .ok_or("missing cases")?
        .iter()
        .filter(|row| row["verdict"] == "expected-difference")
    {
        writeln!(
            output,
            "| `{}` | {} |",
            row["id"].as_str().ok_or("missing ID")?,
            row["reason"].as_str().ok_or("missing reason")?
        )?;
    }
    output.push_str("\n## Per-case results\n\n`n/a` covers failed executions or non-JSON CLI cases. A TOON `failure` means its encoding did not preserve the successful JSON result; it is excluded from size totals.\n\n| Case | Verdict | TOON encoding | JSON chars | TOON chars | Saved |\n| --- | --- | --- | ---: | ---: | ---: |\n");
    for row in report["cases"].as_array().ok_or("missing cases")? {
        let toon = match row["toon_equivalent"].as_bool() {
            Some(true) => "match",
            Some(false) => "failure",
            None => "n/a",
        };
        let count = |key| {
            row["characters"][key]
                .as_i64()
                .map_or_else(|| "n/a".to_owned(), |count| count.to_string())
        };
        writeln!(
            output,
            "| `{}` | {} | {toon} | {} | {} | {} |",
            row["id"].as_str().ok_or("missing ID")?,
            row["verdict"].as_str().ok_or("missing verdict")?,
            count("json"),
            count("toon"),
            count("saved")
        )?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    #[test]
    fn checked_in_report_matches_renderer_without_redundant_column() {
        let report = tq_test_support::fixture_data::from_toon(include_bytes!(
            "../../../../tests/compatibility/reviews/manual-comparison.toon"
        ))
        .unwrap();
        let markdown = super::render(&report).unwrap();
        assert!(!markdown.contains("| JSON equivalent |"));
        assert_eq!(
            markdown,
            include_str!("../../../../tests/compatibility/reviews/manual-comparison.md")
        );
    }
}
