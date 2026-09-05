//! Validates each complete row before committing a header or row bytes.

use std::{borrow::Cow, collections::BTreeSet, io::Write, sync::Arc};

use tq_core::Value;

use crate::{DelimitedLimits, OutputError};

pub(crate) struct DelimitedOutput {
    header: Option<Vec<Arc<str>>>,
    keys: BTreeSet<Arc<str>>,
    delimiter: u8,
}

impl DelimitedOutput {
    pub(crate) const fn new(delimiter: u8) -> Self {
        Self {
            header: None,
            keys: BTreeSet::new(),
            delimiter,
        }
    }

    pub(crate) fn write_result(
        &mut self,
        writer: &mut impl Write,
        value: &Value,
        strict_conversion: bool,
        limits: DelimitedLimits,
    ) -> Result<(), OutputError> {
        let Value::Object(values) = value else {
            return Err(OutputError::Profile(
                "delimited output requires an object root",
            ));
        };
        if values.len() > limits.fields {
            return Err(OutputError::Resource("field-count"));
        }
        for (key, value) in values.iter() {
            if matches!(value, Value::Array(_) | Value::Object(_)) {
                return Err(OutputError::Profile(
                    "delimited fields cannot contain arrays or objects",
                ));
            }
            if self.header.is_some() && !self.keys.contains(key) {
                return Err(OutputError::Profile(
                    "row has a key outside the established header",
                ));
            }
        }
        if strict_conversion
            && self
                .header
                .as_ref()
                .is_some_and(|header| header.len() != values.len())
        {
            return Err(OutputError::Profile(
                "strict conversion would normalize missing keys to null",
            ));
        }
        let new_header = self
            .header
            .is_none()
            .then(|| values.keys().cloned().collect::<Vec<_>>());
        let header = self
            .header
            .as_ref()
            .or(new_header.as_ref())
            .expect("header selected");
        let row = header
            .iter()
            .map(|key| {
                PreparedField::value(
                    values.get(key.as_ref()).unwrap_or(&Value::Null),
                    self.delimiter,
                    limits,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_row(&row, limits)?;
        let header_row = if new_header.is_some() {
            let fields = header
                .iter()
                .map(|key| {
                    PreparedField::text(Cow::Borrowed(key), self.delimiter, key.is_empty(), limits)
                })
                .collect::<Result<Vec<_>, _>>()?;
            validate_row(&fields, limits)?;
            Some(fields)
        } else {
            None
        };
        if let Some(fields) = header_row.as_ref() {
            write_row(writer, fields, self.delimiter)?;
        }
        write_row(writer, &row, self.delimiter)?;
        if let Some(header) = new_header {
            self.keys.extend(header.iter().cloned());
            self.header = Some(header);
        }
        Ok(())
    }
}

struct PreparedField<'a> {
    text: Cow<'a, str>,
    quoted: bool,
    encoded_bytes: usize,
}

impl<'a> PreparedField<'a> {
    fn value(
        value: &'a Value,
        delimiter: u8,
        limits: DelimitedLimits,
    ) -> Result<Self, OutputError> {
        let (text, quoted) = match value {
            Value::Null => (Cow::Borrowed(""), false),
            Value::Bool(value) => (Cow::Borrowed(if *value { "true" } else { "false" }), false),
            Value::Number(value) => (Cow::Owned(value.to_string()), false),
            Value::String(text) => (
                Cow::Borrowed(text.as_ref()),
                crate::delimited_profile::inferred_scalar(text).is_some(),
            ),
            Value::Array(_) | Value::Object(_) => unreachable!("complete row validated"),
        };
        Self::text(text, delimiter, quoted, limits)
    }

    fn text(
        text: Cow<'a, str>,
        delimiter: u8,
        force_quote: bool,
        limits: DelimitedLimits,
    ) -> Result<Self, OutputError> {
        if text.len() > limits.field_bytes {
            return Err(OutputError::Resource("field-bytes"));
        }
        let quoted = force_quote || needs_quotes(&text, delimiter);
        let encoded_bytes = if quoted {
            text.len()
                .saturating_add(2)
                .saturating_add(text.bytes().filter(|byte| *byte == b'"').count())
        } else {
            text.len()
        };
        Ok(Self {
            text,
            quoted,
            encoded_bytes,
        })
    }
}

fn validate_row(fields: &[PreparedField<'_>], limits: DelimitedLimits) -> Result<(), OutputError> {
    if fields.len() > limits.fields {
        return Err(OutputError::Resource("field-count"));
    }
    let bytes = fields.iter().fold(fields.len().max(1), |bytes, field| {
        bytes.saturating_add(field.encoded_bytes)
    });
    if bytes > limits.row_bytes {
        return Err(OutputError::Resource("row-bytes"));
    }
    Ok(())
}

fn write_row(
    writer: &mut impl Write,
    fields: &[PreparedField<'_>],
    delimiter: u8,
) -> Result<(), OutputError> {
    for (index, field) in fields.iter().enumerate() {
        if index != 0 {
            writer.write_all(&[delimiter])?;
        }
        write_string(writer, &field.text, delimiter, field.quoted)?;
    }
    writer.write_all(b"\n")?;
    Ok(())
}

fn needs_quotes(text: &str, delimiter: u8) -> bool {
    text.bytes()
        .any(|byte| byte == delimiter || byte == b'"' || byte.is_ascii_control())
}

fn write_string(
    writer: &mut impl Write,
    text: &str,
    delimiter: u8,
    force_quote: bool,
) -> Result<(), OutputError> {
    let quoted = force_quote || needs_quotes(text, delimiter);
    if !quoted {
        writer.write_all(text.as_bytes())?;
        return Ok(());
    }
    writer.write_all(b"\"")?;
    for (index, part) in text.split('"').enumerate() {
        if index != 0 {
            writer.write_all(b"\"\"")?;
        }
        writer.write_all(part.as_bytes())?;
    }
    writer.write_all(b"\"")?;
    Ok(())
}
