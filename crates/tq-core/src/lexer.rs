//! UTF-8 jq-token lexer with stable spans and deferred-family recognition.

use std::sync::Arc;

use crate::{Diagnostic, DiagnosticClass, SourceFile, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TokenKind {
    Dot,
    DotDot,
    Question,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Colon,
    Semicolon,
    Comma,
    Pipe,
    Assign,
    Update,
    AddUpdate,
    SubtractUpdate,
    MultiplyUpdate,
    DivideUpdate,
    AlternativeUpdate,
    Alternative,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    If,
    Then,
    Elif,
    Else,
    End,
    And,
    Or,
    Not,
    As,
    Try,
    Catch,
    Reduce,
    Foreach,
    True,
    False,
    Null,
    Identifier(Arc<str>),
    Variable(Arc<str>),
    String(Arc<str>),
    Number(Arc<str>),
    Deferred(Arc<str>),
    EndOfInput,
}

pub(crate) fn lex(source: &SourceFile) -> Result<Vec<Token>, Box<Diagnostic>> {
    Lexer {
        source,
        bytes: source.text().as_bytes(),
        index: 0,
        tokens: Vec::new(),
    }
    .run()
}

pub(crate) fn validate_utf8<'a>(
    bytes: &'a [u8],
    source_name: &str,
) -> Result<&'a str, Box<Diagnostic>> {
    std::str::from_utf8(bytes).map_err(|error| {
        let source = crate::SourceId::new(0);
        Box::new(
            Diagnostic::new(
                "TQ-LEX-UTF8-001",
                DiagnosticClass::Compile,
                format!("query {source_name:?} is not valid UTF-8"),
            )
            .at(
                Span::new(
                    source,
                    error.valid_up_to() as u64,
                    error.valid_up_to().saturating_add(1) as u64,
                ),
                "invalid UTF-8 byte",
            ),
        )
    })
}

