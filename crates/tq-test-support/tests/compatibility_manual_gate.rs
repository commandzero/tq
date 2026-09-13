#![allow(missing_docs)]

use std::{collections::BTreeSet, path::Path};

use serde_json::Value;
use tq_test_support::compatibility::{
    CaseStatus, ContractKind, DisparityEvidence, ReviewedDisparity, TqContract,
    apply_reviewed_disparities, case_fingerprint, encode_hex, load_catalog, manual_verdict_counts,
    read_gap_inventory, read_manual_review_case_ids, validate_completion_manual_report,
    validate_gap_inventory, validate_strict_manual_report,
};

#[path = "support/manual_report.rs"]
mod manual_report;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn manual_comparison_cli_requires_an_explicit_approval_file_for_completion() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tq-manual-compare"))
        .arg("--completion")
        .output()
        .expect("run comparison command");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--completion requires an approval file")
    );
    let help = std::process::Command::new(env!("CARGO_BIN_EXE_tq-manual-compare"))
        .arg("--help")
        .output()
        .expect("comparison help");
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--completion APPROVALS.toon"));
    assert!(String::from_utf8_lossy(&help.stdout).contains("--markdown-dir DIRECTORY"));
}

fn historical_baseline_report() -> Value {
    manual_report::historical_baseline_report()
}

fn passing_report() -> Value {
    manual_report::passing_report()
}

#[test]
fn encoding_campaigns_cannot_be_omitted_or_forged_by_a_semantic_match() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    for (field, value) in [
        ("compact", Value::Null),
        ("tq_toon", Value::Null),
        ("toon_contract_match", false.into()),
    ] {
        let mut report = passing_report();
        let row = report["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["contract"] == "result-sequence")
            .unwrap();
        row[field] = value;
        assert!(validate_strict_manual_report(&report, &catalog, &inventory, &review_ids).is_err());
    }
    let mut report = passing_report();
    let row = report["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["contract"] == "result-sequence")
        .unwrap();
    row["compact"]["tq"]["stdout_hex"] = "changed-bytes".into();
    assert!(validate_strict_manual_report(&report, &catalog, &inventory, &review_ids).is_err());
}

#[test]
fn adapter_query_rewrites_cannot_mask_a_manual_language_gap() {
    let (mut catalog, inventory, review_ids, _) = baseline_inputs();
    let case = catalog
        .cases
        .iter_mut()
        .find(|case| case.id == "manual.types.fence-variable-key")
        .unwrap();
    case.adapters.tq.query = Some("null".to_owned());
    let error = validate_strict_manual_report(&passing_report(), &catalog, &inventory, &review_ids)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("manual query must not be rewritten")
    );
}

#[test]
fn strict_gate_rejects_capability_workaround_flags_in_selected_tq_adapters() {
    let (mut catalog, inventory, review_ids, _) = baseline_inputs();
    catalog
        .cases
        .iter_mut()
        .find(|case| case.id == "manual.io.input-filename")
        .expect("manual input filename case")
        .adapters
        .tq
        .args
        .push("--allow-platform".to_owned());
    let error = validate_strict_manual_report(&passing_report(), &catalog, &inventory, &review_ids)
        .expect_err("selected tq cases must not bypass process defaults");
    assert!(
        error
            .to_string()
            .contains("remove capability override --allow-platform")
    );
}

#[test]
fn declared_stderr_payloads_cannot_be_hidden_by_clearing_differences() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let declared = catalog
        .cases
        .iter()
        .find(|case| case.expected.compare_stderr && review_ids.contains(&case.id))
        .unwrap();
    let mut report = passing_report();
    let row = report["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == declared.id)
        .unwrap();
    row["tq"]["stderr_hex"] = "changed-payload".into();
    assert!(
        validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
            .unwrap_err()
            .to_string()
            .contains("explicit stderr payload")
    );
}

fn baseline_inputs() -> (
    tq_test_support::compatibility::CompatibilityCatalog,
    tq_test_support::compatibility::ManualGapInventory,
    BTreeSet<String>,
    Value,
) {
    let root = root();
    let catalog = load_catalog(&root.join("tests/compatibility/cases")).expect("case catalog");
    let inventory =
        read_gap_inventory(&root.join("tests/compatibility/reviews/jq-manual/gap-inventory.toon"))
            .expect("gap inventory");
    let review_ids =
        read_manual_review_case_ids(&root.join("tests/compatibility/reviews/jq-manual"))
            .expect("review case IDs");
    let report = passing_report();
    (catalog, inventory, review_ids, report)
}

