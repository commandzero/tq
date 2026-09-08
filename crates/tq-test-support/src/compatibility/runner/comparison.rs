//! Manual results separate JSON semantics, CLI contracts, and encoded size.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
    time::Duration,
};

use serde_json::{Value, json};

use super::super::{
    CaseAdapter, CompatibilityCase, CompatibilityCatalog, ContractKind, ExecutableConfig,
    ObservationState, ToolIdentity, ToolKind, ToolObservation, discover_tool, manual_case_ids,
    read_manual_ledger,
};
use super::{ExecutionFixture, execute, fixture_bytes, semantic_diffs, skipped};

/// Runs the manual's referenced cases with explicit JSON and TOON output.
///
/// Raw CLI contracts retain their original arguments and are not size samples.
/// Only successful, equivalent jq/JSON/TOON results contribute size savings.
///
/// # Errors
///
/// Returns catalog, inventory, executable-discovery, or fixture I/O errors.
pub fn compare_manual(
    catalog: &CompatibilityCatalog,
    config: &ExecutableConfig,
    root: &Path,
    timeout: Duration,
) -> Result<Value, Box<dyn std::error::Error>> {
    let reviews = root.join("tests/compatibility/reviews");
    let mut ids = BTreeSet::new();
    for entry in fs::read_dir(&reviews)? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with("manual-")
            || !(name.ends_with(".json") || name.ends_with(".toon"))
            || name.ends_with(".json") && path.with_extension("toon").exists()
        {
            continue;
        }
        let ledger = read_manual_ledger(&path)?;
        if let Some(examples) = ledger["examples"].as_array() {
            for example in examples {
                for id in manual_case_ids(example)? {
                    ids.insert(id.to_owned());
                }
            }
        }
    }
    // Include witnesses added during integration, even when a ledger links a
    // different representative for the same prose claim.
    ids.extend(
        catalog
            .cases
            .iter()
            .filter(|case| case.id.starts_with("manual.") || case.id.starts_with("manual-"))
            .map(|case| case.id.clone()),
    );
    let jq = discover_tool(ToolKind::Jq, config, root)?.ok_or("jq is unavailable")?;
    let tq = discover_tool(ToolKind::Tq, config, root)?.ok_or("tq is unavailable")?;
    let mut rows = Vec::new();
    for id in ids {
        let case = catalog
            .cases
            .iter()
            .find(|case| case.id == id)
            .ok_or("unknown case ID")?;
        rows.push(compare_case(case, &jq, &tq, root, timeout)?);
    }
    let mut verdicts = BTreeMap::<String, usize>::new();
    let mut json_chars = 0_u64;
    let mut toon_chars = 0_u64;
    let mut samples = 0;
    for row in &rows {
        *verdicts
            .entry(row["verdict"].as_str().ok_or("missing verdict")?.to_owned())
            .or_default() += 1;
        if let Some(count) = row["characters"]["json"].as_u64() {
            samples += 1;
            json_chars += count;
            toon_chars += row["characters"]["toon"]
                .as_u64()
                .ok_or("missing TOON count")?;
        }
    }
    Ok(json!({
        "schema_version": 1, "corpus": catalog.identity, "tools": [jq, tq],
        "method": "Original input fixture; structured cases run tq -o json and tq -o toon. Ordered JSON values use the shared numeric normalizer; whitespace and object key order do not matter. Raw CLI cases retain their arguments and are assessed separately. Unicode scalar counts include all stdout framing and newlines. Size totals include only successful jq/JSON/TOON-equivalent cases using default encoder layouts, not compact JSON. No tokenizer measurement.",
        "summary": {"cases": rows.len(), "verdicts": verdicts, "size_samples": samples,
            "characters": size_value(json_chars, toon_chars)},
        "cases": rows,
    }))
}

