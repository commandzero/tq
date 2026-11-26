//! Parse and execute jq-compatible test files and match their expected results.

use super::automatic::estimate_capture_value_bytes;
use super::input::{open_path, read_limited};
use super::{RUN_TEST_FILE_BYTES, RunError, run_with_io, run_with_io_policy_capture};
use crate::{CliError, Command, ExitStatus, parse_args};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tq_core::{Diagnostic, JsonInputOptions, Label, Value, parse_json_bytes};

pub(super) struct RunTestCase {
    pub(super) line: usize,
    pub(super) filter: String,
    pub(super) input: String,
    pub(super) expected: Vec<String>,
    pub(super) compile_failure: bool,
}

struct RunTestOutcome {
    status: ExitStatus,
    diagnostic: String,
    structured: Option<Box<Diagnostic>>,
    output: Vec<u8>,
    typed_capture: Option<RunTestCaptureSnapshot>,
}

#[derive(Clone)]
pub(super) struct RunTestCaptureSnapshot {
    pub(super) values: Vec<Value>,
    pub(super) overflowed: bool,
}

pub(super) struct RunTestCapture {
    values: Vec<Value>,
    retained_bytes: usize,
    maximum_values: usize,
    maximum_bytes: usize,
    overflowed: bool,
}

pub(super) type RunTestCaptureHandle = Arc<Mutex<RunTestCapture>>;

impl RunTestCapture {
    pub(super) fn new(expected_values: usize) -> Self {
        Self {
            values: Vec::new(),
            retained_bytes: 0,
            maximum_values: expected_values.saturating_add(1),
            maximum_bytes: usize::try_from(RUN_TEST_FILE_BYTES)
                .expect("run-tests file cap fits in usize"),
            overflowed: false,
        }
    }

    pub(super) fn record(&mut self, value: &Value) {
        if self.overflowed {
            return;
        }
        let estimated = estimate_capture_value_bytes(value);
        if self.values.len() >= self.maximum_values
            || self.retained_bytes.saturating_add(estimated) > self.maximum_bytes
            || self.values.try_reserve_exact(1).is_err()
        {
            self.overflowed = true;
            return;
        }
        self.retained_bytes = self.retained_bytes.saturating_add(estimated);
        self.values.push(value.clone());
    }

    pub(super) fn snapshot(&self) -> RunTestCaptureSnapshot {
        RunTestCaptureSnapshot {
            values: self.values.clone(),
            overflowed: self.overflowed,
        }
    }
}

pub(super) fn run_tests<R: Read, W: Write, E: Write>(
    path: Option<&Path>,
    stdin: &mut R,
    stdout: &mut W,
    _stderr: &mut E,
    process_ambient: bool,
) -> Result<ExitStatus, RunError> {
    if path.is_some_and(|path| path != Path::new("-")) && !process_ambient {
        return Err(RunError::Cli(CliError::Incompatible(
            "run-tests file access is disabled for embedded execution".to_owned(),
        )));
    }
    let bytes = if let Some(path) = path.filter(|path| *path != Path::new("-")) {
        read_limited(
            open_path(path)?,
            RUN_TEST_FILE_BYTES,
            &path.display().to_string(),
        )?
    } else {
        read_limited(&mut *stdin, RUN_TEST_FILE_BYTES, "<run-tests>")?
    };
    let source = String::from_utf8(bytes)
        .map_err(|_| RunError::Cli(CliError::Usage("test file must be UTF-8".to_owned())))?;
    let (cases, mut malformed) = parse_run_test_cases(&source);
    let mut passed = 0_usize;
    for (number, case) in cases.iter().enumerate() {
        writeln!(
            stdout,
            "Test #{}: '{}' at line number {}",
            number + 1,
            case.filter,
            case.line
        )?;
        let outcome = run_test_case(case, process_ambient)?;
        let RunTestOutcome {
            status,
            diagnostic,
            structured,
            output,
            typed_capture,
        } = outcome;
        let good = if case.compile_failure {
            status == ExitStatus::Compile
                && compile_diagnostic_matches(
                    &diagnostic,
                    structured.as_deref(),
                    &case.expected,
                    &case.filter,
                )
        } else {
            status == ExitStatus::Success
                && run_test_output_matches(&output, &case.expected, typed_capture.as_ref())
        };
        if good {
            passed = passed.saturating_add(1);
        } else if case.compile_failure {
            malformed = malformed.saturating_add(1);
            if status == ExitStatus::Compile {
                let displayed_diagnostic =
                    compile_diagnostic_report(&diagnostic, structured.as_deref(), &case.filter);
                writeln!(
                    stdout,
                    "*** Erroneous program failed with '{}', but expected '{}' at line number {}: {}",
                    displayed_diagnostic,
                    case.expected.join("\\n"),
                    case.line.saturating_add(1),
                    case.filter
                )?;
            } else {
                writeln!(
                    stdout,
                    "*** Test program did not fail to compile at line {}: {}",
                    case.line, case.filter
                )?;
            }
        } else {
            writeln!(
                stdout,
                "*** Expected {}, but got {} for test at line number {}: {}",
                case.expected.join("\\n"),
                String::from_utf8_lossy(&output).trim_end(),
                case.line,
                case.filter
            )?;
        }
    }
    writeln!(
        stdout,
        "{} of {} tests passed ({} malformed, 0 skipped)",
        passed,
        cases.len(),
        malformed
    )?;
    let self_checks_passed = passed == cases.len() && malformed == 0 && run_tests_self_checks();
    if self_checks_passed {
        writeln!(stdout, "Test jq_state: .[]")?;
        writeln!(
            stdout,
            "Test jq_state: .[] | if .%2 == 0 then halt_error else . end"
        )?;
    }
    if self_checks_passed {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::FalseOrNull)
    }
}