#[test]
fn gap_inventory_closes_against_catalog_review_model_and_report() {
    let (catalog, inventory, review_ids, report) = baseline_inputs();
    let report_ids = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["id"].as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();

    let summary = validate_gap_inventory(&inventory, &catalog, &review_ids, &report_ids)
        .expect("all baseline gaps are executable and reviewed");
    assert_eq!(summary.total, 215);
    assert_eq!(summary.failures, 198);
    assert_eq!(summary.expected_differences, 15);
    assert_eq!(summary.reference_discrepancies, 2);
}

#[test]
fn gap_inventory_rejects_duplicate_rows_and_missing_catalog_or_report_ids() {
    let (catalog, mut inventory, review_ids, report) = baseline_inputs();
    let report_ids = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["id"].as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();

    inventory.entries[1].case_id = inventory.entries[0].case_id.clone();
    assert!(
        validate_gap_inventory(&inventory, &catalog, &review_ids, &report_ids)
            .expect_err("duplicate gap IDs must fail")
            .to_string()
            .contains("duplicate case ID")
    );

    let (_, mut inventory, _, _) = baseline_inputs();
    inventory.entries[0].case_id = "manual.deleted-from-catalog".to_owned();
    assert!(
        validate_gap_inventory(&inventory, &catalog, &review_ids, &report_ids)
            .expect_err("a gap absent from the executable catalog must fail")
            .to_string()
            .contains("executable catalog")
    );

    let (_, inventory, _, _) = baseline_inputs();
    let mut missing_report = report_ids;
    missing_report.remove(&inventory.entries[0].case_id);
    assert!(
        validate_gap_inventory(&inventory, &catalog, &review_ids, &missing_report)
            .expect_err("a gap absent from the comparison report must fail")
            .to_string()
            .contains("comparison report")
    );
}

#[test]
fn composition_review_mapping_removal_keeps_strict_gate_red() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let report = passing_report();
    let removed_case = "manual.composition.arity.abs.0";
    let mut missing_mapping = review_ids;
    assert!(missing_mapping.remove(removed_case));
    let error = validate_strict_manual_report(&report, &catalog, &inventory, &missing_mapping)
        .expect_err("removing a supplemental composition mapping must fail the gate");
    let message = error.to_string();
    assert!(message.contains("missing source mapping"));
    assert!(message.contains(removed_case));
}

#[test]
fn strict_gate_rejects_the_baseline_report_instead_of_accepting_old_differences() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let report = historical_baseline_report();
    let error = validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
        .expect_err("baseline gaps must keep the strict campaign red");
    let message = error.to_string();
    assert!(message.contains("manual.math.acos"));
    assert!(message.contains("manual.invoking.version"));
    assert!(message.contains("reference-discrepancy"));
}

#[test]
fn verdict_counts_keep_historical_differences_in_the_failure_bucket() {
    let counts =
        manual_verdict_counts(&historical_baseline_report()).expect("historical verdict counts");
    assert_eq!(counts.exact_matches, 303);
    assert_eq!(counts.reviewed_disparities, 0);
    assert_eq!(counts.failures, 215);
}

#[test]
fn strict_gate_rejects_injected_mismatch_skip_missing_reference_timeout_and_normalization_error() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let baseline = passing_report();
    validate_strict_manual_report(&baseline, &catalog, &inventory, &review_ids)
        .expect("the unmodified synthetic report is a passing campaign");
    for (label, mutate) in [
        (
            "mismatch",
            Box::new(|report: &mut Value| {
                report["cases"][0]["verdict"] = Value::String("match".to_owned());
                report["cases"][0]["differences"] = serde_json::json!([{
                    "summary": "result sequence"
                }]);
            }) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "skip",
            Box::new(|report: &mut Value| {
                report["cases"][0]["tq"]["state"] = Value::String("unsupported".to_owned());
            }),
        ),
        (
            "missing reference",
            Box::new(|report: &mut Value| {
                report["cases"][0].as_object_mut().unwrap().remove("jq");
            }),
        ),
        (
            "timeout",
            Box::new(|report: &mut Value| {
                report["cases"][0]["jq"]["process_status"] = Value::String("timed-out".to_owned());
                report["cases"][0]["jq"]["error_class"] = Value::String("timeout".to_owned());
            }),
        ),
        (
            "normalization",
            Box::new(|report: &mut Value| {
                report["cases"][0]["tq"]["state"] = Value::String("harness-error".to_owned());
            }),
        ),
    ] {
        let mut report = baseline.clone();
        mutate(&mut report);
        let error = validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
            .expect_err("injected strict-campaign failure");
        assert!(
            error.to_string().contains(label),
            "strict gate did not identify injected {label}: {error}"
        );
    }
}

