use std::io::{Read, Write};

use tq_core::presentation::{ColorPalette, ColorRole, write_span};

use crate::writer::{ScalarContext, WriterConfig, WriterError};

const HEX: &[u8; 16] = b"0123456789abcdef";

#[derive(Clone, Copy)]
pub(crate) struct ScalarRenderOptions<'a> {
    pub(crate) tag: u8,
    pub(crate) length: u64,
    pub(crate) quote_mask: u8,
    pub(crate) config: WriterConfig,
    pub(crate) context: ScalarContext,
    pub(crate) palette: Option<&'a ColorPalette>,
}

pub(crate) fn write_scalar_range<R: Read, W: Write>(
    reader: &mut R,
    scratch: &mut [u8],
    output: &mut W,
    options: ScalarRenderOptions<'_>,
) -> Result<(), WriterError> {
    let ScalarRenderOptions {
        tag,
        length,
        quote_mask,
        config,
        context,
        palette,
    } = options;
    let role = match tag {
        0 => Some((ColorRole::Null, b"null".as_slice())),
        3 => Some((ColorRole::False, b"false".as_slice())),
        4 => Some((ColorRole::True, b"true".as_slice())),
        _ => None,
    };
    if let Some((role, bytes)) = role {
        write_span(output, palette, role, bytes)?;
        return Ok(());
    }
    if tag == 2 {
        copy_range(reader, length, scratch, output, ColorRole::Number, palette)?;
        return Ok(());
    }
    if tag != 1 {
        return Err(invalid_data("invalid scalar tag"));
    }
    if scratch.is_empty() {
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty scalar scratch").into(),
        );
    }
    let delimiter = match config.delimiter {
        crate::Delimiter::Comma => 1,
        crate::Delimiter::Tab => 2,
        crate::Delimiter::Pipe => 4,
    };
    let root_bom = matches!(context, ScalarContext::Root) && quote_mask & 16 != 0;
    if quote_mask & (delimiter | 8) == 0 && !root_bom {
        return copy_utf8_range(reader, length, scratch, output, palette, ColorRole::String);
    }
    let quote_role = match context {
        ScalarContext::Array => ColorRole::Array,
        ScalarContext::Root | ScalarContext::Object => ColorRole::Object,
    };
    write_span(output, palette, quote_role, b"\"")?;
    write_escaped_string(reader, scratch, output, length, palette)?;
    write_span(output, palette, quote_role, b"\"")?;
    Ok(())
}

fn write_escaped_string<R: Read, W: Write>(
    reader: &mut R,
    scratch: &mut [u8],
    output: &mut W,
    length: u64,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    let mut remaining = length;
    let mut carry = [0_u8; 4];
    let mut carry_len = 0;
    let mut escaped_content = [0_u8; 256];
    let mut escaped_content_len = 0;
    while remaining != 0 {
        let capacity = scratch
            .len()
            .min(usize::try_from(remaining).unwrap_or(usize::MAX));
        let count = reader.read(&mut scratch[..capacity])?;
        if count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated scalar range",
            )
            .into());
        }
        remaining -=
            u64::try_from(count).map_err(|_| invalid_data("scalar read length overflow"))?;
        for byte in &scratch[..count] {
            if carry_len == carry.len() {
                return Err(invalid_data("invalid UTF-8 scalar range"));
            }
            carry[carry_len] = *byte;
            carry_len += 1;
            match utf8_width(&carry[..carry_len]) {
                Some(0) => return Err(invalid_data("invalid UTF-8 scalar range")),
                Some(width) if width == carry_len => {
                    append_character(
                        carry,
                        &mut carry_len,
                        &mut escaped_content,
                        &mut escaped_content_len,
                    )?;
                    if escaped_content_len > escaped_content.len() - 8 {
                        write_span(
                            output,
                            palette,
                            ColorRole::String,
                            &escaped_content[..escaped_content_len],
                        )?;
                        escaped_content_len = 0;
                    }
                }
                _ => {}
            }
        }
    }
    if carry_len != 0 {
        return Err(invalid_data("truncated UTF-8 scalar range"));
    }
    if escaped_content_len != 0 {
        write_span(
            output,
            palette,
            ColorRole::String,
            &escaped_content[..escaped_content_len],
        )?;
    }
    Ok(())
}