fn compile_diagnostic_report(
    diagnostic: &str,
    structured: Option<&Diagnostic>,
    filter: &str,
) -> String {
    render_jq_compile_diagnostic(filter, diagnostic, structured)
        .and_then(|rendered| rendered.lines().next().map(str::to_owned))
        .unwrap_or_else(|| diagnostic.to_owned())
}

fn run_test_case(case: &RunTestCase, process_ambient: bool) -> Result<RunTestOutcome, RunError> {
    let arguments = vec![
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "json".to_owned(),
        "--compact-output".to_owned(),
        case.filter.clone(),
    ];
    let mut command = parse_args(arguments)?;
    if process_ambient && let Command::Run(options) = &mut command {
        // `--run-tests` is a process-facing command. Its test programs
        // observe the same ambient environment and platform metadata as
        // an ordinary process invocation; embedded `run_with_io` callers
        // retain deny-by-default policy.
        options.allow_environment = true;
        options.allow_platform = true;
    }
    let input = if case.input.is_empty() {
        b"null\n".to_vec()
    } else {
        format!("{}\n", case.input).into_bytes()
    };
    let mut output = Vec::new();
    let mut error = Vec::new();
    let mut input_reader = input.as_slice();
    let capture = Arc::new(Mutex::new(RunTestCapture::new(case.expected.len())));
    let result = run_with_io_policy_capture(
        command,
        &mut input_reader,
        &mut output,
        &mut error,
        false,
        Some(Arc::clone(&capture)),
    );
    let (status, diagnostic, structured) = match result {
        Ok(status) => (status, String::new(), None),
        Err(RunError::Compile(error)) => {
            let diagnostic = format!("query compilation failed: {error}");
            (ExitStatus::Compile, diagnostic, Some(error))
        }
        Err(error) => (error.status(), error.to_string(), None),
    };
    Ok(RunTestOutcome {
        status,
        diagnostic,
        structured,
        output,
        typed_capture: capture.lock().ok().map(|capture| capture.snapshot()),
    })
}

pub(super) fn run_tests_self_checks() -> bool {
    let first = run_tests_self_check(".[]", b"[]\n");
    let second = run_tests_self_check(".[] | if .%2 == 0 then halt_error else . end", b"[1,2,3]\n");
    first.0 == ExitStatus::Success
        && first.1.is_empty()
        && second.0 == ExitStatus::Halt(5)
        && second.1 == b"1\n"
}

fn run_tests_self_check(filter: &str, input: &[u8]) -> (ExitStatus, Vec<u8>) {
    let arguments = vec![
        "--input-format".to_owned(),
        "json".to_owned(),
        "--output-format".to_owned(),
        "json".to_owned(),
        "--compact-output".to_owned(),
        filter.to_owned(),
    ];
    let Ok(command) = parse_args(arguments) else {
        return (ExitStatus::Compile, Vec::new());
    };
    let mut input_reader = input;
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_with_io(command, &mut input_reader, &mut output, &mut error)
        .unwrap_or_else(|failure| failure.status());
    (status, output)
}

