//! Bounded regex and UTC-first date/time helpers for jq-compatible built-ins.

use std::sync::Arc;

use indexmap::IndexMap;
use jiff::{
    Timestamp,
    civil::DateTime,
    fmt::strtime,
    tz::{Offset, TimeZone},
};
use num_traits::ToPrimitive as _;
use regex::{Captures, Regex, RegexBuilder};

use crate::{Number, Value, VmError, VmLimits};

pub(crate) struct RegexProgram {
    regex: Regex,
    global: bool,
    ignore_empty: bool,
}

pub(crate) fn regex_test(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
) -> Result<Value, VmError> {
    let program = compile_regex(input, pattern, flags, limits)?;
    Ok(Value::Bool(program.regex.is_match(input)))
}

pub(crate) fn regex_matches(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
) -> Result<Vec<Value>, VmError> {
    let program = compile_regex(input, pattern, flags, limits)?;
    let mut values = Vec::new();
    for captures in program.regex.captures_iter(input) {
        let whole = captures.get(0).expect("regex captures include whole match");
        if program.ignore_empty && whole.is_empty() {
            continue;
        }
        values.push(match_object(input, &program.regex, &captures)?);
        if !program.global {
            break;
        }
    }
    Ok(values)
}

pub(crate) fn regex_capture(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
) -> Result<Vec<Value>, VmError> {
    let program = compile_regex(input, pattern, flags, limits)?;
    let mut values = Vec::new();
    for captures in program.regex.captures_iter(input) {
        let whole = captures.get(0).expect("regex captures include whole match");
        if program.ignore_empty && whole.is_empty() {
            continue;
        }
        values.push(named_capture_object(&program.regex, &captures));
        if !program.global {
            break;
        }
    }
    Ok(values)
}

pub(crate) fn regex_scan(
    input: &str,
    pattern: &str,
    flags: &str,
    limits: VmLimits,
) -> Result<Vec<Value>, VmError> {
    let program = compile_regex(input, pattern, flags, limits)?;
    let mut values = Vec::new();
    for captures in program.regex.captures_iter(input) {
        let whole = captures.get(0).expect("regex captures include whole match");
        if program.ignore_empty && whole.is_empty() {
            continue;
        }
        if captures.len() == 1 {
            values.push(Value::string(whole.as_str()));
        } else if captures.len() == 2 {
            values.push(capture_string(captures.get(1)));
        } else {
            values.push(Value::array(
                (1..captures.len())
                    .map(|index| capture_string(captures.get(index)))
                    .collect::<Vec<_>>(),
            ));
        }
    }
    Ok(values)
}

pub(crate) fn regex_split(
    input: &str,
    pattern: &str,
    flags: &str,
    stream: bool,
    limits: VmLimits,
) -> Result<Vec<Value>, VmError> {
    let program = compile_regex(input, pattern, flags, limits)?;
    let pieces = program
        .regex
        .split(input)
        .map(Value::string)
        .collect::<Vec<_>>();
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
    mut replacement: impl FnMut(&Value) -> Result<Arc<str>, VmError>,
) -> Result<Value, VmError> {
    let program = compile_regex(input, pattern, flags, limits)?;
    let mut output = String::with_capacity(input.len().min(limits.output_bytes));
    let mut copied = 0;
    for captures in program.regex.captures_iter(input) {
        let whole = captures.get(0).expect("regex captures include whole match");
        if program.ignore_empty && whole.is_empty() {
            continue;
        }
        push_bounded(
            &mut output,
            &input[copied..whole.start()],
            limits.output_bytes,
        )?;
        let context = named_capture_object(&program.regex, &captures);
        push_bounded(&mut output, &replacement(&context)?, limits.output_bytes)?;
        copied = whole.end();
        if !force_global && !program.global {
            break;
        }
    }
    push_bounded(&mut output, &input[copied..], limits.output_bytes)?;
    Ok(Value::string(output))
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
            // jq/Oniguruma's `m` makes dot match line terminators.
            'm' | 'p' => {
                builder.dot_matches_new_line(true);
            }
            // jq's `s` selects single-line anchors. Rust regex anchors are
            // single-line by default, so this is intentionally a no-op.
            's' => {}
            'x' => {
                builder.ignore_whitespace(true);
            }
            'n' => ignore_empty = true,
            'l' => {
                return Err(unsupported("regex flag 'l' (longest-match mode)"));
            }
            other => return Err(unsupported(format!("regex flag '{other}'"))),
        }
    }
    builder
        .size_limit(limits.regex_compiled_bytes)
        .dfa_size_limit(limits.regex_compiled_bytes.saturating_mul(4));
    let regex = builder.build().map_err(|error| match error {
        regex::Error::CompiledTooBig(_) => resource("regex-compiled-bytes"),
        regex::Error::Syntax(_) => {
            unsupported("regex syntax is not supported by the selected engine")
        }
        _ => unsupported("regex could not be compiled by the selected engine"),
    })?;
    Ok(RegexProgram {
        regex,
        global,
        ignore_empty,
    })
}

fn match_object(input: &str, regex: &Regex, captures: &Captures<'_>) -> Result<Value, VmError> {
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
        value.insert(
            Arc::from("length"),
            capture.map_or_else(
                || number_i64(0),
                |matched| number_usize(matched.as_str().chars().count()),
            )?,
        );
        value.insert(
            Arc::from("string"),
            capture.map_or(Value::Null, |matched| Value::string(matched.as_str())),
        );
        value.insert(Arc::from("name"), name.map_or(Value::Null, Value::string));
        capture_values.push(Value::object(value));
    }
    object.insert(Arc::from("captures"), Value::array(capture_values));
    Ok(Value::object(object))
}

fn named_capture_object(regex: &Regex, captures: &Captures<'_>) -> Value {
    let mut object = IndexMap::new();
    for (index, name) in regex.capture_names().enumerate().skip(1) {
        if let Some(name) = name {
            object.insert(Arc::from(name), capture_string(captures.get(index)));
        }
    }
    Value::object(object)
}

fn capture_string(capture: Option<regex::Match<'_>>) -> Value {
    capture.map_or(Value::Null, |matched| Value::string(matched.as_str()))
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
    output.push_str(value);
    Ok(())
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
        Err(runtime(format!(
            "{operation} requires platform access permitted by capability policy"
        )))
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
