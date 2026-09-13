//! JSON serialization with semantic output spans.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::ser::{CharEscape, Formatter};
use tq_core::{
    Value,
    presentation::{ColorPalette, ColorRole},
};

use crate::output_color::{write_marked_byte, write_span_bounded};

pub(crate) fn write_json_value<W: Write>(
    writer: &mut W,
    value: &Value,
    pretty: bool,
    indent: &[u8],
    ascii: bool,
    palette: Option<&ColorPalette>,
) -> Result<(), serde_json::Error> {
    if pretty {
        let formatter = ColoringFormatter::new(
            serde_json::ser::PrettyFormatter::with_indent(indent),
            ascii,
            palette,
        );
        let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
        value.serialize(&mut serializer)
    } else {
        let formatter = ColoringFormatter::new(serde_json::ser::CompactFormatter, ascii, palette);
        let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
        value.serialize(&mut serializer)
    }
}

struct ColoringFormatter<'a, F> {
    inner: F,
    ascii: bool,
    palette: Option<&'a ColorPalette>,
    containers: Vec<ColorRole>,
    in_object_key: bool,
}

impl<'a, F> ColoringFormatter<'a, F> {
    fn new(inner: F, ascii: bool, palette: Option<&'a ColorPalette>) -> Self {
        Self {
            inner,
            ascii,
            palette,
            containers: Vec::new(),
            in_object_key: false,
        }
    }

    fn quote_role(&self) -> ColorRole {
        if self.in_object_key {
            ColorRole::Object
        } else {
            self.containers.last().copied().unwrap_or(ColorRole::Object)
        }
    }

    fn scalar_role(&self, role: ColorRole) -> ColorRole {
        if self.in_object_key {
            ColorRole::Key
        } else {
            role
        }
    }

    fn scalar<W, Fmt>(&mut self, writer: &mut W, role: ColorRole, emit: Fmt) -> io::Result<()>
    where
        W: Write + ?Sized,
        Fmt: FnOnce(&mut F, &mut Vec<u8>) -> io::Result<()>,
    {
        let mut bytes = Vec::new();
        emit(&mut self.inner, &mut bytes)?;
        write_span_bounded(writer, self.palette, role, &bytes)
    }

    fn formatting<W, Fmt>(
        &mut self,
        writer: &mut W,
        emit: Fmt,
        role: ColorRole,
        marker: Option<u8>,
    ) -> io::Result<()>
    where
        W: Write + ?Sized,
        Fmt: FnOnce(&mut F, &mut Vec<u8>) -> io::Result<()>,
    {
        let mut bytes = Vec::new();
        emit(&mut self.inner, &mut bytes)?;
        if let Some(marker) = marker {
            write_marked_byte(writer, self.palette, role, &bytes, marker)
        } else {
            write_span_bounded(writer, self.palette, role, &bytes)
        }
    }
}

macro_rules! number_method {
    ($name:ident, $type:ty) => {
        fn $name<W>(&mut self, writer: &mut W, value: $type) -> io::Result<()>
        where
            W: ?Sized + Write,
        {
            if self.palette.is_none() {
                return self.inner.$name(writer, value);
            }
            let role = self.scalar_role(ColorRole::Number);
            self.scalar(writer, role, |inner, output| inner.$name(output, value))
        }
    };
}

