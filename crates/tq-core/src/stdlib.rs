//! Bounded regex and UTC-first date/time helpers for jq-compatible built-ins.

use std::sync::Arc;

use fancy_regex::{Captures, Error as RegexError, Match, Regex, RegexBuilder, RegexInput};
use indexmap::IndexMap;
use jiff::{
    Timestamp,
    civil::DateTime,
    fmt::strtime,
    tz::{Offset, TimeZone},
};
use num_traits::ToPrimitive as _;

use crate::{Number, Value, VmError, VmLimits};

pub(crate) struct RegexProgram {
    regex: Regex,
    global: bool,
    ignore_empty: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RegexPullKind {
    Match,
    Capture,
    Scan,
    Replacement,
    Span,
}

#[derive(Debug)]
pub(crate) enum RegexPull {
    Match {
        value: Value,
    },
    Capture {
        value: Value,
    },
    Scan {
        value: Value,
    },
    Replacement {
        start: usize,
        end: usize,
        context: Value,
    },
    Span {
        start: usize,
        end: usize,
    },
}

/// An owned, pull-driven regex operation.
///
/// The compiled program and haystack are owned by `Arc`s so this cursor can be
/// stored in a suspended generator without a self-referential iterator. Each
/// search iteration performs one public fancy-regex engine search, with a
/// checkpoint before and after that bounded call, and converts temporary
/// captures into an owned `Value` before returning. A pull may skip duplicate
/// or ignored empty matches. Checkpoints do not interrupt execution inside a
/// single engine call; the configured engine limits bound that work.
pub(crate) struct RegexCursor {
    program: Arc<RegexProgram>,
    input: Arc<str>,
    kind: RegexPullKind,
    limits: VmLimits,
    next_start: usize,
    last_match_end: Option<usize>,
    last_skipped_empty: bool,
    seen: usize,
    stop_after_one: bool,
    done: bool,
}

/// Compile one regex operation into an owned cursor. `force_global` is used by
/// `gsub`, `scan`, and split operations; otherwise the compiled `g` flag
/// controls whether one or all matches are pulled.
pub(crate) fn regex_cursor(
    input: Arc<str>,
    pattern: &str,
    flags: &str,
    kind: RegexPullKind,
    force_global: bool,
    limits: VmLimits,
) -> Result<RegexCursor, VmError> {
    let program = Arc::new(compile_regex(&input, pattern, flags, limits)?);
    let stop_after_one = !force_global && !program.global;
    Ok(RegexCursor {
        program,
        input,
        kind,
        limits,
        next_start: 0,
        last_match_end: None,
        last_skipped_empty: false,
        seen: 0,
        stop_after_one,
        done: false,
    })
}

impl RegexCursor {
    pub(crate) fn next(
        &mut self,
        checkpoint: &dyn Fn() -> Result<(), VmError>,
    ) -> Result<Option<RegexPull>, VmError> {
        loop {
            if self.done {
                return Ok(None);
            }
            checkpoint()?;
            if self.next_start > self.input.len() {
                self.done = true;
                return Ok(None);
            }
            let search_start = self.next_start;
            let mut search_input = RegexInput::new(self.input.as_ref()).from_pos(search_start);
            if self.last_skipped_empty {
                // This is the public equivalent of fancy-regex's private
                // OPTION_NOT_CONTINUED_FROM_PREVIOUS_MATCH flag.
                search_input = search_input.continue_from_previous_match_end(false);
            }
            let captures = self
                .program
                .regex
                .captures_input(search_input)
                .map_err(regex_runtime_error);
            let captures = match captures {
                Ok(captures) => {
                    checkpoint()?;
                    captures
                }
                Err(error) => {
                    self.done = true;
                    checkpoint()?;
                    return Err(error);
                }
            };
            let Some(captures) = captures else {
                self.done = true;
                return Ok(None);
            };
            let whole = captures.get(0).expect("regex captures include whole match");
            let start = whole.start();
            let end = whole.end();
            if start == end {
                self.next_start = advance_regex_position(self.input.as_ref(), end);
                self.last_skipped_empty = end == search_start;
                if self.last_match_end == Some(end) {
                    continue;
                }
            } else {
                self.next_start = end;
                self.last_skipped_empty = false;
            }
            self.last_match_end = Some(end);
            let seen = self.seen;
            self.seen = self.seen.saturating_add(1);
            if seen >= self.limits.regex_match_limit {
                self.done = true;
                return Err(resource("regex-match-count"));
            }
            if self.program.ignore_empty && start == end {
                continue;
            }
            if self.stop_after_one {
                self.done = true;
            }
            let pull = match self.kind {
                RegexPullKind::Match => RegexPull::Match {
                    value: match_object(self.input.as_ref(), &self.program.regex, &captures)?,
                },
                RegexPullKind::Capture => RegexPull::Capture {
                    value: named_capture_object(&self.program.regex, &captures),
                },
                RegexPullKind::Scan => RegexPull::Scan {
                    value: scan_value(&captures),
                },
                RegexPullKind::Replacement => RegexPull::Replacement {
                    start,
                    end,
                    context: named_capture_object(&self.program.regex, &captures),
                },
                RegexPullKind::Span => RegexPull::Span { start, end },
            };
            return Ok(Some(pull));
        }
    }
}

/// One pull from a replacement stream.
///
/// A replacement callback is deliberately not represented as a `Vec` here:
/// the managed evaluator can suspend after a match and feed each callback
/// result separately.  `RegexSubstitutionState` retains the match until the
/// caller explicitly closes that replacement stream.
#[derive(Debug)]
pub(crate) enum RegexSubstitutionEvent {
    Replace { context: Value },
    Output(Value),
}

/// Pull-driven state for `sub` and `gsub` replacement callbacks.
///
/// The branch policy is the existing jq-compatible zip policy: one callback
/// stream contributes one replacement to each existing branch, while extra
/// choices create branches seeded with the unmatched prefix.  This is not a
/// Cartesian product.  The state owns all strings and the regex cursor, so a
/// suspended operation has no borrowed captures or self-referential iterator.
pub(crate) struct RegexSubstitutionState {
    cursor: RegexCursor,
    input: Arc<str>,
    limits: VmLimits,
    copied: usize,
    matched: bool,
    unmatched_prefix: String,
    branches: Vec<String>,
    pending_end: Option<usize>,
    pending_replacements: Vec<Arc<str>>,
    finalized: bool,
    next_output: usize,
    done: bool,
    terminal_error: Option<VmError>,
}

/// Compile an owned, pull-driven replacement operation.
pub(crate) fn regex_substitution(
    input: Arc<str>,
    pattern: &str,
    flags: &str,
    force_global: bool,
    limits: VmLimits,
) -> Result<RegexSubstitutionState, VmError> {
    let cursor = regex_cursor(
        Arc::clone(&input),
        pattern,
        flags,
        RegexPullKind::Replacement,
        force_global,
        limits,
    )?;
    Ok(RegexSubstitutionState {
        cursor,
        input,
        limits,
        copied: 0,
        matched: false,
        unmatched_prefix: String::new(),
        branches: Vec::new(),
        pending_end: None,
        pending_replacements: Vec::new(),
        finalized: false,
        next_output: 0,
        done: false,
        terminal_error: None,
    })
}

impl RegexSubstitutionState {
    fn fail<T>(&mut self, error: VmError) -> Result<T, VmError> {
        self.terminal_error = Some(error.clone());
        Err(error)
    }

