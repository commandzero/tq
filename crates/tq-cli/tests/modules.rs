//! Public process contracts for jq-compatible modules and data imports.

use std::{
    fs,
    io::Write,
    ops::Deref,
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::{Builder, TempDir};

fn run(args: &[&str], input: &[u8], home: &std::path::Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tq"))
        .args(args)
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("HOME", home)
        .env_remove("JQ_LIBRARY_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tq module test process");
    child
        .stdin
        .take()
        .expect("module test stdin")
        .write_all(input)
        .expect("write module test input");
    child.wait_with_output().expect("collect tq module output")
}

struct TempRoot(TempDir);

impl Deref for TempRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.path()
    }
}

impl AsRef<Path> for TempRoot {
    fn as_ref(&self) -> &Path {
        self.0.path()
    }
}

fn temp_root(name: &str) -> TempRoot {
    TempRoot(
        Builder::new()
            .prefix(&format!("tq-{name}-"))
            .tempdir()
            .expect("module test root creates"),
    )
}

#[test]
fn json_data_import_uses_the_variable_namespace() {
    let root = temp_root("json-data-module");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("module test home creates");
    fs::write(root.join("data.json"), br#"{"name":"manual","count":3}"#)
        .expect("data module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "data" as $data; $data::data"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        br#"[{"name":"manual","count":3}]
"#
    );
}

#[test]
fn json_data_import_preserves_runtime_number_types() {
    let root = temp_root("json-data-runtime-numbers");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("module test home creates");
    fs::write(root.join("numbers.json"), b"NaN Infinity -Infinity\n")
        .expect("runtime-number data module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "numbers" as $numbers; $numbers::numbers | map({type: type, isnan: isnan, isinfinite: isinfinite})"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"[{\"type\":\"number\",\"isnan\":true,\"isinfinite\":false},{\"type\":\"number\",\"isnan\":false,\"isinfinite\":true},{\"type\":\"number\",\"isnan\":false,\"isinfinite\":true}]\n"
    );
}

#[test]
fn modulemeta_loads_requested_and_input_derived_modules_without_import() {
    let root = temp_root("modulemeta-dynamic");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("modulemeta home creates");
    let expected = br#"{"homepage":"https://example.invalid/basic","deps":[],"defs":["value/0"]}
"#;
    for (query, input, null_input) in [
        (r#""basic" | modulemeta"#, b"".as_slice(), true),
        ("modulemeta", b"\"basic\"\n".as_slice(), false),
    ] {
        let mut args = vec![
            "-L",
            "tests/fixtures/manual-modules",
            "--output-format",
            "json",
            "--compact-output",
        ];
        if null_input {
            args.push("-n");
        }
        args.push(query);
        let output = run(&args, input, &home);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected);
    }
}

#[test]
fn dynamic_modulemeta_shares_the_module_count_budget_across_requests() {
    let root = temp_root("modulemeta-budget");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("modulemeta budget home creates");
    fs::write(root.join("first.jq"), "module {homepage:\"first\"};\n")
        .expect("first dynamic module writes");
    fs::write(root.join("second.jq"), "module {homepage:\"second\"};\n")
        .expect("second dynamic module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "--max-depth",
            "1",
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"("first", "second") | modulemeta"#,
        ],
        b"",
        &home,
    );
    assert!(!output.status.success());
    assert_eq!(
        output.stdout,
        br#"{"homepage":"first","deps":[],"defs":[]}
"#
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("module count limit"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn module_metadata_search_substitutes_only_prefix_tokens() {
    let root = temp_root("module-metadata-prefixes");
    let home = root.join("home");
    fs::create_dir_all(root.join("origin-modules")).expect("origin metadata root creates");
    fs::create_dir_all(home.join("home-modules")).expect("home metadata root creates");
    fs::create_dir_all(root.join("literal~/$ORIGIN")).expect("literal metadata root creates");
    fs::write(
        root.join("origin.jq"),
        "module {search:\"$ORIGIN/origin-modules\"}; import \"value\" as v; def value: v::value;\n",
    )
    .expect("origin metadata module writes");
    fs::write(
        root.join("home-module.jq"),
        "module {search:\"~/home-modules\"}; import \"value\" as v; def value: v::value;\n",
    )
    .expect("home metadata module writes");
    fs::write(
        root.join("literal.jq"),
        "module {search:\"./literal~/$ORIGIN\"}; import \"value\" as v; def value: v::value;\n",
    )
    .expect("literal metadata module writes");
    for (directory, value) in [
        (root.join("origin-modules"), "origin"),
        (home.join("home-modules"), "home"),
        (root.join("literal~/$ORIGIN"), "literal"),
    ] {
        fs::write(
            directory.join("value.jq"),
            format!("def value: {value:?};\n"),
        )
        .expect("metadata dependency writes");
    }
    let root_text = root.to_string_lossy().into_owned();
    for (module, expected) in [
        ("origin", "\"origin\"\n"),
        ("home-module", "\"home\"\n"),
        ("literal", "\"literal\"\n"),
    ] {
        let output = run(
            &[
                "-L",
                &root_text,
                "-n",
                "--output-format",
                "json",
                "--compact-output",
                &format!("import \"{module}\" as m; m::value"),
            ],
            b"",
            &home,
        );
        assert!(
            output.status.success(),
            "module {module}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected.as_bytes());
    }
}

#[test]
fn explicit_empty_library_path_keeps_later_explicit_roots() {
    let root = temp_root("module-library-terminator");
    let first = root.join("first");
    let second = root.join("second");
    let home = root.join("home");
    fs::create_dir_all(&first).expect("first library root creates");
    fs::create_dir_all(&second).expect("second library root creates");
    fs::create_dir_all(&home).expect("terminator home creates");
    fs::write(second.join("value.jq"), "def value: \"second\";\n")
        .expect("second library module writes");
    let first_text = first.to_string_lossy().into_owned();
    let second_text = second.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &first_text,
            "-L",
            "",
            "-L",
            &second_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "value" as v; v::value"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\"second\"\n");
}

#[test]
fn json_data_import_namespace_uses_alias_not_filename() {
    let root = temp_root("json-data-alias");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("data alias home creates");
    fs::create_dir_all(root.join("nested")).expect("data alias nested root creates");
    fs::write(root.join("input-data.json"), br#"{"x":1}"#).expect("data alias module writes");
    fs::write(root.join("nested/input-data.json"), br#"{"x":2}"#)
        .expect("nested data alias module writes");
    let root_text = root.to_string_lossy().into_owned();
    for (query, expected) in [
        (
            r#"import "input-data" as $d; $d::d"#,
            b"[{\"x\":1}]\n".as_slice(),
        ),
        (
            r#"import "nested/input-data" as $d; $d::d"#,
            b"[{\"x\":2}]\n".as_slice(),
        ),
    ] {
        let output = run(
            &[
                "-L",
                &root_text,
                "-n",
                "--output-format",
                "json",
                "--compact-output",
                query,
            ],
            b"",
            &home,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected);
    }
}

#[test]
fn module_suffix_is_appended_after_existing_filename_dots() {
    let root = temp_root("module-suffix");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("suffix home creates");
    fs::write(root.join("versioned.name.jq"), "def answer: 7;\n").expect("dotted module writes");
    fs::write(root.join("versioned.name.json"), br#"{"x":7}"#).expect("dotted data module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "versioned.name" as v; v::answer"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"7\n");
    let output = run(
        &[
            "-L",
            &root_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "versioned.name" as $d; $d::d"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[{\"x\":7}]\n");
}

#[test]
fn json_data_import_slurps_multiple_values() {
    let root = temp_root("json-data-multiple");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("module test home creates");
    fs::write(
        root.join("multi.json"),
        br#"{"x":1}
{"x":2}
"#,
    )
    .expect("multi-value data module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "multi" as $data; $data::data"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[{\"x\":1},{\"x\":2}]\n");
}

#[test]
fn empty_json_data_import_is_an_empty_array() {
    let root = temp_root("json-data-empty");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("module test home creates");
    fs::write(root.join("empty.json"), b"").expect("empty data module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "empty" as $data; $data::data"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[]\n");
}

#[test]
fn json_data_import_enforces_the_module_byte_limit_before_parsing() {
    let root = temp_root("json-data-limit");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("module limit home creates");
    let payload = format!("\"{}\"", "x".repeat(300));
    fs::write(root.join("large.json"), payload).expect("large data module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "--max-input-bytes",
            "256",
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "large" as $data; $data::data"#,
        ],
        b"",
        &home,
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("TQ-RESOURCE-MODULE-BYTES-001"), "{stderr}");
}

#[test]
fn startup_file_is_loaded_from_controlled_home() {
    let root = temp_root("startup-module");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("startup home creates");
    fs::write(home.join(".jq"), "def home_value: \"auto-home\";\n").expect("startup file writes");
    let output = run(
        &[
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            "home_value",
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\"auto-home\"\n");
}

#[test]
fn startup_and_query_location_metadata_keep_distinct_sources() {
    let root = temp_root("startup-location");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("startup location home creates");
    let startup = home.join(".jq");
    fs::write(
        &startup,
        "def startup_location: $__loc__;\ndef startup_value: 1;\n",
    )
    .expect("startup location writes");
    let output = run(
        &[
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            "[$__loc__, startup_location]",
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let startup_location = serde_json::json!({
        "file": startup.display().to_string(),
        "line": 1,
    });
    let expected = serde_json::to_vec(&serde_json::json!([
        {"file":"<top-level>","line":1},
        startup_location,
    ]))
    .expect("location result serializes");
    assert_eq!(output.stdout, [expected, b"\n".to_vec()].concat());
}

#[test]
fn default_search_path_uses_controlled_home_jq_directory() {
    let root = temp_root("default-module-search");
    let home = root.join("home");
    let library = home.join(".jq");
    fs::create_dir_all(&library).expect("default module home creates");
    fs::write(library.join("default.jq"), "def value: \"default-home\";\n")
        .expect("default module writes");
    let output = run(
        &[
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "default" as d; d::value"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\"default-home\"\n");
}

#[test]
fn library_path_substitutions_use_the_tq_executable_origin() {
    let executable = Path::new(env!("CARGO_BIN_EXE_tq"));
    let executable_dir = executable
        .parent()
        .expect("tq executable has a containing directory");
    let origin_fixture = Builder::new()
        .prefix("tq-origin-module-search-")
        .tempdir_in(executable_dir)
        .expect("origin module fixture creates beside tq");
    fs::write(
        origin_fixture.path().join("foo.jq"),
        "def value: \"origin-path\";\n",
    )
    .expect("origin module fixture writes");
    let origin_directory = origin_fixture
        .path()
        .file_name()
        .expect("origin module fixture has a directory name")
        .to_string_lossy()
        .into_owned();
    let temp = temp_root("origin-module-search");
    let home = temp.join("home");
    fs::create_dir_all(&home).expect("origin module home creates");
    let output = run(
        &[
            "-L",
            &format!("$ORIGIN/{origin_directory}"),
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "foo" as f; f::value"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\"origin-path\"\n");
    assert!(
        origin_fixture.path().exists(),
        "origin fixture remains available"
    );
}

#[test]
fn empty_module_search_metadata_stops_enclosing_roots() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/manual-modules");
    let temp = temp_root("module-search-terminator");
    let home = temp.join("home");
    fs::create_dir_all(&home).expect("terminator module home creates");
    let first = root
        .join("roots/termination/first")
        .to_string_lossy()
        .into_owned();
    let second = root
        .join("roots/termination/second")
        .to_string_lossy()
        .into_owned();
    let output = run(
        &[
            "-L",
            &first,
            "-L",
            "",
            "-L",
            &second,
            "-n",
            r#"import "consumer" as c; c::value"#,
        ],
        b"",
        &home,
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("TQ-MODULE-NOT-FOUND-001"), "{stderr}");
}

#[test]
fn null_module_search_metadata_terminates_after_prior_candidates() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = temp_root("module-search-null-terminator");
    let home = temp.join("home");
    fs::create_dir_all(&home).expect("null terminator home creates");
    let output = run(
        &[
            "-L",
            "tests/fixtures/manual-modules",
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "basic" as b {search:["tests/fixtures/manual-modules",null]}; b::value"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"42\n");
    assert!(root.join("tests/fixtures/manual-modules/basic.jq").exists());
}

#[test]
fn metadata_search_and_dependency_shape_are_preserved() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/manual-modules");
    let home = temp_root("metadata-home");
    let search_root = root.join("roots/search-prefix");
    let search_text = search_root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &search_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "meta/consumer" as c; "meta/consumer" | modulemeta"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        br#"{"deps":[{"search":"..","as":"helper","is_data":false,"relpath":"helper"}],"defs":["value/0"]}
"#
    );
}

#[test]
fn location_magic_in_a_module_keeps_module_filename_and_line() {
    let root = temp_root("module-location");
    let home = root.join("home");
    fs::create_dir_all(&home).expect("module location home creates");
    fs::write(root.join("loc.jq"), "def where: \"\\($__loc__)\";\n")
        .expect("location module writes");
    let root_text = root.to_string_lossy().into_owned();
    let output = run(
        &[
            "-L",
            &root_text,
            "-n",
            "--output-format",
            "json",
            "--compact-output",
            r#"import "loc" as l; l::where"#,
        ],
        b"",
        &home,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let location: String = serde_json::from_slice(&output.stdout).expect("location is JSON text");
    assert!(location.contains("loc.jq"), "{location}");
    assert!(location.ends_with("\"line\":1}"), "{location}");
}
