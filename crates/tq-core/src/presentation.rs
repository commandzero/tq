//! Bounded ANSI presentation for serializer-owned semantic spans.
//!
//! This module never reads the environment or interprets serialized syntax.
//! Writers select roles while encoding; callers select whether to decorate.

use std::io::{self, Write};

/// The eight roles accepted by `JQ_COLORS`, in its documented order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum ColorRole {
    /// Null scalar content.
    Null,
    /// False scalar content.
    False,
    /// True scalar content.
    True,
    /// Number scalar content and TOON counts.
    Number,
    /// String content, including encoded escapes.
    String,
    /// Array syntax and quotes directly inside sequences.
    Array,
    /// Object syntax, mapping quotes, and standalone scalar quotes.
    Object,
    /// Object keys and table header content.
    Key,
}

/// Caller-selected SGR styles; construction does not enable terminal output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColorPalette {
    styles: [String; 8],
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            styles: [
                "0;39", "0;94", "0;94", "0;35", "0;32", "0;90", "0;90", "0;36",
            ]
            .map(str::to_owned),
        }
    }
}

impl ColorPalette {
    /// Parses the existing seven/eight-slot interface, falling back as a whole.
    /// In the seven-slot form, object keys inherit the number style.
    #[must_use]
    pub fn from_jq_colors(value: &str) -> Self {
        let mut entries = value.split(':');
        let styles: [Option<&str>; 8] = std::array::from_fn(|_| entries.next());
        if entries.next().is_some()
            || styles[..7].iter().any(Option::is_none)
            || styles.iter().flatten().any(|style| {
                style.is_empty()
                    || style
                        .bytes()
                        .any(|byte| byte != b';' && !byte.is_ascii_digit())
            })
        {
            return Self::default();
        }
        Self {
            styles: std::array::from_fn(|index| {
                styles[index].or(styles[3]).unwrap_or_default().to_owned()
            }),
        }
    }

    /// Returns an SGR parameter string without the escape prefix or terminator.
    #[must_use]
    pub fn style(&self, role: ColorRole) -> &str {
        &self.styles[role as usize]
    }
}

const RESET: &[u8] = b"\x1b[0m";
const CHUNK: usize = 4096;

/// Writes one semantic span, leaving terminal state reset at every boundary.
///
/// Plain spans allocate nothing. Decorated spans use a fixed stack chunk for
/// ordinary palettes, independent of payload length. A custom style exceeding
/// that chunk needs storage proportional only to that caller-supplied style.
/// Each chunk includes its closing reset in the same write offered to a bounded
/// sink, so the sink can reject it before publishing an unclosable style.
///
/// # Errors
/// Returns the original output error. After a partial write, attempts a reset
/// only if the sink has not reported a broken pipe.
pub fn write_span<W: Write + ?Sized>(
    writer: &mut W,
    palette: Option<&ColorPalette>,
    role: ColorRole,
    bytes: &[u8],
) -> io::Result<()> {
    let Some(palette) = palette else {
        return writer.write_all(bytes);
    };
    if bytes.is_empty() {
        return Ok(());
    }
    let style = palette.style(role).as_bytes();
    let overhead = style.len() + 3 + RESET.len();
    let mut stack = [0_u8; CHUNK];
    let mut large;
    let buffer = if overhead + 4 <= CHUNK {
        &mut stack[..]
    } else {
        large = vec![0; overhead + CHUNK];
        &mut large[..]
    };
    buffer[..2].copy_from_slice(b"\x1b[");
    buffer[2..2 + style.len()].copy_from_slice(style);
    buffer[2 + style.len()] = b'm';
    let prefix = style.len() + 3;
    for part in bytes.split_inclusive(|byte| matches!(byte, b'\r' | b'\n' | 0x1e)) {
        let (mut content, boundary) = if part
            .last()
            .is_some_and(|byte| matches!(byte, b'\r' | b'\n' | 0x1e))
        {
            part.split_at(part.len() - 1)
        } else {
            (part, &[][..])
        };
        while !content.is_empty() {
            let mut count = content.len().min(buffer.len() - overhead);
            // Do not insert SGR inside an encoded UTF-8 codepoint.
            if count < content.len() {
                while count > 0 && content[count] & 0xc0 == 0x80 {
                    count -= 1;
                }
            }
            debug_assert!(count > 0, "a chunk has room for a UTF-8 codepoint");
            buffer[prefix..prefix + count].copy_from_slice(&content[..count]);
            let end = prefix + count + RESET.len();
            buffer[prefix + count..end].copy_from_slice(RESET);
            write_styled_bytes(writer, &buffer[..end])?;
            content = &content[count..];
        }
        writer.write_all(boundary)?;
    }
    Ok(())
}

