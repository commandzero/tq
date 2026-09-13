//! jq manual argument contracts observed through the process interface.

use std::{fs, path::PathBuf, process::Command};

use tempfile::tempdir;

fn manual_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/manual-invoking")
        .join(name)
}

#[test]
fn negative_numeric_filters_are_not_options() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "-1"])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"-1\n");
}

#[test]
fn malformed_json_arguments_are_usage_errors_before_input_is_opened() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-c",
            "--argjson",
            "x",
            "not-json",
            "$x",
            "must-not-open-missing-input",
        ])
        .output()
        .expect("run tq");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("must-not-open-missing-input"));
}

#[test]
fn json_arguments_preserve_nonfinite_runtime_number_types() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-nc",
            "--argjson",
            "x",
            "NaN",
            "[$x|type, ($x|isnan), ($x|isinfinite)]",
        ])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"number\",true,false]\n");
}

#[test]
fn negative_json_arguments_are_values_not_options() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "--jsonargs", "$ARGS.positional", "-1", "-0.5"])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[-1,-0.5]\n");
}

#[test]
fn jsonargs_preserve_nonfinite_runtime_number_types() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-nc",
            "--jsonargs",
            "$ARGS.positional | map({type: type, isnan: isnan, isinfinite: isinfinite})",
            "NaN",
            "Infinity",
        ])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"[{\"type\":\"number\",\"isnan\":true,\"isinfinite\":false},{\"type\":\"number\",\"isnan\":false,\"isinfinite\":true}]\n"
    );
}

#[test]
fn option_values_and_double_dash_arguments_are_not_short_option_bundles() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-nc",
            "--arg",
            "x",
            "-raw",
            "--args",
            "[$x,$ARGS.positional]",
            "--",
            "-literal",
        ])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"-raw\",[\"-literal\"]]\n");
}

#[test]
fn positional_argument_mode_still_parses_options_until_double_dash() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-nc",
            "--args",
            "$ARGS",
            "--arg",
            "x",
            "value",
            "foo",
            "--",
            "--literal",
        ])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Object member order is not part of this argument contract.
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON result");
    assert_eq!(
        value,
        serde_json::json!({"named":{"x":"value"},"positional":["foo","--literal"]})
    );
}

#[test]
fn repeated_named_arguments_keep_the_first_binding() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-nc",
            "--arg",
            "x",
            "first",
            "--arg",
            "x",
            "second",
            "[$x,$ARGS.named.x]",
        ])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"first\",\"first\"]\n");
}

#[test]
fn named_arguments_accept_names_that_are_not_filter_identifiers() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-nc",
            "--arg",
            "foo-bar",
            "value",
            "--arg",
            "",
            "empty",
            "--arg",
            "💡",
            "light",
            "$ARGS.named",
        ])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        "{\"foo-bar\":\"value\",\"\":\"empty\",\"💡\":\"light\"}\n".as_bytes()
    );
}

#[test]
fn mixed_positional_modes_decode_each_value_at_consumption_time() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args([
            "-nc",
            "--args",
            "$ARGS.positional",
            "word",
            "--jsonargs",
            "1",
        ])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"word\",1]\n");
}

#[test]
fn rawfile_binds_complete_utf8_contents() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "--rawfile", "foo"])
        .arg(manual_fixture("bar"))
        .arg("$foo")
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON result");
    assert_eq!(value, serde_json::json!("{\"a\":1}\n\"two\"\n"));
}

#[test]
fn slurpfile_binds_ordered_json_texts() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "--slurpfile", "foo"])
        .arg(manual_fixture("bar"))
        .arg("$foo")
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON result");
    assert_eq!(value, serde_json::json!([{ "a": 1 }, "two"]));
}

#[test]
fn slurpfile_preserves_nonfinite_runtime_number_types() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("nonfinite.json");
    fs::write(&path, b"NaN\nInfinity\n").expect("write nonfinite JSON");
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "--slurpfile", "numbers"])
        .arg(&path)
        .arg("$numbers | map({type: type, isnan: isnan, isinfinite: isinfinite})")
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"[{\"type\":\"number\",\"isnan\":true,\"isinfinite\":false},{\"type\":\"number\",\"isnan\":false,\"isinfinite\":true}]\n"
    );
}

#[test]
fn invalid_slurpfile_fails_without_consuming_input() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("invalid.json");
    fs::write(&path, b"{not-json").expect("write invalid JSON");
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "--slurpfile", "foo"])
        .arg(&path)
        .args(["$foo", "missing-input.json"])
        .output()
        .expect("run tq");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("missing-input.json"));
}

#[test]
fn rawfile_replaces_invalid_utf8_like_jq() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("invalid-utf8");
    fs::write(&path, [0xff, 0xfe]).expect("write invalid UTF-8");
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "--rawfile", "foo"])
        .arg(&path)
        .args(["$foo", "missing-input.json"])
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON result");
    assert_eq!(value, serde_json::json!("��"));
}

#[test]
fn duplicate_rawfile_name_ignores_second_path_without_reading_it() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-nc", "--rawfile", "foo"])
        .arg(manual_fixture("bar"))
        .args(["--rawfile", "foo"])
        .arg(manual_fixture("missing"))
        .arg("$foo")
        .output()
        .expect("run tq");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON result");
    assert_eq!(value, serde_json::json!("{\"a\":1}\n\"two\"\n"));
}

#[test]
fn removed_argfile_form_is_rejected_before_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(["-n", "--argfile", "foo", "missing.json", "$foo"])
        .output()
        .expect("run tq");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported option '--argfile'"));
}
