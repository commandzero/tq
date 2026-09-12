//! Manual results separate JSON semantics, CLI contracts, and encoded size.

use std::{collections::BTreeMap, io, path::Path, time::Duration};

use serde_json::{Value, json};
use tiktoken_rs::{CoreBPE, cl100k_base, o200k_base};

use super::super::{
    CaseAdapter, CompatibilityCase, CompatibilityCatalog, ContractKind, ExecutableConfig,
    ObservationState, ReviewedDisparity, SemanticDiff, ToolIdentity, ToolKind, ToolObservation,
    apply_reviewed_disparities, case_fingerprint, discover_tool, manual_verdict_counts,
    read_manual_review_case_ids, tq_contract_matches,
};
use super::{ExecutionFixture, OutputMode, execute, fixture_bytes, semantic_diffs, skipped};

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
    compare_manual_with_disparities(catalog, config, root, timeout, &[])
}

/// Runs the manual campaign and applies only explicitly reviewed disparities.
///
/// The default [`compare_manual`] path supplies an empty approval set and is
/// therefore exact-only: an observed difference remains a failure. An
/// approval must match the generated row's case ID, contract, semantic-
/// difference summary, and complete stable evidence before it can be recorded
/// as a disparity.
///
/// # Errors
///
/// Returns catalog, inventory, executable-discovery, fixture I/O, or stale
/// disparity-approval errors.
pub fn compare_manual_with_disparities(
    catalog: &CompatibilityCatalog,
    config: &ExecutableConfig,
    root: &Path,
    timeout: Duration,
    approvals: &[ReviewedDisparity],
) -> Result<Value, Box<dyn std::error::Error>> {
    let reviews = root.join("tests/compatibility/reviews/jq-manual");
    let mut ids = read_manual_review_case_ids(&reviews)?;
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
    let tokenizers = Tokenizers::new()?;
    let mut rows = Vec::new();
    for id in ids {
        let case = catalog
            .cases
            .iter()
            .find(|case| case.id == id)
            .ok_or("unknown case ID")?;
        rows.push(compare_case(case, &jq, &tq, root, timeout, &tokenizers)?);
    }
    let mut report = json!({
        "schema_version": 1,
        "corpus": catalog.identity,
        "tools": [jq, tq],
        "method": "Original input fixture; structured cases independently compare jq with tq -o json, tq -o toon, and compact jq -c with tq -o json -c. Semantic JSON ignores presentation whitespace and object key order while preserving ordered results and exact decimal values. The compact campaign compares stdout bytes exactly; the TOON campaign uses LF-terminated TOON values by default and explicit --seq framing only for original sequence adapters; independently captured JSON results resolve TOON value boundaries, and decoding must consume the complete TOON stdout. Raw CLI cases retain their arguments and are assessed separately. Token totals use the o200k_base and cl100k_base encodings over successful default TOON output; explicit sequence output is excluded from size totals. Size totals include only successful jq/JSON/TOON-equivalent cases using default JSON encoder layouts, not compact JSON.",
        "cases": rows,
    });
    apply_reviewed_disparities(&mut report, approvals)?;
    summarize_manual_comparison(&mut report)?;
    Ok(report)
}

/// Recomputes totals for the observations in a manual report or section.
///
/// # Errors
/// Returns an error when a report row lacks required counts or verdicts.
pub fn summarize_manual_comparison(report: &mut Value) -> Result<(), Box<dyn std::error::Error>> {
    let counts = manual_verdict_counts(report)?;
    let rows = report["cases"]
        .as_array()
        .ok_or("comparison report cases disappeared")?;
    let mut verdicts = BTreeMap::<String, usize>::new();
    let mut tokens = BTreeMap::<String, (u64, u64)>::from([
        ("o200k_base".to_owned(), (0, 0)),
        ("cl100k_base".to_owned(), (0, 0)),
    ]);
    let mut samples = 0;
    for row in rows {
        *verdicts
            .entry(row["verdict"].as_str().ok_or("missing verdict")?.to_owned())
            .or_default() += 1;
        if row["toon_sequence"] != true
            && row["json_equivalent"] == true
            && row["toon_equivalent"] == true
            && let Some(token_rows) = row["tokens"].as_object()
        {
            samples += 1;
            for (encoding, counts) in token_rows {
                let totals = tokens.entry(encoding.clone()).or_default();
                totals.0 += counts["json"].as_u64().ok_or("missing JSON token count")?;
                totals.1 += counts["toon"].as_u64().ok_or("missing TOON token count")?;
            }
        }
    }
    let token_summary = tokens
        .into_iter()
        .map(|(encoding, (json, toon))| (encoding, size_value(json, toon)))
        .collect::<BTreeMap<_, _>>();
    report["summary"] = json!({
        "cases": rows.len(), "verdicts": verdicts,
        "exact_matches": counts.exact_matches,
        "reviewed_disparities": counts.reviewed_disparities,
        "failures": counts.failures,
        "size_samples": samples,
        "tokens": token_summary,
        "encoding_campaigns": {
            "compact_json": {
                "cases": rows.iter().filter(|row| row["compact"].is_object()).count(),
                "matches": rows.iter().filter(|row| row["compact"]["exact"] == true).count(),
            },
            "toon": {
                "cases": rows.iter().filter(|row| row["toon_contract_match"].is_boolean()).count(),
                "matches": rows.iter().filter(|row| row["toon_contract_match"] == true).count(),
            },
        },
    });
    Ok(())
}