struct Lexer<'a> {
    source: &'a SourceFile,
    bytes: &'a [u8],
    index: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn run(mut self) -> Result<Vec<Token>, Box<Diagnostic>> {
        while self.index < self.bytes.len() {
            match self.bytes[self.index] {
                byte if byte.is_ascii_whitespace() => self.index += 1,
                b'#' => self.comment(),
                b'.' => self.dot(),
                b'?' => self.single(TokenKind::Question),
                b'(' => self.single(TokenKind::LeftParen),
                b')' => self.single(TokenKind::RightParen),
                b'[' => self.single(TokenKind::LeftBracket),
                b']' => self.single(TokenKind::RightBracket),
                b'{' => self.single(TokenKind::LeftBrace),
                b'}' => self.single(TokenKind::RightBrace),
                b':' => self.single(TokenKind::Colon),
                b';' => self.single(TokenKind::Semicolon),
                b',' => self.single(TokenKind::Comma),
                b'|' => self.operator(b'=', TokenKind::Update, TokenKind::Pipe),
                b'+' => self.operator(b'=', TokenKind::AddUpdate, TokenKind::Plus),
                b'-' => self.operator(b'=', TokenKind::SubtractUpdate, TokenKind::Minus),
                b'*' => self.operator(b'=', TokenKind::MultiplyUpdate, TokenKind::Star),
                b'/' => self.slash(),
                b'%' => self.single(TokenKind::Percent),
                b'=' => self.operator(b'=', TokenKind::Equal, TokenKind::Assign),
                b'!' => self.required_operator(b'=', TokenKind::NotEqual)?,
                b'<' => self.operator(b'=', TokenKind::LessEqual, TokenKind::Less),
                b'>' => self.operator(b'=', TokenKind::GreaterEqual, TokenKind::Greater),
                b'"' => self.string()?,
                b'$' => self.variable()?,
                byte if byte.is_ascii_digit() => self.number()?,
                byte if identifier_start(byte) => self.identifier(),
                _ => return Err(self.error("TQ-LEX-TOKEN-001", "unexpected query character")),
            }
        }
        let offset = self.index as u64;
        self.tokens.push(Token {
            kind: TokenKind::EndOfInput,
            span: Span::new(self.source.id(), offset, offset),
        });
        Ok(self.tokens)
    }

    fn comment(&mut self) {
        while self.index < self.bytes.len() && self.bytes[self.index] != b'\n' {
            self.index += 1;
        }
    }

    fn dot(&mut self) {
        let start = self.index;
        self.index += 1;
        let kind = if self.take(b'.') {
            TokenKind::DotDot
        } else {
            TokenKind::Dot
        };
        self.push(start, kind);
    }

    fn slash(&mut self) {
        let start = self.index;
        self.index += 1;
        let kind = if self.take(b'/') {
            if self.take(b'=') {
                TokenKind::AlternativeUpdate
            } else {
                TokenKind::Alternative
            }
        } else if self.take(b'=') {
            TokenKind::DivideUpdate
        } else {
            TokenKind::Slash
        };
        self.push(start, kind);
    }

    fn operator(&mut self, second: u8, combined: TokenKind, single: TokenKind) {
        let start = self.index;
        self.index += 1;
        let kind = if self.take(second) { combined } else { single };
        self.push(start, kind);
    }

    fn required_operator(
        &mut self,
        second: u8,
        combined: TokenKind,
    ) -> Result<(), Box<Diagnostic>> {
        let start = self.index;
        self.index += 1;
        if !self.take(second) {
            return Err(self.error_at(
                "TQ-LEX-TOKEN-001",
                "expected '=' after '!'",
                start,
                self.index,
            ));
        }
        self.push(start, combined);
        Ok(())
    }

    fn single(&mut self, kind: TokenKind) {
        let start = self.index;
        self.index += 1;
        self.push(start, kind);
    }

    fn string(&mut self) -> Result<(), Box<Diagnostic>> {
        let start = self.index;
        self.index += 1;
        let mut escaped = false;
        while self.index < self.bytes.len() {
            let byte = self.bytes[self.index];
            self.index += 1;
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                let token = &self.source.text()[start..self.index];
                if token.contains("\\(") {
                    return Err(self.error_at(
                        "TQ-CAP-INTERPOLATION",
                        "string interpolation is deferred",
                        start,
                        self.index,
                    ));
                }
                let decoded: String = serde_json::from_str(token).map_err(|_| {
                    self.error_at(
                        "TQ-LEX-STRING-001",
                        "invalid string escape",
                        start,
                        self.index,
                    )
                })?;
                self.push(start, TokenKind::String(decoded.into()));
                return Ok(());
            } else if byte < 0x20 {
                return Err(self.error_at(
                    "TQ-LEX-STRING-001",
                    "unescaped control byte in string",
                    start,
                    self.index,
                ));
            }
        }
        Err(self.error_at(
            "TQ-LEX-STRING-001",
            "unterminated string",
            start,
            self.index,
        ))
    }

    fn variable(&mut self) -> Result<(), Box<Diagnostic>> {
        let start = self.index;
        self.index += 1;
        let name_start = self.index;
        if !self
            .bytes
            .get(self.index)
            .is_some_and(|byte| identifier_start(*byte))
        {
            return Err(self.error_at(
                "TQ-LEX-VARIABLE-001",
                "expected variable name after '$'",
                start,
                self.index,
            ));
        }
        self.index += 1;
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| identifier_continue(*byte))
        {
            self.index += 1;
        }
        let name: Arc<str> = self.source.text()[name_start..self.index].into();
        self.push(start, TokenKind::Variable(name));
        Ok(())
    }

    fn number(&mut self) -> Result<(), Box<Diagnostic>> {
        let start = self.index;
        if self.bytes[self.index] == b'0' {
            self.index += 1;
        } else {
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
        }
        if self.take(b'.') {
            let fraction = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if fraction == self.index {
                return Err(self.error_at(
                    "TQ-LEX-NUMBER-001",
                    "expected digits after decimal point",
                    start,
                    self.index,
                ));
            }
        }
        if matches!(self.bytes.get(self.index), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.bytes.get(self.index), Some(b'+' | b'-')) {
                self.index += 1;
            }
            let exponent = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if exponent == self.index {
                return Err(self.error_at(
                    "TQ-LEX-NUMBER-001",
                    "expected exponent digits",
                    start,
                    self.index,
                ));
            }
        }
        let value: Arc<str> = self.source.text()[start..self.index].into();
        self.push(start, TokenKind::Number(value));
        Ok(())
    }

    fn identifier(&mut self) {
        let start = self.index;
        self.index += 1;
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| identifier_continue(*byte))
        {
            self.index += 1;
        }
        let text = &self.source.text()[start..self.index];
        let kind = match text {
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "end" => TokenKind::End,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "as" => TokenKind::As,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "reduce" => TokenKind::Reduce,
            "foreach" => TokenKind::Foreach,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "def" => TokenKind::Deferred("function".into()),
            "module" => TokenKind::Deferred("modules".into()),
            "label" => TokenKind::Deferred("labels".into()),
            "import" | "include" | "break" => TokenKind::Deferred(text.into()),
            _ => TokenKind::Identifier(text.into()),
        };
        self.push(start, kind);
    }

    fn take(&mut self, byte: u8) -> bool {
        if self.bytes.get(self.index) == Some(&byte) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn push(&mut self, start: usize, kind: TokenKind) {
        self.tokens.push(Token {
            kind,
            span: Span::new(self.source.id(), start as u64, self.index as u64),
        });
    }

    fn error(&self, code: &str, message: &str) -> Box<Diagnostic> {
        self.error_at(code, message, self.index, self.index.saturating_add(1))
    }

    fn error_at(&self, code: &str, message: &str, start: usize, end: usize) -> Box<Diagnostic> {
        Box::new(Diagnostic::new(code, DiagnosticClass::Compile, message).at(
            Span::new(self.source.id(), start as u64, end as u64),
            message,
        ))
    }
}

