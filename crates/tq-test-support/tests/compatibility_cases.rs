//! Data-driven compatibility catalog schema and coverage tests.

use std::{collections::BTreeSet, fs, path::Path};

use serde_json::Value;

const SCHEMA: &str = include_str!("../../../schemas/compatibility-case-v1.schema.json");

#[test]
fn every_catalog_case_is_schema_valid_and_uniquely_identified() {
    let schema: Value = serde_json::from_str(SCHEMA).expect("case schema");
    let validator = jsonschema::Validator::new(&schema).expect("case validator");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility/cases");
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for entry in fs::read_dir(root).expect("case directory") {
        let path = entry.expect("case entry").path();
        for (line_index, line) in fs::read_to_string(&path)
            .expect("case file")
            .lines()
            .enumerate()
        {
            if line.trim().is_empty() {
                continue;
            }
            let case: Value = serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("{}:{}: {error}", path.display(), line_index + 1));
            assert!(
                validator.is_valid(&case),
                "{}:{} is not schema-valid",
                path.display(),
                line_index + 1
            );
            let id = case["id"].as_str().expect("case id");
            assert!(ids.insert(id.to_owned()), "duplicate case id {id}");
            count += 1;
        }
    }
    assert!(
        count >= 20,
        "catalog should contain the initial common surface"
    );
}

#[test]
fn common_and_navigation_capability_groups_are_present() {
    let cases = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility/cases/common.jsonl"),
    )
    .expect("common cases");
    for capability in [
        "value.null",
        "value.boolean",
        "value.number",
        "value.string",
        "value.array",
        "value.object",
        "cardinality.zero",
        "cardinality.one",
        "cardinality.many",
    ] {
        assert!(cases.contains(capability), "missing {capability}");
    }

    let navigation = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/compatibility/cases/navigation.jsonl"),
    )
    .expect("navigation cases");
    for capability in [
        "navigation.field",
        "navigation.computed",
        "navigation.index",
        "navigation.negative-index",
        "navigation.slice",
        "navigation.iteration",
        "navigation.optional",
        "error.runtime-type",
    ] {
        assert!(navigation.contains(capability), "missing {capability}");
    }
}

#[test]
fn composition_construction_operator_numeric_and_variable_groups_are_complete() {
    assert_capabilities(
        "composition.jsonl",
        &[
            "composition.pipe",
            "composition.comma",
            "composition.parentheses",
            "composition.nested-iteration",
            "composition.downstream-multiplicity",
        ],
    );
    assert_capabilities(
        "construction.jsonl",
        &[
            "construction.array",
            "construction.object",
            "construction.shorthand",
            "construction.computed-key",
            "construction.duplicate-key",
        ],
    );
    assert_capabilities(
        "operators.jsonl",
        &[
            "control.conditional",
            "control.truthiness",
            "control.short-circuit",
            "control.alternative",
            "comparison.equality",
            "comparison.ordering",
            "arithmetic.add",
            "arithmetic.subtract",
            "arithmetic.multiply",
            "arithmetic.divide",
            "arithmetic.modulo",
            "arithmetic.type-error",
        ],
    );
    assert_capabilities(
        "numeric.jsonl",
        &[
            "numeric.ordinary",
            "numeric.literal",
            "numeric.exponent",
            "numeric.negative-zero",
            "numeric.precision-boundary",
            "numeric.literal-invalidation",
            "numeric.envelope",
            "numeric.digit-limit",
            "numeric.exponent-limit",
            "numeric.expansion-limit",
            "numeric.index-range",
            "numeric.overflow",
            "numeric.underflow",
            "numeric.nonfinite-result",
        ],
    );
    assert_capabilities(
        "variables.jsonl",
        &[
            "variable.binding",
            "variable.scope",
            "variable.generator",
            "variable.unknown",
            "cli.arg",
            "cli.argjson",
            "cli.argtoon",
        ],
    );
}

#[test]
fn every_mvp_builtin_has_a_case_and_execution_classification() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/compatibility/cases/builtins.jsonl");
    let cases = fs::read_to_string(path).expect("built-in cases");
    for builtin in [
        "empty",
        "error",
        "type",
        "length",
        "utf8bytelength",
        "keys",
        "keys-unsorted",
        "has",
        "in",
        "select",
        "map",
        "map-values",
        "values",
        "scalars",
        "arrays",
        "objects",
        "iterables",
        "booleans",
        "numbers",
        "strings",
        "nulls",
        "tostring",
        "tonumber",
        "add",
        "min",
        "max",
        "sort",
        "sort-by",
        "unique",
        "unique-by",
        "reverse",
        "flatten",
        "range",
        "to-entries",
        "with-entries",
        "group-by",
        "min-by",
        "max-by",
        "limit",
        "paths",
        "getpath",
        "setpath",
        "path",
        "tostream",
        "tojson",
        "fromjson",
        "inputs",
        "any",
        "all",
        "ltrimstr",
        "ascii-downcase",
        "explode",
        "implode",
        "floor",
        "ceil",
        "fabs",
    ] {
        assert!(
            cases.contains(&format!("\"builtin.{builtin}\"")),
            "missing built-in case for {builtin}"
        );
    }
    for (index, line) in cases.lines().enumerate() {
        let case: Value = serde_json::from_str(line).expect("built-in case JSON");
        let capabilities = case["capabilities"].as_array().expect("capability array");
        assert!(
            capabilities.iter().any(|value| {
                matches!(
                    value.as_str(),
                    Some("execution.streaming" | "execution.blocking")
                )
            }),
            "builtins.jsonl:{} lacks execution classification",
            index + 1
        );
    }
}

