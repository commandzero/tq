//! Ordered, loss-aware format adapters and independent per-source probing.

use std::{
    fmt,
    io::{self, Cursor, Read},
    sync::Arc,
};

use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use tq_core::{Number, Object, SourceId, Value};
use tq_toon::{DecoderConfig, decode_to_value};

use crate::{Document, DocumentSource, FormatError, InputFormat, VecDeque};

/// Structured decode controls shared by CLI sources.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeOptions {
    /// Explicit parser or best-effort detection.
    pub format: InputFormat,
    /// Maximum bytes accepted for a document-at-a-time source.
    pub maximum_source_bytes: usize,
    /// TOON decoder controls.
    pub toon: DecoderConfig,
}

/// Observable bounded auto-detection decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeReport {
    /// Parser selected at the commitment point.
    pub selected: InputFormat,
    /// Prefix bytes inspected, bounded by the configured lookahead.
    pub lookahead_bytes: usize,
    /// Byte offset after which faildown is no longer permitted.
    pub commitment_bytes: usize,
    /// Earlier candidate rejections in probe order.
    pub rejections: Vec<(InputFormat, String)>,
}

/// Reader that replays the bounded detection prefix before continuing with the
/// untouched source.
#[derive(Debug)]
pub struct ReplayReader<R> {
    prefix: Cursor<Vec<u8>>,
    reader: R,
}

impl<R: Read> Read for ReplayReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let replayed = self.prefix.read(buffer)?;
        if replayed == 0 {
            self.reader.read(buffer)
        } else {
            Ok(replayed)
        }
    }
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            format: InputFormat::Auto,
            maximum_source_bytes: 2 * 1024 * 1024 * 1024,
            toon: DecoderConfig::default(),
        }
    }
}

/// In-memory pull source used by document-at-a-time adapters.
#[derive(Debug, Default)]
pub struct VecDocumentSource {
    documents: VecDeque<Document>,
}

impl VecDocumentSource {
    /// Wraps documents in pull order.
    #[must_use]
    pub fn new(documents: Vec<Document>) -> Self {
        Self {
            documents: documents.into(),
        }
    }
}

impl DocumentSource for VecDocumentSource {
    fn next_document(&mut self) -> Result<Option<Document>, FormatError> {
        Ok(self.documents.pop_front())
    }
}

/// Decodes one byte source using an override or TOON→YAML→JSON faildown.
///
/// Each call probes independently using a bounded prefix. Once probing commits,
/// later syntax failures belong to the selected format and do not restart
/// detection.
///
/// # Errors
///
/// Returns a selected-parser error, combined probe error, or resource failure.
pub fn decode_bytes(
    bytes: &[u8],
    identity: impl Into<String>,
    options: DecodeOptions,
) -> Result<Vec<Document>, FormatError> {
    if bytes.len() > options.maximum_source_bytes {
        return Err(FormatError::Resource("source-bytes"));
    }
    let identity = identity.into();
    match options.format {
        InputFormat::Toon => decode_toon(bytes, identity, options.toon),
        InputFormat::Yaml => decode_yaml(bytes, identity),
        InputFormat::Json => decode_json(bytes, identity),
        InputFormat::ToonSequence => decode_toon_sequence(bytes, identity, options.toon),
        InputFormat::Auto => {
            let report = probe_format(bytes, options.toon.maximum_lookahead_bytes)?;
            match report.selected {
                InputFormat::Toon => decode_toon(bytes, identity, options.toon),
                InputFormat::Yaml => decode_yaml(bytes, identity),
                InputFormat::Json => decode_json(bytes, identity),
                InputFormat::Auto | InputFormat::ToonSequence => unreachable!("probe candidate"),
            }
        }
    }
}

/// Selects a parser from a bounded prefix and records the commitment point.
///
/// # Errors
///
/// Returns combined bounded context when no candidate can safely inspect the
/// prefix, such as invalid UTF-8 shared by every structured parser.
pub fn probe_format(
    bytes: &[u8],
    maximum_lookahead_bytes: usize,
) -> Result<ProbeReport, FormatError> {
    let inspected = bytes.len().min(maximum_lookahead_bytes);
    let prefix = &bytes[..inspected];
    let text = match std::str::from_utf8(prefix) {
        Ok(text) => text,
        Err(error) if error.error_len().is_none() && inspected < bytes.len() => {
            std::str::from_utf8(&prefix[..error.valid_up_to()]).map_err(|_| FormatError::Probe {
                summary: "TOON, YAML, and JSON could not validate the lookahead prefix".to_owned(),
            })?
        }
        Err(error) => {
            return Err(FormatError::Probe {
                summary: format!(
                    "TOON: invalid UTF-8 at {}; YAML: invalid UTF-8; JSON: invalid UTF-8",
                    error.valid_up_to()
                ),
            });
        }
    };
    let trimmed = text.trim_start();
    let first_line = trimmed.lines().next().unwrap_or("");
    let toon_header = root_toon_array_header(first_line);
    let rejection = if trimmed.starts_with('{') {
        Some("JSON/YAML object opener is not canonical TOON".to_owned())
    } else if trimmed.starts_with('[') && !toon_header {
        Some("bracketed value is not a TOON counted-array header".to_owned())
    } else if trimmed.starts_with("---") || trimmed.starts_with('%') {
        Some("YAML document/directive marker".to_owned())
    } else if trimmed.starts_with("- ") {
        Some("root sequence marker requires YAML".to_owned())
    } else {
        None
    };
    let commitment = first_line.len().min(inspected);
    Ok(if let Some(rejection) = rejection {
        ProbeReport {
            selected: InputFormat::Yaml,
            lookahead_bytes: inspected,
            commitment_bytes: commitment,
            rejections: vec![(InputFormat::Toon, rejection)],
        }
    } else {
        ProbeReport {
            selected: InputFormat::Toon,
            lookahead_bytes: inspected,
            commitment_bytes: commitment,
            rejections: Vec::new(),
        }
    })
}