pub(super) fn parse_run_test_cases(source: &str) -> (Vec<RunTestCase>, usize) {
    let mut cases = Vec::new();
    let mut malformed = 0_usize;
    let mut block: Vec<(usize, &str)> = Vec::new();
    let mut finish = |block: &mut Vec<(usize, &str)>| {
        if block.is_empty() {
            return;
        }
        let (_, first) = block[0];
        let compile_failure = first == "%%FAIL";
        let filter_offset = usize::from(compile_failure);
        // jq's compile-failure form is `%%FAIL`, filter, expected diagnostic;
        // it deliberately has no input line because compilation stops first.
        let required = if compile_failure {
            filter_offset + 2
        } else {
            2
        };
        if block.len() < required {
            malformed = malformed.saturating_add(1);
        } else {
            cases.push(RunTestCase {
                line: block[filter_offset].0,
                filter: block[filter_offset].1.to_owned(),
                input: if compile_failure {
                    String::new()
                } else {
                    block[1].1.to_owned()
                },
                expected: block[if compile_failure {
                    filter_offset + 1
                } else {
                    2
                }..]
                    .iter()
                    .map(|(_, value)| (*value).to_owned())
                    .collect(),
                compile_failure,
            });
        }
        block.clear();
    };
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            finish(&mut block);
        } else if !line.trim_start().starts_with('#') {
            block.push((line_number, line));
        }
    }
    finish(&mut block);
    (cases, malformed)
}

pub(super) fn compile_diagnostic_matches(
    diagnostic: &str,
    structured: Option<&Diagnostic>,
    expected: &[String],
    filter: &str,
) -> bool {
    let actual = diagnostic.lines().collect::<Vec<_>>();
    if actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| *actual == expected)
    {
        return true;
    }

    if !expected
        .first()
        .is_some_and(|line| line.starts_with("jq: error: "))
    {
        return false;
    }
    let Some(rendered) = render_jq_compile_diagnostic(filter, diagnostic, structured) else {
        return false;
    };
    rendered.lines().eq(expected.iter().map(String::as_str))
}

struct JqDiagnosticParts<'a> {
    code: &'a str,
    message: &'a str,
    label_start: Option<usize>,
    label_end: Option<usize>,
    if_context: Option<(usize, usize)>,
    if_condition: Option<(usize, usize)>,
    object_value: Option<(usize, usize)>,
    construct_context: Option<(usize, usize)>,
}

fn jq_diagnostic_parts<'a>(
    diagnostic: &'a str,
    structured: Option<&'a Diagnostic>,
) -> Option<JqDiagnosticParts<'a>> {
    if let Some(diagnostic) = structured {
        let primary = diagnostic.labels.iter().find(|label| label.primary);
        let span_for = |message| {
            diagnostic
                .labels
                .iter()
                .find(|label| !label.primary && label.message == message)
                .and_then(label_byte_span)
        };
        Some(JqDiagnosticParts {
            code: diagnostic.code.as_str(),
            message: diagnostic.message.as_str(),
            label_start: primary.and_then(|label| usize::try_from(label.span.start).ok()),
            label_end: primary.and_then(|label| usize::try_from(label.span.end).ok()),
            if_context: span_for("unterminated if expression"),
            if_condition: span_for("if condition"),
            object_value: span_for("object value"),
            construct_context: span_for("try expression")
                .or_else(|| span_for("definition context"))
                .or_else(|| span_for("label context")),
        })
    } else {
        let diagnostic = diagnostic.strip_prefix("query compilation failed: ")?;
        let (code, message) = diagnostic.split_once(": ")?;
        Some(JqDiagnosticParts {
            code,
            message,
            label_start: None,
            label_end: None,
            if_context: None,
            if_condition: None,
            object_value: None,
            construct_context: None,
        })
    }
}

