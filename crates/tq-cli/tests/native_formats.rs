//! Native format selection through the CLI boundary.

use tq_cli::{Command, parse_args};
use tq_formats::{OutputFormat, ToonFraming};

#[test]
fn sequence_evaluation_publishes_generator_results_before_runtime_failure() {
    for (format, input) in [
        ("csv", "id\n1\n2\n"),
        ("tsv", "id\n1\n2\n"),
        ("jsonl", "{\"id\":1}\n{\"id\":2}\n"),
        ("json-seq", "\x1e{\"id\":1}\n\x1e{\"id\":2}\n"),
        ("toon-seq", "\x1eid: 1\n\x1eid: 2\n"),
    ] {
        let command = parse_args([
            "-i",
            format,
            "-o",
            "json",
            "-c",
            ".id, (.id + 10), error(\"stop\")",
        ])
        .unwrap();
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let status =
            tq_cli::run_with_io(command, &mut input.as_bytes(), &mut output, &mut errors).unwrap();
        assert_eq!(output, b"1\n11\n2\n12\n", "{format}");
        assert_eq!(status, tq_cli::ExitStatus::Runtime, "{format}");
        let errors = String::from_utf8(errors).unwrap();
        assert_eq!(errors.matches("stop").count(), 2, "{format}: {errors}");
    }
}

#[test]
fn sequence_output_limit_stops_a_large_generator() {
    let command = parse_args([
        "-i",
        "jsonl",
        "-o",
        "json",
        "-c",
        "--max-output-bytes",
        "2",
        ".id, range(0; 100000000)",
    ])
    .unwrap();
    let mut output = Vec::new();
    let error = tq_cli::run_with_io(
        command,
        &mut b"{\"id\":1}\n".as_slice(),
        &mut output,
        &mut Vec::new(),
    )
    .unwrap_err();
    assert_eq!(output, b"1\n");
    assert_eq!(error.status(), tq_cli::ExitStatus::Resource);
    assert!(error.to_string().contains("output"));
}