fn compare_case(
    case: &CompatibilityCase,
    jq: &ToolIdentity,
    tq: &ToolIdentity,
    root: &Path,
    timeout: Duration,
) -> Result<Value, io::Error> {
    let id = &case.id;
    let structured = matches!(
        case.expected.contract,
        ContractKind::ResultSequence | ContractKind::Error
    );
    let reference = observe(case, &case.adapters.jq, jq, root, timeout, false)?;
    let mut json_adapter = case.adapters.tq.clone();
    if structured {
        json_adapter
            .args
            .splice(0..0, ["-o".to_owned(), "json".to_owned()]);
    }
    let actual = observe(case, &json_adapter, tq, root, timeout, structured)?;
    let toon = if structured {
        let mut adapter = case.adapters.tq.clone();
        adapter
            .args
            .splice(0..0, ["-o".to_owned(), "toon".to_owned()]);
        Some(observe(case, &adapter, tq, root, timeout, false)?)
    } else {
        None
    };
    let differences = semantic_diffs(
        &[reference.clone(), actual.clone()],
        case.expected.compare_stderr,
    );
    let completed = reference.state == ObservationState::Executed
        && actual.state == ObservationState::Executed
        && reference.process_status == Some(super::super::ProcessStatus::Exited)
        && actual.process_status == Some(super::super::ProcessStatus::Exited);
    let successful = successful_observation(&reference) && successful_observation(&actual);
    let json_equivalent = (structured && successful).then_some(reference.results == actual.results);
    let error_contract_met = case.expected.contract != ContractKind::Error
        || actual.exit_code.is_some_and(|code| code != 0)
            && expected_error(case.expected.error_class.as_deref(), actual.error_class);
    let (verdict, reason) = if id.ends_with("-reference-unavailable")
        && completed
        && reference.exit_code == Some(3)
        && error_contract_met
    {
        (
            "reference-discrepancy",
            "The imported manual lists /2, but jq 1.8.1 provides /0; see manual-math.md.",
        )
    } else if reference.state == ObservationState::Unsupported
        || actual.state == ObservationState::Unsupported
    {
        (
            "unverified",
            "An adapter is unavailable; inspect observations.",
        )
    } else if completed
        && differences.is_empty()
        && error_contract_met
        && (case.expected.contract != ContractKind::ResultSequence || successful)
    {
        (
            "match",
            "The JSON result sequence and process contract match, or the non-JSON CLI contract matches.",
        )
    } else if let Some(reason) = reviewed_difference(id, &reference, &actual) {
        ("expected-difference", reason)
    } else {
        (
            "failure",
            "jq compatibility or the declared error contract failed; this difference has not been accepted as expected.",
        )
    };
    let toon_equivalent = toon
        .as_ref()
        .filter(|_| successful_observation(&actual))
        .map(|value| successful_observation(value) && value.results == actual.results);
    let sizes = if structured
        && successful
        && json_equivalent == Some(true)
        && toon_equivalent == Some(true)
    {
        toon.as_ref()
            .and_then(|toon| character_sizes(&actual, toon))
    } else {
        None
    };
    Ok(json!({
        "id": id, "contract": case.expected.contract, "verdict": verdict,
        "reason": reason, "json_equivalent": json_equivalent,
        "toon_equivalent": toon_equivalent, "characters": sizes,
        "comparison_mode": if structured { "json" } else { "cli-contract" },
        "differences": differences, "jq": reference, "tq": actual, "tq_toon": toon,
    }))
}

fn observe(
    case: &CompatibilityCase,
    adapter: &CaseAdapter,
    tool: &ToolIdentity,
    root: &Path,
    timeout: Duration,
    json_output: bool,
) -> Result<ToolObservation, io::Error> {
    if !adapter.supported {
        return Ok(skipped(
            tool.tool,
            Some(case.fixture.format),
            ObservationState::Unsupported,
            "adapter unsupported",
        ));
    }
    execute(
        case,
        adapter,
        tool,
        root,
        timeout,
        ExecutionFixture {
            format: case.fixture.format,
            bytes: fixture_bytes(case, root)?,
            pin_format: false,
        },
        json_output,
    )
}

fn successful_observation(value: &ToolObservation) -> bool {
    value.state == ObservationState::Executed
        && value.exit_code == Some(0)
        && value.process_status == Some(super::super::ProcessStatus::Exited)
}

fn expected_error(expected: Option<&str>, actual: Option<super::super::ErrorClass>) -> bool {
    expected.is_none_or(|expected| {
        let expected = if expected == "compile" {
            "query-compile"
        } else {
            expected
        };
        serde_json::to_value(actual).is_ok_and(|actual| actual.as_str() == Some(expected))
    })
}