fn render_jq_compile_diagnostic(
    filter: &str,
    diagnostic: &str,
    structured: Option<&Diagnostic>,
) -> Option<String> {
    let parts = jq_diagnostic_parts(diagnostic, structured)?;
    let (summary, needle) = jq_diagnostic_summary(filter, &parts)?;
    let JqDiagnosticParts {
        code,
        message,
        label_start,
        label_end,
        if_context,
        if_condition,
        object_value,
        construct_context,
    } = parts;
    let span = JqDiagnosticSpan {
        filter,
        code,
        message,
        label_start,
        label_end,
        needle: &needle,
        if_context,
        if_condition,
        object_value,
        construct_context,
    };
    let (start, end, byte_width) = jq_diagnostic_span(&span)?;
    let line_start = filter[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = filter[start..]
        .find('\n')
        .map_or(filter.len(), |offset| start + offset);
    let line = filter[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let column = filter[line_start..start].chars().count() + 1;
    let width = if byte_width {
        end.min(line_end).saturating_sub(start).max(1)
    } else {
        filter[start..end.min(line_end)].chars().count().max(1)
    };
    let source_line = &filter[line_start..line_end];
    let caret = format!(
        "{}{}",
        " ".repeat(column.saturating_sub(1)),
        "^".repeat(width)
    );
    Some(format!(
        "jq: error: {summary} at <top-level>, line {line}, column {column}:\n    {source_line}\n    {caret}"
    ))
}

fn jq_diagnostic_summary(filter: &str, parts: &JqDiagnosticParts<'_>) -> Option<(String, String)> {
    let JqDiagnosticParts {
        code,
        message,
        label_start,
        label_end,
        if_context,
        if_condition,
        object_value,
        ..
    } = parts;
    let (summary, needle) = if *code == "TQ-RESOLVE-VARIABLE-001" {
        let variable = message.strip_prefix("unknown variable ")?;
        (format!("{variable} is not defined"), variable.to_owned())
    } else if *code == "TQ-RESOLVE-BUILTIN-001" {
        let filter = message.strip_prefix("unknown filter ")?;
        let name = filter.split_once('/').map_or(filter, |(name, _)| name);
        (format!("{filter} is not defined"), name.to_owned())
    } else if *code == "TQ-LEX-STRING-001" && *message == "invalid string escape" {
        let slash = filter.find('\\').unwrap_or(0);
        let column = filter[..slash].chars().count() + 3;
        (
            format!("Invalid escape at line 1, column {column} (while parsing '{filter}')"),
            String::new(),
        )
    } else if object_value.is_some() {
        let (start, end) = object_value.expect("object context was checked");
        (
            jq_unexpected_literal_summary(filter, start, Some(end)),
            String::new(),
        )
    } else if *code == "TQ-PARSE-EXPRESSION-001" {
        if if_context.is_some() {
            (
                "Possibly unterminated 'if' statement".to_owned(),
                String::new(),
            )
        } else {
            match (*label_start).filter(|start| *start < filter.len()) {
                Some(start) => (
                    jq_unexpected_token_summary(filter, start, *label_end),
                    String::new(),
                ),
                None => (
                    "syntax error, unexpected end of file".to_owned(),
                    String::new(),
                ),
            }
        }
    } else if *code == "TQ-PARSE-UNEXPECTED-001"
        && *message == "expected 'then'"
        && (*label_start).is_some_and(|start| start < filter.len())
    {
        let start = label_start.expect("token label was checked");
        (
            format!(
                "syntax error, unexpected {}, expecting then or '|' or ','",
                jq_unexpected_word_summary(filter, start, *label_end)
            ),
            String::new(),
        )
    } else if *code == "TQ-PARSE-UNEXPECTED-001"
        && (*label_start).is_none_or(|start| start >= filter.len())
    {
        if if_context.is_some() && *message != "expected 'then'" {
            (
                "Possibly unterminated 'if' statement".to_owned(),
                String::new(),
            )
        } else if *message == "expected 'then'" && if_condition.is_some() {
            (
                "syntax error, unexpected end of file, expecting then or '|' or ','".to_owned(),
                String::new(),
            )
        } else {
            (jq_unexpected_eof_summary(message)?, String::new())
        }
    } else {
        jq_simple_diagnostic_summary(filter, code, message, *label_start, *label_end)?
    };
    Some((summary, needle))
}

fn jq_simple_diagnostic_summary(
    filter: &str,
    code: &str,
    message: &str,
    label_start: Option<usize>,
    label_end: Option<usize>,
) -> Option<(String, String)> {
    if code == "TQ-LEX-FORMAT-001" {
        return Some((
            format!(
                "syntax error, unexpected INVALID_CHARACTER{}",
                if label_start == Some(0) {
                    ", expecting end of file"
                } else {
                    ""
                }
            ),
            String::new(),
        ));
    }
    if code == "TQ-LEX-STRING-001" && message == "unterminated string" {
        return Some((
            "syntax error, unexpected end of file, expecting QQSTRING_TEXT or QQSTRING_INTERP_START or QQSTRING_END".to_owned(),
            String::new(),
        ));
    }
    if let Some(start) = label_start.filter(|start| *start < filter.len()) {
        let expected = match code {
            "TQ-PARSE-DEF-PARAMETER-001" => Some("IDENT or BINDING"),
            "TQ-PARSE-DEF-001" => Some("IDENT"),
            "TQ-PARSE-LABEL-001" => Some("BINDING"),
            _ => None,
        };
        if let Some(expected) = expected {
            return Some((
                format!(
                    "syntax error, unexpected {}, expecting {expected}",
                    jq_parser_token_name(filter, start, label_end)
                ),
                String::new(),
            ));
        }
    }
    if code == "TQ-LEX-VARIABLE-001" {
        return Some((
            "syntax error, unexpected end of file, expecting '$'".to_owned(),
            String::new(),
        ));
    }
    if label_start.is_none_or(|start| start >= filter.len()) {
        let summary = match code {
            "TQ-PARSE-OBJECT-001" => "syntax error, unexpected end of file",
            "TQ-PARSE-DEF-PARAMETER-001" => {
                "syntax error, unexpected end of file, expecting IDENT or BINDING"
            }
            "TQ-PARSE-DEF-001" => "syntax error, unexpected end of file, expecting IDENT",
            "TQ-PARSE-LABEL-001" => "syntax error, unexpected end of file, expecting BINDING",
            _ => return None,
        };
        return Some((summary.to_owned(), String::new()));
    }
    None
}

#[derive(Clone, Copy)]
struct JqDiagnosticSpan<'a> {
    filter: &'a str,
    code: &'a str,
    message: &'a str,
    label_start: Option<usize>,
    label_end: Option<usize>,
    needle: &'a str,
    if_context: Option<(usize, usize)>,
    if_condition: Option<(usize, usize)>,
    object_value: Option<(usize, usize)>,
    construct_context: Option<(usize, usize)>,
}

fn jq_diagnostic_span(span: &JqDiagnosticSpan<'_>) -> Option<(usize, usize, bool)> {
    let JqDiagnosticSpan {
        filter,
        code,
        message,
        label_start,
        label_end,
        needle,
        if_context,
        if_condition,
        object_value,
        construct_context,
    } = *span;
    let byte_width = code == "TQ-LEX-STRING-001"
        && matches!(message, "unterminated string" | "invalid string escape");
    let span_override = if if_context.is_some()
        && !(code == "TQ-PARSE-UNEXPECTED-001" && message == "expected 'then'")
    {
        if_context
    } else if code == "TQ-PARSE-UNEXPECTED-001"
        && message == "expected 'then'"
        && if_condition.is_some()
        && label_start.is_none_or(|start| start >= filter.len())
    {
        if_condition
    } else if object_value.is_some() {
        object_value
    } else if construct_context.is_some() && label_start.is_none_or(|start| start >= filter.len()) {
        construct_context
    } else if byte_width {
        let start = label_start.unwrap_or(0).min(filter.len());
        let content_start = if filter.as_bytes().get(start) == Some(&b'"') {
            start.saturating_add(1)
        } else {
            start
        };
        if message == "invalid string escape" {
            let slash = filter[content_start..]
                .find('\\')
                .map_or(content_start, |offset| content_start + offset);
            Some((slash, slash.saturating_add(2).min(filter.len())))
        } else {
            Some((
                content_start,
                label_end
                    .unwrap_or(filter.len())
                    .min(filter.len())
                    .max(content_start),
            ))
        }
    } else {
        None
    };
    let (start, end) = if let Some((start, end)) = span_override {
        (start, end)
    } else if needle.is_empty() {
        let start = label_start
            .filter(|start| *start < filter.len())
            .unwrap_or_else(|| jq_eof_token_span(filter).0);
        (
            start,
            label_end.unwrap_or_else(|| jq_eof_token_span(filter).1),
        )
    } else {
        let hint = label_start.unwrap_or(0).min(filter.len());
        let start = filter[hint..]
            .find(needle)
            .map_or_else(|| filter.find(needle), |offset| Some(hint + offset))?;
        (start, start.saturating_add(needle.len()))
    };
    Some((start, end, byte_width))
}

fn label_byte_span(label: &Label) -> Option<(usize, usize)> {
    Some((
        usize::try_from(label.span.start).ok()?,
        usize::try_from(label.span.end).ok()?,
    ))
}

fn jq_unexpected_token_summary(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = filter[start..end].chars().next().unwrap_or('\0');
    let token = match token {
        ']' | '}' | ')' => "INVALID_CHARACTER".to_owned(),
        character => format!("'{character}'"),
    };
    let expectation = if filter[..start].trim().is_empty() {
        ", expecting end of file"
    } else {
        ""
    };
    format!("syntax error, unexpected {token}{expectation}")
}

fn jq_unexpected_literal_summary(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = filter[start..end].chars().next().unwrap_or('\0');
    format!("syntax error, unexpected '{token}'")
}

fn jq_unexpected_word_summary(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = &filter[start..end];
    if token.chars().all(char::is_alphanumeric) {
        token.to_owned()
    } else {
        jq_unexpected_literal_summary(filter, start, Some(end))
            .trim_start_matches("syntax error, unexpected ")
            .to_owned()
    }
}

fn jq_parser_token_name(filter: &str, start: usize, end: Option<usize>) -> String {
    let end = end
        .unwrap_or_else(|| start.saturating_add(1))
        .min(filter.len());
    let token = &filter[start..end];
    if !token.is_empty()
        && token.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, '.' | '-' | '+' | 'e' | 'E')
        })
    {
        "LITERAL".to_owned()
    } else if token.chars().all(char::is_alphanumeric) {
        "IDENT".to_owned()
    } else {
        jq_unexpected_word_summary(filter, start, Some(end))
    }
}

