use json5::Position;

#[derive(Debug)]
pub(crate) enum PreprocessError {
    Parse {
        offset: usize,
        message: &'static str,
    },
    Resource(&'static str),
}

#[derive(Debug)]
pub(crate) struct NormalizedJson5 {
    text: String,
    source_offsets: Vec<usize>,
    source_len: usize,
}

impl NormalizedJson5 {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn translate_error(&self, error: &json5::Error, source: &str) -> String {
        let Some(position) = error.position() else {
            return error.to_string();
        };
        let normalized_offset = offset_for_position(&self.text, position);
        let source_offset = self
            .source_offsets
            .get(normalized_offset)
            .copied()
            .unwrap_or(self.source_len);
        let source_position = Position::from_offset(source_offset.min(source.len()), source);
        let rendered = error.to_string();
        let message = rendered
            .rsplit_once(" at line ")
            .map_or(rendered.as_str(), |(message, _)| message);
        format!("{message} at {source_position}")
    }
}

struct Builder {
    text: String,
    source_offsets: Vec<usize>,
    source_len: usize,
}

impl Builder {
    fn new(source_len: usize) -> Self {
        Self {
            text: String::with_capacity(source_len),
            source_offsets: Vec::with_capacity(source_len),
            source_len,
        }
    }

    fn copy(&mut self, source: &str, start: usize, end: usize) {
        self.text.push_str(&source[start..end]);
        self.source_offsets.extend(start..end);
    }

    fn push(&mut self, value: &str, source_offset: usize) {
        self.text.push_str(value);
        self.source_offsets
            .extend(std::iter::repeat_n(source_offset, value.len()));
    }

    fn finish(self) -> NormalizedJson5 {
        NormalizedJson5 {
            text: self.text,
            source_offsets: self.source_offsets,
            source_len: self.source_len,
        }
    }
}

pub(crate) fn preprocess(
    source: &str,
    maximum_token_bytes: usize,
    maximum_depth: usize,
) -> Result<NormalizedJson5, PreprocessError> {
    let bytes = source.as_bytes();
    let mut output = Builder::new(bytes.len());
    let mut index = 0;
    let mut depth = 0usize;

    while index < bytes.len() {
        if bytes[index..].starts_with(b"//") {
            let end = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n' || *byte == b'\r')
                .map_or(bytes.len(), |offset| index + offset);
            output.copy(source, index, end);
            index = end;
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            let end = bytes[index + 2..]
                .windows(2)
                .position(|window| window == b"*/")
                .map_or(bytes.len(), |offset| index + 2 + offset + 2);
            output.copy(source, index, end);
            index = end;
            continue;
        }
        if bytes[index..].starts_with(b"\"\"\"") {
            normalize_triple_string(source, &mut index, maximum_token_bytes, &mut output)?;
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"') {
            copy_quoted_string(source, &mut index, maximum_token_bytes, &mut output)?;
            continue;
        }

        match bytes[index] {
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > maximum_depth {
                    return Err(PreprocessError::Resource("depth"));
                }
                output.copy(source, index, index + 1);
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                output.copy(source, index, index + 1);
                index += 1;
            }
            byte if is_delimiter(byte) => {
                output.copy(source, index, index + 1);
                index += 1;
            }
            _ => {
                let start = index;
                while index < bytes.len() && !is_delimiter(bytes[index]) {
                    index += 1;
                    check_token(index - start, maximum_token_bytes)?;
                }
                output.copy(source, start, index);
            }
        }
    }

    Ok(output.finish())
}

fn normalize_triple_string(
    source: &str,
    index: &mut usize,
    maximum_token_bytes: usize,
    output: &mut Builder,
) -> Result<(), PreprocessError> {
    let bytes = source.as_bytes();
    let start = *index;
    output.push("\"", *index);
    *index += 3;
    let mut token_bytes = 0usize;
    while *index < bytes.len() {
        if bytes[*index] == b'"' {
            let run_start = *index;
            while *index < bytes.len() && bytes[*index] == b'"' {
                *index += 1;
            }
            let quotes = *index - run_start;
            if quotes >= 3 {
                for quote in 0..quotes - 3 {
                    token_bytes = token_bytes.saturating_add(1);
                    check_token(token_bytes, maximum_token_bytes)?;
                    output.push("\\\"", run_start + quote);
                }
                output.push("\"", *index - 3);
                return Ok(());
            }
            for quote in 0..quotes {
                token_bytes = token_bytes.saturating_add(1);
                check_token(token_bytes, maximum_token_bytes)?;
                output.push("\\\"", run_start + quote);
            }
            continue;
        }

        let character = source[*index..].chars().next().expect("character boundary");
        let width = character.len_utf8();
        token_bytes = token_bytes.saturating_add(width);
        check_token(token_bytes, maximum_token_bytes)?;
        match character {
            '\\' => output.push("\\\\", *index),
            '\n' => output.push("\\n", *index),
            '\r' => output.push("\\r", *index),
            '\t' => output.push("\\t", *index),
            '\u{2028}' => output.push("\\u2028", *index),
            '\u{2029}' => output.push("\\u2029", *index),
            character if character.is_control() => {
                output.push(&format!("\\u{:04x}", u32::from(character)), *index);
            }
            character => output.push(&character.to_string(), *index),
        }
        *index += width;
    }
    Err(PreprocessError::Parse {
        offset: start,
        message: "unterminated triple-quoted string",
    })
}

fn copy_quoted_string(
    source: &str,
    index: &mut usize,
    maximum_token_bytes: usize,
    output: &mut Builder,
) -> Result<(), PreprocessError> {
    let bytes = source.as_bytes();
    let quote = bytes[*index];
    let start = *index;
    *index += 1;
    let mut escaped = false;
    while *index < bytes.len() {
        let byte = bytes[*index];
        *index += 1;
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == quote {
            break;
        }
        check_token(index.saturating_sub(start + 1), maximum_token_bytes)?;
    }
    output.copy(source, start, *index);
    Ok(())
}

fn check_token(bytes: usize, maximum: usize) -> Result<(), PreprocessError> {
    if bytes > maximum {
        Err(PreprocessError::Resource("token-bytes"))
    } else {
        Ok(())
    }
}

const fn is_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            b'{' | b'}' | b'[' | b']' | b',' | b':' | b'/' | b'\'' | b'"'
        )
}

fn offset_for_position(input: &str, target: Position) -> usize {
    let mut line = 0usize;
    let mut column = 0usize;
    let mut characters = input.char_indices().peekable();
    while let Some((offset, character)) = characters.next() {
        if line == target.line && column == target.column {
            return offset;
        }
        if matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            if character == '\r' && characters.peek().is_some_and(|(_, next)| *next == '\n') {
                characters.next();
            }
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    input.len()
}