/// Reads bounded lookahead, returns its decision, and preserves every byte in a
/// replay reader for the selected parser.
///
/// Up to three continuation bytes beyond the configured lookahead are retained
/// solely to distinguish a split UTF-8 scalar from invalid input; the reported
/// inspected and commitment offsets remain within the configured bound.
///
/// # Errors
///
/// Returns lookahead I/O or the same combined rejection as [`probe_format`].
pub fn probe_reader<R: Read>(
    mut reader: R,
    maximum_lookahead_bytes: usize,
) -> Result<(ProbeReport, ReplayReader<R>), FormatError> {
    let capacity = maximum_lookahead_bytes.saturating_add(3);
    let mut prefix = Vec::with_capacity(capacity);
    let mut limited = (&mut reader).take(capacity as u64);
    limited.read_to_end(&mut prefix)?;
    let report = probe_format(&prefix, maximum_lookahead_bytes)?;
    Ok((
        report,
        ReplayReader {
            prefix: Cursor::new(prefix),
            reader,
        },
    ))
}

fn root_toon_array_header(line: &str) -> bool {
    let Some(close) = line.find(']') else {
        return false;
    };
    let declaration = &line[1..close];
    let count = declaration
        .strip_suffix([',', '|', '\t'])
        .unwrap_or(declaration);
    !count.is_empty()
        && count.bytes().all(|byte| byte.is_ascii_digit())
        && line[close + 1..].starts_with([':', '{'])
}

/// Decodes one strict TOON document.
///
/// # Errors
///
/// Returns the source-positioned TOON failure.
pub fn decode_toon(
    bytes: &[u8],
    identity: impl Into<String>,
    config: DecoderConfig,
) -> Result<Vec<Document>, FormatError> {
    let value = decode_to_value(Cursor::new(bytes), SourceId::new(0), config).map_err(|error| {
        FormatError::Parse {
            format: InputFormat::Toon,
            message: error.to_string(),
        }
    })?;
    Ok(vec![Document {
        value,
        identity: identity.into(),
        format: InputFormat::Toon,
        index: 0,
    }])
}

/// Decodes one ordered, arbitrary-precision-aware JSON document.
///
/// # Errors
///
/// Returns JSON syntax/trailing-content or numeric-envelope failures.
pub fn decode_json(
    bytes: &[u8],
    identity: impl Into<String>,
) -> Result<Vec<Document>, FormatError> {
    let json: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| FormatError::Parse {
            format: InputFormat::Json,
            message: error.to_string(),
        })?;
    let value = Value::from_json(json).map_err(|error| FormatError::Parse {
        format: InputFormat::Json,
        message: error.to_string(),
    })?;
    Ok(vec![Document {
        value,
        identity: identity.into(),
        format: InputFormat::Json,
        index: 0,
    }])
}

/// Decodes a YAML stream one document at a time through `yaml_serde`.
///
/// Mapping keys must deserialize as strings; duplicates and tags are rejected
/// by the custom runtime-value visitor. YAML floats enter the explicit binary64
/// arithmetic side of tq's hybrid number model, and non-finite values fail.
///
/// # Errors
///
/// Returns YAML syntax or tq profile failures.
pub fn decode_yaml(
    bytes: &[u8],
    identity: impl Into<String>,
) -> Result<Vec<Document>, FormatError> {
    let identity = identity.into();
    let mut documents = Vec::new();
    for (index, document) in yaml_serde::Deserializer::from_slice(bytes).enumerate() {
        let value = YamlRuntime::deserialize(document)
            .map_err(|error| FormatError::Parse {
                format: InputFormat::Yaml,
                message: error.to_string(),
            })?
            .0;
        documents.push(Document {
            value,
            identity: identity.clone(),
            format: InputFormat::Yaml,
            index: index as u64,
        });
    }
    Ok(documents)
}