#[test]
fn strict_gate_rejects_invalid_tq_identity_contract_observations() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let assert_rejected = |report: Value, label: &str| {
        let error = validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
            .expect_err("an invalid tq identity observation must fail the strict gate");
        assert!(
            error
                .to_string()
                .contains("tq-native CLI contract assertion"),
            "strict gate did not report {label}: {error}"
        );
    };

    let mut wrong_identity = passing_report();
    let row = wrong_identity["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == "manual.invoking.version")
        .unwrap();
    row["tq"]["raw_stdout_hex"] = encode_hex(b"jq-1.8.1\n").into();
    assert_rejected(wrong_identity, "wrong identity");

    let mut missing_option = passing_report();
    let row = missing_option["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == "manual.invoking.help")
        .unwrap();
    row["tq"]["raw_stdout_hex"] = encode_hex(b"Usage: tq\n--help\n").into();
    assert_rejected(missing_option, "missing documented option");

    let mut failed_status = passing_report();
    let row = failed_status["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == "manual.invoking.version")
        .unwrap();
    row["tq"]["process_status"] = "timed-out".into();
    assert_rejected(failed_status, "failed status");

    let mut stderr = passing_report();
    let row = stderr["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == "manual.invoking.version")
        .unwrap();
    row["tq"]["stderr_hex"] = encode_hex(b"diagnostic\n").into();
    assert_rejected(stderr, "stderr");

    let (mut ordinary_catalog, inventory, review_ids, _) = baseline_inputs();
    ordinary_catalog
        .cases
        .iter_mut()
        .find(|case| case.id == "manual.io.debug")
        .expect("ordinary raw/debug case")
        .expected
        .tq_contract = Some(TqContract::Version);
    let error = validate_strict_manual_report(
        &passing_report(),
        &ordinary_catalog,
        &inventory,
        &review_ids,
    )
    .expect_err("identity contracts must not exempt ordinary raw/debug cases");
    assert!(
        error
            .to_string()
            .contains("invalid tq-native CLI contract association")
    );

    let mut failed_reference = passing_report();
    let row = failed_reference["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == "manual.invoking.version")
        .unwrap();
    row["jq"]["exit_code"] = 1.into();
    let error = validate_strict_manual_report(&failed_reference, &catalog, &inventory, &review_ids)
        .expect_err("identity contracts must not hide a failed reference invocation");
    assert!(error.to_string().contains("reference CLI contract"));
}

#[test]
fn strict_gate_rejects_deleted_source_mapping_as_missing_coverage() {
    let (catalog, inventory, review_ids, report) = baseline_inputs();
    let mut deleted = review_ids.clone();
    deleted.remove("manual.math.acos");
    let error = validate_strict_manual_report(&report, &catalog, &inventory, &deleted)
        .expect_err("deleted source mapping must block the campaign");
    assert!(error.to_string().contains("missing source mapping"));
}

#[test]
fn strict_gate_pins_the_518_case_denominator_including_reused_witnesses() {
    let (mut catalog, inventory, mut review_ids, _) = baseline_inputs();
    let mut report = passing_report();
    catalog.cases.retain(|case| case.id != "date.strptime");
    assert!(review_ids.remove("date.strptime"));
    report["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|case| case["id"] != "date.strptime");
    let error = validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
        .expect_err("removing a non-prefixed reused witness must shrink no denominator");
    assert!(
        error
            .to_string()
            .contains("original case missing: date.strptime")
    );
}

#[test]
fn strict_gate_rejects_unknown_source_references_with_an_otherwise_passing_report() {
    let (catalog, inventory, mut review_ids, _) = baseline_inputs();
    let report = passing_report();
    review_ids.insert("manual.unknown-source-reference".to_owned());
    let error = validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
        .expect_err("source references must resolve to catalog cases");
    assert!(error.to_string().contains("unknown source mapping"));
}

#[test]
fn strict_gate_rejects_a_deferred_case_even_when_its_report_row_matches() {
    let (mut catalog, inventory, review_ids, _) = baseline_inputs();
    let report = passing_report();
    let case = catalog
        .cases
        .iter_mut()
        .find(|case| case.id == "date.strptime")
        .unwrap();
    case.status = CaseStatus::Deferred;
    let error = validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
        .expect_err("a deferred case cannot satisfy strict coverage");
    assert!(error.to_string().contains("catalog case is not executable"));
}

fn one_disparity_report() -> Value {
    let mut report = passing_report();
    let catalog = load_catalog(&root().join("tests/compatibility/cases")).expect("case catalog");
    let index = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .position(|case| case["id"] == "manual-bof-prose-halt-error")
        .unwrap();
    let catalog_case = catalog
        .cases
        .iter()
        .find(|case| case.id == "manual-bof-prose-halt-error")
        .unwrap();
    report["cases"][index]["case_fingerprint"] =
        Value::String(case_fingerprint(catalog_case).expect("case fingerprint"));
    report["cases"][index]["contract"] = Value::String("result-sequence".to_owned());
    report["cases"][index]["json_equivalent"] = Value::Bool(true);
    report["cases"][index]["verdict"] = Value::String("failure".to_owned());
    report["cases"][index]["differences"] = serde_json::json!([{
        "summary": "result sequence"
    }]);
    report
}

fn reviewed_disparity(report: &Value) -> ReviewedDisparity {
    let index = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .position(|case| case["id"] == "manual-bof-prose-halt-error")
        .unwrap();
    ReviewedDisparity {
        case_id: "manual-bof-prose-halt-error".to_owned(),
        contract: ContractKind::ResultSequence,
        difference_summary: "result sequence".to_owned(),
        rationale: "Measured safe-library limitation with a retained regression witness."
            .to_owned(),
        evidence: DisparityEvidence::from_report(report, &report["cases"][index])
            .expect("disparity evidence"),
    }
}

#[test]
fn reviewed_disparity_requires_exact_observed_contract_and_stays_out_of_exact_count() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let completion_report = one_disparity_report();
    let mut report = completion_report.clone();
    let approval = reviewed_disparity(&completion_report);
    apply_reviewed_disparities(&mut report, std::slice::from_ref(&approval))
        .expect("matching case, contract, and observed difference are reviewable");
    let counts = manual_verdict_counts(&report).expect("verdict counts");
    assert_eq!(counts.exact_matches, review_ids.len() - 1);
    assert_eq!(counts.reviewed_disparities, 1);
    assert_eq!(counts.failures, 0);
    validate_completion_manual_report(
        &completion_report,
        &catalog,
        &inventory,
        &review_ids,
        &[approval],
    )
    .expect("completion mode can account for an explicit reviewed disparity");
    let strict_error = validate_strict_manual_report(&report, &catalog, &inventory, &review_ids)
        .expect_err("exact mode must remain red for a disparity");
    assert!(strict_error.to_string().contains("verdict disparity"));
}

#[test]
fn reviewed_disparity_rejects_unknown_stale_duplicate_and_broad_approvals() {
    let mut report = one_disparity_report();
    let unknown = ReviewedDisparity {
        case_id: "manual.unknown".to_owned(),
        ..reviewed_disparity(&report)
    };
    assert!(apply_reviewed_disparities(&mut report, &[unknown]).is_err());

    let stale = ReviewedDisparity {
        difference_summary: "raw stderr".to_owned(),
        ..reviewed_disparity(&report)
    };
    assert!(apply_reviewed_disparities(&mut report, &[stale]).is_err());

    let mut changed_observation = one_disparity_report();
    let observation_approval = reviewed_disparity(&changed_observation);
    let index = changed_observation["cases"]
        .as_array()
        .unwrap()
        .iter()
        .position(|case| case["id"] == "manual-bof-prose-halt-error")
        .unwrap();
    changed_observation["cases"][index]["tq"]["results"] =
        serde_json::json!(["changed-after-review"]);
    assert!(
        apply_reviewed_disparities(&mut changed_observation, &[observation_approval]).is_err(),
        "same summary cannot approve changed stable observation content"
    );

    let duplicate = reviewed_disparity(&report);
    assert!(apply_reviewed_disparities(&mut report, &[duplicate.clone(), duplicate]).is_err());

    let broad = ReviewedDisparity {
        case_id: String::new(),
        difference_summary: String::new(),
        rationale: String::new(),
        ..reviewed_disparity(&report)
    };
    assert!(apply_reviewed_disparities(&mut report, &[broad]).is_err());
}

#[test]
fn reviewed_disparity_binds_every_independent_output_observation() {
    for role in ["tq_toon", "compact-jq", "compact-tq"] {
        let mut report = one_disparity_report();
        let approval = reviewed_disparity(&report);
        let row = report["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["id"] == approval.case_id)
            .unwrap();
        let observation = match role {
            "tq_toon" => &mut row["tq_toon"],
            "compact-jq" => &mut row["compact"]["jq"],
            "compact-tq" => &mut row["compact"]["tq"],
            _ => unreachable!(),
        };
        observation["stdout_hex"] = "6368616e676564".into();
        assert!(
            apply_reviewed_disparities(&mut report, &[approval]).is_err(),
            "an approval must become stale when {role} output changes"
        );
    }
}

#[test]
fn completion_gate_rejects_independent_campaign_timeout_or_missing_observation() {
    for mutation in ["timeout", "missing"] {
        let (catalog, inventory, review_ids, _) = baseline_inputs();
        let mut report = one_disparity_report();
        let approval = reviewed_disparity(&report);
        let row = report["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["id"] == approval.case_id)
            .unwrap();
        if mutation == "timeout" {
            row["compact"]["jq"]["state"] = "executed".into();
            row["compact"]["jq"]["process_status"] = "timed-out".into();
        } else {
            row["compact"]["jq"] = Value::Null;
        }
        assert!(
            validate_completion_manual_report(
                &report,
                &catalog,
                &inventory,
                &review_ids,
                &[approval],
            )
            .is_err(),
            "completion must reject an independent {mutation} observation"
        );
    }
}

#[test]
fn completion_gate_does_not_treat_existing_disparity_as_approved_without_input() {
    let (catalog, inventory, review_ids, report) = baseline_inputs();
    let mut report = report;
    report["cases"][0]["verdict"] = Value::String("disparity".to_owned());
    let error = validate_completion_manual_report(&report, &catalog, &inventory, &review_ids, &[])
        .expect_err("completion mode requires explicit disparity approvals");
    assert!(error.to_string().contains("before explicit approvals"));
}

#[test]
fn completion_gate_rejects_unlisted_embedded_disparity_alongside_a_valid_approval() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let mut report = one_disparity_report();
    let approval = reviewed_disparity(&report);
    let row = report["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] != approval.case_id)
        .unwrap();
    let catalog_case = catalog
        .cases
        .iter()
        .find(|case| row["id"] == case.id)
        .unwrap();
    row["case_fingerprint"] = Value::String(case_fingerprint(catalog_case).unwrap());
    row["verdict"] = Value::String("disparity".to_owned());
    row["reviewed_disparity"] = serde_json::json!({"forged": true});
    assert!(
        validate_completion_manual_report(&report, &catalog, &inventory, &review_ids, &[approval])
            .is_err()
    );
}

#[test]
fn completion_gate_can_approve_a_new_reviewed_source_witness() {
    let (catalog, inventory, review_ids, _) = baseline_inputs();
    let mut report = one_disparity_report();
    let original_approval = reviewed_disparity(&report);
    let index = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| row["id"] == "manual.audit.streaming.multiple-reconstruction")
        .unwrap();
    let id = report["cases"][index]["id"].as_str().unwrap().to_owned();
    let catalog_case = catalog.cases.iter().find(|case| case.id == id).unwrap();
    report["cases"][index]["case_fingerprint"] =
        Value::String(case_fingerprint(catalog_case).unwrap());
    report["cases"][index]["verdict"] = Value::String("failure".to_owned());
    report["cases"][index]["differences"] = serde_json::json!([{"summary":"result sequence"}]);
    let approval = ReviewedDisparity {
        case_id: id,
        contract: serde_json::from_value(report["cases"][index]["contract"].clone()).unwrap(),
        evidence: DisparityEvidence::from_report(&report, &report["cases"][index]).unwrap(),
        ..original_approval.clone()
    };
    validate_completion_manual_report(
        &report,
        &catalog,
        &inventory,
        &review_ids,
        &[original_approval, approval],
    )
    .expect("reviewed source witnesses may be approved beyond the frozen gap rows");
}

#[test]
fn reviewed_disparity_accepts_exact_decimal_spellings() {
    let expanded_max = expanded_finite_max();
    let scientific_max: Value =
        serde_json::from_str("1.7976931348623157e+308").expect("scientific finite maximum");
    let expanded_max: Value = serde_json::from_str(&expanded_max).expect("expanded maximum");
    let mut report = one_disparity_report();
    let report_case_id = "manual-bof-prose-halt-error";
    let row = report["cases"]
        .as_array_mut()
        .expect("report cases")
        .iter_mut()
        .find(|case| case["id"] == report_case_id)
        .expect("synthetic disparity report row");
    let scientific_results = serde_json::json!([[0, scientific_max, 0, 0, 0]]);
    row["jq"]["results"] = scientific_results.clone();
    row["compact"]["jq"]["results"] = scientific_results;
    let mut approval = reviewed_disparity(&report);
    approval.evidence.jq.results[0][1] = expanded_max.clone();
    approval
        .evidence
        .compact_jq
        .as_mut()
        .expect("compact jq evidence")
        .results[0][1] = expanded_max;

    let report_case = report["cases"]
        .as_array()
        .expect("report cases")
        .iter()
        .find(|case| case["id"] == approval.case_id)
        .expect("integer-scale boundary report row");
    let observed = DisparityEvidence::from_report(&report, report_case)
        .expect("raw report disparity evidence");
    assert_eq!(approval.evidence, observed);

    apply_reviewed_disparities(&mut report, std::slice::from_ref(&approval))
        .expect("the checked-in approval must round-trip against its raw report");

    let assert_stale = |mut changed: Value, reason: &str| {
        assert!(
            apply_reviewed_disparities(&mut changed, std::slice::from_ref(&approval)).is_err(),
            "approval must become stale after {reason}"
        );
    };
    let mut changed = report.clone();
    let mut max_plus_one = expanded_finite_max().into_bytes();
    *max_plus_one.last_mut().expect("maximum digits") = b'1';
    changed["cases"]
        .as_array_mut()
        .expect("report cases")
        .iter_mut()
        .find(|case| case["id"] == approval.case_id)
        .expect("integer-scale boundary report row")["jq"]["results"][0][1] =
        serde_json::from_str(&String::from_utf8(max_plus_one).expect("maximum UTF-8"))
            .expect("maximum plus one");
    assert_stale(changed, "the expanded maximum changed by one");

    let mut changed = report.clone();
    changed["cases"]
        .as_array_mut()
        .expect("report cases")
        .iter_mut()
        .find(|case| case["id"] == approval.case_id)
        .expect("integer-scale boundary report row")["jq"]["results"][0][0] =
        Value::Number(serde_json::Number::from_f64(-0.0).expect("negative zero"));
    assert_stale(changed, "the signed zero changed");

    let mut changed = report.clone();
    let row = changed["cases"]
        .as_array_mut()
        .expect("report cases")
        .iter_mut()
        .find(|case| case["id"] == approval.case_id)
        .expect("integer-scale boundary report row");
    let first = row["jq"]["results"][0][0].clone();
    row["jq"]["results"][0][0] = row["jq"]["results"][0][1].clone();
    row["jq"]["results"][0][1] = first;
    assert_stale(changed, "the result array order changed");

    let mut changed = report.clone();
    changed["cases"]
        .as_array_mut()
        .expect("report cases")
        .iter_mut()
        .find(|case| case["id"] == approval.case_id)
        .expect("integer-scale boundary report row")["jq"]["results"][0][1] =
        Value::String("1.7976931348623157e+308".to_owned());
    assert_stale(changed, "the result type changed");

    let mut changed = report.clone();
    changed["cases"]
        .as_array_mut()
        .expect("report cases")
        .iter_mut()
        .find(|case| case["id"] == approval.case_id)
        .expect("integer-scale boundary report row")["jq"]["results"][0][1] =
        serde_json::from_str("1e1000000001").expect("large exponent");
    assert_stale(changed, "the exponent changed");

    let mut changed = report;
    changed["cases"]
        .as_array_mut()
        .expect("report cases")
        .iter_mut()
        .find(|case| case["id"] == approval.case_id)
        .expect("integer-scale boundary report row")["jq"]["stdout_hex"] =
        Value::String("00".to_owned());
    assert_stale(changed, "the exact stdout bytes changed");
}

fn expanded_finite_max() -> String {
    let mut value = String::from("17976931348623157");
    value.extend(std::iter::repeat_n('0', 292));
    value
}
