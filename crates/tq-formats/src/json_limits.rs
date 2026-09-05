//! Bounds serde's recursive parsing and scratch storage during byte consumption.

use std::{
    cell::Cell,
    io::{self, Read},
    rc::Rc,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum StringState {
    Outside,
    Plain,
    Escape,
}

pub(crate) type LimitFailure = Rc<Cell<Option<&'static str>>>;

pub(crate) struct JsonLimitReader<R> {
    reader: R,
    depth: usize,
    maximum_depth: usize,
    maximum_token_bytes: usize,
    string: StringState,
    unicode_digits: u8,
    unicode_value: u16,
    high_surrogate: bool,
    token_bytes: usize,
    numeric: bool,
    failure: LimitFailure,
    checked: usize,
    pending_failure: Option<&'static str>,
}

impl<R> JsonLimitReader<R> {
    pub(crate) fn new(
        reader: R,
        maximum_depth: usize,
        maximum_token_bytes: usize,
    ) -> (Self, LimitFailure) {
        let failure = Rc::new(Cell::new(None));
        (
            Self {
                reader,
                depth: 0,
                maximum_depth,
                maximum_token_bytes,
                string: StringState::Outside,
                unicode_digits: 0,
                unicode_value: 0,
                high_surrogate: false,
                token_bytes: 0,
                numeric: false,
                failure: Rc::clone(&failure),
                checked: 0,
                pending_failure: None,
            },
            failure,
        )
    }

    fn reject(&self, resource: &'static str) -> io::Error {
        self.failure.set(Some(resource));
        io::Error::other(format!("input resource limit exceeded: {resource}"))
    }

    pub(crate) fn check(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.checked = 0;
        while self.checked < bytes.len() {
            if self.string == StringState::Plain && self.unicode_digits == 0 {
                let remaining = &bytes[self.checked..];
                let length = memchr::memchr2(b'"', b'\\', remaining).unwrap_or(remaining.len());
                let accepted =
                    length.min(self.maximum_token_bytes.saturating_sub(self.token_bytes));
                self.token_bytes += accepted;
                self.checked += accepted;
                if accepted != length {
                    return Err(self.reject("token-bytes"));
                }
                if self.checked == bytes.len() {
                    break;
                }
            }
            let byte = bytes[self.checked];
            if self.string != StringState::Outside {
                self.string_byte(byte)?;
                self.checked += 1;
                continue;
            }
            if byte.is_ascii_whitespace()
                || matches!(byte, b',' | b':' | b'[' | b']' | b'{' | b'}' | b'"')
            {
                self.numeric = false;
                self.token_bytes = 0;
            } else if self.token_bytes == 0 {
                self.numeric = byte == b'-' || byte.is_ascii_digit();
                self.token_bytes = 1;
            } else {
                self.token_bytes = self.token_bytes.saturating_add(1);
            }
            if self.numeric && self.token_bytes > self.maximum_token_bytes {
                return Err(self.reject("token-bytes"));
            }
            match byte {
                b'"' => self.string = StringState::Plain,
                b'[' | b'{' => {
                    if self.depth >= self.maximum_depth {
                        return Err(self.reject("depth"));
                    }
                    self.depth += 1;
                }
                b']' | b'}' => self.depth = self.depth.saturating_sub(1),
                _ => {}
            }
            self.checked += 1;
        }
        Ok(())
    }

    fn string_byte(&mut self, byte: u8) -> io::Result<()> {
        let decoded = if self.unicode_digits != 0 {
            self.unicode_value = (self.unicode_value << 4)
                | u16::try_from(char::from(byte).to_digit(16).unwrap_or(0)).unwrap_or(0);
            self.unicode_digits -= 1;
            if self.unicode_digits != 0 {
                return Ok(());
            }
            let value = self.unicode_value;
            let length = match value {
                0..=0x7f => 1,
                0x80..=0x7ff => 2,
                0xdc00..=0xdfff if self.high_surrogate => 1,
                _ => 3,
            };
            // Charge a surrogate pair as three bytes for its first escape and
            // one for the second. Serde remains responsible for syntax validity.
            self.high_surrogate = (0xd800..=0xdbff).contains(&value);
            length
        } else if self.string == StringState::Escape {
            self.string = StringState::Plain;
            if byte == b'u' {
                self.unicode_digits = 4;
                self.unicode_value = 0;
                return Ok(());
            }
            1
        } else {
            match byte {
                b'"' => {
                    self.string = StringState::Outside;
                    self.token_bytes = 0;
                    self.high_surrogate = false;
                    return Ok(());
                }
                b'\\' => {
                    self.string = StringState::Escape;
                    return Ok(());
                }
                _ => 1,
            }
        };
        if decoded > self.maximum_token_bytes.saturating_sub(self.token_bytes) {
            return Err(self.reject("token-bytes"));
        }
        self.token_bytes += decoded;
        Ok(())
    }
}

impl<R: Read> Read for JsonLimitReader<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if let Some(resource) = self.pending_failure {
            return Err(self.reject(resource));
        }
        let count = self.reader.read(output)?;
        if let Err(error) = self.check(&output[..count]) {
            if self.checked == 0 {
                return Err(error);
            }
            // Publish the valid prefix first. Read-ahead must not make a later
            // limit violation suppress an earlier complete Document or event.
            self.pending_failure = self.failure.take();
            return Ok(self.checked);
        }
        Ok(count)
    }
}
