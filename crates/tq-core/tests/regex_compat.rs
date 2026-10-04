//! Public regex built-in behavior at the parser, resolver, and VM seam.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tq_core::{ResolveOptions, Value, Vm, VmError, VmLimits, analyze, parse, resolve};

fn evaluate(query: &str, input: &str) -> Vec<Value> {
    evaluate_with_limits(query, input, VmLimits::default()).expect("query evaluates")
}

fn evaluate_with_limits(query: &str, input: &str, limits: VmLimits) -> Result<Vec<Value>, VmError> {
    let input = Value::from_json(serde_json::from_str(input).expect("valid JSON")).unwrap();
    let resolved = resolve(
        parse(query).expect("query parses"),
        &ResolveOptions::default(),
    )
    .expect("query resolves");
    let plan = analyze(resolved)
        .compile()
        .expect("query compiles")
        .document_plan();
    let mut vm = Vm::new(&plan, input, limits);
    let mut values = Vec::new();
    while let Some(value) = vm.next_result()? {
        values.push(value);
    }
    Ok(values)
}

#[test]
fn regex_accepts_array_pattern_flags_and_null_flags() {
    assert_eq!(
        evaluate(r#"match(["foo", "g"])"#, r#""foo foo""#),
        evaluate(r#"match("foo"; "g")"#, r#""foo foo""#)
    );
    assert_eq!(
        evaluate(r#"test(["FOO", "i"])"#, r#""foo""#),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate(r#"split(","; null)"#, r#""a,b,c""#),
        [Value::array([
            Value::string("a"),
            Value::string("b"),
            Value::string("c"),
        ])]
    );
    assert_eq!(
        evaluate(r#"split("a+")"#, r#""aab""#),
        [Value::array([Value::string("aab")])]
    );
    assert_eq!(
        evaluate(r#"split("a+"; null)"#, r#""aab""#),
        [Value::array([Value::string(""), Value::string("b")])]
    );
    assert_eq!(
        evaluate(r#"[splits("a+")]"#, r#""aab""#),
        [Value::array([Value::string(""), Value::string("b")])]
    );
    let array_error = evaluate_with_limits(r#"split(["a", "g"])"#, r#""a""#, VmLimits::default())
        .expect_err("split does not accept array shorthand");
    assert!(matches!(array_error, VmError::Runtime { message } if message.contains("array")));
    let error = evaluate_with_limits(r#"sub(["p", "g"]; "x")"#, r#""pp""#, VmLimits::default())
        .expect_err("sub does not accept the array shorthand");
    assert!(matches!(error, VmError::Runtime { message } if message.contains("array")));
    for query in [r#"scan(["a", "g"])"#, r#"splits(["a", "g"])"#] {
        let error = evaluate_with_limits(query, r#""aAa""#, VmLimits::default())
            .expect_err("scan and splits do not accept the array shorthand");
        assert!(matches!(error, VmError::Runtime { message } if message.contains("array")));
    }
}

#[test]
fn regex_scan_single_capture_keeps_capture_arrays() {
    assert_eq!(
        evaluate(r#"[scan("(.)")]"#, r#""ab""#),
        [Value::array([
            Value::array([Value::string("a")]),
            Value::array([Value::string("b")]),
        ])]
    );
}

#[test]
fn regex_two_argument_capture_and_scan_follow_jq_overloads() {
    assert_eq!(
        evaluate(r#"capture("(?<x>foo)"; "i")"#, r#""FOO""#)[0].to_string(),
        r#"{"x":"FOO"}"#
    );
    assert_eq!(
        evaluate(r#"[scan("a"; "g")]"#, r#""aAa""#)[0].to_string(),
        r#"["a","a"]"#
    );
}

#[test]
fn regex_supports_scoped_case_and_extended_flags() {
    assert_eq!(
        evaluate(r#"test("foo (?i:bar)"; "x")"#, r#""fooBAR""#),
        [Value::Bool(true)]
    );
    assert_eq!(
        evaluate(r#"test("foo (?i:bar)"; "x")"#, r#""foo bar""#),
        [Value::Bool(false)]
    );
}

#[test]
fn regex_line_flags_match_pinned_jq_probe() {
    assert_eq!(
        evaluate(
            r#"[test("^b"), test("^b"; "s"), test("a.b"), test("a.b"; "s"), test("a.b"; "m"), test("^b"; "m"), test("^b"; "p")]"#,
            r#""a\nb""#,
        ),
        [Value::array([
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(false),
        ])]
    );
}

#[test]
fn regex_reports_unicode_offsets_and_respects_empty_match_suppression() {
    assert_eq!(
        evaluate(r#"[match("a"; "g")]"#, r#""éaé""#),
        [Value::array([Value::object(indexmap::IndexMap::from([
            (
                "offset".into(),
                Value::from_json(serde_json::json!(1)).unwrap(),
            ),
            (
                "length".into(),
                Value::from_json(serde_json::json!(1)).unwrap(),
            ),
            ("string".into(), Value::string("a")),
            ("captures".into(), Value::array(Vec::new())),
        ]))])]
    );
    assert_eq!(evaluate(r#"[match(""; "g")]"#, r#""a""#).len(), 1);
    assert_eq!(
        evaluate(r#"[match(""; "gn")]"#, r#""a""#),
        [Value::array(Vec::new())]
    );
    assert_eq!(
        evaluate(r#"[gsub(""; "X")]"#, r#""a""#),
        [Value::array([Value::string("XaX")])]
    );
    assert_eq!(
        evaluate(r#"[gsub(""; "X"; "n")]"#, r#""a""#),
        [Value::array([Value::string("a")])]
    );
    assert_eq!(
        evaluate(r#"split(""; "n")"#, r#""ab""#),
        [Value::array([Value::string("ab")])]
    );
    assert_eq!(
        evaluate(r#"split("")"#, r#""ab""#),
        [Value::array([Value::string("a"), Value::string("b")])]
    );
}

#[test]
fn regex_unmatched_capture_preserves_jq_compact_field_order() {
    assert_eq!(
        evaluate(r#"[match("(?<x>a)?b")]"#, r#""b""#)[0].to_string(),
        r#"[{"offset":0,"length":1,"string":"b","captures":[{"offset":-1,"string":null,"length":0,"name":"x"}]}]"#
    );
}

#[test]
fn regex_replacement_generators_preserve_branch_cardinality() {
    assert_eq!(
        evaluate(
            r#"[gsub("(?<d>[0-9])"; (if .d == "1" then "a" else "x", "y" end))]"#,
            r#""12""#,
        ),
        [Value::array([Value::string("ax"), Value::string("y")])]
    );
    assert_eq!(
        evaluate(
            r#"[gsub("(?<d>[0-9])"; (if .d == "1" then empty else "x", "y" end))]"#,
            r#""12""#,
        ),
        [Value::array([Value::string("x"), Value::string("y")])]
    );
    assert_eq!(
        evaluate(r#"[gsub("p"; ("a", "b"))]"#, r#""pp""#),
        [Value::array([Value::string("aa"), Value::string("bb")])]
    );
    assert_eq!(
        evaluate(r#"[sub("p"; ("a", "b"))]"#, r#""pp""#),
        [Value::array([Value::string("ap"), Value::string("bp")])]
    );
    assert_eq!(
        evaluate(r#"[gsub("(?<x>[a-z])"; (.x, "z"))]"#, r#""pq""#,),
        [Value::array([Value::string("pq"), Value::string("zz")])]
    );
    assert_eq!(
        evaluate(r#"[sub("(?<x>a)?b"; .x)]"#, r#""b""#),
        [Value::array([Value::string("")])]
    );
    assert_eq!(
        evaluate(
            r#"[gsub("(?<d>[0-9])"; (if .d == "1" then "a", "b" else "x" end))]"#,
            r#""12""#,
        ),
        [Value::array([Value::string("ax"), Value::string("b")])]
    );
}

#[test]
fn empty_replacement_stream_preserves_the_original_input() {
    assert_eq!(
        evaluate(r#"[sub("a"; empty), gsub("a"; empty)]"#, r#""a""#),
        [Value::array([Value::string("a"), Value::string("a")])]
    );
}

#[test]
fn regex_longest_match_selects_longer_alternative() {
    assert_eq!(
        evaluate(r#"match("a|ab"; "l")"#, r#""ab""#)[0].to_string(),
        r#"{"offset":0,"length":2,"string":"ab","captures":[]}"#
    );
}

#[test]
fn regex_longest_match_uses_consumed_bytes_across_start_positions() {
    for (input, pattern, expected) in [
        (
            r#""abXYZ""#,
            "a|XYZ",
            r#"{"offset":2,"length":3,"string":"XYZ","captures":[]}"#,
        ),
        (
            r#""éaa""#,
            "é|aa",
            r#"{"offset":0,"length":1,"string":"é","captures":[]}"#,
        ),
        (
            r#""aé""#,
            "a|é",
            r#"{"offset":1,"length":1,"string":"é","captures":[]}"#,
        ),
        (
            r#""ab""#,
            r"a\Kb|ab",
            r#"{"offset":1,"length":1,"string":"b","captures":[]}"#,
        ),
    ] {
        let query = format!("match({}; \"l\")", serde_json::to_string(pattern).unwrap());
        assert_eq!(evaluate(&query, input)[0].to_string(), expected);
    }
}

#[test]
fn regex_longest_match_preserves_engine_paths_and_full_input_context() {
    for (pattern, expected) in [
        ("a.*?", "ab"),
        ("(?>a|ab)", "a"),
        ("a(?=b)|ab", "ab"),
        ("a(?!b)|b", "b"),
        ("a$|b", "b"),
        ("(?<=a)b|a", "a"),
    ] {
        let query = format!(
            "match({}; \"l\").string",
            serde_json::to_string(pattern).unwrap()
        );
        assert_eq!(evaluate(&query, r#""ab""#), [Value::string(expected)]);
    }
    assert_eq!(
        evaluate(r#"match("(a|ab)(b?)"; "l")"#, r#""ab""#)[0].to_string(),
        r#"{"offset":0,"length":2,"string":"ab","captures":[{"offset":0,"length":1,"string":"a","name":null},{"offset":1,"length":1,"string":"b","name":null}]}"#
    );
}

#[test]
fn regex_longest_match_is_shared_by_all_builtins_and_user_calls() {
    assert_eq!(
        evaluate(
            r#"def call: match(["a|ab", "l"]); [call.string, test("a|ab"; "l"), capture("(?<x>a|ab)"; "l"), scan("a|ab"; "l"), split("a|ab"; "l"), splits("a|ab"; "l"), sub("a|ab"; "X"; "l"), gsub("a|ab"; "X"; "l")]"#,
            r#""ab""#,
        )[0].to_string(),
        r#"["ab",true,{"x":"ab"},"ab",["",""],"","","X","X"]"#
    );
}

#[test]
fn regex_longest_match_composes_with_flags_and_global_empty_matches() {
    for query in [
        r#"match("a|ab # trailing comment"; "lx").string"#,
        r#"match("(?x)a|ab # trailing comment"; "l").string"#,
        r#"match("A|AB"; "li").string"#,
    ] {
        assert_eq!(evaluate(query, r#""ab""#), [Value::string("ab")]);
    }
    for flag in ["m", "p"] {
        let query = format!("match(\"a.*?\"; \"l{flag}\").string");
        assert_eq!(evaluate(&query, r#""a\nb""#), [Value::string("a\nb")]);
    }
    assert_eq!(
        evaluate(
            r#"[match("a|ab|XYZ"; "lg") | [.offset, .string]]"#,
            r#""abXYZab""#
        )[0]
        .to_string(),
        r#"[[2,"XYZ"],[5,"ab"]]"#
    );
    assert_eq!(
        evaluate(r#"[match(""; "lg") | .offset]"#, r#""ab""#)[0].to_string(),
        "[0,1,2]"
    );
    assert_eq!(
        evaluate(r#"[match(""; "lgn")]"#, r#""ab""#)[0].to_string(),
        "[]"
    );
    assert_eq!(
        evaluate(r#"[match("|ab"; "lgn") | .string]"#, r#""ab""#)[0].to_string(),
        r#"["ab"]"#
    );
    assert_eq!(
        evaluate(r#"[match("z"; "l")]"#, r#""ab""#)[0].to_string(),
        "[]"
    );
}

#[test]
fn regex_longest_match_global_keeps_empty_match_after_nonempty_match() {
    assert_eq!(
        evaluate(r#"[match("|ab"; "lg") | [.offset, .string]]"#, r#""abb""#)[0].to_string(),
        r#"[[0,"ab"],[2,""],[3,""]]"#
    );
}

#[test]
fn regex_longest_match_empty_capture_keeps_jq_compact_field_order() {
    assert_eq!(
        evaluate(r#"match("(a|ab)(b?)"; "l")"#, r#""a""#)[0].to_string(),
        r#"{"offset":0,"length":1,"string":"a","captures":[{"offset":0,"length":1,"string":"a","name":null},{"offset":1,"string":"","length":0,"name":null}]}"#
    );
}

#[test]
fn regex_longest_match_nonempty_flag_counts_consumption_before_keep_reset() {
    assert_eq!(
        evaluate(r#"match("a\\K"; "ln")"#, r#""a""#)[0].to_string(),
        r#"{"offset":1,"length":0,"string":"","captures":[]}"#
    );
}

#[test]
fn regex_longest_match_bounded_patterns_fit_small_work_budget() {
    let input = serde_json::to_string(&format!("a{}", "b".repeat(500))).unwrap();
    for (pattern, expected) in [("a", "a"), ("a|ab", "ab")] {
        let query = format!(
            "match({}; \"l\").string",
            serde_json::to_string(pattern).unwrap()
        );
        assert_eq!(
            evaluate_with_limits(
                &query,
                &input,
                VmLimits {
                    regex_backtrack_limit: 4_000,
                    ..VmLimits::default()
                }
            )
            .expect("bounded matches must not spend the budget probing haystack-sized endpoints"),
            [Value::string(expected)]
        );
    }
}

#[test]
fn regex_longest_match_bounds_preserve_unicode_flags_and_context() {
    for (pattern, flags, input, expected) in [
        (r"\x61{1,2}|abc", "l", "abc", r#"[0,"abc"]"#),
        ("a{1,3}?", "l", "aaaa", r#"[0,"aaa"]"#),
        ("(?:a|ab){1,2}", "l", "abab", r#"[0,"abab"]"#),
        // Keep the Kelvin sign distinct from its NFC-normalized ASCII K.
        ("(?i:k)|aa", "l", "aa\u{212a}", "[2,\"\u{212a}\"]"),
        ("K|aa", "li", "aa\u{212a}", "[2,\"\u{212a}\"]"),
        (
            "(?i:k)(?-i:a)|aaa",
            "l",
            "aaa\u{212a}a",
            "[3,\"\u{212a}a\"]",
        ),
        ("a|é|aa", "l", "aéaa", r#"[1,"é"]"#),
        (".|ab", "l", "ab😀", r#"[2,"😀"]"#),
        ("[a😀]|ab", "l", "ab😀", r#"[2,"😀"]"#),
        (r"\R|a", "l", "a\r\n", r#"[1,"\r\n"]"#),
        (r"\R|aa", "l", "aa\u{2028}", "[2,\"\u{2028}\"]"),
        ("a . # comment", "lxm", "a\nb", r#"[0,"a\n"]"#),
        ("a.", "lm", "a\nb", r#"[0,"a\n"]"#),
        ("(?s:a.)", "l", "a\nb", r#"[0,"a\n"]"#),
        ("(?>a|ab)|bb", "l", "abb", r#"[1,"bb"]"#),
        ("a(?=bb)|bb", "l", "abb", r#"[1,"bb"]"#),
        ("(?<=a)bb|a", "l", "abb", r#"[1,"bb"]"#),
        (r"a\Kbb|bbb", "l", "abbbbb", r#"[1,"bb"]"#),
        (r"a|(?<=\Ga)bb", "l", "abb", r#"[1,"bb"]"#),
    ] {
        let query = format!(
            "match({}; {}) | [.offset, .string]",
            serde_json::to_string(pattern).unwrap(),
            serde_json::to_string(flags).unwrap(),
        );
        let input = serde_json::to_string(input).unwrap();
        assert_eq!(evaluate(&query, &input)[0].to_string(), expected, "{query}");
    }
}

#[test]
fn regex_longest_match_absent_literal_accepts_large_input() {
    for size in [1_000_000, 1_048_576] {
        let input = serde_json::to_string(&"b".repeat(size)).unwrap();
        assert_eq!(evaluate(r#"test("a")"#, &input), [Value::Bool(false)]);
        assert_eq!(evaluate(r#"test("a"; "l")"#, &input), [Value::Bool(false)]);
        assert_eq!(evaluate(r#"match("a")"#, &input), [] as [Value; 0]);
        assert_eq!(evaluate(r#"match("a"; "l")"#, &input), [] as [Value; 0]);
    }
}

#[test]
fn regex_longest_match_presence_scan_keeps_work_limits() {
    let input = serde_json::to_string(&"b".repeat(1_048_576)).unwrap();
    assert_eq!(
        evaluate_with_limits(
            r#"match("a"; "l")"#,
            &input,
            VmLimits {
                regex_backtrack_limit: 512,
                ..VmLimits::default()
            }
        ),
        Ok(Vec::new())
    );
    for limit in [1, 256] {
        assert_eq!(
            evaluate_with_limits(
                r#"match("a"; "l")"#,
                &input,
                VmLimits {
                    regex_backtrack_limit: limit,
                    ..VmLimits::default()
                }
            ),
            Err(VmError::Resource {
                resource: "regex-backtrack"
            })
        );
    }
    let input = serde_json::to_string(&format!("a{}", "b".repeat(1_048_575))).unwrap();
    assert_eq!(
        evaluate_with_limits(
            r#"match("a"; "l")"#,
            &input,
            VmLimits {
                regex_backtrack_limit: 512,
                ..VmLimits::default()
            }
        ),
        Err(VmError::Resource {
            resource: "regex-backtrack"
        })
    );
}

#[test]
fn regex_longest_match_hostile_presence_scan_keeps_engine_limit() {
    assert_eq!(
        evaluate_with_limits(
            r#"match("^(?:(a|aa)+)\\1$"; "l")"#,
            r#""aaaaaaaaaaaaaaaaaaaab""#,
            VmLimits {
                regex_backtrack_limit: 1_000,
                ..VmLimits::default()
            }
        ),
        Err(VmError::Resource {
            resource: "regex-backtrack"
        })
    );
}

#[test]
fn regex_longest_match_moderate_input_skips_nonmatching_starts() {
    let input = serde_json::to_string(&format!("a{}", "b".repeat(500))).unwrap();
    let started = std::time::Instant::now();
    assert_eq!(
        evaluate(r#"match("a"; "l").string"#, &input),
        [Value::string("a")]
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "a single short match must not probe every endpoint at nonmatching starts"
    );
}

#[test]
fn regex_longest_match_rejects_whole_pattern_recursion_without_changing_normal_mode() {
    assert_eq!(
        evaluate(r#"match("a(?:\\g<0>)?").string"#, r#""aaa""#),
        [Value::string("aaa")]
    );
    let error = evaluate_with_limits(
        r#"match("a(?:\\g<0>)?"; "l")"#,
        r#""aaa""#,
        VmLimits::default(),
    )
    .expect_err("longest mode must not silently change whole-pattern recursion");
    assert!(
        matches!(error, VmError::Unsupported { operation } if operation.contains("whole-pattern recursion"))
    );
}

#[test]
fn regex_longest_match_recursion_detection_uses_parsed_syntax() {
    for (query, input, expected) in [
        (r#"match("a # \\g<0>"; "lx").string"#, r#""a""#, "a"),
        (r#"match("\\\\g<0>"; "l").string"#, r#""\\g<0>""#, r"\g<0>"),
        (r#"match("(?<x>a)\\g<x>"; "l").string"#, r#""aa""#, "aa"),
    ] {
        assert_eq!(evaluate(query, input), [Value::string(expected)]);
    }
    let error = evaluate_with_limits(
        r#"match("(?x)a (?: \\g<0> )?"; "l")"#,
        r#""aaa""#,
        VmLimits::default(),
    )
    .expect_err("inline flags must not hide whole-pattern recursion");
    assert!(matches!(error, VmError::Unsupported { .. }));
}

#[test]
fn regex_longest_match_cumulative_budget_is_shared_across_global_pulls() {
    let limits = VmLimits {
        regex_backtrack_limit: 1_500,
        ..VmLimits::default()
    };
    assert_eq!(
        evaluate_with_limits(r#"first(match("a"; "lg") | .string)"#, r#""aa""#, limits).unwrap(),
        [Value::string("a")]
    );
    assert_eq!(
        evaluate_with_limits(r#"[match("a"; "lg")]"#, r#""aa""#, limits),
        Err(VmError::Resource {
            resource: "regex-backtrack"
        })
    );
}

#[test]
fn regex_longest_match_unknown_bounds_exhaust_small_weighted_budget_promptly() {
    let input = serde_json::to_string(&"a".repeat(100)).unwrap();
    let started = std::time::Instant::now();
    for pattern in [r"(a)\1?", r"(?<x>a)\g<x>?", "(a)(?(1)a|b)", "a(?=a*)"] {
        let query = format!("match({}; \"l\")", serde_json::to_string(pattern).unwrap());
        assert_eq!(
            evaluate_with_limits(
                &query,
                &input,
                VmLimits {
                    regex_backtrack_limit: 10_000,
                    ..VmLimits::default()
                }
            ),
            Err(VmError::Resource {
                resource: "regex-backtrack"
            }),
            "{query} must retain the exhaustive fallback"
        );
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn regex_longest_match_preserves_search_anchor_origin_when_checking_later_starts() {
    assert_eq!(
        evaluate(r#"match("a|(?<=\\Ga)bb"; "l").string"#, r#""abb""#),
        [Value::string("bb")]
    );
}

#[test]
fn regex_longest_match_bounds_candidate_searches() {
    let error = evaluate_with_limits(
        r#"match("a"; "l")"#,
        r#""abcdef""#,
        VmLimits {
            regex_backtrack_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("candidate searches must not reset an unlimited work budget");
    assert_eq!(
        error,
        VmError::Resource {
            resource: "regex-backtrack"
        }
    );
}

#[test]
fn regex_longest_match_preserves_input_pattern_and_match_resource_limits() {
    for (limits, query, input, resource) in [
        (
            VmLimits {
                regex_input_bytes: 1,
                ..VmLimits::default()
            },
            r#"match("a"; "l")"#,
            r#""ab""#,
            "regex-input-bytes",
        ),
        (
            VmLimits {
                regex_pattern_bytes: 1,
                ..VmLimits::default()
            },
            r#"match("a|ab"; "l")"#,
            r#""ab""#,
            "regex-pattern-bytes",
        ),
        (
            VmLimits {
                regex_match_limit: 1,
                ..VmLimits::default()
            },
            r#"[match("a"; "lg")]"#,
            r#""aa""#,
            "regex-match-count",
        ),
    ] {
        assert_eq!(
            evaluate_with_limits(query, input, limits),
            Err(VmError::Resource { resource })
        );
    }
}

#[test]
fn malformed_patterns_and_flags_are_runtime_regex_errors() {
    let pattern = evaluate_with_limits(r#"test("[")"#, r#""x""#, VmLimits::default())
        .expect_err("malformed regex patterns are runtime errors");
    assert!(matches!(pattern, VmError::Runtime { message } if message.contains("regex pattern")));

    let flag = evaluate_with_limits(r#"test("x"; "z")"#, r#""x""#, VmLimits::default())
        .expect_err("malformed regex flags are runtime errors");
    assert!(matches!(flag, VmError::Runtime { message } if message.contains("regex flag")));
}

#[test]
fn managed_regex_runtime_errors_flow_through_user_catch() {
    assert_eq!(
        evaluate(r#"def call: test("["); try call catch "caught""#, r#""x""#),
        [Value::string("caught")]
    );
    assert_eq!(
        evaluate(
            r#"def call: test("x"; "z"); try call catch "caught""#,
            r#""x""#,
        ),
        [Value::string("caught")]
    );
    assert_eq!(
        evaluate(r#"def call: test("x"); try call catch "caught""#, r"[1]"),
        [Value::string("caught")]
    );
}

#[test]
fn managed_splits_is_pull_driven_and_stops_after_first_result() {
    let input = serde_json::to_string(&"a,".repeat(1_000)).expect("valid JSON string");
    let result = evaluate_with_limits(
        r#"first(splits("a"))"#,
        &input,
        VmLimits {
            steps: 32,
            ..VmLimits::default()
        },
    )
    .expect("first split result should stop before the tail");
    assert_eq!(result, [Value::string("")]);
}

#[test]
fn regex_backtracking_limit_is_an_observed_resource_boundary() {
    let error = evaluate_with_limits(
        r#"test("(?:(a|aa)+)\\1")"#,
        r#""aa""#,
        VmLimits {
            regex_backtrack_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("hostile backtracking must exhaust the configured budget");
    assert_eq!(
        error,
        VmError::Resource {
            resource: "regex-backtrack"
        }
    );
}

#[test]
fn regex_hostile_input_hits_a_small_nonzero_backtracking_budget() {
    let error = evaluate_with_limits(
        r#"test("^(?:(a|aa)+)\\1$")"#,
        r#""aaaaaaaaaaaaaaaaaaaab""#,
        VmLimits {
            regex_backtrack_limit: 100,
            ..VmLimits::default()
        },
    )
    .expect_err("hostile input must exhaust a small nonzero backtracking budget");
    assert_eq!(
        error,
        VmError::Resource {
            resource: "regex-backtrack"
        }
    );
}

#[test]
fn regex_substitution_vectors_have_explicit_resource_bounds() {
    let matches = evaluate_with_limits(
        r#"gsub("p"; "x")"#,
        r#""pp""#,
        VmLimits {
            regex_match_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("retained regex matches must be bounded");
    assert_eq!(
        matches,
        VmError::Resource {
            resource: "regex-match-count"
        }
    );

    let replacements = evaluate_with_limits(
        r#"gsub("p"; ("x", "y"))"#,
        r#""p""#,
        VmLimits {
            regex_replacement_limit: 1,
            ..VmLimits::default()
        },
    )
    .expect_err("replacement branches must be bounded");
    assert_eq!(
        replacements,
        VmError::Resource {
            resource: "regex-replacement-count"
        }
    );
}

#[test]
fn regex_result_vectors_obey_match_limits_across_all_collectors() {
    for query in [
        r#"[match("p"; "g")]"#,
        r#"[capture("p"; "g")]"#,
        r#"[scan("p"; "g")]"#,
        r#"[splits("p"; "g")]"#,
    ] {
        let error = evaluate_with_limits(
            query,
            r#""pp""#,
            VmLimits {
                regex_match_limit: 1,
                ..VmLimits::default()
            },
        )
        .expect_err("regex collectors must bound retained matches and pieces");
        assert_eq!(
            error,
            VmError::Resource {
                resource: "regex-match-count"
            }
        );
    }
}

#[test]
fn regex_rejects_pre_cancelled_matching_operation() {
    let plan = analyze(
        resolve(
            parse(r#"test("(?:(a|aa)+)\\1")"#).unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let cancellation = Arc::new(AtomicBool::new(true));
    let mut vm = Vm::new(&plan, Value::string("aa"), VmLimits::default())
        .with_cancellation(Arc::clone(&cancellation));
    assert_eq!(vm.next_result(), Err(VmError::Interrupted));
    cancellation.store(false, Ordering::Relaxed);
}

#[test]
fn regex_cancellation_is_checked_between_result_operations() {
    let plan = analyze(
        resolve(
            parse(r#"test("a"), test("b")"#).unwrap(),
            &ResolveOptions::default(),
        )
        .unwrap(),
    )
    .compile()
    .unwrap()
    .document_plan();
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut vm = Vm::new(&plan, Value::string("a"), VmLimits::default())
        .with_cancellation(Arc::clone(&cancellation));
    let mut values = Vec::new();
    let result = vm.for_each_result(|value| {
        values.push(value);
        cancellation.store(true, Ordering::Relaxed);
        true
    });
    assert_eq!(result, Err(VmError::Interrupted));
    assert_eq!(values, [Value::Bool(true)]);
}
