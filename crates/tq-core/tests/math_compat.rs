//! Public VM compatibility checks for safe math and non-finite behavior.

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
fn unary_tuple_and_nonfinite_math_are_executable_through_the_vm() {
    assert_eq!(evaluate("sqrt", "9")[0].to_string(), "3");
    assert_eq!(evaluate("frexp", "8")[0].to_string(), "[0.5,4]");
    assert_eq!(evaluate("modf", "3.5")[0].to_string(), "[0.5,3]");

    let nan = &evaluate("nan", "null")[0];
    let Value::Number(nan) = nan else {
        panic!("nan must remain a runtime number");
    };
    assert!(nan.as_f64().is_nan());
    let infinite = &evaluate("infinite", "null")[0];
    let Value::Number(infinite) = infinite else {
        panic!("infinite must remain a runtime number");
    };
    assert!(infinite.as_f64().is_infinite());
    assert_eq!(evaluate("nan | isnan", "null"), [Value::Bool(true)]);
    assert_eq!(
        evaluate("infinite | isinfinite", "null"),
        [Value::Bool(true)]
    );
    assert_eq!(evaluate("1 | isfinite", "null"), [Value::Bool(true)]);
    assert_eq!(evaluate("1 | isnormal", "null"), [Value::Bool(true)]);
    assert_eq!(
        evaluate(
            "[null, \"x\", false | [isnan, isinfinite, isfinite, isnormal]]",
            "null",
        )[0]
        .to_string(),
        "[[false,false,false,false],[false,false,false,false],[false,false,false,false]]"
    );
}

// These exact values are witnesses from the pinned aarch64 macOS jq build.
// Other platforms can choose different last-bit rounding for transcendental math.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acos_and_exp_match_the_pinned_platform_rounding() {
    assert_eq!(evaluate("acos", "0.5")[0].to_string(), "1.0471975511965976");
    assert_eq!(evaluate("exp", "1")[0].to_string(), "2.718281828459045");
}

#[test]
fn finite_and_normal_selectors_match_jq_nonfinite_contract() {
    assert_eq!(
        evaluate(
            "[(nan | isfinite), (nan | [finites]), (infinite | [finites]), (1 | [normals]), (1e-400 | [normals])]",
            "null",
        )[0]
        .to_string(),
        "[true,[null],[],[1],[]]"
    );
    assert_eq!(
        evaluate(
            "[\"finites/0\", \"normals/0\", \"builtins/0\", \"not/0\"] - builtins",
            "null",
        ),
        [Value::array(Vec::new())]
    );
    assert_eq!(evaluate("not", "null"), [Value::Bool(true)]);
    assert_eq!(evaluate("not", "false"), [Value::Bool(true)]);
    assert_eq!(evaluate("not", "true"), [Value::Bool(false)]);
}

#[test]
fn decimal_capability_reports_the_runtime_number_model() {
    assert_eq!(evaluate("have_decnum", "null"), [Value::Bool(true)]);
    assert_eq!(
        evaluate("have_literal_numbers", "null"),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate("1.000 | tojson", "null")[0].to_string(),
        "\"1.000\""
    );
    assert_eq!(
        evaluate("100e-2 | tojson", "null")[0].to_string(),
        "\"1.00\""
    );
    assert_eq!(
        evaluate("1E1234567890 | tojson", "null")[0].to_string(),
        "\"1.7976931348623157e+308\""
    );
    assert_eq!(
        evaluate("map([., . == 1]) | tojson", "[1,1.000,1.0,100e-2]")[0].to_string(),
        "\"[[1,true],[1.000,true],[1.0,true],[1.00,true]]\""
    );
    assert_eq!(evaluate("1e-400 == 0", "null"), [Value::Bool(false)]);
}

#[test]
fn unary_negation_preserves_large_decimal_literal_identity() {
    assert_eq!(
        evaluate(
            "[1234567890987654321,-1234567890987654321 | tojson]",
            "null",
        )[0]
        .to_string(),
        "[\"1234567890987654321\",\"-1234567890987654321\"]"
    );
}

#[test]
fn unary_negation_of_zero_keeps_positive_literal_scale() {
    assert_eq!(
        evaluate(
            "[(-0),(-0.0),(-0e3),(0|-.),(0.0|-.),(copysign(0;-1)|-.),(\"-0.0\"|tonumber|-.),(\"-0.0\"|tonumber|tgamma)] | map(tojson)",
            "null",
        )[0]
        .to_string(),
        r#"["0","0.0","0E+3","0","0.0","0","0.0","-1.7976931348623157e+308"]"#
    );
}