/// Decodes an RS-prefix/LF-suffix TOON Text Sequence into ordered documents.
///
/// # Errors
///
/// Returns framing or per-record TOON failures.
pub fn decode_toon_sequence(
    bytes: &[u8],
    identity: impl Into<String>,
    config: DecoderConfig,
) -> Result<Vec<Document>, FormatError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.first() != Some(&0x1e) {
        return Err(FormatError::Parse {
            format: InputFormat::ToonSequence,
            message: "record must begin with ASCII RS".to_owned(),
        });
    }
    let identity = identity.into();
    let mut documents = Vec::new();
    for (index, record) in bytes[1..].split(|byte| *byte == 0x1e).enumerate() {
        let Some(record) = record.strip_suffix(b"\n") else {
            return Err(FormatError::Parse {
                format: InputFormat::ToonSequence,
                message: format!("record {index} is missing LF suffix"),
            });
        };
        let mut decoded = decode_toon(record, identity.clone(), config)?;
        let mut document = decoded.pop().ok_or_else(|| FormatError::Parse {
            format: InputFormat::ToonSequence,
            message: format!("record {index} produced no document"),
        })?;
        document.format = InputFormat::ToonSequence;
        document.index = index as u64;
        documents.push(document);
    }
    Ok(documents)
}

struct YamlRuntime(Value);

impl<'de> Deserialize<'de> for YamlRuntime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(YamlVisitor)
    }
}

struct YamlVisitor;

impl<'de> Visitor<'de> for YamlVisitor {
    type Value = YamlRuntime;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON-shaped YAML value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::Null))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::Null))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        number(value.to_string()).map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        number(value.to_string()).map_err(E::custom)
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(YamlRuntime)
            .map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::string(value)))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(YamlRuntime(Value::string(value)))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<YamlRuntime>()? {
            values.push(value.0);
        }
        Ok(YamlRuntime(Value::array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Object::new();
        while let Some(key) = map.next_key::<yaml_serde::Value>()? {
            let yaml_serde::Value::String(key) = key else {
                return Err(de::Error::custom("YAML mapping keys must be strings"));
            };
            let key: Arc<str> = key.into();
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate mapping key '{key}'")));
            }
            let value = map.next_value::<YamlRuntime>()?;
            values.insert(key, value.0);
        }
        Ok(YamlRuntime(Value::object(values)))
    }

    fn visit_enum<A>(self, _data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        Err(de::Error::custom("custom YAML tags are unsupported"))
    }
}

fn number<E: fmt::Display>(literal: E) -> Result<YamlRuntime, tq_core::NumberError> {
    Number::parse(&literal.to_string())
        .map(Value::Number)
        .map(YamlRuntime)
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use tq_toon::DecoderConfig;

    use super::{DecodeOptions, decode_bytes, decode_toon_sequence, decode_yaml};
    use crate::InputFormat;

    #[test]
    fn equivalent_formats_share_one_ordered_value_model() {
        let toon = decode_bytes(b"z: 1\na[2]: true,x", "t", DecodeOptions::default()).unwrap();
        let yaml = decode_yaml(b"z: 1\na: [true, x]", "y").unwrap();
        let json = decode_bytes(
            br#"{"z":1,"a":[true,"x"]}"#,
            "j",
            DecodeOptions {
                format: InputFormat::Json,
                ..DecodeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(toon[0].value, yaml[0].value);
        assert_eq!(yaml[0].value, json[0].value);
        assert_eq!(toon[0].format, InputFormat::Toon);
    }

    #[test]
    fn yaml_is_multi_document_and_rejects_profile_violations() {
        let documents = decode_yaml(b"---\na: 1\n---\na: 2\n", "stream").unwrap();
        assert_eq!(documents.len(), 2);
        assert!(decode_yaml(b"true: value", "bad-key").is_err());
        assert!(decode_yaml(b"a: 1\na: 2", "duplicate").is_err());
        assert!(decode_yaml(b"a: .nan", "non-finite").is_err());
        assert!(decode_yaml(b"a: !custom value", "tag").is_err());
    }

    #[test]
    fn sequence_framing_is_strict_and_ordered() {
        let documents = decode_toon_sequence(
            b"\x1ea: 1\n\x1eb: 2\n",
            "sequence",
            DecoderConfig::default(),
        )
        .unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[1].index, 1);
        assert!(decode_toon_sequence(b"a: 1\n", "bad", DecoderConfig::default()).is_err());
        assert!(decode_toon_sequence(b"\x1ea: 1", "bad", DecoderConfig::default()).is_err());
    }

    #[test]
    fn bounded_probe_is_observable_and_late_failures_do_not_fail_down() {
        let json = super::probe_format(br#"{"a":1}"#, 4).unwrap();
        assert_eq!(json.selected, InputFormat::Yaml);
        assert_eq!(json.lookahead_bytes, 4);
        assert_eq!(json.rejections[0].0, InputFormat::Toon);

        let error = decode_bytes(br#"{"a":"#, "late", DecodeOptions::default()).unwrap_err();
        assert!(matches!(
            error,
            crate::FormatError::Parse {
                format: InputFormat::Yaml,
                ..
            }
        ));
        assert!(super::probe_format(&[0xff], 64).is_err());

        let source = "😀: value\nrest: intact".as_bytes();
        let (report, mut replay) = super::probe_reader(source, 2).unwrap();
        assert!(report.lookahead_bytes <= 2);
        let mut recovered = Vec::new();
        replay.read_to_end(&mut recovered).unwrap();
        assert_eq!(recovered, source);
    }
}