impl<F: Formatter> Formatter for ColoringFormatter<'_, F> {
    fn write_null<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            return self.inner.write_null(writer);
        }
        let role = self.scalar_role(ColorRole::Null);
        self.scalar(writer, role, Formatter::write_null)
    }

    fn write_bool<W>(&mut self, writer: &mut W, value: bool) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            return self.inner.write_bool(writer, value);
        }
        let role = self.scalar_role(if value {
            ColorRole::True
        } else {
            ColorRole::False
        });
        self.scalar(writer, role, |inner, output| {
            inner.write_bool(output, value)
        })
    }

    number_method!(write_i8, i8);
    number_method!(write_i16, i16);
    number_method!(write_i32, i32);
    number_method!(write_i64, i64);
    number_method!(write_i128, i128);
    number_method!(write_u8, u8);
    number_method!(write_u16, u16);
    number_method!(write_u32, u32);
    number_method!(write_u64, u64);
    number_method!(write_u128, u128);
    number_method!(write_f32, f32);
    number_method!(write_f64, f64);

    fn write_number_str<W>(&mut self, writer: &mut W, value: &str) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            return self.inner.write_number_str(writer, value);
        }
        let role = self.scalar_role(ColorRole::Number);
        write_span_bounded(writer, self.palette, role, value.as_bytes())
    }

    fn begin_string<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            return self.inner.begin_string(writer);
        }
        self.formatting(
            writer,
            Formatter::begin_string,
            self.quote_role(),
            Some(b'"'),
        )
    }

    fn end_string<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            return self.inner.end_string(writer);
        }
        self.formatting(writer, Formatter::end_string, self.quote_role(), Some(b'"'))
    }

    fn write_string_fragment<W>(&mut self, writer: &mut W, value: &str) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        let role = if self.in_object_key {
            ColorRole::Key
        } else {
            ColorRole::String
        };
        if self.ascii {
            return write_ascii_fragment(writer, self.palette, role, value);
        }
        if self.palette.is_none() {
            return self.inner.write_string_fragment(writer, value);
        }
        write_span_bounded(writer, self.palette, role, value.as_bytes())
    }

    fn write_char_escape<W>(&mut self, writer: &mut W, escape: CharEscape) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            return self.inner.write_char_escape(writer, escape);
        }
        let mut bytes = Vec::new();
        self.inner.write_char_escape(&mut bytes, escape)?;
        write_span_bounded(
            writer,
            self.palette,
            if self.in_object_key {
                ColorRole::Key
            } else {
                ColorRole::String
            },
            &bytes,
        )
    }

    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            let result = self.inner.begin_array(writer);
            if result.is_ok() {
                self.containers.push(ColorRole::Array);
            }
            return result;
        }
        self.formatting(writer, Formatter::begin_array, ColorRole::Array, Some(b'['))?;
        self.containers.push(ColorRole::Array);
        Ok(())
    }

    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            let result = self.inner.end_array(writer);
            if result.is_ok() {
                self.containers.pop();
            }
            return result;
        }
        self.formatting(writer, Formatter::end_array, ColorRole::Array, Some(b']'))?;
        self.containers.pop();
        Ok(())
    }

    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            return self.inner.begin_array_value(writer, first);
        }
        self.formatting(
            writer,
            |inner, output| inner.begin_array_value(output, first),
            ColorRole::Array,
            Some(b','),
        )
    }

    fn end_array_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.inner.end_array_value(writer)
    }

    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            let result = self.inner.begin_object(writer);
            if result.is_ok() {
                self.containers.push(ColorRole::Object);
            }
            return result;
        }
        self.formatting(
            writer,
            Formatter::begin_object,
            ColorRole::Object,
            Some(b'{'),
        )?;
        self.containers.push(ColorRole::Object);
        Ok(())
    }

    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            let result = self.inner.end_object(writer);
            if result.is_ok() {
                self.containers.pop();
            }
            return result;
        }
        self.formatting(writer, Formatter::end_object, ColorRole::Object, Some(b'}'))?;
        self.containers.pop();
        Ok(())
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            let result = self.inner.begin_object_key(writer, first);
            if result.is_ok() {
                self.in_object_key = true;
            }
            return result;
        }
        self.formatting(
            writer,
            |inner, output| inner.begin_object_key(output, first),
            ColorRole::Object,
            Some(b','),
        )?;
        self.in_object_key = true;
        Ok(())
    }

    fn end_object_key<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.inner.end_object_key(writer)
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.palette.is_none() {
            let result = self.inner.begin_object_value(writer);
            if result.is_ok() {
                self.in_object_key = false;
            }
            return result;
        }
        let result = self.formatting(
            writer,
            Formatter::begin_object_value,
            ColorRole::Object,
            Some(b':'),
        );
        if result.is_ok() {
            self.in_object_key = false;
        }
        result
    }

    fn end_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.inner.end_object_value(writer)
    }

    fn write_raw_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.inner.write_raw_fragment(writer, fragment)
    }
}

fn write_ascii_fragment<W: Write + ?Sized>(
    writer: &mut W,
    palette: Option<&ColorPalette>,
    role: ColorRole,
    value: &str,
) -> io::Result<()> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut chunk = [0_u8; 4096];
    let mut length = 0;
    for character in value.chars() {
        if character.is_ascii() {
            if length == chunk.len() {
                write_span_bounded(writer, palette, role, &chunk)?;
                length = 0;
            }
            chunk[length] = character as u8;
            length += 1;
            continue;
        }
        let mut units = [0_u16; 2];
        for unit in character.encode_utf16(&mut units).iter() {
            if length + 6 > chunk.len() {
                write_span_bounded(writer, palette, role, &chunk[..length])?;
                length = 0;
            }
            chunk[length..length + 6].copy_from_slice(&[
                b'\\',
                b'u',
                HEX[usize::from(*unit >> 12)],
                HEX[usize::from((*unit >> 8) & 0x0f)],
                HEX[usize::from((*unit >> 4) & 0x0f)],
                HEX[usize::from(*unit & 0x0f)],
            ]);
            length += 6;
        }
    }
    if length != 0 {
        write_span_bounded(writer, palette, role, &chunk[..length])?;
    }
    Ok(())
}
