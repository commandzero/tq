//! Small adapters for the shared bounded presentation sink.

use std::io::{self, Write};

use tq_core::presentation::{ColorPalette, ColorRole, write_span};

/// Calls the shared span writer from a serde formatter, whose writer may be
/// unsized. The extra reference is a sized `Write` adapter and does not buffer
/// or copy the span.
pub(crate) fn write_span_bounded<W: Write + ?Sized>(
    writer: &mut W,
    palette: Option<&ColorPalette>,
    role: ColorRole,
    bytes: &[u8],
) -> io::Result<()> {
    write_span(writer, palette, role, bytes)
}

pub(crate) fn write_marked_byte<W: Write + ?Sized>(
    writer: &mut W,
    palette: Option<&ColorPalette>,
    role: ColorRole,
    bytes: &[u8],
    marker: u8,
) -> io::Result<()> {
    let Some(index) = bytes.iter().position(|byte| *byte == marker) else {
        return writer.write_all(bytes);
    };
    writer.write_all(&bytes[..index])?;
    write_span_bounded(writer, palette, role, &bytes[index..=index])?;
    writer.write_all(&bytes[index + 1..])
}