fn compare_case(
    case: &CompatibilityCase,
    jq: &ToolIdentity,
    tq: &ToolIdentity,
    root: &Path,
    timeout: Duration,
    tokenizers: &Tokenizers,
) -> Result<Value, io::Error> {
    let structured = matches!(
        case.expected.contract,
        ContractKind::ResultSequence | ContractKind::Error
    );
    let reference = observe(case, &case.adapters.jq, jq, root, timeout, false, false)?;
    let mut json_adapter = case.adapters.tq.clone();
    if structured {
        json_adapter
            .args
            .splice(0..0, ["-o".to_owned(), "json".to_owned()]);
    }
    let actual = observe(case, &json_adapter, tq, root, timeout, structured, false)?;
    let mut toon_sequence = false;
    let toon = if structured {
        let mut adapter = case.adapters.tq.clone();
        toon_sequence = adapter.args.iter().any(|argument| argument == "--seq");
        adapter
            .args
            .splice(0..0, ["-o".to_owned(), "toon".to_owned()]);
        Some(observe_toon(
            case,
            &adapter,
            tq,
            root,
            timeout,
            toon_sequence,
            &actual,
        )?)
    } else {
        None
    };
    let differences = semantic_diffs(
        &[reference.clone(), actual.clone()],
        case.expected.compare_stderr,
    );
    let successful = successful_observation(&reference) && successful_observation(&actual);
    let json_equivalent = (structured && successful).then_some(reference.results == actual.results);
    let (verdict, reason) = case_verdict(case, &reference, &actual, &differences);
    let toon_equivalent = toon
        .as_ref()
        .filter(|_| successful_observation(&actual))
        .map(|value| successful_observation(value) && value.results == actual.results);
    let toon_contract_match = toon.as_ref().map(|toon| {
        toon.state == ObservationState::Executed
            && toon.process_status == Some(super::super::ProcessStatus::Exited)
            && semantic_diffs(
                &[actual.clone(), toon.clone()],
                case.expected.compare_stderr,
            )
            .is_empty()
    });
    let compact = if structured {
        Some(compare_compact(case, jq, tq, root, timeout)?)
    } else {
        None
    };
    let tokens = token_sizes(&reference, &actual, toon.as_ref(), tokenizers);
    Ok(json!({
        "id": case.id, "input": jq_invocation(case), "case_fingerprint": case_fingerprint(case).map_err(|error| {
            io::Error::new(io::ErrorKind::InvalidData, error)
        })?, "contract": case.expected.contract, "verdict": verdict,
        "reason": reason, "json_equivalent": json_equivalent,
        "toon_sequence": toon_sequence,
        "toon_equivalent": toon_equivalent, "tokens": tokens,
        "toon_contract_match": toon_contract_match, "compact": compact,
        "comparison_mode": if structured { "json" } else { "cli-contract" },
        "differences": differences, "jq": reference, "tq": actual, "tq_toon": toon,
    }))
}