const fn identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

const fn identifier_continue(byte: u8) -> bool {
    identifier_start(byte) || byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use crate::{SourceFile, SourceId};

    use super::{TokenKind, lex, validate_utf8};

    #[test]
    fn tokenizes_operators_strings_comments_and_spans() {
        let source = SourceFile::new(
            SourceId::new(7),
            "query",
            ".a //= \"x\\n\" # comment\n| $v?",
        );
        let tokens = lex(&source).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
        assert!(matches!(tokens[2].kind, TokenKind::AlternativeUpdate));
        assert_eq!(tokens[0].span.start, 0);
        assert!(matches!(&tokens[3].kind, TokenKind::String(value) if &**value == "x\n"));
        assert!(matches!(&tokens[5].kind, TokenKind::Variable(value) if &**value == "v"));
    }

    #[test]
    fn rejects_invalid_utf8_and_recognizes_fold_tokens() {
        assert!(validate_utf8(&[0xff], "query").is_err());
        let source = SourceFile::new(
            SourceId::new(0),
            "query",
            "reduce . as $x (0; .) foreach . as $x (0; .; .)",
        );
        let tokens = lex(&source).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Reduce));
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Foreach))
        );
    }

    #[test]
    fn golden_covers_every_mvp_token_family() {
        let source = SourceFile::new(
            SourceId::new(1),
            "query",
            ". .. ? ( ) [ ] { } : ; , | = |= += -= *= /= //= // + - * / % == != < <= > >= if then elif else end and or not as try catch true false null name $v \"s\" 12 def",
        );
        let tokens = lex(&source).unwrap();
        assert_eq!(tokens.len(), 52);
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
        assert!(matches!(tokens[1].kind, TokenKind::DotDot));
        assert!(matches!(tokens[14].kind, TokenKind::Update));
        assert!(matches!(tokens[19].kind, TokenKind::AlternativeUpdate));
        assert!(matches!(tokens[31].kind, TokenKind::GreaterEqual));
        assert!(matches!(&tokens[46].kind, TokenKind::Identifier(name) if &**name == "name"));
        assert!(matches!(&tokens[47].kind, TokenKind::Variable(name) if &**name == "v"));
        assert!(matches!(&tokens[48].kind, TokenKind::String(value) if &**value == "s"));
        assert!(matches!(&tokens[49].kind, TokenKind::Number(value) if &**value == "12"));
        assert!(matches!(&tokens[50].kind, TokenKind::Deferred(name) if &**name == "function"));
        assert!(matches!(tokens[51].kind, TokenKind::EndOfInput));
        assert!(
            tokens
                .windows(2)
                .all(|pair| pair[0].span.end <= pair[1].span.start)
        );
    }

    #[test]
    fn rejects_malformed_strings_numbers_variables_and_characters() {
        for query in ["\"unterminated", "1.", "$", "@"] {
            let source = SourceFile::new(SourceId::new(0), "query", query);
            assert!(lex(&source).is_err(), "{query}");
        }
    }
}