#[test]
fn unary_negation_of_computed_zero_preserves_runtime_sign_rules() {
    assert_eq!(
        evaluate(
            "[(0+0|-.),(copysign(0;1)|-.),(copysign(0;-1)|-.)] | map(tojson)",
            "null",
        )[0]
        .to_string(),
        r#"["-0","-0","0"]"#
    );
}

#[test]
fn abs_preserves_positive_literal_identity_but_negates_negative_numbers() {
    assert_eq!(
        evaluate("map(abs)", "[-10,-1.1,-1e-1,1.000]")[0].to_string(),
        "[10,1.1,0.1,1.000]"
    );
    assert_eq!(evaluate("abs", "\"x\"")[0].to_string(), "\"x\"");
}

#[test]
fn abs_compares_underflowing_negative_literals_before_binary64_projection() {
    assert_eq!(
        evaluate("[., . < 0, abs] | map(tojson)", "-1e-400")[0].to_string(),
        "[\"-1E-400\",\"true\",\"1E-400\"]"
    );
}

#[test]
fn explicit_math_arguments_preserve_generator_cartesian_order() {
    assert_eq!(
        evaluate("pow((2,3); (2,3))", "null")
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["4", "9", "8", "27"]
    );
    assert_eq!(evaluate("fma((1,2); 3; 4)", "null")[0].to_string(), "7");
    assert_eq!(evaluate("fma((1,2); 3; 4)", "null")[1].to_string(), "10");

    let later_error =
        evaluate_with_limits("pow(empty; error(\"later\"))", "null", VmLimits::default())
            .expect_err("later argument errors remain observable after an empty argument");
    assert!(later_error.to_string().contains("later"));
    assert!(evaluate("pow(error(\"first\"); empty)", "null").is_empty());
}

#[test]
fn all_registered_math_dispatches_execute_a_finite_witness() {
    let unary = [
        "acos",
        "acosh",
        "asin",
        "asinh",
        "atan",
        "atanh",
        "cbrt",
        "ceil",
        "cos",
        "cosh",
        "erf",
        "erfc",
        "exp",
        "exp10",
        "exp2",
        "expm1",
        "fabs",
        "floor",
        "frexp",
        "gamma",
        "j0",
        "j1",
        "lgamma",
        "log",
        "log10",
        "log1p",
        "log2",
        "logb",
        "modf",
        "nearbyint",
        "rint",
        "round",
        "significand",
        "sin",
        "sinh",
        "sqrt",
        "tan",
        "tanh",
        "tgamma",
        "trunc",
        "y0",
        "y1",
    ];
    for name in unary {
        let values = evaluate(name, "2");
        assert_eq!(values.len(), 1, "{name}");
    }
    for (name, query) in [
        ("atan2", "atan2(0; 1)"),
        ("copysign", "copysign(2; -1)"),
        ("drem", "drem(5; 2)"),
        ("fdim", "fdim(5; 2)"),
        ("fmax", "fmax(2; 3)"),
        ("fmin", "fmin(2; 3)"),
        ("fmod", "fmod(5; 2)"),
        ("hypot", "hypot(3; 4)"),
        ("jn", "jn(0; 0)"),
        ("ldexp", "ldexp(1; 3)"),
        ("nextafter", "nextafter(1; 2)"),
        ("nexttoward", "nexttoward(1; 2)"),
        ("pow", "pow(2; 3)"),
        ("remainder", "remainder(5; 2)"),
        ("scalb", "scalb(2; 3)"),
        ("scalbln", "scalbln(2; 3)"),
        ("yn", "yn(0; 1)"),
    ] {
        assert_eq!(evaluate(query, "null").len(), 1, "{name}");
    }
    assert_eq!(evaluate("fma(2; 3; 4)", "null").len(), 1);
}

#[test]
fn bessel_work_is_charged_and_bounded() {
    let limits = VmLimits {
        steps: 10,
        ..VmLimits::default()
    };
    let error = evaluate_with_limits("jn(1024; 0)", "null", limits)
        .expect_err("bounded Bessel work should consume the VM step budget");
    assert_eq!(
        error,
        tq_core::VmError::Resource {
            resource: "vm-steps"
        }
    );

    let error = evaluate_with_limits("jn(1025; 0)", "null", VmLimits::default())
        .expect_err("Bessel order above the fixed safe cap must fail");
    assert_eq!(
        error,
        tq_core::VmError::Resource {
            resource: "math-bessel-order"
        }
    );
}