// The JSON execution provides ordered value boundaries without changing input
// flags. Decode each complete result and require all stdout bytes to be consumed.
#[allow(
    clippy::too_many_arguments,
    reason = "paired output observations share one invocation"
)]
fn observe_toon(
    case: &CompatibilityCase,
    adapter: &CaseAdapter,
    tq: &ToolIdentity,
    root: &Path,
    timeout: Duration,
    toon_sequence: bool,
    json_output: &ToolObservation,
) -> Result<ToolObservation, io::Error> {
    let mut observed = observe(case, adapter, tq, root, timeout, false, toon_sequence)?;
    if toon_sequence {
        return Ok(observed);
    }
    if json_output.state == ObservationState::Executed
        && observed
            .stdout_hex
            .as_deref()
            .and_then(decode_hex)
            .is_some_and(|bytes| {
                super::super::normalization::toon_values_match(&bytes, &json_output.results)
            })
    {
        observed.results.clone_from(&json_output.results);
        observed.state = ObservationState::Executed;
        // Document normalization may have replaced the actual process error
        // with MalformedOutput. Reclassify the captured process independently.
        if let Some(status) = observed.process_status {
            observed.error_class = super::super::classify_process(
                ToolKind::Tq,
                &super::super::ProcessOutcome {
                    status,
                    exit_code: observed.exit_code,
                    signal: None,
                    stdout: Vec::new(),
                    stderr: observed
                        .stderr_hex
                        .as_deref()
                        .and_then(decode_hex)
                        .unwrap_or_default(),
                    wall_time_micros: 0,
                    recorded_command: Vec::new(),
                },
            );
        }
        observed.note = None;
        observed.raw_stdout_hex = None;
    } else {
        observed.state = ObservationState::HarnessError;
        observed.error_class = Some(super::super::ErrorClass::MalformedOutput);
        observed.note = Some("TOON stdout does not match the ordered JSON results".to_owned());
    }
    Ok(observed)
}

fn compare_compact(
    case: &CompatibilityCase,
    jq: &ToolIdentity,
    tq: &ToolIdentity,
    root: &Path,
    timeout: Duration,
) -> Result<Value, io::Error> {
    let mut reference_adapter = case.adapters.jq.clone();
    reference_adapter.args.insert(0, "-c".to_owned());
    let mut target_adapter = case.adapters.tq.clone();
    target_adapter
        .args
        .splice(0..0, ["-o".to_owned(), "json".to_owned(), "-c".to_owned()]);
    let reference = observe(case, &reference_adapter, jq, root, timeout, true, false)?;
    let actual = observe(case, &target_adapter, tq, root, timeout, true, false)?;
    let differences = semantic_diffs(
        &[reference.clone(), actual.clone()],
        case.expected.compare_stderr,
    );
    let (verdict, _) = case_verdict(case, &reference, &actual, &differences);
    let exact = verdict == "match"
        && reference.stdout_hex.is_some()
        && reference.stdout_hex == actual.stdout_hex;
    Ok(json!({
        "exact": exact, "jq": reference, "tq": actual,
        "differences": differences,
    }))
}

fn case_verdict(
    case: &CompatibilityCase,
    reference: &ToolObservation,
    actual: &ToolObservation,
    differences: &[SemanticDiff],
) -> (&'static str, &'static str) {
    let completed = reference.state == ObservationState::Executed
        && actual.state == ObservationState::Executed
        && reference.process_status == Some(super::super::ProcessStatus::Exited)
        && actual.process_status == Some(super::super::ProcessStatus::Exited);
    let successful = successful_observation(reference) && successful_observation(actual);
    let error_contract_met = case.expected.contract != ContractKind::Error
        || actual.exit_code.is_some_and(|code| code != 0)
            && expected_error(case.expected.error_class.as_deref(), actual.error_class);
    if let Some(contract) = case.expected.tq_contract.as_ref() {
        return if contract.is_valid_for_case(case)
            && successful_observation(reference)
            && completed
            && tq_contract_matches(*contract, actual)
        {
            (
                "match",
                "The tq-native CLI contract passed its explicit status, output, and stderr assertions.",
            )
        } else {
            (
                "failure",
                "The tq-native CLI contract failed its explicit status, output, or stderr assertions.",
            )
        };
    }
    if reference.state == ObservationState::Unsupported
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
    } else {
        (
            "failure",
            "jq compatibility or the declared error contract failed; this difference has not been accepted as expected.",
        )
    }
}

