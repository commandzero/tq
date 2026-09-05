//! Logical rows retain field quoting for the native scalar profile.

use std::io::{self, BufRead};

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Field {
    pub(crate) text: String,
    pub(crate) quoted: bool,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DelimitedError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("input resource limit exceeded: {0}")]
    Resource(&'static str),
    #[error("{0}")]
    Syntax(&'static str),
}

enum FieldState {
    Start,
    Unquoted,
    Quoted,
    AfterQuote,
}

pub(crate) struct DelimitedFramer<R> {
    reader: R,
    delimiter: u8,
    maximum_row_bytes: usize,
    maximum_field_bytes: usize,
    maximum_fields: usize,
}

impl<R: BufRead> DelimitedFramer<R> {
    pub(crate) const fn new(
        reader: R,
        delimiter: u8,
        maximum_row_bytes: usize,
        maximum_field_bytes: usize,
        maximum_fields: usize,
    ) -> Self {
        Self {
            reader,
            delimiter,
            maximum_row_bytes,
            maximum_field_bytes,
            maximum_fields,
        }
    }

    pub(crate) fn next_row(&mut self) -> Result<Option<Vec<Field>>, DelimitedError> {
        let mut fields = Vec::new();
        let mut field = Vec::new();
        let mut state = FieldState::Start;
        let mut quoted = false;
        let mut row_bytes = 0;
        loop {
            let Some(&byte) = self.reader.fill_buf()?.first() else {
                if matches!(state, FieldState::Quoted) {
                    return Err(DelimitedError::Syntax("unterminated quoted field"));
                }
                if row_bytes == 0 {
                    return Ok(None);
                }
                self.finish_field(&mut fields, field, quoted)?;
                return Ok(Some(fields));
            };
            self.reader.consume(1);
            row_bytes += 1;
            if row_bytes > self.maximum_row_bytes {
                return Err(DelimitedError::Resource("row-bytes"));
            }
            if matches!(state, FieldState::Quoted) {
                if byte == b'"' {
                    state = FieldState::AfterQuote;
                } else {
                    self.append(&mut field, byte)?;
                }
                continue;
            }
            if matches!(state, FieldState::AfterQuote) && byte == b'"' {
                self.append(&mut field, byte)?;
                state = FieldState::Quoted;
            } else if byte == self.delimiter {
                self.finish_field(&mut fields, std::mem::take(&mut field), quoted)?;
                quoted = false;
                state = FieldState::Start;
            } else if byte == b'\n' || byte == b'\r' {
                if byte == b'\r' && self.reader.fill_buf()?.first() == Some(&b'\n') {
                    self.reader.consume(1);
                    row_bytes += 1;
                    if row_bytes > self.maximum_row_bytes {
                        return Err(DelimitedError::Resource("row-bytes"));
                    }
                }
                if !fields.is_empty() || !field.is_empty() || quoted {
                    self.finish_field(&mut fields, field, quoted)?;
                }
                return Ok(Some(fields));
            } else if matches!(state, FieldState::AfterQuote) {
                return Err(DelimitedError::Syntax(
                    "expected delimiter or row ending after closing quote",
                ));
            } else if byte == b'"' {
                if !matches!(state, FieldState::Start) {
                    return Err(DelimitedError::Syntax("quote inside an unquoted field"));
                }
                quoted = true;
                state = FieldState::Quoted;
            } else {
                self.append(&mut field, byte)?;
                state = FieldState::Unquoted;
            }
        }
    }

    fn append(&self, field: &mut Vec<u8>, byte: u8) -> Result<(), DelimitedError> {
        if field.len() >= self.maximum_field_bytes {
            return Err(DelimitedError::Resource("field-bytes"));
        }
        field.push(byte);
        Ok(())
    }

    fn finish_field(
        &self,
        fields: &mut Vec<Field>,
        bytes: Vec<u8>,
        quoted: bool,
    ) -> Result<(), DelimitedError> {
        if fields.len() >= self.maximum_fields {
            return Err(DelimitedError::Resource("field-count"));
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| DelimitedError::Syntax("field contains invalid UTF-8"))?;
        fields.push(Field { text, quoted });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn delimited_framing_rejects_ambiguous_quotes_and_enforces_bounds() {
        for bytes in [b"a\"b\n".as_slice(), b"\"a\"x\n", b"\"unterminated"] {
            let mut rows = DelimitedFramer::new(BufReader::new(bytes), b',', 64, 64, 8);
            assert!(matches!(rows.next_row(), Err(DelimitedError::Syntax(_))));
        }
        for (bytes, row, field, count, expected) in [
            (b"\"a\nb\"\n".as_slice(), 5, 8, 8, "row-bytes"),
            (b"abc\n".as_slice(), 8, 2, 8, "field-bytes"),
            (b"a,b,c\n".as_slice(), 8, 8, 2, "field-count"),
        ] {
            for capacity in 1..=8 {
                let mut rows = DelimitedFramer::new(
                    BufReader::with_capacity(capacity, bytes),
                    b',',
                    row,
                    field,
                    count,
                );
                assert!(
                    matches!(rows.next_row(), Err(DelimitedError::Resource(resource)) if resource == expected)
                );
            }
        }
    }

    #[test]
    fn delimited_framing_is_invariant_across_tiny_chunks() {
        for &delimiter in b",\t" {
            let bytes = format!(
                "a{}b\r\n\"one\ntwo\"{}\"a\"\"b\"\n",
                delimiter as char, delimiter as char
            );
            for capacity in 1..=16 {
                let mut scanner = DelimitedFramer::new(
                    BufReader::with_capacity(capacity, bytes.as_bytes()),
                    delimiter,
                    64,
                    16,
                    2,
                );
                assert_eq!(
                    scanner.next_row().unwrap().unwrap(),
                    vec![
                        Field {
                            text: "a".into(),
                            quoted: false
                        },
                        Field {
                            text: "b".into(),
                            quoted: false
                        }
                    ]
                );
                assert_eq!(
                    scanner.next_row().unwrap().unwrap(),
                    vec![
                        Field {
                            text: "one\ntwo".into(),
                            quoted: true
                        },
                        Field {
                            text: "a\"b".into(),
                            quoted: true
                        }
                    ]
                );
                assert!(scanner.next_row().unwrap().is_none());
            }
        }
    }
}