#[test]
fn jq_nonfinite_comparison_is_not_value_total_ordering() {
    assert_eq!(evaluate("nan == nan", "null"), [Value::Bool(false)]);
    assert_eq!(evaluate("nan != nan", "null"), [Value::Bool(true)]);
    assert_eq!(evaluate("nan < 0", "null"), [Value::Bool(true)]);
    assert_eq!(evaluate("0 > nan", "null"), [Value::Bool(true)]);
    assert_eq!(evaluate("[nan] == [nan]", "null"), [Value::Bool(false)]);
    assert_eq!(
        evaluate("([nan] as $x | $x == $x)", "null"),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate("({x:nan} as $x | $x == $x)", "null"),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate("[nan, nan] | unique", "null")[0]
            .to_json()
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        evaluate("[nan, nan] | group_by(.)", "null")[0]
            .to_json()
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let sorted = evaluate("[nan, -infinite, -1, 0, 1, infinite] | sort", "null")[0]
        .to_json()
        .unwrap_or_else(|error| panic!("sorted projection failed: {error}"));
    assert_eq!(sorted[0], serde_json::Value::Null);
    assert_eq!(sorted[1].as_f64(), Some(-f64::MAX));
    assert_eq!(sorted[2].as_f64(), Some(-1.0));
    assert_eq!(sorted[3].as_f64(), Some(0.0));
    assert_eq!(sorted[4].as_f64(), Some(1.0));
    assert_eq!(sorted[5].as_f64(), Some(f64::MAX));
    let nested = evaluate("[[nan], [0]] | sort", "null")[0]
        .to_json()
        .unwrap();
    assert_eq!(nested[0][0], serde_json::Value::Null);
    assert_eq!(nested[1][0].as_f64(), Some(0.0));
}

#[test]
fn jq_nonfinite_arithmetic_keeps_runtime_values_and_projects_on_output() {
    for query in [
        "nan + 1",
        "nan - 1",
        "nan * 0",
        "nan / 1",
        "infinite - infinite",
    ] {
        let Value::Number(number) = &evaluate(query, "null")[0] else {
            panic!("{query} must produce a runtime number");
        };
        assert!(number.as_f64().is_nan(), "{query}");
        assert_eq!(evaluate(query, "null")[0].to_string(), "null", "{query}");
    }
    let Value::Number(number) = &evaluate("-infinite", "null")[0] else {
        panic!("unary negation must preserve a runtime number");
    };
    assert!(number.as_f64().is_sign_negative() && number.as_f64().is_infinite());
    assert_eq!(
        evaluate("infinite + 1", "null")[0]
            .to_json()
            .unwrap()
            .as_f64(),
        Some(f64::MAX)
    );
    assert_eq!(
        evaluate("1e308 * 1e308", "null")[0]
            .to_json()
            .unwrap()
            .as_f64(),
        Some(f64::MAX)
    );
    assert_eq!(
        evaluate("1e-308 / 1e308", "null")[0]
            .to_json()
            .unwrap()
            .as_f64(),
        Some(0.0)
    );
    assert_eq!(evaluate("5.5 % 2", "null")[0].to_string(), "1");
    assert_eq!(evaluate("5 % 2.5", "null")[0].to_string(), "1");
    assert_eq!(evaluate("infinite % 2", "null")[0].to_string(), "1");
    assert_eq!(evaluate("-infinite % 2", "null")[0].to_string(), "0");
    assert_eq!(evaluate("5 % infinite", "null")[0].to_string(), "5");
    assert_eq!(evaluate("nan % 2", "null")[0].to_string(), "null");
}

#[test]
fn platform_scalb_contract() {
    #[cfg(not(target_os = "linux"))]
    assert_eq!(evaluate("scalb(2; 0.5)", "null")[0].to_string(), "2");
    #[cfg(target_os = "linux")]
    assert_eq!(evaluate("scalb(2; 0.5)", "null")[0].to_string(), "null");
    assert_eq!(
        evaluate("scalbln(2; 2147483648)", "null")[0]
            .to_json()
            .unwrap()
            .as_f64(),
        Some(f64::MAX)
    );
    let Value::Number(number) = &evaluate("scalb(2; nan)", "null")[0] else {
        panic!("NaN scale must remain a runtime number");
    };
    assert!(number.as_f64().is_nan());
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        evaluate("scalb(infinite; -infinite)", "null")[0]
            .to_json()
            .unwrap()
            .as_f64(),
        Some(f64::MAX)
    );
    #[cfg(target_os = "linux")]
    assert_eq!(
        evaluate("scalb(infinite; -infinite)", "null")[0]
            .to_json()
            .unwrap()
            .as_f64(),
        None
    );
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        evaluate(
            "[scalb(0; infinite), scalb(0; -infinite), scalb(infinite; -infinite), scalb(2; 0.5)]",
            "null",
        )[0]
        .to_string(),
        "[0,0,1.7976931348623157e+308,2]"
    );
    assert_eq!(
        evaluate(
            "[ldexp(2; 2147483648), ldexp(2; 2147483647), ldexp(2; -2147483649), ldexp(2; nan), scalbln(2; nan)]",
            "null",
        )[0]
        .to_string(),
        "[1.7976931348623157e+308,1.7976931348623157e+308,0,2,2]"
    );
    #[cfg(target_os = "linux")]
    assert_eq!(
        evaluate(
            "[scalb(2; infinite), scalb(2; -infinite), scalb(0; infinite), scalb(0; -infinite), scalb(infinite; infinite), scalb(infinite; -infinite), scalb(2; 0.5), (2 | gamma)]",
            "null",
        )[0]
        .to_string(),
        "[1.7976931348623157e+308,0,null,0,1.7976931348623157e+308,null,null,0]"
    );
}