fn jq_eof_token_span(filter: &str) -> (usize, usize) {
    let Some((last_start, last)) = filter.char_indices().next_back() else {
        return (0, 1);
    };
    if !(last.is_alphanumeric() || matches!(last, '_' | '$')) {
        return (last_start, last_start + last.len_utf8());
    }
    let mut start = last_start;
    for (index, character) in filter[..last_start].char_indices().rev() {
        if character.is_alphanumeric() || matches!(character, '_' | '$') {
            start = index;
        } else {
            break;
        }
    }
    (start, filter.len())
}

fn jq_unexpected_eof_summary(message: &str) -> Option<String> {
    let expectation = match message {
        "expected ']' after array constructor" => "'|' or ',' or ']'".to_owned(),
        "expected '}' after object constructor" => "'}'".to_owned(),
        "expected ')' after grouped expression" | "expected ')' after computed object key" => {
            "'|' or ',' or ')'".to_owned()
        }
        "expected ')' after function arguments" => "';' or ')'".to_owned(),
        "expected ')' after string interpolation" => "QQSTRING_INTERP_END or '|' or ','".to_owned(),
        "expected 'then'" => "then or '|' or ','".to_owned(),
        "expected '|' after label variable" => "'|'".to_owned(),
        message => message.strip_prefix("expected ")?.to_owned(),
    };
    Some(format!(
        "syntax error, unexpected end of file, expecting {expectation}"
    ))
}

pub(super) fn run_test_output_matches(
    actual: &[u8],
    expected: &[String],
    typed_capture: Option<&RunTestCaptureSnapshot>,
) -> bool {
    let Some(capture) = typed_capture else {
        return false;
    };
    if capture.overflowed {
        return false;
    }
    let actual_lines = actual
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if actual_lines.len() != expected.len() {
        return false;
    }
    let parsed_actual = actual_lines
        .iter()
        .map(|line| parse_json_bytes(line, JsonInputOptions::default()))
        .collect::<Result<Vec<_>, _>>();
    let parsed_expected = expected
        .iter()
        .map(|line| parse_json_bytes(line.as_bytes(), JsonInputOptions::default()))
        .collect::<Result<Vec<_>, _>>();
    let expected_is_json = parsed_expected.is_ok();
    match (parsed_actual, parsed_expected) {
        (Ok(actual), Ok(expected)) => {
            if capture.values != expected {
                return false;
            }
            actual == expected
        }
        _ if expected_is_json => false,
        _ => actual_lines
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual == &expected.as_bytes()),
    }
}