#[test]
fn sequence_reports_keep_per_document_observations_when_requested() {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("report.json");
    let command = parse_args([
        "-i",
        "jsonl",
        "-o",
        "json",
        "-c",
        "--report-file",
        report.to_str().unwrap(),
        ".id, (.id + 10)",
    ])
    .unwrap();
    let mut output = Vec::new();
    tq_cli::run_with_io(
        command,
        &mut b"{\"id\":1}\n{\"id\":2}\n".as_slice(),
        &mut output,
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(output, b"1\n11\n2\n12\n");
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert_eq!(report["documents"], 2);
    assert_eq!(report["results"], 4);
    let observations = report["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 2);
    for observation in observations {
        assert_eq!(observation["results"], 2);
        assert!(observation["steps"].as_u64().unwrap() > 0);
    }
}

#[test]
fn sequence_reports_preserve_trivial_query_observations() {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("report.json");
    for (query, expected, results) in [
        (".", "1\n2\n", 1),
        ("42", "42\n42\n", 1),
        ("$n", "7\n7\n", 1),
        ("empty", "", 0),
    ] {
        let command = parse_args([
            "-i",
            "jsonl",
            "-o",
            "json",
            "-c",
            "--argjson",
            "n",
            "7",
            "--report-file",
            report.to_str().unwrap(),
            query,
        ])
        .unwrap();
        let mut output = Vec::new();
        tq_cli::run_with_io(
            command,
            &mut b"1\n2\n".as_slice(),
            &mut output,
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(output, expected.as_bytes(), "{query}");
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(report["documents"], 2);
        for observation in report["observations"].as_array().unwrap() {
            assert_eq!(observation["steps"], 0, "{query}");
            assert_eq!(observation["results"], results, "{query}");
        }
    }
}

#[test]
fn proxy_stream_inputs_accepts_a_detected_empty_json_sequence() {
    let command = parse_args(["-x", "--stream", "inputs"]).unwrap();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    assert_eq!(
        tq_cli::run_with_io(command, &mut b"\x1e".as_slice(), &mut output, &mut errors).unwrap(),
        tq_cli::ExitStatus::Success
    );
    assert_eq!(output, [] as [u8; 0]);
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn json_sequence_input_stream_records_share_the_query_cursor() {
    for (query, expected) in [
        ("inputs", "[[0],1]\n[[1],2]\n[[1]]\n"),
        ("[inputs]", "[[[0],1],[[1],2],[[1]]]\n"),
        ("input | (.[1] = 42)", "[[0],42]\n"),
    ] {
        let command = parse_args([
            "-n", "--stream", "-i", "json-seq", "-o", "json", "-c", query,
        ])
        .unwrap();
        let mut input = b"\x1e[1,2]\n".as_slice();
        let mut output = Vec::new();
        let mut errors = Vec::new();
        tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
        assert_eq!(output, expected.as_bytes(), "{query}");
        assert_eq!(errors, [] as [u8; 0]);
    }
}

#[test]
fn json_sequence_input_stream_cursor_consumes_files_in_order() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.json-seq");
    let second = directory.path().join("second.json-seq");
    std::fs::write(&first, b"\x1e[1]\n").unwrap();
    std::fs::write(&second, b"\x1e[2]\n").unwrap();
    let command = parse_args([
        "-n",
        "--stream",
        "-i",
        "json-seq",
        "-o",
        "json",
        "-c",
        "[inputs]",
        first.to_str().unwrap(),
        second.to_str().unwrap(),
    ])
    .unwrap();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut b"".as_slice(), &mut output, &mut errors).unwrap();
    assert_eq!(output, b"[[[0],1],[[0]],[[0],2],[[0]]]\n");
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn json_sequence_input_stream_errors_reset_paths_between_recoverable_failures() {
    let command =
        parse_args(["--stream-errors", "-i", "json-seq", "-o", "json", "-c", "."]).unwrap();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(
        command,
        &mut b"\x1e[1,broken x ".as_slice(),
        &mut output,
        &mut errors,
    )
    .unwrap();
    assert_eq!(output, b"[[0],1]\n[\"Invalid numeric literal at line 1, column 11 (need RS to resync)\",[1]]\n[\"Invalid numeric literal at line 1, column 13 (need RS to resync)\",[]]\n");
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn json_sequence_input_stream_cursor_exposes_catchable_failures() {
    let expected = b"[[0],1]\n[[1],2]\n[[1]]\n[[0],3]\n";
    for query in ["inputs", "[inputs]", "try inputs catch ."] {
        let command = parse_args([
            "-n", "--stream", "-i", "json-seq", "-o", "json", "-c", query,
        ])
        .unwrap();
        let mut input = b"\x1e[1,2]\n\x1e[3,broken]\n".as_slice();
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let result = tq_cli::run_with_io(command, &mut input, &mut output, &mut errors);
        if query == "try inputs catch ." {
            assert_eq!(result.unwrap(), tq_cli::ExitStatus::Success);
            let mut caught = expected.to_vec();
            caught.extend_from_slice(
                b"\"Invalid numeric literal at line 2, column 11 (need RS to resync)\"\n",
            );
            assert_eq!(output, caught);
        } else {
            assert_eq!(result.unwrap_err().status(), tq_cli::ExitStatus::Runtime);
            assert_eq!(
                output,
                if query == "inputs" {
                    expected.as_slice()
                } else {
                    b""
                }
            );
        }
        assert!(
            errors.is_empty(),
            "query-consumed failures must not produce warnings"
        );
    }
}

#[test]
fn delimited_output_preserves_header_across_sources_and_proxy_bytes_with_one_budget() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.json");
    let rejected = directory.path().join("rejected.json");
    let last = directory.path().join("last.json");
    std::fs::write(&first, b"{\"a\":1,\"b\":2}").unwrap();
    std::fs::write(&rejected, b"invalid\n").unwrap();
    std::fs::write(&last, b"{\"b\":4,\"a\":3}").unwrap();
    for budget in ["20", "19"] {
        let command = parse_args([
            "-x",
            "-i",
            "json",
            "-o",
            "csv",
            "--max-output-bytes",
            budget,
            ".",
            first.to_str().unwrap(),
            rejected.to_str().unwrap(),
            last.to_str().unwrap(),
        ])
        .unwrap();
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let result = tq_cli::run_with_io(command, &mut b"".as_slice(), &mut output, &mut errors);
        if budget == "20" {
            assert_eq!(result.unwrap(), tq_cli::ExitStatus::Success);
            assert_eq!(output, b"a,b\n1,2\ninvalid\n3,4\n");
        } else {
            assert_eq!(result.unwrap_err().status(), tq_cli::ExitStatus::Resource);
            assert!(output.starts_with(b"a,b\n1,2\ninvalid\n"));
            assert!(output.len() <= 19);
        }
        assert_eq!(errors, [] as [u8; 0]);
    }
}

#[test]
fn json_sequence_input_stream_slurp_is_one_input_in_the_shared_cursor() {
    for (flags, query, expected) in [
        (vec!["-s"], ".", "[[[0],1],[[1],2],[[1]]]\n"),
        (vec!["-n", "-s"], "[inputs]", "[[[[0],1],[[1],2],[[1]]]]\n"),
        (vec!["-n", "-s"], ".", "null\n"),
    ] {
        let mut args = vec!["--stream", "-i", "json-seq", "-o", "json", "-c"];
        args.extend(flags);
        args.push(query);
        let command = parse_args(args).unwrap();
        let mut output = Vec::new();
        let mut errors = Vec::new();
        tq_cli::run_with_io(
            command,
            &mut b"\x1e[1,2]\n".as_slice(),
            &mut output,
            &mut errors,
        )
        .unwrap();
        assert_eq!(output, expected.as_bytes(), "{query}");
        assert_eq!(errors, [] as [u8; 0]);
    }
}

#[test]
fn json_sequence_input_stream_errors_are_values_on_the_remaining_cursor() {
    let command = parse_args([
        "--stream-errors",
        "-n",
        "-i",
        "json-seq",
        "-o",
        "json",
        "-c",
        "[inputs]",
    ])
    .unwrap();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(
        command,
        &mut b"\x1e[1,broken]\x1e2\n".as_slice(),
        &mut output,
        &mut errors,
    )
    .unwrap();
    assert_eq!(output, b"[[[0],1],[\"Invalid numeric literal at line 1, column 11 (need RS to resync)\",[1]],[[],2]]\n");
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn stream_projection_remains_available_with_proxy_and_remaining_input() {
    let command = parse_args([
        "-x", "-n", "--stream", "-i", "json", "-o", "json", "-c", "[inputs]",
    ])
    .unwrap();
    let mut output = Vec::new();
    tq_cli::run_with_io(
        command,
        &mut b"[1,2]".as_slice(),
        &mut output,
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(output, b"[[[0],1],[[1],2],[[1]]]\n");
}

#[test]
fn proxy_stream_slurp_preserves_invalid_json_source_bytes() {
    let command = parse_args(["-x", "-s", "--stream", "-i", "json", "."]).unwrap();
    let input = b"{\"unfinished\":\xff\n";
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let status =
        tq_cli::run_with_io(command, &mut input.as_slice(), &mut output, &mut errors).unwrap();
    assert_eq!(status, tq_cli::ExitStatus::Success);
    assert_eq!(output, input);
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn json_sequence_input_frame_limit_is_fatal_even_with_a_later_separator() {
    let command = parse_args(["--seq", "--max-frame-bytes", "3", "."]).unwrap();
    let mut input = b"\x1e[1,2]\n\x1e3\n".as_slice();
    let mut output = Vec::new();
    let error = tq_cli::run_with_io(command, &mut input, &mut output, &mut Vec::new()).unwrap_err();
    assert_eq!(error.status(), tq_cli::ExitStatus::Resource);
    assert_eq!(output, [] as [u8; 0]);
}

#[test]
fn json_sequence_stream_enforces_input_byte_limit() {
    let command = parse_args([
        "--stream",
        "--seq",
        "-o",
        "json",
        "--max-input-bytes",
        "3",
        ".",
    ])
    .unwrap();
    let mut input = b"\x1e[1]\n".as_slice();
    let mut output = Vec::new();
    let error = tq_cli::run_with_io(command, &mut input, &mut output, &mut Vec::new()).unwrap_err();
    assert_eq!(error.status(), tq_cli::ExitStatus::Resource);
    assert_eq!(output, [] as [u8; 0]);
}

#[test]
fn delimited_cli_converts_rows_and_enforces_configured_field_counts() {
    let command = parse_args(["-i", "csv", "-o", "tsv", "."]).unwrap();
    let mut input = b"a,b\n1,\"42\"\n2,\n".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
    assert_eq!(output, b"a\tb\n1\t\"42\"\n2\t\n");
    let command = parse_args(["-i", "csv", "--max-fields", "1", "."]).unwrap();
    let mut input = b"a,b\n1,2\n".as_slice();
    let error =
        tq_cli::run_with_io(command, &mut input, &mut Vec::new(), &mut Vec::new()).unwrap_err();
    assert_eq!(error.status(), tq_cli::ExitStatus::Resource);
}

#[test]
fn delimited_options_reject_incompatible_controls_before_input() {
    for format in ["csv", "tsv"] {
        for controls in [
            vec!["--pretty-output"],
            vec!["--indent", "1"],
            vec!["--tab"],
            vec!["--ascii-output"],
            vec!["--raw-output"],
            vec!["--join-output"],
            vec!["--color-output"],
            vec!["--monochrome-output"],
            vec!["--unframed"],
        ] {
            let mut args = vec!["-o", format];
            args.extend(controls);
            assert!(parse_args(args.clone()).is_err(), "{args:?}");
        }
    }
}

#[test]
fn strict_conversion_rejects_missing_key_normalization_after_prior_rows() {
    let command = parse_args([
        "-n",
        "-o",
        "csv",
        "--strict-conversion",
        "{a: 1, b: 2}, {a: 3}",
    ])
    .unwrap();
    let mut input = b"".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let error = tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap_err();
    assert!(error.to_string().contains("strict conversion"));
    assert_eq!(output, b"a,b\n1,2\n");
}

#[test]
fn seq_defaults_to_toon_and_accepts_json_output_in_either_order() {
    let Command::Run(options) = parse_args(["--seq", "."]).unwrap() else {
        panic!("run")
    };
    assert_eq!(options.input_format, tq_formats::InputFormat::JsonSequence);
    assert_eq!(options.output_format, OutputFormat::Toon);
    assert_eq!(options.framing, ToonFraming::Sequence);
    assert!(options.json_sequence);

    for args in [
        vec!["--seq", "-i", "json"],
        vec!["-i", "json", "--seq"],
        vec!["--seq", "-i", "json", "-o", "toon"],
        vec!["-i", "json", "--seq", "-o", "toon"],
        vec!["--seq", "-o", "toon-seq"],
        vec!["-o", "toon-seq", "--seq"],
        vec!["--seq", "-o", "json"],
        vec!["-o", "json", "--seq"],
        vec!["--seq", "-c"],
    ] {
        let Command::Run(options) = parse_args(args.clone()).unwrap() else {
            panic!("run: {args:?}")
        };
        assert_eq!(
            options.input_format,
            tq_formats::InputFormat::JsonSequence,
            "{args:?}"
        );
        assert_eq!(options.framing, ToonFraming::Sequence, "{args:?}");
        assert!(options.json_sequence, "{args:?}");
    }
    for input in ["csv", "tsv", "yaml"] {
        assert!(
            parse_args(["--seq", "-i", input]).is_err(),
            "--seq should reject {input} input"
        );
    }
    for args in [
        vec!["--seq", "--toon-sequence-input"],
        vec!["--toon-sequence-input", "--seq"],
        vec!["--unframed", "--seq"],
        vec!["--seq", "--unframed"],
    ] {
        assert!(parse_args(args.clone()).is_err(), "{args:?}");
    }
    let command = parse_args(["--seq", "-i", "jsonseq", "-o", "json-seq", "-c", "."]).unwrap();
    let mut input = b"\x1e{\"a\":1}\n\x1e{\"b\":2}\n".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
    assert_eq!(output, b"\x1e{\"a\":1}\n\x1e{\"b\":2}\n");
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn json_sequence_rejects_forced_color_output() {
    assert!(parse_args(["-o", "json-seq", "--color-output", "."]).is_err());
    assert!(parse_args(["-o", "json-seq", "-C", "."]).is_err());
}

#[test]
fn json_sequence_output_uses_json_formatting_controls() {
    let command = parse_args([
        "-n",
        "-o",
        "json-seq",
        "--indent",
        "1",
        "--ascii-output",
        "{a: \"é\"}",
    ])
    .unwrap();
    let mut input = b"".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
    assert_eq!(output, b"\x1e{\n \"a\": \"\\u00e9\"\n}\n");
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn explicit_toon_sequence_output_uses_sequence_framing() {
    let Command::Run(options) = parse_args(["-o", "toon-seq", "."]).unwrap() else {
        panic!("run command")
    };
    assert_eq!(options.output_format, OutputFormat::Toon);
    assert_eq!(options.framing, ToonFraming::Sequence);
    assert!(parse_args(["-o", "toon-seq", "--unframed", "."]).is_err());
}

#[test]
fn null_input_runs_once_but_leaves_documents_available_to_input_functions() {
    for (query, expected) in [
        ("inputs | .id", "1\n2\n"),
        ("input | .id", "1\n"),
        ("[input, input] | map(.id)", "[1,2]\n"),
        (
            "[inputs], (try input catch .)",
            "[{\"id\":1},{\"id\":2}]\n\"break\"\n",
        ),
    ] {
        let command = parse_args(["-n", "-i", "jsonl", "-o", "json", "-c", query]).unwrap();
        let mut input = b"{\"id\":1}\n{\"id\":2}\n".as_slice();
        let mut output = Vec::new();
        let mut errors = Vec::new();
        assert_eq!(
            tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap(),
            tq_cli::ExitStatus::Success
        );
        assert_eq!(String::from_utf8(output).unwrap(), expected, "{query}");
        assert_eq!(errors, [] as [u8; 0]);
    }
}

#[test]
fn event_input_resets_paths_for_each_json_document() {
    let command = parse_args(["--stream", "-i", "json", "-o", "json", "-c", "."]).unwrap();
    let mut input = b"{\"a\":1} {\"b\":2}".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "[[\"a\"],1]\n[[\"a\"]]\n[[\"b\"],2]\n[[\"b\"]]\n"
    );
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn event_input_retains_jq_error_value_mapping() {
    let command = parse_args(["--stream-errors", "-i", "json", "-o", "json", "-c", "."]).unwrap();
    let mut input = b"[1, bad, 2]".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "[[0],1]\n[\"Invalid numeric literal at line 1, column 8\",[1]]\n"
    );
    assert_eq!(errors, [] as [u8; 0]);
}

#[test]
fn json_sequence_input_warns_between_published_documents() {
    let command = parse_args(["-i", "json-seq", "-o", "json", "-c", "."]).unwrap();
    let mut input = b"\x1e{\"a\":1} broken {\"skip\":2}\x1e{\"b\":3}\n".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "{\"a\":1}\n{\"skip\":2}\n{\"b\":3}\n"
    );
    assert!(
        String::from_utf8(errors)
            .unwrap()
            .contains("ignoring parse error")
    );
}

#[test]
fn json_sequence_input_functions_raise_parse_failures() {
    let command = parse_args(["-n", "-i", "json-seq", "-o", "json", "-c", "[inputs]"]).unwrap();
    let mut input = b"\x1e{\"a\":1} broken\x1e{\"b\":2}\n".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let error = tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap_err();
    assert_eq!(error.status(), tq_cli::ExitStatus::Runtime);
    assert_eq!(output, [] as [u8; 0]);
    assert!(
        errors.is_empty(),
        "query errors must not also emit recovery warnings"
    );
}

#[test]
fn json_sequence_input_parse_failures_are_catchable_without_warnings() {
    for query in ["try inputs catch .", "try input catch ."] {
        let command = parse_args(["-n", "-i", "json-seq", "-o", "json", "-c", query]).unwrap();
        let mut input = b"\x1ebroken\x1e2\n".as_slice();
        let mut output = Vec::new();
        let mut errors = Vec::new();
        assert_eq!(
            tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap(),
            tq_cli::ExitStatus::Success
        );
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value, "Truncated value at line 1, column 8");
        assert!(
            errors.is_empty(),
            "caught errors must not emit recovery warnings"
        );
    }
}

#[test]
fn json_sequence_input_outer_driver_resumes_after_query_failure() {
    for (bytes, expected, status) in [
        (
            b"\x1e0\n\x1ebad true 2\n\x1e3\n".as_slice(),
            "2\n3\n",
            tq_cli::ExitStatus::Success,
        ),
        (
            b"\x1e0\n\x1ebad\n".as_slice(),
            "",
            tq_cli::ExitStatus::Runtime,
        ),
        (
            b"\x1e0\n\x1ebad\n\x1e3\n".as_slice(),
            "",
            tq_cli::ExitStatus::Success,
        ),
    ] {
        let command = parse_args(["-i", "json-seq", "-o", "json", "-c", "inputs"]).unwrap();
        let mut input = bytes;
        let mut output = Vec::new();
        let mut errors = Vec::new();
        assert_eq!(
            tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap(),
            status
        );
        assert_eq!(output, expected.as_bytes());
        let errors = String::from_utf8(errors).unwrap();
        assert!(errors.contains("Invalid numeric literal"));
        assert!(!errors.contains("ignoring parse error"));
    }
}

#[test]
fn json_sequence_input_slurp_keeps_complete_documents_around_failures() {
    let command = parse_args(["-s", "-i", "json-seq", "-o", "json", "-c", "."]).unwrap();
    let mut input = b"\x1e1\n\x1ebroken\x1e2\n".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap();
    assert_eq!(String::from_utf8(output).unwrap(), "[1,2]\n");
    assert!(
        String::from_utf8(errors)
            .unwrap()
            .contains("ignoring parse error")
    );
}

#[test]
fn json_sequence_input_keeps_prior_warnings_before_a_fatal_resource_error() {
    let command = parse_args([
        "-i",
        "json-seq",
        "-o",
        "json",
        "--max-token-bytes",
        "5",
        "inputs",
    ])
    .unwrap();
    let mut input = b"\x1ebad\x1e\"abcdefghijklmnop\"\n".as_slice();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let error = tq_cli::run_with_io(command, &mut input, &mut output, &mut errors).unwrap_err();
    assert_eq!(error.status(), tq_cli::ExitStatus::Resource);
    assert_eq!(output, [] as [u8; 0]);
    assert!(
        String::from_utf8(errors)
            .unwrap()
            .contains("ignoring parse error")
    );
}