    fn checked<T>(&mut self, result: Result<T, VmError>) -> Result<T, VmError> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => self.fail(error),
        }
    }

    fn invalid_pending() -> VmError {
        VmError::InvalidProgram {
            message: "regex substitution replacement stream is pending",
        }
    }

    fn invalid_missing_pending() -> VmError {
        VmError::InvalidProgram {
            message: "regex substitution replacement stream is not pending",
        }
    }

    /// Pull the next replacement callback request or one finished output.
    ///
    /// Pulling a `Replace` event copies only the unmatched prefix since the
    /// previous match.  No later regex search runs until the caller feeds the
    /// replacement stream and calls `finish_replacement_stream`.
    pub(crate) fn next(
        &mut self,
        checkpoint: &dyn Fn() -> Result<(), VmError>,
    ) -> Result<Option<RegexSubstitutionEvent>, VmError> {
        if let Some(error) = self.terminal_error.clone() {
            return Err(error);
        }
        if self.pending_end.is_some() {
            return self.fail(Self::invalid_pending());
        }
        if self.done {
            return Ok(None);
        }
        if self.finalized {
            if self.next_output >= self.branches.len() {
                self.done = true;
                return Ok(None);
            }
            self.checked(checkpoint())?;
            let branch = std::mem::take(&mut self.branches[self.next_output]);
            self.next_output = self.next_output.saturating_add(1);
            return Ok(Some(RegexSubstitutionEvent::Output(Value::string(branch))));
        }

        let pull = self.cursor.next(checkpoint);
        let pull = self.checked(pull)?;
        let Some(pull) = pull else {
            if !self.matched || self.branches.is_empty() {
                self.done = true;
                return Ok(Some(RegexSubstitutionEvent::Output(Value::string(
                    Arc::clone(&self.input),
                ))));
            }

            let input = Arc::clone(&self.input);
            let tail = &input[self.copied..];
            for index in 0..self.branches.len() {
                self.checked(checkpoint())?;
                let result =
                    push_bounded(&mut self.branches[index], tail, self.limits.output_bytes);
                self.checked(result)?;
            }
            self.finalized = true;
            return self.next(checkpoint);
        };

        let RegexPull::Replacement {
            start,
            end,
            context,
        } = pull
        else {
            return self.fail(VmError::InvalidProgram {
                message: "regex substitution cursor returned an unexpected pull",
            });
        };

        self.matched = true;
        if self.branches.len() > self.limits.regex_replacement_limit {
            return self.fail(resource("regex-replacement-count"));
        }
        let input = Arc::clone(&self.input);
        let copied = self.copied;
        let between = &input[copied..start];
        for index in 0..self.branches.len() {
            self.checked(checkpoint())?;
            let result = push_bounded(&mut self.branches[index], between, self.limits.output_bytes);
            self.checked(result)?;
        }
        self.checked(checkpoint())?;
        let result = push_bounded(
            &mut self.unmatched_prefix,
            between,
            self.limits.output_bytes,
        );
        self.checked(result)?;
        self.pending_replacements.clear();
        self.pending_end = Some(end);
        Ok(Some(RegexSubstitutionEvent::Replace { context }))
    }

    /// Feed one replacement result for the most recent `Replace` event.
    pub(crate) fn push_replacement(
        &mut self,
        text: Arc<str>,
        checkpoint: &dyn Fn() -> Result<(), VmError>,
    ) -> Result<(), VmError> {
        if let Some(error) = self.terminal_error.clone() {
            return Err(error);
        }
        if self.pending_end.is_none() {
            return self.fail(Self::invalid_missing_pending());
        }
        if self.pending_replacements.len() >= self.limits.regex_replacement_limit {
            return self.fail(resource("regex-replacement-count"));
        }
        self.checked(checkpoint())?;
        if text.len() > self.limits.output_bytes {
            return self.fail(resource("output-bytes"));
        }
        let reserve = self
            .pending_replacements
            .try_reserve(1)
            .map_err(|_| resource("regex-replacement-count"));
        self.checked(reserve)?;
        self.pending_replacements.push(text);
        Ok(())
    }

    /// Close the current replacement callback stream and advance to the next
    /// regex match.  Calling this with no pending `Replace` event is a
    /// protocol error and permanently terminates the state.
    pub(crate) fn finish_replacement_stream(
        &mut self,
        checkpoint: &dyn Fn() -> Result<(), VmError>,
    ) -> Result<(), VmError> {
        if let Some(error) = self.terminal_error.clone() {
            return Err(error);
        }
        let Some(end) = self.pending_end.take() else {
            return self.fail(Self::invalid_missing_pending());
        };
        if self.branches.len() > self.limits.regex_replacement_limit {
            return self.fail(resource("regex-replacement-count"));
        }

        let replacements = std::mem::take(&mut self.pending_replacements);
        if replacements.len() > self.limits.regex_replacement_limit {
            return self.fail(resource("regex-replacement-count"));
        }
        if replacements.len() > self.branches.len() {
            let additional = replacements.len() - self.branches.len();
            let reserve = self
                .branches
                .try_reserve(additional)
                .map_err(|_| resource("regex-replacement-count"));
            self.checked(reserve)?;
            for _ in replacements.iter().skip(self.branches.len()) {
                self.checked(checkpoint())?;
                let mut branch = String::new();
                self.checked(push_bounded(
                    &mut branch,
                    &self.unmatched_prefix,
                    self.limits.output_bytes,
                ))?;
                self.branches.push(branch);
            }
        }
        for (index, value) in replacements.iter().enumerate() {
            if index >= self.branches.len() {
                break;
            }
            self.checked(checkpoint())?;
            let result = push_bounded(&mut self.branches[index], value, self.limits.output_bytes);
            self.checked(result)?;
        }
        self.copied = end;
        Ok(())
    }
}