#[test]
fn signed_zero_and_remainder_math_keep_binary64_signs() {
    let number_bits = |query: &str, input: &str| {
        let Value::Number(number) = &evaluate(query, input)[0] else {
            panic!("{query} must produce a number");
        };
        number.as_f64().to_bits()
    };
    assert_eq!(number_bits("rint", "-0.5"), (-0.0f64).to_bits());
    assert_eq!(number_bits("nearbyint", "-0.5"), (-0.0f64).to_bits());
    assert_eq!(number_bits("trunc", "-0.5"), (-0.0f64).to_bits());
    assert_eq!(number_bits("copysign(0; -1)", "null"), (-0.0f64).to_bits());
    assert_eq!(number_bits("fmin(-0; 0)", "null"), 0.0f64.to_bits());
    assert_eq!(number_bits("fmax(-0; 0)", "null"), 0.0f64.to_bits());
    let runtime_zero = evaluate(
        "[fmin(copysign(0;-1);0), fmin(0;copysign(0;-1)), fmax(copysign(0;-1);0), fmax(0;copysign(0;-1))]",
        "null",
    );
    let Value::Array(values) = &runtime_zero[0] else {
        panic!("runtime-zero result is an array");
    };
    let bits = values
        .iter()
        .map(|value| match value {
            Value::Number(number) => number.as_f64().to_bits(),
            _ => panic!("runtime-zero result is numeric"),
        })
        .collect::<Vec<_>>();
    let expected_mixed_zero_bits = if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        vec![
            (-0.0f64).to_bits(),
            0.0f64.to_bits(),
            (-0.0f64).to_bits(),
            0.0f64.to_bits(),
        ]
    } else {
        vec![
            (-0.0f64).to_bits(),
            (-0.0f64).to_bits(),
            0.0f64.to_bits(),
            0.0f64.to_bits(),
        ]
    };
    assert_eq!(bits, expected_mixed_zero_bits);
    let negative_zero = evaluate(
        "[fmin(copysign(0;-1);copysign(0;-1)), fmax(copysign(0;-1);copysign(0;-1)), fmin(nan;copysign(0;-1)), fmin(copysign(0;-1);nan), fmax(nan;copysign(0;-1)), fmax(copysign(0;-1);nan)]",
        "null",
    );
    let bits = match &negative_zero[0] {
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Number(number) => number.as_f64().to_bits(),
                _ => panic!("negative-zero result is numeric"),
            })
            .collect::<Vec<_>>(),
        _ => panic!("negative-zero result is an array"),
    };
    assert_eq!(bits, vec![(-0.0f64).to_bits(); 6]);
    assert_eq!(
        evaluate("[fmin(nan;2), fmax(2;nan)]", "null")[0].to_string(),
        "[2,2]"
    );
    assert_eq!(
        number_bits("nextafter(0; -1)", "null"),
        (-f64::from_bits(1)).to_bits()
    );
    assert_eq!(
        number_bits("nextafter(0; 1)", "null"),
        f64::from_bits(1).to_bits()
    );
    assert_eq!(number_bits("remainder(5; 2)", "null"), 1.0f64.to_bits());
    assert_eq!(number_bits("remainder(7; 2)", "null"), (-1.0f64).to_bits());
    assert_eq!(number_bits("fmod(-5; 2)", "null"), (-1.0f64).to_bits());
}