fn expected_difference(id: &str) -> Option<&'static str> {
    match id {
        "manual.invoking.version"
        | "manual.invoking.help"
        | "manual.invoking.build-configuration" => Some(
            "Tool identity/help text is intentionally tq-specific. See docs/jq-1.8-cli-options.md.",
        ),
        "manual.colors.default-palette"
        | "manual.colors.custom-1-31"
        | "manual.colors.ansi-values"
        | "manual.invoking.color-output"
        | "manual.invoking.jq-colors"
        | "manual.invoking.no-color-forced" => Some(
            "tq uses its fixed ANSI palette and ignores JQ_COLORS. See docs/jq-1.8-cli-options.md.",
        ),
        "manual.invoking.seq" => Some(
            "tq --seq emits TOON Text Sequence rather than jq JSON-seq. See docs/jq-1.8-cli-options.md.",
        ),
        _ => None,
    }
}

fn reviewed_difference(
    id: &str,
    jq: &ToolObservation,
    tq: &ToolObservation,
) -> Option<&'static str> {
    if !successful_observation(jq) || tq.state != ObservationState::Executed {
        return None;
    }
    if successful_observation(tq) {
        if id == "manual.io.input-line-number" && tq.results == [json!(1), json!(1)] {
            return Some(
                "tq reports the first logical input's line for multi-document input. See docs/jq-regex-date-platform.md#environment-and-input-metadata.",
            );
        }
        return expected_difference(id);
    }
    let stderr = decode_hex(tq.stderr_hex.as_deref()?)?;
    let stderr = std::str::from_utf8(&stderr).ok()?;
    if !tq.results.is_empty() {
        return None;
    }
    match (id, tq.exit_code) {
        ("manual.regex.flag-l", Some(2))
            if stderr.contains("regex flag 'l' (longest-match mode)") =>
        {
            Some(
                "The selected regex engine intentionally rejects longest-match mode. See docs/jq-regex-date-platform.md#selected-dependencies-and-limits.",
            )
        }
        ("manual.basic.identity-large-exponent", Some(3))
            if stderr.contains("numeric exponent exceeds 1000000") =>
        {
            Some(
                "The literal exceeds the accepted exponent envelope. See tests/compatibility/reviews/numeric-policy-v1.toon.",
            )
        }
        ("manual.modules.home-auto-source", Some(3))
            if stderr.contains("unknown filter home_value/0") =>
        {
            Some(
                "tq uses explicit confined module roots, not jq's automatic home startup module. See docs/jq-1.8-cli-options.md.",
            )
        }
        ("manual.invoking.run-tests", Some(2))
            if stderr.contains("unsupported option '--run-tests'") =>
        {
            Some(
                "jq's internal test runner is explicitly unsupported. See docs/jq-1.8-cli-options.md.",
            )
        }
        ("manual.invoking.seq", Some(5)) if stderr.contains("Toon input rejected") => Some(
            "tq --seq uses TOON sequences and does not infer JSON-seq input. See docs/jq-1.8-cli-options.md.",
        ),
        _ => None,
    }
}

fn character_sizes(json: &ToolObservation, toon: &ToolObservation) -> Option<Value> {
    Some(size_value(
        character_count(json.stdout_hex.as_deref()?)?,
        character_count(toon.stdout_hex.as_deref()?)?,
    ))
}

fn character_count(hex: &str) -> Option<u64> {
    let bytes = decode_hex(hex)?;
    Some(std::str::from_utf8(&bytes).ok()?.chars().count() as u64)
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

// Percentages are approximate display values; integer counts remain exact.
#[allow(clippy::cast_precision_loss)]
fn size_value(json: u64, toon: u64) -> Value {
    json!({"json": json, "toon": toon, "saved": i128::from(json) - i128::from(toon),
        "saved_percent": (json != 0).then(|| 100.0 * (json as f64 - toon as f64) / json as f64)})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compatibility::encode_hex;

    #[test]
    fn counts_unicode_scalars_and_retains_framing() {
        assert_eq!(
            character_count(&encode_hex("\u{1e}é😀\n".as_bytes())),
            Some(4)
        );
        assert_eq!(character_count("ff"), None);
        assert_eq!(character_count("0"), None);
        assert_eq!(size_value(0, 0)["saved_percent"], Value::Null);
        assert_eq!(size_value(2, 4)["saved"], -2);
    }

    #[test]
    fn missing_features_are_not_automatically_expected() {
        assert!(expected_difference("manual.math.acos").is_none());
        assert!(expected_difference("manual.invoking.version").is_some());
        assert!(!expected_error(Some("query-compile"), None));
    }
}
