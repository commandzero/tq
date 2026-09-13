//! Executable contracts for the corrected Stack Overflow benchmark scenarios.

#![allow(missing_docs)]

use std::path::Path;

use serde::Deserialize;
use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

#[derive(Debug, Deserialize)]
struct Scenario {
    benchmark: Benchmark,
}

#[derive(Debug, Deserialize)]
struct Benchmark {
    query: String,
    input: serde_json::Value,
    #[serde(default)]
    output_mode: Option<String>,
}

struct ScenarioContract {
    file: &'static str,
    expected_json: &'static str,
    old_query: Option<&'static str>,
    output_mode: Option<&'static str>,
    raw_expected: Option<&'static [u8]>,
}

const CONTRACTS: &[ScenarioContract] = &[
    ScenarioContract {
        file: "23-how-to-format-a-json-string-as-a-table-using-jq.toon",
        expected_json: r#"["12\tGeorge","18\tJack","19\tJoe"]"#,
        old_query: Some(".[] | [.id, .name]"),
        output_mode: Some("raw"),
        raw_expected: Some(b"12\tGeorge\n18\tJack\n19\tJoe\n"),
    },
    ScenarioContract {
        file: "24-jq-print-key-and-value-for-each-entry-in-an-object.toon",
        expected_json: r#"["host1, 10.1.2.3","host2, 10.1.2.2","host3, 10.1.18.1"]"#,
        old_query: Some("keys[]"),
        output_mode: Some("raw"),
        raw_expected: Some(b"host1, 10.1.2.3\nhost2, 10.1.2.2\nhost3, 10.1.18.1\n"),
    },
    ScenarioContract {
        file: "38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.toon",
        expected_json: r#"[{"a":"red","b":"white","c":"purple"}]"#,
        old_query: Some("map(.a)"),
        output_mode: None,
        raw_expected: None,
    },
    ScenarioContract {
        file: "39-jq-not-working-on-tag-name-with-dashes-and-numbers.toon",
        expected_json: r#"[[{"status-code":200,"component":"Service1","status":"OK"},{"status-code":200,"component":"Service2","status":"OK"}]]"#,
        old_query: Some(".status"),
        output_mode: None,
        raw_expected: None,
    },
    ScenarioContract {
        file: "40-convert-string-to-json-in-jq.toon",
        expected_json: r#"["Hello World"]"#,
        old_query: Some(".response.text"),
        output_mode: None,
        raw_expected: None,
    },
    ScenarioContract {
        file: "42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.toon",
        expected_json: r#"["three"]"#,
        old_query: Some(".[] | .name"),
        output_mode: None,
        raw_expected: None,
    },
    ScenarioContract {
        file: "43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.toon",
        expected_json: r#"[{"name":"jq","color":"terminal"}]"#,
        old_query: None,
        output_mode: Some("color"),
        raw_expected: None,
    },
    ScenarioContract {
        file: "44-exclude-column-from-jq-json-output.toon",
        expected_json: r#"[[{"group":"employees","uid":"elgalu"},{"group":"employees","uid":"mike"},{"group":"services","uid":"pacts"}]]"#,
        old_query: Some(".[].group"),
        output_mode: None,
        raw_expected: None,
    },
    ScenarioContract {
        file: "45-how-to-use-jq-when-the-variable-has-reserved-characters.toon",
        expected_json: r#"[{"volume24":0.932166,"price":0.09995,"updated":"2016-05-04T03:03:29.000Z"}]"#,
        old_query: Some(".USD.price"),
        output_mode: None,
        raw_expected: None,
    },
];

#[test]
fn corrected_scenarios_execute_the_selected_answers() {
    let catalog = load_catalog();
    for contract in CONTRACTS {
        let scenario = load_scenario(contract.file);
        let catalog_entry = catalog
            .get(catalog_rank(contract.file))
            .unwrap_or_else(|| panic!("missing aggregate entry for {}", contract.file));
        assert_eq!(
            catalog_entry.query, scenario.benchmark.query,
            "aggregate query for {}",
            contract.file
        );
        assert_eq!(
            catalog_entry.input, scenario.benchmark.input,
            "aggregate input for {}",
            contract.file
        );
        assert_eq!(
            effective_output_mode(catalog_entry.output_mode.as_deref()),
            effective_output_mode(scenario.benchmark.output_mode.as_deref()),
            "aggregate output mode for {}",
            contract.file
        );
        assert_eq!(
            effective_output_mode(scenario.benchmark.output_mode.as_deref()),
            effective_output_mode(contract.output_mode),
            "output mode contract for {}",
            contract.file
        );

        let expected_json: serde_json::Value =
            serde_json::from_str(contract.expected_json).expect("valid expected JSON");
        let serde_json::Value::Array(expected_json) = expected_json else {
            panic!("expected output must be a JSON array: {}", contract.file);
        };
        let expected = expected_json
            .into_iter()
            .map(|value| Value::from_json(value).expect("valid expected JSON"))
            .collect::<Vec<_>>();
        let actual = evaluate(&scenario.benchmark.query, &scenario.benchmark.input);
        assert_eq!(actual, expected, "corrected result for {}", contract.file);

        if let Some(old_query) = contract.old_query {
            let old = evaluate(old_query, &scenario.benchmark.input);
            assert_ne!(
                old, actual,
                "the old incorrect query must not satisfy {}",
                contract.file
            );
        }

        if let Some(expected_bytes) = contract.raw_expected {
            assert_eq!(
                raw_bytes(&actual),
                expected_bytes,
                "raw output bytes for {}",
                contract.file
            );
        }
    }
}

fn load_scenario(file: &str) -> Scenario {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/stack-overflow")
        .join(file);
    tq_test_support::fixture_data::read(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn load_catalog() -> Vec<Benchmark> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/stack-overflow-benchmarks.toon");
    tq_test_support::fixture_data::read(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn effective_output_mode(mode: Option<&str>) -> &str {
    mode.unwrap_or("structured")
}

fn catalog_rank(file: &str) -> usize {
    match file {
        "23-how-to-format-a-json-string-as-a-table-using-jq.toon" => 22,
        "24-jq-print-key-and-value-for-each-entry-in-an-object.toon" => 23,
        "38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.toon" => 37,
        "39-jq-not-working-on-tag-name-with-dashes-and-numbers.toon" => 38,
        "40-convert-string-to-json-in-jq.toon" => 39,
        "42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.toon" => 41,
        "43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.toon" => 42,
        "44-exclude-column-from-jq-json-output.toon" => 43,
        "45-how-to-use-jq-when-the-variable-has-reserved-characters.toon" => 44,
        _ => panic!("unknown Stack Overflow contract file"),
    }
}

fn evaluate(query: &str, input: &serde_json::Value) -> Vec<Value> {
    let input = Value::from_json(input.clone()).expect("fixture input is valid JSON");
    let resolved = resolve(
        parse(query).expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, input, VmLimits::default());
    let mut results = Vec::new();
    while let Some(value) = vm.next_result().expect("query evaluates") {
        results.push(value);
    }
    results
}

fn raw_bytes(values: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in values {
        let Value::String(text) = value else {
            panic!("raw scenario produced a non-string result: {value:?}");
        };
        bytes.extend_from_slice(text.as_bytes());
        bytes.push(b'\n');
    }
    bytes
}