fn invalid_data(message: &'static str) -> WriterError {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message).into()
}

fn copy_utf8_range<R: Read, W: Write>(
    reader: &mut R,
    mut remaining: u64,
    scratch: &mut [u8],
    output: &mut W,
    palette: Option<&ColorPalette>,
    role: ColorRole,
) -> Result<(), WriterError> {
    if scratch.is_empty() {
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty scalar scratch").into(),
        );
    }
    let mut carry = [0_u8; 4];
    let mut carry_len = 0;
    while remaining != 0 {
        let capacity = scratch
            .len()
            .min(usize::try_from(remaining).unwrap_or(usize::MAX));
        let count = reader.read(&mut scratch[..capacity])?;
        if count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated scalar range",
            )
            .into());
        }
        remaining -=
            u64::try_from(count).map_err(|_| invalid_data("scalar read length overflow"))?;
        let mut start = 0;
        if carry_len != 0 {
            while start < count {
                carry[carry_len] = scratch[start];
                carry_len += 1;
                start += 1;
                match utf8_width(&carry[..carry_len]) {
                    Some(0) => return Err(invalid_data("invalid UTF-8 scalar range")),
                    Some(width) if width == carry_len => {
                        write_span(output, palette, role, &carry[..carry_len])?;
                        carry_len = 0;
                        break;
                    }
                    _ => {}
                }
            }
            if carry_len != 0 {
                continue;
            }
        }
        let bytes = &scratch[start..count];
        match std::str::from_utf8(bytes) {
            Ok(_) => write_span(output, palette, role, bytes)?,
            Err(error) if error.error_len().is_some() => {
                return Err(invalid_data("invalid UTF-8 scalar range"));
            }
            Err(error) => {
                let (complete, partial) = bytes.split_at(error.valid_up_to());
                write_span(output, palette, role, complete)?;
                carry[..partial.len()].copy_from_slice(partial);
                carry_len = partial.len();
            }
        }
    }
    if carry_len != 0 {
        return Err(invalid_data("truncated UTF-8 scalar range"));
    }
    Ok(())
}

fn copy_range<R: Read, W: Write>(
    reader: &mut R,
    mut remaining: u64,
    scratch: &mut [u8],
    output: &mut W,
    role: ColorRole,
    palette: Option<&ColorPalette>,
) -> Result<(), WriterError> {
    if scratch.is_empty() {
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty scalar scratch").into(),
        );
    }
    while remaining != 0 {
        let capacity = scratch
            .len()
            .min(usize::try_from(remaining).unwrap_or(usize::MAX));
        let count = reader.read(&mut scratch[..capacity])?;
        if count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated scalar range",
            )
            .into());
        }
        write_span(output, palette, role, &scratch[..count])?;
        remaining -=
            u64::try_from(count).map_err(|_| invalid_data("scalar read length overflow"))?;
    }
    Ok(())
}

fn utf8_width(bytes: &[u8]) -> Option<usize> {
    let first = *bytes.first()?;
    let width = if first < 0x80 {
        1
    } else if (0xc2..=0xdf).contains(&first) {
        2
    } else if (0xe0..=0xef).contains(&first) {
        3
    } else if (0xf0..=0xf4).contains(&first) {
        4
    } else {
        return Some(0);
    };
    match std::str::from_utf8(bytes) {
        Ok(_) => Some(width),
        Err(error) if error.error_len().is_some() => Some(0),
        Err(_) => None,
    }
}