/// Publishes already-styled bytes without adding another layer of color.
/// Attempts a reset after a partial non-broken-pipe failure, preserving the
/// original error. A rejected first write emits no recovery bytes.
///
/// # Errors
/// Returns the original publication error, including a zero-length write.
pub fn write_styled_bytes<W: Write + ?Sized>(writer: &mut W, bytes: &[u8]) -> io::Result<()> {
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let error = match writer.write(remaining) {
            Ok(0) => io::Error::from(io::ErrorKind::WriteZero),
            Ok(count) => {
                remaining = &remaining[count..];
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => error,
        };
        if remaining.len() != bytes.len() && error.kind() != io::ErrorKind::BrokenPipe {
            let _ = writer.write_all(RESET);
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_preserves_seven_and_eight_slot_interface() {
        let seven = ColorPalette::from_jq_colors("30:31:32:33:34:35:36");
        assert_eq!(seven.style(ColorRole::Key), "33");
        let eight = ColorPalette::from_jq_colors("30:31:32:33:34:35:36:37");
        assert_eq!(eight.style(ColorRole::Key), "37");
        for invalid in [
            "",
            "30:31",
            "30:31:32:33:34:35:",
            "30:31:32:33:34:35:bad",
            "1:2:3:4:5:6:7:8:9",
        ] {
            assert_eq!(
                ColorPalette::from_jq_colors(invalid),
                ColorPalette::default()
            );
        }
    }

    #[test]
    fn spans_reset_before_transport_bytes_and_emit_nothing_for_empty_content() {
        let mut output = Vec::new();
        write_span(
            &mut output,
            Some(&ColorPalette::default()),
            ColorRole::String,
            b"a\n\rb\x1e",
        )
        .unwrap();
        assert_eq!(output, b"\x1b[0;32ma\x1b[0m\n\r\x1b[0;32mb\x1b[0m\x1e");
        write_span(
            &mut output,
            Some(&ColorPalette::default()),
            ColorRole::String,
            b"",
        )
        .unwrap();
        assert!(output.ends_with(b"\x1e"));
    }

    #[test]
    fn long_unicode_spans_remain_utf8_with_bounded_writes() {
        struct Bounded(Vec<u8>);
        impl Write for Bounded {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                assert!(bytes.len() <= CHUNK);
                std::str::from_utf8(bytes).unwrap();
                self.0.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let text = "😀".repeat(10000);
        let mut output = Bounded(Vec::new());
        write_span(
            &mut output,
            Some(&ColorPalette::default()),
            ColorRole::String,
            text.as_bytes(),
        )
        .unwrap();
        let rendered = String::from_utf8(output.0).unwrap();
        assert_eq!(
            rendered.replace("\x1b[0;32m", "").replace("\x1b[0m", ""),
            text
        );
    }

    #[test]
    fn budget_rejection_does_not_publish_an_unclosable_style() {
        struct Budget {
            output: Vec<u8>,
            remaining: usize,
        }
        impl Write for Budget {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if bytes.len() > self.remaining {
                    return Err(io::Error::other("output limit"));
                }
                self.remaining -= bytes.len();
                self.output.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut output = Budget {
            output: Vec::new(),
            remaining: 10,
        };
        let error = write_span(
            &mut output,
            Some(&ColorPalette::default()),
            ColorRole::Number,
            b"1",
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "output limit");
        assert!(output.output.is_empty());
        output.remaining = CHUNK + 1;
        assert!(
            write_span(
                &mut output,
                Some(&ColorPalette::default()),
                ColorRole::String,
                &vec![b'x'; CHUNK * 2]
            )
            .is_err()
        );
        assert!(output.output.ends_with(RESET));
        assert_eq!(output.output.len(), CHUNK);
    }

    #[test]
    fn partial_failures_reset_without_replacing_errors_or_retrying_broken_pipes() {
        struct Partial {
            output: Vec<u8>,
            calls: usize,
            kind: io::ErrorKind,
        }
        impl Write for Partial {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.calls += 1;
                if self.calls == 2 {
                    return Err(io::Error::new(self.kind, "original failure"));
                }
                let count = if self.calls == 1 {
                    3.min(bytes.len())
                } else {
                    bytes.len()
                };
                self.output.extend_from_slice(&bytes[..count]);
                Ok(count)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        for kind in [io::ErrorKind::Other, io::ErrorKind::BrokenPipe] {
            let mut output = Partial {
                output: Vec::new(),
                calls: 0,
                kind,
            };
            let error = write_span(
                &mut output,
                Some(&ColorPalette::default()),
                ColorRole::String,
                b"text",
            )
            .unwrap_err();
            assert_eq!(error.to_string(), "original failure");
            if kind == io::ErrorKind::BrokenPipe {
                assert_eq!(output.calls, 2);
            } else {
                assert!(output.output.ends_with(RESET));
            }
        }
    }
}