#[test]
fn error_update_cli_recursive_and_deferred_groups_cover_the_spec() {
    assert_capabilities(
        "errors.jsonl",
        &[
            "error.empty",
            "error.explicit",
            "error.optional",
            "error.try-catch",
            "error.after-output",
            "error.source-location",
        ],
    );
    assert_capabilities(
        "updates.jsonl",
        &[
            "update.assignment",
            "update.pipe",
            "update.add",
            "update.subtract",
            "update.multiply",
            "update.divide",
            "update.alternative",
            "update.multi-path",
            "update.invalid-lvalue",
        ],
    );
    assert_capabilities(
        "cli.jsonl",
        &[
            "cli.detection-order",
            "cli.ambiguous-json-yaml",
            "cli.strict-override",
            "cli.late-failure",
            "cli.stdin",
            "cli.file",
            "cli.yaml-multi-document",
            "cli.null-input",
            "cli.raw-input",
            "cli.slurp",
            "cli.stream",
            "cli.stream-errors",
            "cli.raw-output",
            "cli.join-output",
            "cli.framing",
            "cli.strictness",
            "cli.variable",
            "cli.option-validation",
            "cli.exit-status",
        ],
    );
    assert_capabilities(
        "folds.jsonl",
        &[
            "fold.reduce",
            "fold.foreach",
            "fold.update-multiplicity",
            "fold.extract-multiplicity",
            "result.partial",
        ],
    );
    assert_capabilities(
        "recursive-interpolation.jsonl",
        &[
            "recursive.descent",
            "recursive.order",
            "recursive.scalar",
            "recursive.deep",
            "interpolation.string",
            "interpolation.conversion",
            "interpolation.escape",
            "interpolation.generator",
            "interpolation.nested",
            "result.partial",
        ],
    );
    assert_capabilities(
        "recursive-labels.jsonl",
        &[
            "label.issue-6",
            "label.shadowing",
            "label.try-inside",
            "label.try-outside",
            "recurse.default",
            "recurse.conditional",
            "recurse.generator",
            "walk.post-order",
            "walk.cardinality",
        ],
    );
    assert_capabilities("deferred.jsonl", &["deferred.nonfinite-result"]);

    let deferred = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/compatibility/cases/deferred.jsonl"),
    )
    .expect("deferred cases");
    for (index, line) in deferred.lines().enumerate() {
        let case: Value = serde_json::from_str(line).expect("deferred case JSON");
        assert_eq!(case["classification"], "deferred", "line {}", index + 1);
        assert_eq!(case["status"], "deferred", "line {}", index + 1);
    }
}

#[test]
fn regex_date_and_platform_cases_cover_portable_and_governed_behavior() {
    assert_capabilities(
        "regex-date-platform.jsonl",
        &[
            "regex.test",
            "regex.match",
            "regex.capture",
            "regex.scan",
            "regex.split",
            "regex.splits",
            "regex.sub",
            "regex.gsub",
            "regex.unsupported",
            "date.fromdateiso8601",
            "date.todateiso8601",
            "date.strptime",
            "date.strftime",
            "date.range",
            "date.localtime",
            "environment.snapshot",
            "environment.denied",
            "platform.denied",
            "platform.input-filename",
            "platform.now",
        ],
    );
}

#[test]
fn functions_and_modules_cover_scope_calls_loading_and_failures() {
    assert_capabilities(
        "functions-modules.jsonl",
        &[
            "function.definition",
            "function.callback.collection",
            "function.callback.predicate",
            "function.callback.scalar",
            "function.callback.keyed",
            "function.parameter.value",
            "function.parameter.filter",
            "function.scope",
            "function.recursion",
            "function.failure.unknown",
            "function.failure.arity",
            "module.import",
            "module.include",
            "module.metadata",
            "module.cycle",
            "module.confinement",
        ],
    );
}

fn assert_capabilities(file: &str, required: &[&str]) {
    let cases = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/compatibility/cases")
            .join(file),
    )
    .unwrap_or_else(|error| panic!("failed to read {file}: {error}"));
    for capability in required {
        assert!(
            cases.contains(&format!("\"{capability}\"")),
            "{file} is missing {capability}"
        );
    }
}