fn append_character(
    bytes: [u8; 4],
    length: &mut usize,
    output: &mut [u8; 256],
    output_len: &mut usize,
) -> Result<(), WriterError> {
    let text = std::str::from_utf8(&bytes[..*length])
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let character = text.chars().next().expect("nonempty character");
    let mut escaped = [0_u8; 6];
    let value: &[u8] = match character {
        '"' => b"\\\"",
        '\\' => b"\\\\",
        '\n' => b"\\n",
        '\r' => b"\\r",
        '\t' => b"\\t",
        character if character.is_control() => {
            let code = character as u32;
            escaped[..2].copy_from_slice(b"\\u");
            escaped[2] = HEX[((code >> 12) & 15) as usize];
            escaped[3] = HEX[((code >> 8) & 15) as usize];
            escaped[4] = HEX[((code >> 4) & 15) as usize];
            escaped[5] = HEX[(code & 15) as usize];
            &escaped
        }
        _ => &bytes[..*length],
    };
    output[*output_len..*output_len + value.len()].copy_from_slice(value);
    *output_len += value.len();
    *length = 0;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ScalarRenderOptions, write_scalar_range};
    use crate::writer::{ScalarContext, WriterConfig};

    #[test]
    fn fragmented_unicode_and_controls_keep_canonical_quoted_bytes() {
        let value = "λ🦀\u{0001}\"".as_bytes();
        let mut scratch = [0_u8; 1];
        let mut output = Vec::new();
        write_scalar_range(
            &mut &value[..],
            &mut scratch,
            &mut output,
            ScalarRenderOptions {
                tag: 1,
                length: value.len() as u64,
                quote_mask: 8,
                config: WriterConfig::default(),
                context: ScalarContext::Array,
                palette: None,
            },
        )
        .unwrap();
        assert_eq!(output, "\"λ🦀\\u0001\\\"\"".as_bytes());
    }

    #[test]
    fn number_and_boolean_ranges_keep_canonical_tokens() {
        let mut scratch = [0_u8; 2];
        let mut output = Vec::new();
        write_scalar_range(
            &mut &b"100"[..],
            &mut scratch,
            &mut output,
            ScalarRenderOptions {
                tag: 2,
                length: 3,
                quote_mask: 0,
                config: WriterConfig::default(),
                context: ScalarContext::Root,
                palette: None,
            },
        )
        .unwrap();
        write_scalar_range(
            &mut &[][..],
            &mut scratch,
            &mut output,
            ScalarRenderOptions {
                tag: 4,
                length: 0,
                quote_mask: 0,
                config: WriterConfig::default(),
                context: ScalarContext::Root,
                palette: None,
            },
        )
        .unwrap();
        assert_eq!(output, b"100true");
    }
    #[test]
    fn colored_unquoted_utf8_is_valid_across_fragmented_reads() {
        let text = format!("{}λ🦀{}", "a".repeat(4097), "z".repeat(35));
        let palette = tq_core::presentation::ColorPalette::default();
        for size in [1, 2, 3, 4, 5, 4096] {
            let mut scratch = vec![0_u8; size];
            let mut output = Vec::new();
            write_scalar_range(
                &mut text.as_bytes(),
                &mut scratch,
                &mut output,
                ScalarRenderOptions {
                    tag: 1,
                    length: text.len() as u64,
                    quote_mask: 0,
                    config: WriterConfig::default(),
                    context: ScalarContext::Array,
                    palette: Some(&palette),
                },
            )
            .unwrap();
            assert!(std::str::from_utf8(&output).is_ok(), "scratch size {size}");
            assert_eq!(strip_sgr(&output), text.as_bytes(), "scratch size {size}");
        }
    }

    #[test]
    fn string_ranges_reject_invalid_and_truncated_utf8() {
        for mask in [0, 8] {
            for bytes in [
                &[0x80][..],
                &[0xe0, 0x80, 0x80][..],
                &[0xed, 0xa0, 0x80][..],
                &[0xf4, 0x90, 0x80, 0x80][..],
                &[0xe2, 0x82][..],
            ] {
                let error = write_scalar_range(
                    &mut &bytes[..],
                    &mut [0_u8; 1],
                    &mut Vec::new(),
                    ScalarRenderOptions {
                        tag: 1,
                        length: bytes.len() as u64,
                        quote_mask: mask,
                        config: WriterConfig::default(),
                        context: ScalarContext::Root,
                        palette: None,
                    },
                )
                .unwrap_err();
                let crate::WriterError::Io(error) = error else {
                    panic!("expected invalid UTF-8 I/O error");
                };
                assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            }
        }
    }

    fn strip_sgr(bytes: &[u8]) -> Vec<u8> {
        let mut plain = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index..].starts_with(b"\x1b[") {
                index += 2;
                while bytes[index] != b'm' {
                    index += 1;
                }
                index += 1;
            } else {
                plain.push(bytes[index]);
                index += 1;
            }
        }
        plain
    }
}