fn observe(
    case: &CompatibilityCase,
    adapter: &CaseAdapter,
    tool: &ToolIdentity,
    root: &Path,
    timeout: Duration,
    json_output: bool,
    toon_sequence: bool,
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
        OutputMode {
            json_output,
            toon_sequence,
        },
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

struct Tokenizers {
    o200k_base: CoreBPE,
    cl100k_base: CoreBPE,
}

impl Tokenizers {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            o200k_base: o200k_base()?,
            cl100k_base: cl100k_base()?,
        })
    }

    fn count(&self, encoding: &str, text: &str) -> u64 {
        let tokenizer = match encoding {
            "o200k_base" => &self.o200k_base,
            "cl100k_base" => &self.cl100k_base,
            _ => unreachable!("unknown tokenizer encoding: {encoding}"),
        };
        tokenizer.encode_ordinary(text).len() as u64
    }
}

fn token_sizes(
    jq: &ToolObservation,
    json_output: &ToolObservation,
    toon: Option<&ToolObservation>,
    tokenizers: &Tokenizers,
) -> Option<Value> {
    let mut captured = false;
    let counts = ["o200k_base", "cl100k_base"]
        .into_iter()
        .map(|encoding| {
            let jq_count = output_token_count(jq, encoding, tokenizers);
            let json_count = output_token_count(json_output, encoding, tokenizers);
            let toon_count =
                toon.and_then(|observation| output_token_count(observation, encoding, tokenizers));
            captured |= jq_count.is_some() || json_count.is_some() || toon_count.is_some();
            (
                encoding.to_owned(),
                json!({
                    "jq": jq_count,
                    "json": json_count,
                    "toon": toon_count,
                    "diff": toon_count.zip(json_count).map(|(toon, json)| {
                        i128::from(toon) - i128::from(json)
                    }),
                    "diff_percent": diff_percent(json_count, toon_count),
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    captured.then_some(Value::Object(counts))
}

fn output_token_count(
    observation: &ToolObservation,
    encoding: &str,
    tokenizers: &Tokenizers,
) -> Option<u64> {
    let text = output_text(observation.stdout_hex.as_deref()?)?;
    Some(tokenizers.count(encoding, &text))
}

fn output_text(hex: &str) -> Option<String> {
    String::from_utf8(decode_hex(hex)?).ok()
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

fn size_value(json: u64, toon: u64) -> Value {
    let diff = i128::from(toon) - i128::from(json);
    json!({
        "json": json,
        "toon": toon,
        "diff": diff,
        "diff_percent": diff_percent(Some(json), Some(toon)),
    })
}

#[allow(
    clippy::cast_precision_loss,
    reason = "this is a presentation-only percentage; exact counts remain u64"
)]
fn diff_percent(json: Option<u64>, toon: Option<u64>) -> Option<f64> {
    // Report the signed change from JSON to TOON: savings are negative and
    // growth is positive.
    json.zip(toon)
        .filter(|(json, _)| *json != 0)
        .map(|(json, toon)| (toon as f64 - json as f64) * 100.0 / json as f64)
}

fn jq_invocation(case: &CompatibilityCase) -> String {
    let adapter = &case.adapters.jq;
    let mut args = adapter.args.clone();
    if !adapter.omit_query {
        args.push(adapter.query.clone().unwrap_or_else(|| case.query.clone()));
    }
    args.extend(adapter.trailing_args.iter().cloned());
    if matches!(case.invocation_mode, super::super::InvocationMode::File) {
        args.push(
            case.fixture
                .path
                .clone()
                .unwrap_or_else(|| "<fixture>".to_owned()),
        );
    }
    std::iter::once("jq".to_owned())
        .chain(args.into_iter().map(|argument| shell_quote(&argument)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(argument: &str) -> String {
    if !argument.is_empty()
        && argument
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-._/:=+,".contains(character))
    {
        argument.to_owned()
    } else {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compatibility::{ErrorClass, FixtureFormat, ProcessStatus, TqContract, encode_hex};
    use serde_json::json;

    #[test]
    fn counts_tokens_and_retains_framing() {
        let tokenizers = Tokenizers::new().unwrap();
        assert_eq!(tokenizers.count("o200k_base", "\u{1e}é😀\n"), 4);
        assert_eq!(output_text("ff"), None);
        assert_eq!(output_text("0"), None);
        assert_eq!(size_value(2, 4)["diff"], 2);
        assert_eq!(size_value(2, 4)["diff_percent"], 100.0);
        assert_eq!(size_value(4, 2)["diff"], -2);
        assert_eq!(size_value(4, 2)["diff_percent"], -50.0);
        assert_eq!(diff_percent(None, Some(1)), None);
        assert_eq!(diff_percent(Some(0), Some(u64::MAX)), None);
        assert_eq!(diff_percent(Some(u64::MAX), Some(u64::MAX)), Some(0.0));
    }

    #[test]
    fn literal_special_markers_are_counted_as_ordinary_text() {
        let tokenizers = Tokenizers::new().unwrap();
        let text = "fixture literal <|endoftext|> remains text";
        for (encoding, tokenizer) in [
            ("o200k_base", &tokenizers.o200k_base),
            ("cl100k_base", &tokenizers.cl100k_base),
        ] {
            let expected = tokenizer.encode_ordinary(text).len() as u64;
            assert_eq!(
                tokenizers.count(encoding, text),
                expected,
                "literal marker was interpreted as a protocol token for {encoding}"
            );
        }
    }

    #[test]
    fn differences_are_not_automatically_expected() {
        assert!(!expected_error(Some("query-compile"), None));
    }

    #[test]
    fn tq_identity_contract_requires_status_stderr_and_documented_output() {
        let contract = TqContract::Help;
        let complete_help = concat!(
            "tq - jq-compatible queries over TOON\nUsage: tq\n",
            "-i, --input-format FORMAT\n-o, --output-format FORMAT\n",
            "-n, --null-input\n-R, --raw-input\n-s, --slurp\n",
            "-c, --compact-output\n-r, --raw-output\n--raw-output0\n",
            "-j, --join-output\n-a, --ascii-output\n-S, --sort-keys\n",
            "-C, --color-output\n-M, --monochrome-output\n--tab\n",
            "--indent N\n--unbuffered\n--allow-environment\n--allow-platform\n",
            "--stream\n--stream-errors\n-x, --proxy-on-error\n--seq\n",
            "-f, --from-file FILE\n-L, --library-path DIR\n",
            "--arg NAME VALUE\n--argjson NAME JSON\n--argtoon NAME TOON\n",
            "--slurpfile NAME FILE\n--rawfile NAME FILE\n--args\n--jsonargs\n",
            "-e, --exit-status\n-b, --binary\n-V, --version\n",
            "--build-configuration\n--run-tests [FILE]\n-h, --help\n",
            "Formats: -i, --input-format auto|toon|yaml|json|json5|jsonl|toon-seq|json-seq\n",
            "-o, --output-format toon|yaml|json|jsonl\nselect TOON\nemit compact JSON\n",
        );
        let matching = cli_observation(complete_help, "", 0);
        assert!(tq_contract_matches(contract, &matching));

        let missing_option = complete_help.replace("--stream-errors", "");
        for wrong in [
            cli_observation("jq-1.8.1\n", "", 0),
            cli_observation(&missing_option, "", 0),
            cli_observation(complete_help, "diagnostic\n", 0),
            cli_observation_with_status(complete_help, "", ProcessStatus::TimedOut, 0),
        ] {
            assert!(!tq_contract_matches(contract, &wrong));
        }
    }

    #[test]
    fn tq_identity_contract_replaces_jq_byte_identity_comparison() {
        let case: CompatibilityCase = serde_json::from_value(json!({
            "schema_version": 1,
            "id": "manual.invoking.version",
            "title": "Version option invocation",
            "classification": "jq-target",
            "capabilities": ["manual.identity"],
            "status": "mvp",
            "fixture": {"format": "none", "inline": ""},
            "query": "",
            "adapters": {
                "jq": {"supported": true},
                "tq": {"supported": true, "args": ["--version"], "omit_query": true}
            },
            "invocation_mode": "null-input",
            "expected": {
                "contract": "raw-bytes",
                "baseline": "required",
                "tq_contract": "version"
            }
        }))
        .expect("identity contract case should deserialize");
        let reference = cli_observation("jq-1.8.1\n", "", 0);
        let matching = cli_observation("tq 0.1.0 (jq target 1.8.x)\n", "", 0);
        assert_eq!(case_verdict(&case, &reference, &matching, &[]).0, "match");

        let failed_reference = cli_observation("jq-1.8.1\n", "", 1);
        assert_eq!(
            case_verdict(&case, &failed_reference, &matching, &[]).0,
            "failure"
        );

        let wrong_identity = cli_observation("jq-1.8.1\n", "", 0);
        assert_eq!(
            case_verdict(&case, &reference, &wrong_identity, &[]).0,
            "failure"
        );
    }

    #[test]
    fn tq_identity_contract_rejects_arbitrary_assertion_objects() {
        assert!(serde_json::from_value::<TqContract>(json!({})).is_err());
        assert!(serde_json::from_value::<TqContract>(json!({"stdout_contains": [""]})).is_err());
    }

    fn cli_observation(stdout: &str, stderr: &str, exit_code: i32) -> ToolObservation {
        cli_observation_with_status(stdout, stderr, ProcessStatus::Exited, exit_code)
    }

    fn cli_observation_with_status(
        stdout: &str,
        stderr: &str,
        process_status: ProcessStatus,
        exit_code: i32,
    ) -> ToolObservation {
        ToolObservation {
            tool: ToolKind::Tq,
            input_format: Some(FixtureFormat::None),
            state: ObservationState::Executed,
            results: Vec::new(),
            stdout_hex: Some(encode_hex(stdout.as_bytes())),
            raw_stdout_hex: Some(encode_hex(stdout.as_bytes())),
            stderr_hex: (!stderr.is_empty()).then(|| encode_hex(stderr.as_bytes())),
            process_status: Some(process_status),
            exit_code: Some(exit_code),
            error_class: None,
            wall_time_micros: Some(0),
            note: None,
        }
    }

    #[test]
    fn retained_invalid_math_probes_match_only_the_declared_error_contract() {
        for (id, query) in [
            ("manual.math.frexp-reference-unavailable", "frexp(8; 0)"),
            ("manual.math.modf-reference-unavailable", "modf(3; 0)"),
        ] {
            let case = invalid_math_case(id, query);
            let reference = error_observation(
                ToolKind::Jq,
                ProcessStatus::Exited,
                Some(3),
                Some(ErrorClass::QueryCompile),
            );
            let actual = error_observation(
                ToolKind::Tq,
                ProcessStatus::Exited,
                Some(3),
                Some(ErrorClass::QueryCompile),
            );
            let differences = semantic_diffs(&[reference.clone(), actual.clone()], false);
            assert_eq!(
                case_verdict(&case, &reference, &actual, &differences).0,
                "match"
            );

            for wrong_actual in [
                error_observation(ToolKind::Tq, ProcessStatus::Exited, Some(0), None),
                error_observation(
                    ToolKind::Tq,
                    ProcessStatus::Exited,
                    Some(2),
                    Some(ErrorClass::QueryCompile),
                ),
                error_observation(
                    ToolKind::Tq,
                    ProcessStatus::Exited,
                    Some(3),
                    Some(ErrorClass::RuntimeTypePath),
                ),
                error_observation(
                    ToolKind::Tq,
                    ProcessStatus::TimedOut,
                    Some(3),
                    Some(ErrorClass::QueryCompile),
                ),
            ] {
                let differences = semantic_diffs(&[reference.clone(), wrong_actual.clone()], false);
                assert_eq!(
                    case_verdict(&case, &reference, &wrong_actual, &differences).0,
                    "failure",
                    "{id} accepted an incorrect error observation"
                );
            }
        }
    }

    fn invalid_math_case(id: &str, query: &str) -> CompatibilityCase {
        serde_json::from_value(json!({
            "schema_version": 1,
            "id": id,
            "title": "Retained invalid math arity probe",
            "classification": "jq-target",
            "capabilities": ["manual.math.invalid-arity"],
            "status": "mvp",
            "fixture": {"format": "json", "inline": "null"},
            "query": query,
            "adapters": {
                "jq": {"supported": true},
                "tq": {"supported": true}
            },
            "invocation_mode": "stdin",
            "expected": {
                "contract": "error",
                "baseline": "required",
                "error_class": "query-compile"
            }
        }))
        .expect("invalid math test case should deserialize")
    }

    fn error_observation(
        tool: ToolKind,
        process_status: ProcessStatus,
        exit_code: Option<i32>,
        error_class: Option<ErrorClass>,
    ) -> ToolObservation {
        ToolObservation {
            tool,
            input_format: Some(FixtureFormat::Json),
            state: ObservationState::Executed,
            results: Vec::new(),
            stdout_hex: None,
            raw_stdout_hex: None,
            stderr_hex: None,
            process_status: Some(process_status),
            exit_code,
            error_class,
            wall_time_micros: Some(0),
            note: None,
        }
    }
}
