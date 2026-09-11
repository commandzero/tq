//! Public VM checks for jq collection containment and index built-ins.

use tq_core::{ResolveOptions, Value, Vm, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    evaluate_with_limits(query, input, VmLimits::default()).expect("query evaluates")
}

fn evaluate_with_limits(
    query: &str,
    input: &str,
    limits: VmLimits,
) -> Result<Vec<Value>, tq_core::VmError> {
    let input: Value = serde_json::from_str(input).expect("valid JSON input");
    let plan = analyze(
        resolve(
            parse(query).expect("query parses"),
            &ResolveOptions::default(),
        )
        .expect("query resolves"),
    )
    .compile()
    .expect("query compiles")
    .document_plan();
    let mut vm = Vm::new(&plan, input, limits);
    let mut values = Vec::new();
    loop {
        match vm.next_result()? {
            Some(value) => values.push(value),
            None => return Ok(values),
        }
    }
}

#[test]
fn containment_matches_jq_string_array_and_object_forms() {
    for (query, input, expected) in [
        (r#"contains("bar")"#, r#""foobar""#, "true"),
        (
            r#"contains(["baz", "bar"])"#,
            r#"["foobar","foobaz","blarp"]"#,
            "true",
        ),
        (
            r#"contains(["bazzzzz", "bar"])"#,
            r#"["foobar","foobaz","blarp"]"#,
            "false",
        ),
        (
            r"contains({foo: 12, bar: [{barp: 12}]})",
            r#"{"foo":12,"bar":[1,2,{"barp":12,"blip":13}]}"#,
            "true",
        ),
        (
            r"contains({foo: 12, bar: [{barp: 15}]})",
            r#"{"foo":12,"bar":[1,2,{"barp":12,"blip":13}]}"#,
            "false",
        ),
        (r#"inside("foobar")"#, r#""bar""#, "true"),
        (
            r#"inside(["foobar", "foobaz", "blarp"])"#,
            r#"["baz","bar"]"#,
            "true",
        ),
        (
            r"inside({foo: 12, bar: [1,2,{barp:12, blip:13}]})",
            r#"{"bar":[{"barp":12}]}"#,
            "true",
        ),
        (r"contains(null)", "null", "true"),
        (r"contains(true)", "true", "true"),
        (r"contains(12)", "12", "true"),
    ] {
        assert_eq!(evaluate(query, input)[0].to_string(), expected, "{query}");
    }
}

#[test]
fn indices_index_and_rindex_cover_strings_and_overlapping_arrays() {
    for (query, input, expected) in [
        (r#"indices(", ")"#, r#""a,b, cd, efg, hijk""#, "[3,7,12]"),
        ("indices(1)", "[0,1,2,1,3,1,4]", "[1,3,5]"),
        ("indices([1,2])", "[0,1,2,3,1,4,2,5,1,2,6,7]", "[1,8]"),
        (r#"index(", ")"#, r#""a,b, cd, efg, hijk""#, "3"),
        ("index(1)", "[0,1,2,1,3,1,4]", "1"),
        ("index([1,2])", "[0,1,2,3,1,4,2,5,1,2,6,7]", "1"),
        (r#"rindex(", ")"#, r#""a,b, cd, efg, hijk""#, "12"),
        ("rindex(1)", "[0,1,2,1,3,1,4]", "5"),
        ("rindex([1,2])", "[0,1,2,3,1,4,2,5,1,2,6,7]", "8"),
    ] {
        assert_eq!(evaluate(query, input)[0].to_string(), expected, "{query}");
    }
}

#[test]
fn index_builtins_handle_empty_patterns_and_empty_or_error_generators() {
    assert_eq!(evaluate(r#"indices("")"#, r#""abc""#)[0].to_string(), "[]");
    assert_eq!(evaluate("indices([])", "[1,2,1]")[0].to_string(), "[]");
    assert_eq!(evaluate("index([])", "[1,2]")[0].to_string(), "null");
    assert_eq!(evaluate("rindex([])", "[1,2]")[0].to_string(), "null");
    assert!(evaluate("empty | indices(1)", "[1]").is_empty());
    assert!(evaluate("empty | contains(1)", "[1]").is_empty());
    let error = evaluate_with_limits("contains(error(\"later\"))", "[1]", VmLimits::default())
        .expect_err("argument errors remain observable");
    assert!(error.to_string().contains("later"));
}

#[test]
fn collection_builtins_reject_incompatible_types() {
    assert!(evaluate_with_limits(r"contains([1])", r#""x""#, VmLimits::default()).is_err());
    assert!(evaluate_with_limits("contains(false)", "true", VmLimits::default()).is_err());
    assert!(evaluate_with_limits("indices(1)", r#""x""#, VmLimits::default()).is_err());
    assert_eq!(evaluate("index(\"x\")", "[1]")[0].to_string(), "null");
    assert!(evaluate_with_limits("inside(1)", "[1]", VmLimits::default()).is_err());
}

#[test]
fn array_indices_compare_mixed_type_scalar_needles() {
    for (query, expected) in [
        ("indices(\"1\")", "[1]"),
        ("indices(true)", "[2]"),
        ("indices(null)", "[3]"),
        ("index(\"1\")", "1"),
        ("rindex(\"1\")", "1"),
    ] {
        assert_eq!(
            evaluate(query, r#"[1,"1",true,null]"#)[0].to_string(),
            expected
        );
    }
}

#[test]
fn collection_scans_obey_vm_step_limits() {
    let input = serde_json::to_string(&(0..256).collect::<Vec<_>>()).expect("array JSON");
    let error = evaluate_with_limits(
        "contains([255])",
        &input,
        VmLimits {
            steps: 3,
            ..VmLimits::default()
        },
    )
    .expect_err("collection scan should consume the VM step budget");
    assert!(matches!(
        error,
        tq_core::VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn string_search_charges_each_character_comparison() {
    let error = evaluate_with_limits(
        r#"contains("aaaaab")"#,
        r#""aaaaaaaaaaaaaaaa""#,
        VmLimits {
            steps: 12,
            ..VmLimits::default()
        },
    )
    .expect_err("repeated-prefix string search should consume the VM step budget");
    assert!(matches!(
        error,
        tq_core::VmError::Resource {
            resource: "vm-steps"
        }
    ));
}

#[test]
fn string_indices_use_character_positions() {
    assert_eq!(
        evaluate(r#"indices("😊a")"#, r#""a😊a😊""#)[0].to_string(),
        "[1]"
    );
    assert_eq!(
        evaluate(r#"indices("é")"#, r#""éé""#)[0].to_string(),
        "[0,1]"
    );
}

#[test]
fn entry_conversions_preserve_order_and_jq_key_aliases() {
    assert_eq!(
        evaluate("to_entries", r#"{"z":1,"a":2}"#)[0].to_string(),
        r#"[{"key":"z","value":1},{"key":"a","value":2}]"#
    );
    assert_eq!(
        evaluate("to_entries", "[10,20]")[0].to_string(),
        r#"[{"key":0,"value":10},{"key":1,"value":20}]"#
    );
    assert_eq!(
        evaluate(
            "from_entries",
            r#"[{"Key":"a","Value":1},{"name":"b","Name":"ignored"},{"Name":"c","value":3},{"key":"d"}]"#,
        )[0]
        .to_string(),
        r#"{"a":1,"b":null,"c":3,"d":null}"#
    );
    assert_eq!(
        evaluate(
            "from_entries",
            r#"[{"key":null,"Key":"ok","value":null,"Value":2},{"key":false,"Name":"fallback","value":false,"Value":true}]"#,
        )[0]
        .to_string(),
        r#"{"ok":null,"fallback":false}"#
    );
    assert_eq!(evaluate("from_entries", "[]")[0].to_string(), "{}");
    assert!(evaluate_with_limits("from_entries", "[1]", VmLimits::default()).is_err());
    assert!(
        evaluate_with_limits(
            "from_entries",
            r#"[{"key":2,"value":"x"}]"#,
            VmLimits::default(),
        )
        .is_err()
    );
}

#[test]
fn add_and_map_values_preserve_filter_cardinality() {
    assert_eq!(evaluate("add(.[])", "[1,2,3]")[0].to_string(), "6");
    assert_eq!(evaluate("add(empty)", "[1,2,3]")[0].to_string(), "null");
    assert_eq!(
        evaluate("add(.[].a)", r#"[{"a":3},{"a":5},{"b":6}]"#)[0].to_string(),
        "8"
    );
    assert_eq!(
        evaluate("map_values(empty)", r#"{"a":1,"b":2}"#)[0].to_string(),
        "{}"
    );
    assert_eq!(
        evaluate("map_values(., .+10)", "[1,2]")[0].to_string(),
        "[1,2]"
    );
    assert_eq!(
        evaluate("map_values(., error(\"later\"))", "[1]")[0].to_string(),
        "[1]"
    );
    assert_eq!(
        evaluate("def f: map_values(., .+10); f", "[1,2]")[0].to_string(),
        "[1,2]"
    );
    assert_eq!(
        evaluate("flatten(range(1;3))", "[1,[2,[3]]]")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["[1,2,[3]]", "[1,2,3]"]
    );
    assert_eq!(
        evaluate("flatten(0.5)", "[1,[2,[3]]]")[0].to_string(),
        "[1,2,3]"
    );
}

#[test]
fn any_and_all_support_all_forms_and_short_circuit() {
    for (query, input, expected) in [
        ("any", "[true,false]", "true"),
        ("all", "[true,false]", "false"),
        ("any(. > 1)", "[1,2,0]", "true"),
        ("all(. > 0)", "[1,2,0]", "false"),
        ("any(.[]; . > 1)", "[1,2,0]", "true"),
        ("all(.[]; . < 4)", "[1,2,0]", "true"),
        ("any(.[]; empty)", "[1,2]", "false"),
        ("all(.[]; empty)", "[1,2]", "true"),
        ("any(.[]; false, true)", "[1]", "true"),
        ("all(.[]; true, false)", "[1]", "false"),
    ] {
        assert_eq!(evaluate(query, input)[0].to_string(), expected, "{query}");
    }
    assert_eq!(
        evaluate(
            "any(range(0; 4); if . == 0 then true else error(\"later\") end)",
            "null",
        )[0]
        .to_string(),
        "true"
    );
    assert_eq!(
        evaluate(
            "all(range(0; 4); if . == 0 then false else error(\"later\") end)",
            "null",
        )[0]
        .to_string(),
        "false"
    );
    assert!(
        evaluate_with_limits(
            "any(range(0; 4); if . == 1 then true else error(\"later\") end)",
            "null",
            VmLimits::default(),
        )
        .is_err()
    );
    let error = evaluate_with_limits(
        "any(range(0; 1000); . == -1)",
        "null",
        VmLimits {
            steps: 8,
            ..VmLimits::default()
        },
    )
    .expect_err("predicate scans should consume the VM step budget");
    assert!(matches!(
        error,
        tq_core::VmError::Resource {
            resource: "vm-steps"
        }
    ));
}
