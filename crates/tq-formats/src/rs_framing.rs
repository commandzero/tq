//! Bounded record-separator scanning, independent of payload syntax.

use std::io::{self, BufRead, Read, Write};

use crate::{FormatError, InputFormat};

/// Writes one RS/LF frame without retaining its payload.
pub(crate) fn write_frame<W: Write, E: From<io::Error>>(
    writer: &mut W,
    payload: impl FnOnce(&mut W) -> Result<(), E>,
) -> Result<(), E> {
    writer.write_all(b"\x1e")?;
    payload(writer)?;
    writer.write_all(b"\n")?;
    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum RsPolicy {
    StrictToon,
    JsonRecovery,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum State {
    Preamble,
    Boundary,
    Segment,
    End,
    Limit,
}

pub(crate) struct RsFramer<R> {
    reader: R,
    policy: RsPolicy,
    state: State,
    next_index: u64,
    maximum_bytes: usize,
    consumed: usize,
    line: usize,
    column: usize,
}

impl<R: BufRead> RsFramer<R> {
    pub(crate) const fn new(reader: R, maximum_bytes: usize) -> Self {
        Self::with_policy(reader, maximum_bytes, RsPolicy::StrictToon)
    }

    pub(crate) const fn with_policy(reader: R, maximum_bytes: usize, policy: RsPolicy) -> Self {
        Self {
            reader,
            policy,
            state: State::Preamble,
            next_index: 0,
            maximum_bytes,
            consumed: 0,
            line: 1,
            column: 0,
        }
    }

    pub(crate) const fn exceeded(&self) -> bool {
        matches!(self.state, State::Limit)
    }

    pub(crate) const fn location(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    pub(crate) fn at_separator(&mut self) -> io::Result<bool> {
        Ok(self.reader.fill_buf()?.first() == Some(&0x1e))
    }

    fn consume(&mut self, count: usize) -> io::Result<()> {
        for byte in &self.reader.fill_buf()?[..count] {
            if *byte == b'\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
        self.reader.consume(count);
        Ok(())
    }

    /// Drains any failed payload to its recovery boundary, then enters a segment.
    pub(crate) fn begin_segment(&mut self) -> Result<Option<u64>, FormatError> {
        if self.exceeded() {
            return Err(FormatError::Resource("frame-bytes"));
        }
        if self.state == State::End {
            return Ok(None);
        }
        if self.state == State::Segment {
            if let Err(error) = io::copy(self, &mut io::sink()) {
                return Err(if self.exceeded() {
                    FormatError::Resource("frame-bytes")
                } else {
                    error.into()
                });
            }
            self.state = State::Boundary;
        }
        loop {
            let bytes = self.reader.fill_buf()?;
            if bytes.is_empty() {
                self.state = State::End;
                return Ok(None);
            }
            if self.state == State::Preamble && self.policy == RsPolicy::JsonRecovery {
                let count = bytes
                    .iter()
                    .position(|byte| *byte == 0x1e)
                    .unwrap_or(bytes.len());
                if count != 0 {
                    self.consume(count)?;
                    continue;
                }
            }
            if self.reader.fill_buf()?[0] != 0x1e {
                return Err(FormatError::Parse {
                    format: InputFormat::ToonSequence,
                    message: "record must begin with ASCII RS".to_owned(),
                });
            }
            self.consume(1)?;
            self.state = State::Boundary;
            let index = self.next_index;
            self.next_index = self.next_index.saturating_add(1);
            if self.policy == RsPolicy::JsonRecovery {
                let bytes = self.reader.fill_buf()?;
                if bytes.is_empty() || bytes[0] == 0x1e {
                    continue;
                }
            }
            self.state = State::Segment;
            self.consumed = 0;
            return Ok(Some(index));
        }
    }

    pub(crate) fn next_segment(&mut self) -> Result<Option<(u64, Vec<u8>)>, FormatError> {
        let Some(index) = self.begin_segment()? else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        if let Err(error) = self.read_to_end(&mut bytes) {
            return Err(if self.exceeded() {
                FormatError::Resource("frame-bytes")
            } else {
                error.into()
            });
        }
        Ok(Some((index, bytes)))
    }
}

impl<R: BufRead> Read for RsFramer<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.state != State::Segment {
            return Ok(0);
        }
        let bytes = self.reader.fill_buf()?;
        let boundary = bytes
            .iter()
            .take(output.len())
            .position(|byte| *byte == 0x1e)
            .unwrap_or(bytes.len().min(output.len()));
        if boundary == 0 {
            return Ok(0);
        }
        let remaining = self.maximum_bytes.saturating_sub(self.consumed);
        if remaining == 0 {
            self.state = State::Limit;
            return Err(io::Error::other(
                "input resource limit exceeded: frame-bytes",
            ));
        }
        let length = boundary.min(output.len()).min(remaining);
        output[..length].copy_from_slice(&bytes[..length]);
        self.consume(length)?;
        self.consumed += length;
        Ok(length)
    }
}

#[cfg(test)]
mod tests {
    use super::{RsFramer, RsPolicy};
    use std::io::{BufReader, Read};

    #[test]
    fn rs_framing_limits_are_terminal_at_every_chunk_boundary() {
        for capacity in 1..=16 {
            for policy in [RsPolicy::StrictToon, RsPolicy::JsonRecovery] {
                let reader =
                    BufReader::with_capacity(capacity, b"\x1ex\n\x1eabcd\x1ez\n".as_slice());
                let mut frames = RsFramer::with_policy(reader, 3, policy);
                assert_eq!(frames.next_segment().unwrap(), Some((0, b"x\n".to_vec())));
                assert!(matches!(
                    frames.next_segment(),
                    Err(crate::FormatError::Resource("frame-bytes"))
                ));
                assert!(matches!(
                    frames.begin_segment(),
                    Err(crate::FormatError::Resource("frame-bytes"))
                ));
            }
        }
    }

    #[test]
    fn rs_framing_strict_preamble_rejection_does_not_apply_to_json_recovery() {
        for capacity in 1..=16 {
            let bytes = b"preamble\x1ex\n".as_slice();
            let mut strict = RsFramer::new(BufReader::with_capacity(capacity, bytes), 16);
            assert!(matches!(
                strict.begin_segment(),
                Err(crate::FormatError::Parse { .. })
            ));
            let mut recovering = RsFramer::with_policy(
                BufReader::with_capacity(capacity, bytes),
                16,
                RsPolicy::JsonRecovery,
            );
            assert_eq!(
                recovering.next_segment().unwrap(),
                Some((0, b"x\n".to_vec()))
            );
            assert!(recovering.next_segment().unwrap().is_none());
        }
    }

    #[test]
    fn rs_framing_json_preamble_and_redundant_separators_are_chunk_invariant() {
        for capacity in 1..=16 {
            let reader =
                BufReader::with_capacity(capacity, b"ignored\x1e\x1e1\n\x1e2\n".as_slice());
            let mut frames = RsFramer::with_policy(reader, 16, RsPolicy::JsonRecovery);
            assert_eq!(frames.begin_segment().unwrap(), Some(1));
            let mut bytes = Vec::new();
            frames.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, b"1\n");
            assert_eq!(frames.begin_segment().unwrap(), Some(2));
            bytes.clear();
            frames.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, b"2\n");
            assert_eq!(frames.begin_segment().unwrap(), None);
        }
    }
}