fn advance_regex_position(input: &str, offset: usize) -> usize {
    input
        .get(offset..)
        .and_then(|remaining| remaining.chars().next())
        .map_or_else(
            || offset.saturating_add(1),
            |character| offset + character.len_utf8(),
        )
}

pub(crate) fn regex_test(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
    checkpoint: &dyn Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let program = compile_regex(input, pattern, flags, limits)?;
    checkpoint()?;
    Ok(Value::Bool(
        program.regex.is_match(input).map_err(regex_runtime_error)?,
    ))
}

pub(crate) fn regex_matches(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
    checkpoint: &dyn Fn() -> Result<(), VmError>,
) -> Result<Vec<Value>, VmError> {
    let mut cursor = regex_cursor(
        Arc::from(input),
        pattern,
        flags,
        RegexPullKind::Match,
        false,
        limits,
    )?;
    let mut values = Vec::new();
    while let Some(RegexPull::Match { value, .. }) = cursor.next(checkpoint)? {
        values
            .try_reserve(1)
            .map_err(|_| resource("regex-match-count"))?;
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn regex_capture(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
    checkpoint: &dyn Fn() -> Result<(), VmError>,
) -> Result<Vec<Value>, VmError> {
    let mut cursor = regex_cursor(
        Arc::from(input),
        pattern,
        flags,
        RegexPullKind::Capture,
        false,
        limits,
    )?;
    let mut values = Vec::new();
    while let Some(RegexPull::Capture { value, .. }) = cursor.next(checkpoint)? {
        values
            .try_reserve(1)
            .map_err(|_| resource("regex-match-count"))?;
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn regex_scan(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
    checkpoint: &dyn Fn() -> Result<(), VmError>,
) -> Result<Vec<Value>, VmError> {
    let mut cursor = regex_cursor(
        Arc::from(input),
        pattern,
        flags,
        RegexPullKind::Scan,
        true,
        limits,
    )?;
    let mut values = Vec::new();
    while let Some(RegexPull::Scan { value, .. }) = cursor.next(checkpoint)? {
        values
            .try_reserve(1)
            .map_err(|_| resource("regex-match-count"))?;
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn regex_split(
    input: &str,
    pattern: &str,
    flags: &str,
    stream: bool,
    literal: bool,
    limits: VmLimits,
    checkpoint: &dyn Fn() -> Result<(), VmError>,
) -> Result<Vec<Value>, VmError> {
    if literal {
        if input.len() > limits.regex_input_bytes {
            return Err(resource("regex-input-bytes"));
        }
        if pattern.len() > limits.regex_pattern_bytes {
            return Err(resource("regex-pattern-bytes"));
        }
        let pieces = if pattern.is_empty() {
            let mut pieces = Vec::new();
            for character in input.chars() {
                checkpoint()?;
                let piece = character.to_string();
                push_regex_value(
                    &mut pieces,
                    regex_split_piece(&piece, limits, checkpoint)?,
                    limits,
                )?;
            }
            pieces
        } else {
            let mut pieces = Vec::new();
            for piece in input.split(pattern) {
                checkpoint()?;
                push_regex_value(
                    &mut pieces,
                    regex_split_piece(piece, limits, checkpoint)?,
                    limits,
                )?;
            }
            pieces
        };
        return if stream {
            Ok(pieces)
        } else {
            Ok(vec![Value::array(pieces)])
        };
    }
    let input = Arc::from(input);
    let mut cursor = regex_cursor(
        Arc::clone(&input),
        pattern,
        flags,
        RegexPullKind::Span,
        true,
        limits,
    )?;
    let mut pieces = Vec::new();
    let mut copied = 0;
    while let Some(RegexPull::Span { start, end }) = cursor.next(checkpoint)? {
        push_regex_value(
            &mut pieces,
            regex_split_piece(&input[copied..start], limits, checkpoint)?,
            limits,
        )?;
        copied = end;
    }
    push_regex_value(
        &mut pieces,
        regex_split_piece(&input[copied..], limits, checkpoint)?,
        limits,
    )?;
    if stream {
        Ok(pieces)
    } else {
        Ok(vec![Value::array(pieces)])
    }
}

pub(crate) fn regex_substitute(
    input: &str,
    pattern: &str,
    flags: &str,
    force_global: bool,
    limits: VmLimits,
    checkpoint: &dyn Fn() -> Result<(), VmError>,
    mut replacement: impl FnMut(&Value) -> Result<Vec<Arc<str>>, VmError>,
) -> Result<Vec<Value>, VmError> {
    let mut values = Vec::new();
    let mut state = regex_substitution(Arc::from(input), pattern, flags, force_global, limits)?;
    loop {
        match state.next(checkpoint)? {
            None => break,
            Some(RegexSubstitutionEvent::Replace { context }) => {
                let replacements = replacement(&context)?;
                for value in replacements {
                    state.push_replacement(value, checkpoint)?;
                }
                state.finish_replacement_stream(checkpoint)?;
            }
            Some(RegexSubstitutionEvent::Output(value)) => {
                values
                    .try_reserve(1)
                    .map_err(|_| resource("regex-replacement-count"))?;
                values.push(value);
            }
        }
    }
    Ok(values)
}

pub(crate) fn fromdate_iso8601(input: &Value) -> Result<Value, VmError> {
    let Value::String(text) = input else {
        return Err(type_error("fromdateiso8601", input));
    };
    let datetime = DateTime::strptime("%Y-%m-%dT%H:%M:%SZ", text.as_bytes())
        .map_err(|_| runtime("date/time value does not match %Y-%m-%dT%H:%M:%SZ"))?;
    let timestamp = Offset::UTC
        .to_timestamp(datetime)
        .map_err(|_| numeric_range("date/time value is outside the supported UTC range"))?;
    timestamp_number(timestamp)
}

pub(crate) fn todate_iso8601(input: &Value, output_limit: usize) -> Result<Value, VmError> {
    let timestamp = value_timestamp(input, "todateiso8601")?;
    formatted_timestamp(timestamp, "%Y-%m-%dT%H:%M:%SZ", output_limit)
}

pub(crate) fn gmtime(input: &Value) -> Result<Value, VmError> {
    let timestamp = value_timestamp(input, "gmtime")?;
    Ok(datetime_array(Offset::UTC.to_datetime(timestamp)))
}

pub(crate) fn localtime(input: &Value, allowed: bool) -> Result<Value, VmError> {
    require_platform(allowed, "localtime")?;
    let timestamp = value_timestamp(input, "localtime")?;
    let timezone = TimeZone::try_system()
        .map_err(|_| runtime("system time zone is unavailable on this platform"))?;
    Ok(datetime_array(timestamp.to_zoned(timezone).datetime()))
}

pub(crate) fn mktime(input: &Value) -> Result<Value, VmError> {
    let datetime = array_datetime(input, "mktime")?;
    let timestamp = Offset::UTC
        .to_timestamp(datetime)
        .map_err(|_| numeric_range("date/time value is outside the supported UTC range"))?;
    timestamp_number(timestamp)
}

pub(crate) fn strptime(input: &Value, format: &str) -> Result<Value, VmError> {
    let Value::String(text) = input else {
        return Err(type_error("strptime", input));
    };
    let datetime = DateTime::strptime(format.as_bytes(), text.as_bytes())
        .map_err(|_| runtime("date/time value does not match the requested format"))?;
    Ok(datetime_array(datetime))
}

pub(crate) fn strftime(input: &Value, format: &str, output_limit: usize) -> Result<Value, VmError> {
    let datetime = array_datetime(input, "strftime")?;
    formatted_datetime(datetime, format, output_limit)
}

pub(crate) fn strflocaltime(
    input: &Value,
    format: &str,
    allowed: bool,
    output_limit: usize,
) -> Result<Value, VmError> {
    require_platform(allowed, "strflocaltime")?;
    let formatted = match input {
        Value::Number(_) => {
            let timestamp = value_timestamp(input, "strflocaltime")?;
            let timezone = TimeZone::try_system()
                .map_err(|_| runtime("system time zone is unavailable on this platform"))?;
            strtime::format(format, &timestamp.to_zoned(timezone))
        }
        Value::Array(_) => {
            let datetime = array_datetime(input, "strflocaltime")?;
            strtime::format(format, datetime)
        }
        _ => return Err(type_error("strflocaltime", input)),
    }
    .map_err(|_| runtime("invalid strftime format"))?;
    bounded_string(formatted, output_limit)
}

pub(crate) fn now(allowed: bool) -> Result<Value, VmError> {
    require_platform(allowed, "now")?;
    let timestamp = Timestamp::try_from(std::time::SystemTime::now())
        .map_err(|_| numeric_range("system clock is outside the supported date/time range"))?;
    timestamp_number(timestamp)
}

fn compile_regex(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
) -> Result<RegexProgram, VmError> {
    if input.len() > limits.regex_input_bytes {
        return Err(resource("regex-input-bytes"));
    }
    if pattern.len() > limits.regex_pattern_bytes {
        return Err(resource("regex-pattern-bytes"));
    }
    let mut builder = RegexBuilder::new(pattern);
    let mut global = false;
    let mut ignore_empty = false;
    for flag in flags.chars() {
        match flag {
            'g' => global = true,
            'i' => {
                builder.case_insensitive(true);
            }
            // jq's `m` and `p` make dot match line terminators. jq keeps
            // ^/$ anchored to the complete input for these flags.
            'm' | 'p' => {
                builder.dot_matches_new_line(true);
            }
            // jq's `s` selects single-line anchors. The Rust engine's default
            // anchor behavior already matches this mode.
            's' => {}
            'x' => {
                builder.ignore_whitespace(true);
            }
            'n' => ignore_empty = true,
            'l' => {
                return Err(unsupported("regex flag 'l' (longest-match mode)"));
            }
            other => return Err(runtime(format!("regex flag '{other}' is invalid"))),
        }
    }
    builder
        .backtrack_limit(limits.regex_backtrack_limit)
        .delegate_size_limit(limits.regex_compiled_bytes)
        .delegate_dfa_size_limit(limits.regex_compiled_bytes.saturating_mul(4));
    let regex = builder
        .build()
        .map_err(|error| regex_compile_error(&error))?;
    Ok(RegexProgram {
        regex,
        global,
        ignore_empty,
    })
}

fn regex_compile_error(error: &RegexError) -> VmError {
    let message = error.to_string();
    let debug = format!("{error:?}");
    if message.contains("size") || message.contains("DFA") || debug.contains("ExceededSizeLimit") {
        resource("regex-compiled-bytes")
    } else {
        runtime(format!("regex pattern rejected: {message}"))
    }
}

fn regex_runtime_error(error: RegexError) -> VmError {
    match error {
        RegexError::RuntimeError(_) => resource("regex-backtrack"),
        other => regex_compile_error(&other),
    }
}

fn match_object(
    input: &str,
    regex: &Regex,
    captures: &Captures<'_, str>,
) -> Result<Value, VmError> {
    let whole = captures.get(0).expect("regex captures include whole match");
    let mut object = IndexMap::new();
    object.insert(
        Arc::from("offset"),
        number_usize(char_offset(input, whole.start()))?,
    );
    object.insert(
        Arc::from("length"),
        number_usize(whole.as_str().chars().count())?,
    );
    object.insert(Arc::from("string"), Value::string(whole.as_str()));
    let mut capture_values = Vec::new();
    for (index, name) in regex.capture_names().enumerate().skip(1) {
        let capture = captures.get(index);
        let mut value = IndexMap::new();
        value.insert(
            Arc::from("offset"),
            capture.map_or_else(
                || number_i64(-1),
                |matched| number_usize(char_offset(input, matched.start())),
            )?,
        );
        if let Some(matched) = capture {
            value.insert(
                Arc::from("length"),
                number_usize(matched.as_str().chars().count())?,
            );
            value.insert(Arc::from("string"), Value::string(matched.as_str()));
        } else {
            // jq's compact object contract places the null capture's string
            // before its zero length; retain that observable field order.
            value.insert(Arc::from("string"), Value::Null);
            value.insert(Arc::from("length"), number_i64(0)?);
        }
        value.insert(Arc::from("name"), name.map_or(Value::Null, Value::string));
        capture_values.push(Value::object(value));
    }
    object.insert(Arc::from("captures"), Value::array(capture_values));
    Ok(Value::object(object))
}

fn named_capture_object(regex: &Regex, captures: &Captures<'_, str>) -> Value {
    let mut object = IndexMap::new();
    for (index, name) in regex.capture_names().enumerate().skip(1) {
        if let Some(name) = name {
            object.insert(Arc::from(name), capture_string(captures.get(index)));
        }
    }
    Value::object(object)
}

fn capture_string(capture: Option<Match<'_>>) -> Value {
    capture.map_or(Value::Null, |matched| Value::string(matched.as_str()))
}

fn scan_value(captures: &Captures<'_, str>) -> Value {
    let whole = captures.get(0).expect("regex captures include whole match");
    if captures.len() == 1 {
        Value::string(whole.as_str())
    } else {
        Value::array(
            (1..captures.len())
                .map(|index| capture_string(captures.get(index)))
                .collect::<Vec<_>>(),
        )
    }
}

fn char_offset(input: &str, byte_offset: usize) -> usize {
    input[..byte_offset].chars().count()
}

fn datetime_array(datetime: DateTime) -> Value {
    let second =
        f64::from(datetime.second()) + f64::from(datetime.subsec_nanosecond()) / 1_000_000_000.0;
    Value::array(vec![
        number_i64(i64::from(datetime.year())).expect("year is numeric"),
        number_i64(i64::from(datetime.month()) - 1).expect("month is numeric"),
        number_i64(i64::from(datetime.day())).expect("day is numeric"),
        number_i64(i64::from(datetime.hour())).expect("hour is numeric"),
        number_i64(i64::from(datetime.minute())).expect("minute is numeric"),
        number_f64(second).expect("second is finite"),
        number_i64(i64::from(datetime.weekday().to_sunday_zero_offset()))
            .expect("weekday is numeric"),
        number_i64(i64::from(datetime.day_of_year()) - 1).expect("year day is numeric"),
    ])
}

fn array_datetime(input: &Value, operation: &str) -> Result<DateTime, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error(operation, input));
    };
    if values.len() < 6 {
        return Err(runtime(format!(
            "{operation} requires a date/time array with six fields"
        )));
    }
    let year = integer_component(&values[0], operation, "year")?;
    let month = integer_component(&values[1], operation, "month")?.saturating_add(1);
    let day = integer_component(&values[2], operation, "day")?;
    let hour = integer_component(&values[3], operation, "hour")?;
    let minute = integer_component(&values[4], operation, "minute")?;
    let Value::Number(second_number) = &values[5] else {
        return Err(type_error(operation, &values[5]));
    };
    let second = second_number.as_f64();
    if !second.is_finite() {
        return Err(numeric_range(format!(
            "{operation} second is outside the supported range"
        )));
    }
    let whole = second.trunc();
    let nanos = ((second - whole) * 1_000_000_000.0).round();
    let year = i16::try_from(year).map_err(|_| numeric_range("date/time year is out of range"))?;
    let month =
        i8::try_from(month).map_err(|_| numeric_range("date/time month is out of range"))?;
    let day = i8::try_from(day).map_err(|_| numeric_range("date/time day is out of range"))?;
    let hour = i8::try_from(hour).map_err(|_| numeric_range("date/time hour is out of range"))?;
    let minute =
        i8::try_from(minute).map_err(|_| numeric_range("date/time minute is out of range"))?;
    let second = whole
        .to_i8()
        .ok_or_else(|| numeric_range("date/time second is out of range"))?;
    let nanos = nanos
        .to_i32()
        .ok_or_else(|| numeric_range("date/time fractional second is out of range"))?;
    DateTime::new(year, month, day, hour, minute, second, nanos)
        .map_err(|_| numeric_range("date/time array contains an invalid UTC value"))
}

fn integer_component(value: &Value, operation: &str, field: &str) -> Result<i64, VmError> {
    let Value::Number(number) = value else {
        return Err(type_error(operation, value));
    };
    let value = number.as_f64();
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(numeric_range(format!(
            "{operation} {field} is outside the integer range"
        )));
    }
    value
        .to_i64()
        .ok_or_else(|| numeric_range(format!("{operation} {field} is outside the integer range")))
}

fn value_timestamp(input: &Value, operation: &str) -> Result<Timestamp, VmError> {
    let Value::Number(number) = input else {
        return Err(type_error(operation, input));
    };
    let value = number.as_f64();
    if !value.is_finite() {
        return Err(numeric_range(format!(
            "{operation} timestamp is outside the supported range"
        )));
    }
    let whole = value.trunc();
    let nanos = ((value - whole) * 1_000_000_000.0).round();
    let whole = whole.to_i64().ok_or_else(|| {
        numeric_range(format!(
            "{operation} timestamp is outside the supported range"
        ))
    })?;
    let nanos = nanos.to_i32().ok_or_else(|| {
        numeric_range(format!(
            "{operation} timestamp is outside the supported range"
        ))
    })?;
    Timestamp::new(whole, nanos).map_err(|_| {
        numeric_range(format!(
            "{operation} timestamp is outside the supported range"
        ))
    })
}

fn timestamp_number(timestamp: Timestamp) -> Result<Value, VmError> {
    let value = timestamp
        .as_second()
        .to_f64()
        .ok_or_else(|| runtime("timestamp cannot be represented as a jq number"))?
        + f64::from(timestamp.subsec_nanosecond()) / 1_000_000_000.0;
    number_f64(value)
}

fn formatted_timestamp(
    timestamp: Timestamp,
    format: &str,
    output_limit: usize,
) -> Result<Value, VmError> {
    let formatted =
        strtime::format(format, timestamp).map_err(|_| runtime("invalid strftime format"))?;
    bounded_string(formatted, output_limit)
}

fn formatted_datetime(
    datetime: DateTime,
    format: &str,
    output_limit: usize,
) -> Result<Value, VmError> {
    let formatted =
        strtime::format(format, datetime).map_err(|_| runtime("invalid strftime format"))?;
    bounded_string(formatted, output_limit)
}

fn bounded_string(value: String, limit: usize) -> Result<Value, VmError> {
    if value.len() > limit {
        return Err(resource("output-bytes"));
    }
    Ok(Value::string(value))
}

fn push_bounded(output: &mut String, value: &str, limit: usize) -> Result<(), VmError> {
    let length = output
        .len()
        .checked_add(value.len())
        .ok_or_else(|| resource("output-bytes"))?;
    if length > limit {
        return Err(resource("output-bytes"));
    }
    output
        .try_reserve(value.len())
        .map_err(|_| resource("output-bytes"))?;
    output.push_str(value);
    Ok(())
}

fn push_regex_value(
    values: &mut Vec<Value>,
    value: Value,
    limits: VmLimits,
) -> Result<(), VmError> {
    if values.len() >= limits.regex_match_limit {
        return Err(resource("regex-match-count"));
    }
    values
        .try_reserve(1)
        .map_err(|_| resource("regex-match-count"))?;
    values.push(value);
    Ok(())
}

/// Materializes one split piece only after its output quota is known to fit.
/// The checkpoint precharges each bounded copy chunk before the allocation, so
/// managed callers can reject cancellation without retaining the piece first.
pub(crate) fn regex_split_piece(
    piece: &str,
    limits: VmLimits,
    checkpoint: &dyn Fn() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    if piece.len() > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    for _chunk in piece.as_bytes().chunks(4096) {
        checkpoint()?;
    }
    let mut owned = String::new();
    owned
        .try_reserve(piece.len())
        .map_err(|_| resource("output-bytes"))?;
    owned.push_str(piece);
    Ok(Value::string(owned))
}

fn number_usize(value: usize) -> Result<Value, VmError> {
    Number::parse(&value.to_string())
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn number_i64(value: i64) -> Result<Value, VmError> {
    Number::parse(&value.to_string())
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn number_f64(value: f64) -> Result<Value, VmError> {
    Number::parse(&value.to_string())
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn require_platform(allowed: bool, operation: &str) -> Result<(), VmError> {
    if allowed {
        Ok(())
    } else {
        Err(VmError::CapabilityDenied {
            message: format!("{operation} requires platform access permitted by capability policy")
                .into(),
        })
    }
}

fn type_error(operation: &str, value: &Value) -> VmError {
    let kind = match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    runtime(format!("{operation} cannot be applied to {kind}"))
}

fn runtime(message: impl Into<Arc<str>>) -> VmError {
    VmError::Runtime {
        message: message.into(),
    }
}

fn resource(resource: &'static str) -> VmError {
    VmError::Resource { resource }
}

fn numeric_range(message: impl Into<Arc<str>>) -> VmError {
    VmError::NumericRange {
        message: message.into(),
    }
}

fn unsupported(operation: impl Into<Arc<str>>) -> VmError {
    VmError::Unsupported {
        operation: operation.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::{
        RegexPull, RegexPullKind, RegexSubstitutionEvent, regex_cursor, regex_matches,
        regex_split_piece, regex_substitute, regex_substitution,
    };
    use crate::{VmError, VmLimits};

    #[test]
    fn regex_collectors_checkpoint_between_engine_results() {
        let calls = AtomicUsize::new(0);
        let checkpoint = || {
            if calls.fetch_add(1, Ordering::Relaxed) > 0 {
                Err(VmError::Interrupted)
            } else {
                Ok(())
            }
        };
        let error = regex_matches("pp", "p", "g", VmLimits::default(), &checkpoint)
            .expect_err("the second engine result must observe cancellation");
        assert_eq!(error, VmError::Interrupted);
    }

    #[test]
    fn regex_cursor_returns_first_pull_before_later_cancellation() {
        let calls = AtomicUsize::new(0);
        let checkpoint = || {
            if calls.fetch_add(1, Ordering::Relaxed) >= 2 {
                Err(VmError::Interrupted)
            } else {
                Ok(())
            }
        };
        let mut cursor = regex_cursor(
            Arc::from("pp"),
            "p",
            "g",
            RegexPullKind::Scan,
            true,
            VmLimits::default(),
        )
        .expect("regex cursor compiles");
        let first = cursor
            .next(&checkpoint)
            .expect("first pull is not cancelled")
            .expect("first match exists");
        assert!(matches!(
            first,
            RegexPull::Scan {
                value: crate::Value::String(_),
            }
        ));
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert!(matches!(
            cursor.next(&checkpoint),
            Err(VmError::Interrupted)
        ));
    }

    #[test]
    fn regex_cursor_does_not_count_suppressed_terminal_empty_match() {
        let checkpoint = || Ok(());
        let limits = VmLimits {
            regex_match_limit: 1,
            ..VmLimits::default()
        };
        let mut cursor = regex_cursor(Arc::from("x"), "x*", "g", RegexPullKind::Scan, true, limits)
            .expect("regex cursor compiles");
        assert!(matches!(
            cursor.next(&checkpoint),
            Ok(Some(RegexPull::Scan { .. }))
        ));
        assert!(cursor.next(&checkpoint).is_ok_and(|pull| pull.is_none()));
    }

    #[test]
    fn regex_substitute_consumes_replacements_pull_by_pull() {
        let checkpoint = || Ok(());
        let values = regex_substitute(
            "aba",
            "a",
            "g",
            true,
            VmLimits::default(),
            &checkpoint,
            |_context| Ok(vec![Arc::from("X")]),
        )
        .expect("replacement succeeds");
        assert_eq!(values, vec![crate::Value::string("XbX")]);
    }

    #[test]
    fn regex_substitute_preserves_empty_single_and_multiple_callback_streams() {
        let checkpoint = || Ok(());
        let empty = regex_substitute(
            "aba",
            "a",
            "g",
            true,
            VmLimits::default(),
            &checkpoint,
            |_context| Ok(Vec::new()),
        )
        .expect("empty callback stream succeeds");
        assert_eq!(empty, vec![crate::Value::string("aba")]);

        let multiple = regex_substitute(
            "aba",
            "a",
            "g",
            true,
            VmLimits::default(),
            &checkpoint,
            |_context| Ok(vec![Arc::from("X"), Arc::from("Y")]),
        )
        .expect("multiple callback choices succeed");
        assert_eq!(
            multiple,
            vec![crate::Value::string("XbX"), crate::Value::string("YbY")]
        );
    }

    #[test]
    fn regex_substitute_keeps_zip_branching_for_uneven_callbacks() {
        let checkpoint = || Ok(());
        let mut calls = 0;
        let values = regex_substitute(
            "ab",
            ".",
            "g",
            true,
            VmLimits::default(),
            &checkpoint,
            |_context| {
                calls += 1;
                if calls == 1 {
                    Ok(vec![Arc::from("X"), Arc::from("Y")])
                } else {
                    Ok(vec![Arc::from("Z")])
                }
            },
        )
        .expect("uneven callback streams succeed");
        assert_eq!(
            values,
            vec![crate::Value::string("XZ"), crate::Value::string("Y")]
        );
    }

    #[test]
    fn regex_substitute_handles_unicode_and_empty_matches() {
        let checkpoint = || Ok(());
        let values = regex_substitute(
            "é",
            "",
            "g",
            true,
            VmLimits::default(),
            &checkpoint,
            |_context| Ok(vec![Arc::from("X")]),
        )
        .expect("unicode empty matches succeed");
        assert_eq!(values, vec![crate::Value::string("XéX")]);

        let ignored = regex_substitute(
            "é",
            "",
            "gn",
            true,
            VmLimits::default(),
            &checkpoint,
            |_context| Ok(vec![Arc::from("X")]),
        )
        .expect("ignored empty matches succeed");
        assert_eq!(ignored, vec![crate::Value::string("é")]);
    }

    #[test]
    fn regex_substitute_enforces_match_replacement_and_output_limits() {
        let checkpoint = || Ok(());
        let match_limits = VmLimits {
            regex_match_limit: 1,
            ..VmLimits::default()
        };
        let match_error = regex_substitute(
            "aa",
            "a",
            "g",
            true,
            match_limits,
            &checkpoint,
            |_context| Ok(vec![Arc::from("X")]),
        )
        .expect_err("the second match exceeds the match limit");
        assert_eq!(
            match_error,
            VmError::Resource {
                resource: "regex-match-count"
            }
        );

        let replacement_limits = VmLimits {
            regex_replacement_limit: 1,
            ..VmLimits::default()
        };
        let replacement_error = regex_substitute(
            "a",
            "a",
            "g",
            true,
            replacement_limits,
            &checkpoint,
            |_context| Ok(vec![Arc::from("X"), Arc::from("Y")]),
        )
        .expect_err("the second replacement exceeds the branch limit");
        assert_eq!(
            replacement_error,
            VmError::Resource {
                resource: "regex-replacement-count"
            }
        );

        let output_limits = VmLimits {
            output_bytes: 1,
            ..VmLimits::default()
        };
        let output_error = regex_substitute(
            "a",
            "a",
            "g",
            true,
            output_limits,
            &checkpoint,
            |_context| Ok(vec![Arc::from("XX")]),
        )
        .expect_err("the replacement exceeds the output limit");
        assert_eq!(
            output_error,
            VmError::Resource {
                resource: "output-bytes"
            }
        );
    }

    #[test]
    fn regex_substitute_bounds_pending_replacements_and_unmatched_text() {
        let checkpoint = || Ok(());

        let limits = VmLimits {
            output_bytes: 2,
            ..VmLimits::default()
        };
        let oversized = regex_substitute("a", "a", "g", true, limits, &checkpoint, |_context| {
            Ok(vec![Arc::from("xxx")])
        })
        .expect_err("oversized pending replacement is rejected before storage");
        assert_eq!(
            oversized,
            VmError::Resource {
                resource: "output-bytes"
            }
        );

        let limits = VmLimits {
            output_bytes: 3,
            ..VmLimits::default()
        };
        let unmatched =
            regex_substitute("xxxxa", "a", "g", true, limits, &checkpoint, |_context| {
                Ok(vec![Arc::from("x")])
            })
            .expect_err("a long unmatched prefix is bounded before callback output");
        assert_eq!(
            unmatched,
            VmError::Resource {
                resource: "output-bytes"
            }
        );

        let tail = regex_substitute("axxxx", "a", "g", true, limits, &checkpoint, |_context| {
            Ok(vec![Arc::from("x")])
        })
        .expect_err("a long unmatched tail is bounded before output");
        assert_eq!(
            tail,
            VmError::Resource {
                resource: "output-bytes"
            }
        );
    }

    #[test]
    fn regex_substitution_protocol_is_pull_driven_and_terminal() {
        let checkpoint = || Ok(());
        let mut state = regex_substitution(Arc::from("aa"), "a", "g", true, VmLimits::default())
            .expect("replacement cursor compiles");
        let first = state.next(&checkpoint).expect("first pull succeeds");
        assert!(matches!(
            first,
            Some(RegexSubstitutionEvent::Replace { .. })
        ));
        let pending_error = state
            .next(&checkpoint)
            .expect_err("a second pull cannot rerun a pending callback");
        assert_eq!(
            pending_error,
            VmError::InvalidProgram {
                message: "regex substitution replacement stream is pending"
            }
        );
        assert!(matches!(
            state.next(&checkpoint),
            Err(VmError::InvalidProgram {
                message: "regex substitution replacement stream is pending"
            })
        ));

        let mut state = regex_substitution(Arc::from("a"), "a", "g", true, VmLimits::default())
            .expect("replacement cursor compiles");
        assert_eq!(
            state.push_replacement(Arc::from("X"), &checkpoint),
            Err(VmError::InvalidProgram {
                message: "regex substitution replacement stream is not pending"
            })
        );
    }

    #[test]
    fn regex_substitution_checks_cancellation_between_pulls() {
        use std::sync::atomic::AtomicBool;

        let cancelled = AtomicBool::new(false);
        let checkpoint = || {
            if cancelled.load(Ordering::Relaxed) {
                Err(VmError::Interrupted)
            } else {
                Ok(())
            }
        };
        let mut state = regex_substitution(Arc::from("aa"), "a", "g", true, VmLimits::default())
            .expect("replacement cursor compiles");
        assert!(matches!(
            state.next(&checkpoint),
            Ok(Some(RegexSubstitutionEvent::Replace { .. }))
        ));
        state
            .push_replacement(Arc::from("X"), &checkpoint)
            .expect("first replacement is accepted");
        state
            .finish_replacement_stream(&checkpoint)
            .expect("first replacement stream closes");
        cancelled.store(true, Ordering::Relaxed);
        assert!(matches!(state.next(&checkpoint), Err(VmError::Interrupted)));
    }

    #[test]
    fn regex_split_piece_rejects_before_reserving_oversized_text() {
        let calls = AtomicUsize::new(0);
        let checkpoint = || {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        };
        let limits = VmLimits {
            output_bytes: 4,
            ..VmLimits::default()
        };
        let error = regex_split_piece("xxxxx", limits, &checkpoint)
            .expect_err("oversized split pieces must be rejected before allocation");
        assert_eq!(
            error,
            VmError::Resource {
                resource: "output-bytes"
            }
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn regex_split_piece_charges_bounded_copy_chunks() {
        let calls = AtomicUsize::new(0);
        let checkpoint = || {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        };
        let limits = VmLimits {
            output_bytes: 9_000,
            ..VmLimits::default()
        };
        let piece = "x".repeat(8_193);
        assert_eq!(
            regex_split_piece(&piece, limits, &checkpoint).expect("piece fits"),
            crate::Value::string(piece)
        );
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn regex_split_piece_propagates_exhausted_charge_before_reserve() {
        let checkpoint = || Err(VmError::Interrupted);
        let limits = VmLimits {
            output_bytes: 8_193,
            ..VmLimits::default()
        };
        assert_eq!(
            regex_split_piece(&"x".repeat(8_193), limits, &checkpoint),
            Err(VmError::Interrupted)
        );
    }
}
