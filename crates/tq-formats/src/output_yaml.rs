//! YAML serialization with semantic output spans.

use std::io::Write;

use tq_core::{
    Value,
    presentation::{ColorPalette, ColorRole},
};

use crate::{OutputError, output_color::write_span_bounded};

pub(crate) fn write_yaml_value(
    writer: &mut impl Write,
    value: &Value,
    indent: usize,
    quote_role: ColorRole,
    palette: Option<&ColorPalette>,
) -> Result<(), OutputError> {
    match value {
        Value::Null => write_span_bounded(writer, palette, ColorRole::Null, b"null")?,
        Value::Bool(value) => write_span_bounded(
            writer,
            palette,
            if *value {
                ColorRole::True
            } else {
                ColorRole::False
            },
            if *value { b"true" } else { b"false" },
        )?,
        Value::Number(value) => {
            let text = value.to_string();
            write_span_bounded(writer, palette, ColorRole::Number, text.as_bytes())?;
        }
        Value::String(value) => {
            write_yaml_string(writer, value, quote_role, ColorRole::String, palette)?;
        }
        Value::Array(values) if values.is_empty() => {
            write_span_bounded(writer, palette, ColorRole::Array, b"[]")?;
        }
        Value::Object(values) if values.is_empty() => {
            write_span_bounded(writer, palette, ColorRole::Object, b"{}")?;
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b"\n")?;
                }
                write_indent(writer, indent)?;
                write_span_bounded(writer, palette, ColorRole::Array, b"-")?;
                if matches!(value, Value::Array(values) if !values.is_empty())
                    || matches!(value, Value::Object(values) if !values.is_empty())
                {
                    writer.write_all(b"\n")?;
                    write_yaml_value(writer, value, indent + 2, ColorRole::Array, palette)?;
                } else {
                    writer.write_all(b" ")?;
                    write_yaml_value(writer, value, indent + 2, ColorRole::Array, palette)?;
                }
            }
        }
        Value::Object(values) => {
            for (index, (key, value)) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b"\n")?;
                }
                write_indent(writer, indent)?;
                write_yaml_string(writer, key, ColorRole::Object, ColorRole::Key, palette)?;
                write_span_bounded(writer, palette, ColorRole::Object, b":")?;
                if matches!(value, Value::Array(values) if !values.is_empty())
                    || matches!(value, Value::Object(values) if !values.is_empty())
                {
                    writer.write_all(b"\n")?;
                    write_yaml_value(writer, value, indent + 2, ColorRole::Object, palette)?;
                } else {
                    writer.write_all(b" ")?;
                    write_yaml_value(writer, value, indent + 2, ColorRole::Object, palette)?;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn write_yaml_string(
    writer: &mut impl Write,
    value: &str,
    quote_role: ColorRole,
    content_role: ColorRole,
    palette: Option<&ColorPalette>,
) -> Result<(), OutputError> {
    let encoded = yaml_serde::to_string(value)?;
    let encoded = encoded.strip_suffix('\n').unwrap_or(&encoded);
    if encoded.contains('\n') {
        let mut json = Vec::new();
        serde_json::to_writer(&mut json, value)?;
        write_quoted_or_plain(writer, &json, quote_role, content_role, palette)?;
    } else {
        write_quoted_or_plain(
            writer,
            encoded.as_bytes(),
            quote_role,
            content_role,
            palette,
        )?;
    }
    Ok(())
}

fn write_quoted_or_plain(
    writer: &mut impl Write,
    encoded: &[u8],
    quote_role: ColorRole,
    content_role: ColorRole,
    palette: Option<&ColorPalette>,
) -> Result<(), OutputError> {
    let quoted = encoded.len() >= 2
        && ((encoded[0] == b'"' && encoded[encoded.len() - 1] == b'"')
            || (encoded[0] == b'\'' && encoded[encoded.len() - 1] == b'\''));
    if !quoted {
        write_span_bounded(writer, palette, content_role, encoded)?;
        return Ok(());
    }
    write_span_bounded(writer, palette, quote_role, &encoded[..1])?;
    write_span_bounded(
        writer,
        palette,
        content_role,
        &encoded[1..encoded.len() - 1],
    )?;
    write_span_bounded(writer, palette, quote_role, &encoded[encoded.len() - 1..])?;
    Ok(())
}

fn write_indent(writer: &mut impl Write, indent: usize) -> std::io::Result<()> {
    for _ in 0..indent {
        writer.write_all(b" ")?;
    }
    Ok(())
}
