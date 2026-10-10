//! Untimed corpus materialization and value-aware semantic comparison.

use std::{
    collections::HashSet,
    fmt, fs,
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
};

use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeOwned, DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

use super::{ArtifactIdentity, GeneratedArtifacts, encode_hex};

const SEMANTIC_POLICY_VERSION: &str = "tq-semantic-equivalence-v5-format-specific-header-order";

/// Kind of semantic JSON-model divergence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DifferenceKind {
    /// JSON-model types differ.
    Type,
    /// Primitive values differ.
    Value,
    /// Numeric literals no longer retain the same exact value.
    NumericFidelity,
    /// Array lengths differ.
    Length,
    /// Object key set differs.
    ObjectOrder,
}

/// First semantic difference between two JSON-model values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDifference {
    /// RFC 6901-style path to the difference, with empty string for root.
    pub path: String,
    /// Difference classification.
    pub kind: DifferenceKind,
    /// Compact expected observation.
    pub expected: String,
    /// Compact actual observation.
    pub actual: String,
}

/// Stable cross-format generation and validation failures.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Source or destination I/O failed.
    #[error("cross-format I/O failed: {0}")]
    Io(#[from] io::Error),
    /// Source JSON could not be decoded without numeric loss.
    #[error("source JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    /// YAML generation or decoding failed.
    #[error("YAML failed: {0}")]
    Yaml(String),
    /// TOON generation or decoding failed.
    #[error("TOON failed: {0}")]
    Toon(String),
    /// A generated representation changed ordered JSON semantics.
    #[error("{format} semantic difference at {difference:?}")]
    Semantic {
        /// Generated format name.
        format: String,
        /// First exact difference.
        difference: SemanticDifference,
    },
    /// A verified temporary output could not be installed.
    #[error("could not atomically install generated artifact: {0}")]
    Persist(#[from] tempfile::PersistError),
    /// The external tq corpus generator failed.
    #[error("tq corpus conversion failed for {format}: {message}")]
    Tq {
        /// Input or output format involved in the failed command.
        format: String,
        /// Exit status and bounded stderr text.
        message: String,
    },
    /// tq produced a representation with different value-aware semantics.
    #[error("tq-generated {format} changed corpus semantics: expected {expected}, got {actual}")]
    TqSemantic {
        /// Generated representation format.
        format: String,
        /// Semantic digest of the natural source result stream.
        expected: String,
        /// Semantic digest after decoding the representation.
        actual: String,
    },
}

/// Generates YAML and TOON with the selected tq executable, then validates
/// both through bounded-memory canonical JSON streams.
///
/// # Errors
///
/// Returns process, I/O, semantic, or atomic-install errors.
pub fn generate_representations_with_tq(
    tq: &Path,
    source_json: &Path,
    yaml_output: &Path,
    toon_output: &Path,
    yaml_manifest_path: &str,
    toon_manifest_path: &str,
) -> Result<GeneratedArtifacts, ConversionError> {
    let toon = tq_generate(tq, source_json, "toon", toon_output, toon_manifest_path)?;
    let yaml = tq_generate(tq, source_json, "yaml", yaml_output, yaml_manifest_path)?;
    validate_generated_representations_with_tq(tq, source_json, yaml_output, toon_output)?;
    Ok(GeneratedArtifacts { yaml, toon })
}

/// Validates existing YAML and TOON with bounded-memory tq canonical streams
/// and calculates their raw artifact identities.
///
/// # Errors
///
/// Returns process, I/O, semantic, or hashing errors.
pub fn finalize_generated_representations_with_tq(
    tq: &Path,
    source_json: &Path,
    yaml_input: &Path,
    toon_input: &Path,
    yaml_manifest_path: &str,
    toon_manifest_path: &str,
) -> Result<GeneratedArtifacts, ConversionError> {
    validate_generated_representations_with_tq(tq, source_json, yaml_input, toon_input)?;
    Ok(GeneratedArtifacts {
        yaml: identify_existing(yaml_input, yaml_manifest_path)?,
        toon: identify_existing(toon_input, toon_manifest_path)?,
    })
}

/// Compares tq's canonical JSON result streams using a value-aware digest:
/// Object members, arrays, and result sequences stay in encounter order, except
/// TOON table-value fields may follow their recursive header order. Numeric
/// tokens use exact decimal normalization without binary64 rounding.
///
/// # Errors
///
/// Returns process, I/O, or semantic-stream mismatch errors.
pub fn validate_generated_representations_with_tq(
    tq: &Path,
    source_json: &Path,
    yaml_input: &Path,
    toon_input: &Path,
) -> Result<(), ConversionError> {
    for (format, input) in [("yaml", yaml_input), ("toon", toon_input)] {
        let normalize_headers = format == "toon";
        let source = tq_canonical_digest(tq, source_json, "json", normalize_headers)?;
        let actual = tq_canonical_digest(tq, input, format, normalize_headers)?;
        if actual != source {
            return Err(ConversionError::TqSemantic {
                format: format.to_owned(),
                expected: source.clone(),
                actual,
            });
        }
    }
    Ok(())
}

/// Validates a prepared source and its generated representations, reusing an
/// evidence record only when all artifact identities, candidate binary bytes,
/// and this semantic policy version match exactly.
///
/// Artifact bytes are expected to have been checked by the frozen-snapshot
/// verification cache before this function is called. The semantic cache is
/// deliberately separate from that byte-integrity cache: a historical manifest
/// from another tq binary or policy is never accepted as fresh evidence.
///
/// # Errors
///
/// Returns artifact I/O, validator execution, cache persistence, or semantic
/// equivalence failures.
pub fn validate_generated_representations_cached(
    cache_root: &Path,
    tq: &Path,
    source_json: &Path,
    yaml_input: &Path,
    toon_input: &Path,
    source_identity: &ArtifactIdentity,
    generated: &GeneratedArtifacts,
) -> Result<(), ConversionError> {
    let expected = semantic_validation_entry(tq, source_identity, generated)?;
    let mut cache = read_semantic_cache(cache_root);
    if cache.entries.iter().any(|entry| entry == &expected) {
        return Ok(());
    }

    validate_generated_representations_with_tq(tq, source_json, yaml_input, toon_input)?;
    cache
        .entries
        .retain(|entry| entry.policy_version == SEMANTIC_POLICY_VERSION);
    cache.entries.push(expected);
    write_semantic_cache(cache_root, &cache)?;
    Ok(())
}

/// Records evidence from a just-completed native generation or finalization.
///
/// Call only after `generate_representations_with_tq` or
/// `finalize_generated_representations_with_tq` has validated these artifacts.
///
/// # Errors
///
/// Returns validator identity or cache persistence errors.
pub fn remember_generated_validation(
    cache_root: &Path,
    tq: &Path,
    source: &ArtifactIdentity,
    generated: &GeneratedArtifacts,
) -> Result<(), ConversionError> {
    let expected = semantic_validation_entry(tq, source, generated)?;
    let mut cache = read_semantic_cache(cache_root);
    cache
        .entries
        .retain(|entry| entry.policy_version == SEMANTIC_POLICY_VERSION);
    if !cache.entries.contains(&expected) {
        cache.entries.push(expected);
    }
    write_semantic_cache(cache_root, &cache)
}

fn semantic_validation_entry(
    tq: &Path,
    source: &ArtifactIdentity,
    generated: &GeneratedArtifacts,
) -> Result<SemanticValidationEntry, ConversionError> {
    Ok(SemanticValidationEntry {
        policy_version: SEMANTIC_POLICY_VERSION.to_owned(),
        source: source.clone(),
        yaml: generated.yaml.clone(),
        toon: generated.toon.clone(),
        tq: identify_existing(tq, &tq.to_string_lossy())?,
    })
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct SemanticValidationCache {
    schema_version: u32,
    entries: Vec<SemanticValidationEntry>,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct SemanticValidationEntry {
    policy_version: String,
    source: ArtifactIdentity,
    yaml: ArtifactIdentity,
    toon: ArtifactIdentity,
    tq: ArtifactIdentity,
}

fn semantic_cache_path(root: &Path) -> std::path::PathBuf {
    root.join("semantic-validation-cache-v1.json")
}

fn read_semantic_cache(root: &Path) -> SemanticValidationCache {
    let Ok(file) = fs::File::open(semantic_cache_path(root)) else {
        return SemanticValidationCache {
            schema_version: 1,
            entries: Vec::new(),
        };
    };
    let Ok(cache) = serde_json::from_reader::<_, SemanticValidationCache>(file) else {
        return SemanticValidationCache {
            schema_version: 1,
            entries: Vec::new(),
        };
    };
    if cache.schema_version == 1 {
        cache
    } else {
        SemanticValidationCache {
            schema_version: 1,
            entries: Vec::new(),
        }
    }
}

fn write_semantic_cache(
    root: &Path,
    cache: &SemanticValidationCache,
) -> Result<(), ConversionError> {
    fs::create_dir_all(root)?;
    let mut temporary = NamedTempFile::new_in(root)?;
    serde_json::to_writer_pretty(&mut temporary, cache)?;
    temporary.write_all(b"\n")?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(semantic_cache_path(root))?;
    Ok(())
}
fn tq_generate(
    tq: &Path,
    source_json: &Path,
    format: &str,
    destination: &Path,
    manifest_path: &str,
) -> Result<ArtifactIdentity, ConversionError> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    let stderr = NamedTempFile::new()?;

    let mut command = Command::new(tq);
    // JSON is the lossless YAML 1.2 profile for natural sources whose decimal
    // values cannot survive the YAML library's public number model exactly.
    // Keeping this representation compact also avoids doubling large corpora.
    let output_format = if format == "yaml" { "json" } else { format };
    command
        .arg("-i")
        .arg("json")
        .arg("-o")
        .arg(output_format)
        .arg("--max-input-bytes")
        .arg(fs::metadata(source_json)?.len().to_string());
    if format == "toon" {
        command.arg("--unframed");
    } else if format == "yaml" {
        command.arg("--compact-output");
    }
    let mut child = command
        .arg(".")
        .arg(source_json)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr.reopen()?))
        .spawn()
        .map_err(|error| ConversionError::Tq {
            format: format.to_owned(),
            message: error.to_string(),
        })?;
    let mut stdout = child.stdout.take().ok_or_else(|| ConversionError::Tq {
        format: format.to_owned(),
        message: "tq stdout was not captured".to_owned(),
    })?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = vec![0_u8; 1024 * 1024].into_boxed_slice();
    loop {
        let read = stdout.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        temporary.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        bytes =
            bytes
                .checked_add(u64::try_from(read).map_err(|_| {
                    io::Error::other("generated artifact length does not fit in u64")
                })?)
                .ok_or_else(|| io::Error::other("generated artifact length overflow"))?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(ConversionError::Tq {
            format: format.to_owned(),
            message: tq_failure(status.code(), stderr.path())?,
        });
    }
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(destination)?;
    Ok(ArtifactIdentity {
        path: manifest_path.to_owned(),
        bytes,
        sha256: encode_hex(&hasher.finalize()),
    })
}

fn tq_canonical_digest(
    tq: &Path,
    input: &Path,
    input_format: &str,
    normalize_toon_headers: bool,
) -> Result<String, ConversionError> {
    let stderr = NamedTempFile::new()?;
    let input_bytes = fs::metadata(input)?.len();
    let mut child = Command::new(tq)
        .arg("-i")
        .arg(input_format)
        .arg("-o")
        .arg("json")
        .arg("-c")
        .arg("--max-input-bytes")
        .arg(input_bytes.to_string())
        .arg(".")
        .arg(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr.reopen()?))
        .spawn()
        .map_err(|error| ConversionError::Tq {
            format: input_format.to_owned(),
            message: error.to_string(),
        })?;
    let stdout = child.stdout.take().ok_or_else(|| ConversionError::Tq {
        format: input_format.to_owned(),
        message: "tq stdout was not captured".to_owned(),
    })?;
    let digest = stream_semantic_digest_mode(stdout, normalize_toon_headers);
    if digest.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(ConversionError::Tq {
            format: input_format.to_owned(),
            message: tq_failure(status.code(), stderr.path())?,
        });
    }
    let digest = digest.map_err(|error| ConversionError::Tq {
        format: input_format.to_owned(),
        message: format!("invalid canonical JSON stream: {error}"),
    })?;
    Ok(encode_hex(&digest))
}

/// Reduces each JSON value to a digest while consuming result streams in order.
/// Only TOON streams may use the narrow table/keyed-header normalization.
fn stream_semantic_digest<R: Read>(reader: R) -> Result<[u8; 32], io::Error> {
    stream_semantic_digest_impl::<_, ValueDigest>(reader)
}

fn stream_semantic_digest_mode<R: Read>(
    reader: R,
    toon_headers: bool,
) -> Result<[u8; 32], io::Error> {
    if toon_headers {
        stream_semantic_digest_impl::<_, ToonValueDigest>(reader)
    } else {
        stream_semantic_digest(reader)
    }
}

trait HashedValue {
    fn digest(&self) -> &[u8; 32];
}

impl HashedValue for ValueDigest {
    fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

impl HashedValue for ToonValueDigest {
    fn digest(&self) -> &[u8; 32] {
        &self.0.digest
    }
}

fn stream_semantic_digest_impl<R: Read, T: DeserializeOwned + HashedValue>(
    reader: R,
) -> Result<[u8; 32], io::Error> {
    let mut hasher = Sha256::new();
    hasher.update(b"results\0");
    let mut count = 0_u64;
    for result in serde_json::Deserializer::from_reader(reader).into_iter::<T>() {
        let value = result.map_err(|error| invalid_json(error.to_string()))?;
        hasher.update(value.digest());
        count = count
            .checked_add(1)
            .ok_or_else(|| invalid_json("result count overflow"))?;
    }
    if count == 0 {
        return Err(invalid_json("canonical stream is empty"));
    }
    hasher.update(count.to_be_bytes());
    Ok(hasher.finalize().into())
}

const SERDE_JSON_NUMBER_TOKEN: &str = "$serde_json::private::Number";
const MAX_VALUE_FIELDS: usize = 65_536;
const MAX_VALUE_SCHEMA_BYTES: usize = 16 * 1024 * 1024;

struct ValueDigest {
    digest: [u8; 32],
    header_digest: [u8; 32],
    keyed_digest: Option<[u8; 32]>,
    shape: ValueShape,
    retained_schema_bytes: usize,
}

struct ToonValueDigest(ValueDigest);

#[derive(Clone, Debug, Eq, PartialEq)]
enum ValueShape {
    Primitive,
    Array,
    Object(Option<RowSchema>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RowSchema(Vec<SchemaField>);

#[derive(Clone, Debug, Eq, PartialEq)]
struct SchemaField {
    key: Arc<str>,
    nested: Option<Box<RowSchema>>,
}

impl<'de> Deserialize<'de> for ValueDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ValueDigestSeed {
            toon_headers: false,
            allow_layout: false,
        }
        .deserialize(deserializer)
    }
}

impl<'de> Deserialize<'de> for ToonValueDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ValueDigestSeed {
            toon_headers: true,
            allow_layout: true,
        }
        .deserialize(deserializer)
        .map(Self)
    }
}

struct ValueDigestSeed {
    toon_headers: bool,
    allow_layout: bool,
}

impl<'de> DeserializeSeed<'de> for ValueDigestSeed {
    type Value = ValueDigest;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(ValueDigestVisitor {
            toon_headers: self.toon_headers,
            allow_layout: self.allow_layout,
        })
    }
}

struct ValueDigestVisitor {
    toon_headers: bool,
    allow_layout: bool,
}

impl ValueDigestVisitor {
    fn scalar(digest: [u8; 32]) -> ValueDigest {
        ValueDigest {
            digest,
            header_digest: digest,
            keyed_digest: None,
            shape: ValueShape::Primitive,
            retained_schema_bytes: 0,
        }
    }

    fn number<E: de::Error>(text: &str) -> Result<ValueDigest, E> {
        let canonical = crate::benchmark::canonical_number(text).map_err(E::custom)?;
        Ok(Self::scalar(hash_scalar(b"number", canonical.as_bytes())))
    }
    fn ordered_map<'de, A: MapAccess<'de>>(mut map: A) -> Result<ValueDigest, A::Error> {
        let mut hasher = Sha256::new();
        hasher.update(b"object\0");
        let mut keys = HashSet::<String>::new();
        let mut name_bytes = 0_usize;
        while let Some(key) = map.next_key::<DigestKey>()? {
            if key.synthetic_number {
                let literal = map.next_value::<String>()?;
                return Self::number(&literal);
            }
            if keys.contains(&key.text) {
                return Err(A::Error::custom("duplicate object key"));
            }
            if keys.len() == MAX_VALUE_FIELDS {
                return Err(A::Error::custom(
                    "schema resource limit: too many object fields",
                ));
            }
            name_bytes = name_bytes
                .checked_add(key.text.len())
                .ok_or_else(|| A::Error::custom("schema resource limit: key-byte overflow"))?;
            if name_bytes > MAX_VALUE_SCHEMA_BYTES {
                return Err(A::Error::custom(
                    "schema resource limit: object keys exceed byte limit",
                ));
            }
            let value = map.next_value_seed(ValueDigestSeed {
                toon_headers: false,
                allow_layout: false,
            })?;
            hasher.update((key.text.len() as u64).to_be_bytes());
            hasher.update(key.text.as_bytes());
            hasher.update(value.digest);
            keys.insert(key.text);
        }
        let digest = hasher.finalize().into();
        Ok(ValueDigest {
            digest,
            header_digest: digest,
            keyed_digest: None,
            shape: ValueShape::Object(None),
            retained_schema_bytes: 0,
        })
    }
    fn toon_map<'de, A: MapAccess<'de>>(
        mut map: A,
        allow_layout: bool,
        toon_headers: bool,
    ) -> Result<ValueDigest, A::Error> {
        let mut ordered = Sha256::new();
        ordered.update(b"object\0");
        let mut entries = Vec::<(Arc<str>, ValueDigest)>::new();
        let mut keys = HashSet::<Arc<str>>::new();
        let mut name_bytes = 0_usize;
        let mut retained = 0_usize;
        while let Some(key) = map.next_key::<DigestKey>()? {
            // serde_json's arbitrary_precision numbers arrive as a synthetic
            // one-entry map. Real objects with this key remain ordinary maps.
            if key.synthetic_number {
                let literal = map.next_value::<String>()?;
                return Self::number(&literal);
            }
            if keys.contains(key.text.as_str()) {
                return Err(A::Error::custom("duplicate object key"));
            }
            if keys.len() == MAX_VALUE_FIELDS {
                return Err(A::Error::custom(
                    "schema resource limit: too many object fields",
                ));
            }
            name_bytes = name_bytes
                .checked_add(key.text.len())
                .ok_or_else(|| A::Error::custom("schema resource limit: key-byte overflow"))?;
            if name_bytes > MAX_VALUE_SCHEMA_BYTES {
                return Err(A::Error::custom(
                    "schema resource limit: object keys exceed byte limit",
                ));
            }
            let key: Arc<str> = Arc::from(key.text);
            let value = map.next_value_seed(ValueDigestSeed {
                toon_headers,
                allow_layout: true,
            })?;
            retained = Self::retain_entry::<A::Error>(retained, &key, &value)?;
            keys.insert(Arc::clone(&key));
            ordered.update((key.len() as u64).to_be_bytes());
            ordered.update(key.as_bytes());
            ordered.update(value.digest);
            entries.push((key, value));
        }
        Self::finish_map(&entries, ordered, retained, toon_headers, allow_layout)
    }

    fn retain_entry<E: de::Error>(
        retained: usize,
        key: &Arc<str>,
        value: &ValueDigest,
    ) -> Result<usize, E> {
        let next_retained = retained
            .checked_add(std::mem::size_of::<(Arc<str>, ValueDigest)>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Arc<str>>()))
            .and_then(|bytes| bytes.checked_add(key.len()))
            .and_then(|bytes| bytes.checked_add(value.retained_schema_bytes))
            .ok_or_else(|| E::custom("schema resource limit: byte-count overflow"))?;
        if next_retained > MAX_VALUE_SCHEMA_BYTES {
            return Err(E::custom(
                "schema resource limit: retained object schema exceeds byte limit",
            ));
        }
        Ok(next_retained)
    }

    fn finish_map<E: de::Error>(
        entries: &[(Arc<str>, ValueDigest)],
        ordered: Sha256,
        retained: usize,
        toon_headers: bool,
        allow_layout: bool,
    ) -> Result<ValueDigest, E> {
        let schema_valid = !entries.is_empty()
            && entries.iter().all(|(_, value)| match &value.shape {
                ValueShape::Primitive | ValueShape::Object(Some(_)) => true,
                ValueShape::Object(None) | ValueShape::Array => false,
            });
        let schema_bytes = if schema_valid {
            schema_fields_bytes(entries)
                .ok_or_else(|| E::custom("schema resource limit: schema-size overflow"))?
        } else {
            0
        };
        let sorted_bytes = entries
            .len()
            .checked_mul(std::mem::size_of::<&(Arc<str>, ValueDigest)>())
            .ok_or_else(|| E::custom("schema resource limit: index-size overflow"))?;
        let retained = retained
            .checked_add(schema_bytes)
            .and_then(|bytes| bytes.checked_add(sorted_bytes))
            .ok_or_else(|| E::custom("schema resource limit: byte-count overflow"))?;
        if retained > MAX_VALUE_SCHEMA_BYTES {
            return Err(E::custom(
                "schema resource limit: recursive schema exceeds byte limit",
            ));
        }

        let mut sorted = entries.iter().collect::<Vec<_>>();
        sorted.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        let mut header = Sha256::new();
        header.update(b"object\0");
        let mut schema_fields = schema_valid.then(|| Vec::with_capacity(entries.len()));
        for (key, value) in &sorted {
            header.update((key.len() as u64).to_be_bytes());
            header.update(key.as_bytes());
            header.update(header_field_digest(value));
            if let Some(schema_fields) = &mut schema_fields {
                let nested = match &value.shape {
                    ValueShape::Primitive => None,
                    ValueShape::Object(Some(schema)) => Some(Box::new(schema.clone())),
                    ValueShape::Object(None) | ValueShape::Array => unreachable!(),
                };
                schema_fields.push(SchemaField {
                    key: Arc::clone(key),
                    nested,
                });
            }
        }
        let header_digest = header.finalize().into();
        let object_schema = schema_fields.map(RowSchema);
        let keyed_digest = if toon_headers && allow_layout && entries.len() >= 2 {
            let first_schema = match &entries[0].1.shape {
                ValueShape::Object(Some(schema)) => Some(schema),
                _ => None,
            };
            first_schema
                .filter(|schema| {
                    entries.iter().all(|(_, value)| {
                        matches!(&value.shape, ValueShape::Object(Some(other)) if other == *schema)
                    })
                })
                .map(|_| {
                    let mut keyed = Sha256::new();
                    keyed.update(b"object\0");
                    for (key, value) in entries {
                        keyed.update((key.len() as u64).to_be_bytes());
                        keyed.update(key.as_bytes());
                        keyed.update(value.header_digest);
                    }
                    keyed.finalize().into()
                })
        } else {
            None
        };
        let ordered = ordered.finalize().into();
        Ok(ValueDigest {
            digest: keyed_digest.unwrap_or(ordered),
            header_digest,
            keyed_digest,
            shape: ValueShape::Object(object_schema),
            retained_schema_bytes: schema_bytes,
        })
    }
}

fn header_field_digest(value: &ValueDigest) -> &[u8; 32] {
    value
        .keyed_digest
        .as_ref()
        .unwrap_or(if matches!(&value.shape, ValueShape::Object(_)) {
            &value.header_digest
        } else {
            &value.digest
        })
}

impl<'de> Visitor<'de> for ValueDigestVisitor {
    type Value = ValueDigest;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Self::scalar(hash_scalar(b"null", b"")))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Self::scalar(hash_scalar(
            b"bool",
            if value {
                b"true".as_slice()
            } else {
                b"false".as_slice()
            },
        )))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Self::scalar(hash_scalar(b"string", value.as_bytes())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.visit_str(&value)
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Self::number(&value.to_string())
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Self::number(&value.to_string())
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        if !self.toon_headers {
            return Self::ordered_map(map);
        }
        Self::toon_map(map, self.allow_layout, self.toon_headers)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut ordered = Sha256::new();
        ordered.update(b"array\0");
        let mut candidate = Sha256::new();
        candidate.update(b"array\0");
        let mut row_schema: Option<RowSchema> = None;
        let mut candidate_valid = self.toon_headers && self.allow_layout;
        let mut count = 0_u64;
        while let Some(value) = sequence.next_element_seed(ValueDigestSeed {
            toon_headers: self.toon_headers,
            allow_layout: false,
        })? {
            ordered.update(value.digest);
            if candidate_valid {
                let ValueShape::Object(Some(schema)) = &value.shape else {
                    candidate_valid = false;
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| A::Error::custom("array length overflow"))?;
                    continue;
                };
                if let Some(expected) = &row_schema {
                    if expected != schema {
                        candidate_valid = false;
                    }
                } else {
                    let size = row_schema_bytes(schema).ok_or_else(|| {
                        A::Error::custom("schema resource limit: schema-size overflow")
                    })?;
                    if size > MAX_VALUE_SCHEMA_BYTES {
                        return Err(A::Error::custom(
                            "schema resource limit: table row schema exceeds byte limit",
                        ));
                    }
                    row_schema = Some(schema.clone());
                }
                if candidate_valid {
                    candidate.update(value.header_digest);
                }
            }
            count = count
                .checked_add(1)
                .ok_or_else(|| A::Error::custom("array length overflow"))?;
        }
        ordered.update(count.to_be_bytes());
        candidate.update(count.to_be_bytes());
        let ordered = ordered.finalize().into();
        let digest = if candidate_valid && count != 0 {
            candidate.finalize().into()
        } else {
            ordered
        };
        Ok(ValueDigest {
            digest,
            header_digest: digest,
            keyed_digest: None,
            shape: ValueShape::Array,
            retained_schema_bytes: 0,
        })
    }
}

fn schema_fields_bytes(entries: &[(Arc<str>, ValueDigest)]) -> Option<usize> {
    entries.iter().try_fold(0_usize, |total, (key, value)| {
        let nested_bytes = match &value.shape {
            ValueShape::Primitive => Some(0),
            ValueShape::Object(Some(schema)) => row_schema_bytes(schema),
            ValueShape::Object(None) | ValueShape::Array => return None,
        }?;
        total
            .checked_add(std::mem::size_of::<SchemaField>())
            .and_then(|bytes| bytes.checked_add(key.len()))
            .and_then(|bytes| bytes.checked_add(nested_bytes))
    })
}

fn row_schema_bytes(schema: &RowSchema) -> Option<usize> {
    schema.0.iter().try_fold(0_usize, |total, field| {
        total
            .checked_add(std::mem::size_of::<SchemaField>())
            .and_then(|bytes| bytes.checked_add(field.key.len()))
            .and_then(|bytes| {
                field.nested.as_deref().map_or(Some(bytes), |nested| {
                    row_schema_bytes(nested)?.checked_add(bytes)
                })
            })
    })
}

struct DigestKey {
    text: String,
    synthetic_number: bool,
}

impl<'de> Deserialize<'de> for DigestKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_identifier(DigestKeyVisitor)
    }
}

struct DigestKeyVisitor;

impl<'de> Visitor<'de> for DigestKeyVisitor {
    type Value = DigestKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object key")
    }

    fn visit_borrowed_str<E>(self, text: &'de str) -> Result<Self::Value, E> {
        Ok(DigestKey {
            synthetic_number: text == SERDE_JSON_NUMBER_TOKEN,
            text: text.to_owned(),
        })
    }

    fn visit_str<E>(self, text: &str) -> Result<Self::Value, E> {
        Ok(DigestKey {
            text: text.to_owned(),
            synthetic_number: false,
        })
    }

    fn visit_string<E>(self, text: String) -> Result<Self::Value, E> {
        Ok(DigestKey {
            text,
            synthetic_number: false,
        })
    }
}

fn hash_scalar(tag: &[u8], value: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(tag);
    hasher.update([0]);
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
    hasher.finalize().into()
}

fn invalid_json(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn tq_failure(status: Option<i32>, stderr: &Path) -> Result<String, io::Error> {
    let mut message = fs::read_to_string(stderr)?;
    if message.len() > 4096 {
        let mut boundary = 4096;
        while !message.is_char_boundary(boundary) {
            boundary = boundary.saturating_sub(1);
        }
        message.truncate(boundary);
    }
    let message = message.trim();
    Ok(if message.is_empty() {
        format!(
            "exit status {}",
            status.map_or_else(|| "signal".to_owned(), |value| value.to_string())
        )
    } else {
        format!(
            "exit status {}: {message}",
            status.map_or_else(|| "signal".to_owned(), |value| value.to_string())
        )
    })
}

/// Generates YAML and TOON representations from natural source JSON.
///
/// Both representations are fully prepared before either atomic output write;
/// callers invoke this outside benchmark timing.
///
/// # Errors
///
/// Returns a JSON, YAML, TOON, or filesystem error.
pub fn generate_representations(
    source_json: &Path,
    yaml_output: &Path,
    toon_output: &Path,
    yaml_manifest_path: &str,
    toon_manifest_path: &str,
) -> Result<GeneratedArtifacts, ConversionError> {
    let source: Value = serde_json::from_reader(fs::File::open(source_json)?)?;
    // Serializing `serde_json::Value` directly is not interoperable when
    // serde_json's `arbitrary_precision` feature is enabled: its private
    // number marker is serialized as a YAML mapping. Convert into the YAML
    // library's own value model first so external readers observe scalars.
    let yaml = match json_to_yaml(&source) {
        Ok(yaml_value) => yaml_serde::to_string(&yaml_value)
            .map(String::into_bytes)
            .map_err(|error| ConversionError::Yaml(error.to_string()))?,
        Err(ConversionError::Yaml(_)) => {
            // JSON is a YAML 1.2 subset. Retaining the original JSON text is
            // the lossless YAML profile when yaml_serde's public number model
            // cannot represent an arbitrary decimal scalar. The YAML parser
            // acceptance and exact JSON-model validation happen below.
            fs::read(source_json)?
        }
        Err(error) => return Err(error),
    };
    let toon = encode_toon_exact(&source)?;

    let yaml = write_generated(yaml_output, yaml_manifest_path, &yaml)?;
    let toon = write_generated(toon_output, toon_manifest_path, toon.as_bytes())?;
    Ok(GeneratedArtifacts { yaml, toon })
}

/// Validates generated YAML and TOON against ordered source JSON semantics.
///
/// # Errors
///
/// Returns a parse error or the first type, value, number, array, or object-order
/// divergence. It never sorts objects or arrays before comparison.
pub fn validate_generated_representations(
    source_json: &Path,
    yaml_input: &Path,
    toon_input: &Path,
) -> Result<(), ConversionError> {
    let source: Value = serde_json::from_reader(fs::File::open(source_json)?)?;
    let yaml_model: yaml_serde::Value = yaml_serde::from_reader(fs::File::open(yaml_input)?)
        .map_err(|error| ConversionError::Yaml(error.to_string()))?;
    // A JSON-subset YAML artifact is the exact-decimal fallback. Decode that
    // with the arbitrary-precision value path after proving the YAML parser
    // accepts it; ordinary block YAML uses yaml_serde's value model.
    let yaml: Value = match serde_json::from_reader(fs::File::open(yaml_input)?) {
        Ok(value) => value,
        Err(_) => yaml_to_json(yaml_model)?,
    };
    compare_ordered(&source, &yaml).map_err(|difference| ConversionError::Semantic {
        format: "yaml".to_owned(),
        difference,
    })?;

    let toon_text = fs::read_to_string(toon_input)?;
    let mut documents = tq_formats::decode_toon(
        toon_text.as_bytes(),
        "<corpus-validation>",
        tq_toon::DecoderConfig::default(),
    )
    .map_err(|error| ConversionError::Toon(error.to_string()))?;
    if documents.len() != 1 {
        return Err(ConversionError::Toon(
            "expected exactly one document".to_owned(),
        ));
    }
    let toon = documents
        .pop()
        .ok_or_else(|| ConversionError::Toon("missing document".to_owned()))?
        .value
        .to_json()
        .map_err(|error| ConversionError::Toon(error.to_string()))?;
    compare_toon_ordered(&source, &toon).map_err(|difference| ConversionError::Semantic {
        format: "toon".to_owned(),
        difference,
    })
}

/// Validates already-generated representations and calculates their identities.
///
/// This is the resumable counterpart to [`generate_representations`]. It never
/// rewrites either representation, so an interrupted refresh can finish
/// validation without repeating a potentially expensive generation step.
///
/// # Errors
///
/// Returns a parse, semantic, path, or filesystem error. Identities are only
/// returned after both representations pass value-aware semantic validation.
pub fn finalize_generated_representations(
    source_json: &Path,
    yaml_input: &Path,
    toon_input: &Path,
    yaml_manifest_path: &str,
    toon_manifest_path: &str,
) -> Result<GeneratedArtifacts, ConversionError> {
    validate_generated_representations(source_json, yaml_input, toon_input)?;
    Ok(GeneratedArtifacts {
        yaml: identify_existing(yaml_input, yaml_manifest_path)?,
        toon: identify_existing(toon_input, toon_manifest_path)?,
    })
}

/// Compares two values with ordered object members and arrays.
///
/// Numeric spellings are normalized through the exact `tq_core` number model;
/// this equates `34.0` and `34` without converting through binary64.
///
/// # Errors
///
/// Returns the first semantic difference with its exact path.
pub fn compare_ordered(expected: &Value, actual: &Value) -> Result<(), SemanticDifference> {
    compare_at(expected, actual, "", false, false, true)
}

fn compare_toon_ordered(expected: &Value, actual: &Value) -> Result<(), SemanticDifference> {
    compare_at(expected, actual, "", false, true, true)
}

fn compare_at(
    expected: &Value,
    actual: &Value,
    path: &str,
    header_order_allowed: bool,
    allow_toon_header_order: bool,
    layout_allowed: bool,
) -> Result<(), SemanticDifference> {
    match (expected, actual) {
        (Value::Null, Value::Null) => Ok(()),
        (Value::Bool(left), Value::Bool(right)) => primitive(left, right, path),
        (Value::String(left), Value::String(right)) => primitive(left, right, path),
        (Value::Number(left), Value::Number(right)) => {
            let left_text = left.to_string();
            let right_text = right.to_string();
            let left = crate::benchmark::canonical_number(&left_text).map_err(|error| {
                SemanticDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::NumericFidelity,
                    expected: error,
                    actual: left_text.clone(),
                }
            })?;
            let right = crate::benchmark::canonical_number(&right_text).map_err(|error| {
                SemanticDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::NumericFidelity,
                    expected: right_text.clone(),
                    actual: error,
                }
            })?;
            if left == right {
                Ok(())
            } else {
                Err(SemanticDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::NumericFidelity,
                    expected: left,
                    actual: right,
                })
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Err(SemanticDifference {
                    path: path.to_owned(),
                    kind: DifferenceKind::Length,
                    expected: left.len().to_string(),
                    actual: right.len().to_string(),
                });
            }
            let table_rows = allow_toon_header_order
                && layout_allowed
                && uniform_object_values(left.iter())
                && uniform_object_values(right.iter());
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                compare_at(
                    left,
                    right,
                    &join(path, &index.to_string()),
                    table_rows,
                    allow_toon_header_order,
                    false,
                )?;
            }
            Ok(())
        }
        (Value::Object(left), Value::Object(right)) => {
            if !object_keys_match(left, right, allow_toon_header_order && header_order_allowed) {
                return Err(object_keys_difference(left, right, path));
            }
            let keyed_rows = allow_toon_header_order
                && layout_allowed
                && left.len() >= 2
                && uniform_object_values(left.values())
                && uniform_object_values(right.values());
            for (key, value) in left {
                compare_at(
                    value,
                    &right[key],
                    &join(path, &escape(key)),
                    header_order_allowed || keyed_rows,
                    allow_toon_header_order,
                    true,
                )?;
            }
            Ok(())
        }
        _ => Err(SemanticDifference {
            path: path.to_owned(),
            kind: DifferenceKind::Type,
            expected: type_name(expected).to_owned(),
            actual: type_name(actual).to_owned(),
        }),
    }
}

pub(crate) fn uniform_object_values<'a>(mut values: impl Iterator<Item = &'a Value>) -> bool {
    let Some(first @ Value::Object(_)) = values.next() else {
        return false;
    };
    matching_row_shape(first, first) && values.all(|value| matching_row_shape(first, value))
}

fn matching_row_shape(first: &Value, value: &Value) -> bool {
    let (Value::Object(first), Value::Object(object)) = (first, value) else {
        return false;
    };
    !first.is_empty()
        && object.len() == first.len()
        && first.iter().all(|(key, expected)| {
            object.get(key).is_some_and(|actual| match expected {
                Value::Object(_) => matching_row_shape(expected, actual),
                Value::Array(_) => false,
                _ => !actual.is_object() && !actual.is_array(),
            })
        })
}

fn object_keys_match(
    expected: &serde_json::Map<String, Value>,
    actual: &serde_json::Map<String, Value>,
    allow_reorder: bool,
) -> bool {
    expected.len() == actual.len()
        && if allow_reorder {
            expected.keys().all(|key| actual.contains_key(key))
        } else {
            expected.keys().eq(actual.keys())
        }
}

fn object_keys_difference(
    expected: &serde_json::Map<String, Value>,
    actual: &serde_json::Map<String, Value>,
    path: &str,
) -> SemanticDifference {
    let expected_keys = expected.keys().collect::<Vec<_>>();
    let actual_keys = actual.keys().collect::<Vec<_>>();
    SemanticDifference {
        path: path.to_owned(),
        kind: DifferenceKind::ObjectOrder,
        expected: expected_keys
            .iter()
            .map(|key| key.as_str())
            .collect::<Vec<_>>()
            .join(","),
        actual: actual_keys
            .iter()
            .map(|key| key.as_str())
            .collect::<Vec<_>>()
            .join(","),
    }
}

const NUMBER_MARKER_PREFIX: &str = "tqnumf4c6a91b7e2d";
const NUMBER_MARKER_SUFFIX: char = 'z';

pub(crate) fn encode_toon_exact(source: &Value) -> Result<String, ConversionError> {
    let mut numbers = Vec::new();
    let marked = mark_numbers(source, &mut numbers)?;
    let template = toon_format::encode_default(&marked)
        .map_err(|error| ConversionError::Toon(error.to_string()))?;
    replace_number_markers(&template, &numbers)
}

fn mark_numbers(value: &Value, numbers: &mut Vec<String>) -> Result<Value, ConversionError> {
    Ok(match value {
        Value::Number(number) => {
            let index = numbers.len();
            numbers.push(number.to_string());
            Value::String(format!(
                "{NUMBER_MARKER_PREFIX}{index}{NUMBER_MARKER_SUFFIX}"
            ))
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| mark_numbers(value, numbers))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(values) => {
            let mut object = serde_json::Map::with_capacity(values.len());
            for (key, value) in values {
                if key.contains(NUMBER_MARKER_PREFIX) {
                    return Err(ConversionError::Toon(
                        "source key collides with exact-number marker namespace".to_owned(),
                    ));
                }
                object.insert(key.clone(), mark_numbers(value, numbers)?);
            }
            Value::Object(object)
        }
        Value::String(value) => {
            if value.contains(NUMBER_MARKER_PREFIX) {
                return Err(ConversionError::Toon(
                    "source string collides with exact-number marker namespace".to_owned(),
                ));
            }
            value.clone().into()
        }
        Value::Null => Value::Null,
        Value::Bool(value) => Value::Bool(*value),
    })
}

fn replace_number_markers(template: &str, numbers: &[String]) -> Result<String, ConversionError> {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find(NUMBER_MARKER_PREFIX) {
        output.push_str(&remaining[..start]);
        let marker = &remaining[start + NUMBER_MARKER_PREFIX.len()..];
        let Some(end) = marker.find(NUMBER_MARKER_SUFFIX) else {
            return Err(ConversionError::Toon(
                "unterminated exact-number marker in TOON encoding".to_owned(),
            ));
        };
        let index = marker[..end]
            .parse::<usize>()
            .map_err(|error| ConversionError::Toon(error.to_string()))?;
        output.push_str(numbers.get(index).ok_or_else(|| {
            ConversionError::Toon("unknown exact-number marker in TOON encoding".to_owned())
        })?);
        remaining = &marker[end + NUMBER_MARKER_SUFFIX.len_utf8()..];
    }
    output.push_str(remaining);
    Ok(output)
}

pub(crate) fn json_to_yaml(value: &Value) -> Result<yaml_serde::Value, ConversionError> {
    use yaml_serde::Value as Yaml;

    Ok(match value {
        Value::Null => Yaml::Null,
        Value::Bool(value) => Yaml::Bool(*value),
        Value::String(value) => Yaml::String(value.clone()),
        Value::Number(value) => {
            let number = if let Some(value) = value.as_i64() {
                yaml_serde::Number::from(value)
            } else if let Some(value) = value.as_u64() {
                yaml_serde::Number::from(value)
            } else {
                let float = value.as_f64().ok_or_else(|| {
                    ConversionError::Yaml(format!(
                        "number is outside yaml_serde's lossless envelope: {value}"
                    ))
                })?;
                let round_trip = serde_json::Number::from_f64(float).ok_or_else(|| {
                    ConversionError::Yaml(format!("non-finite JSON number: {value}"))
                })?;
                if round_trip.to_string() != value.to_string() {
                    return Err(ConversionError::Yaml(format!(
                        "number would lose precision in YAML: {value}"
                    )));
                }
                yaml_serde::Number::from(float)
            };
            Yaml::Number(number)
        }
        Value::Array(values) => Yaml::Sequence(
            values
                .iter()
                .map(json_to_yaml)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(values) => {
            let mut mapping = yaml_serde::Mapping::new();
            for (key, value) in values {
                mapping.insert(Yaml::String(key.clone()), json_to_yaml(value)?);
            }
            Yaml::Mapping(mapping)
        }
    })
}

fn yaml_to_json(value: yaml_serde::Value) -> Result<Value, ConversionError> {
    use yaml_serde::Value as Yaml;

    Ok(match value {
        Yaml::Null => Value::Null,
        Yaml::Bool(value) => Value::Bool(value),
        Yaml::String(value) => Value::String(value),
        Yaml::Number(value) => {
            let number = if let Some(value) = value.as_i64() {
                serde_json::Number::from(value)
            } else if let Some(value) = value.as_u64() {
                serde_json::Number::from(value)
            } else {
                let number = serde_json::Number::from_f64(value.as_f64().ok_or_else(|| {
                    ConversionError::Yaml("YAML number has no finite representation".to_owned())
                })?)
                .ok_or_else(|| ConversionError::Yaml("non-finite YAML number".to_owned()))?;
                let source = crate::benchmark::canonical_number(&value.to_string())
                    .map_err(ConversionError::Yaml)?;
                let rendered = crate::benchmark::canonical_number(&number.to_string())
                    .map_err(ConversionError::Yaml)?;
                if source != rendered {
                    return Err(ConversionError::Yaml(format!(
                        "YAML number would lose precision: {value}"
                    )));
                }
                number
            };
            Value::Number(number)
        }
        Yaml::Sequence(values) => Value::Array(
            values
                .into_iter()
                .map(yaml_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Yaml::Mapping(values) => {
            let mut object = serde_json::Map::with_capacity(values.len());
            for (key, value) in values {
                let Yaml::String(key) = key else {
                    return Err(ConversionError::Yaml(
                        "generated YAML contains a non-string mapping key".to_owned(),
                    ));
                };
                object.insert(key, yaml_to_json(value)?);
            }
            Value::Object(object)
        }
        Yaml::Tagged(_) => {
            return Err(ConversionError::Yaml(
                "generated YAML contains a tagged value".to_owned(),
            ));
        }
    })
}

fn primitive<T: PartialEq + std::fmt::Debug>(
    expected: &T,
    actual: &T,
    path: &str,
) -> Result<(), SemanticDifference> {
    if expected == actual {
        Ok(())
    } else {
        Err(SemanticDifference {
            path: path.to_owned(),
            kind: DifferenceKind::Value,
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
        })
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn join(path: &str, component: &str) -> String {
    format!("{path}/{component}")
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn write_generated(
    destination: &Path,
    manifest_path: &str,
    bytes: &[u8],
) -> Result<ArtifactIdentity, ConversionError> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(destination)?;
    Ok(ArtifactIdentity {
        path: manifest_path.to_owned(),
        bytes: u64::try_from(bytes.len())
            .map_err(|_| io::Error::other("generated length does not fit in u64"))?,
        sha256: encode_hex(&Sha256::digest(bytes)),
    })
}

fn identify_existing(
    source: &Path,
    manifest_path: &str,
) -> Result<ArtifactIdentity, ConversionError> {
    let mut source = fs::File::open(source)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        let read = io::Read::read(&mut source, &mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        bytes = bytes
            .checked_add(
                u64::try_from(read)
                    .map_err(|_| io::Error::other("artifact read length does not fit in u64"))?,
            )
            .ok_or_else(|| io::Error::other("artifact byte count overflow"))?;
    }
    Ok(ArtifactIdentity {
        path: manifest_path.to_owned(),
        bytes,
        sha256: encode_hex(&hasher.finalize()),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{compare_ordered, compare_toon_ordered, stream_semantic_digest};
    use serde_json::Value;

    #[test]
    fn ordered_comparison_allows_only_recursive_table_header_order() {
        let expected: Value = serde_json::from_str(
            r#"{"before":{"left":0,"right":1},"rows":[{"id":1,"meta":{"x":2,"y":3}},{"id":2,"meta":{"x":4,"y":5}}],"after":9}"#,
        )
        .unwrap();
        let reordered: Value = serde_json::from_str(
            r#"{"before":{"left":0,"right":1},"rows":[{"meta":{"y":3,"x":2},"id":1},{"meta":{"y":5,"x":4},"id":2}],"after":9}"#,
        )
        .unwrap();
        assert!(compare_ordered(&expected, &reordered).is_err());
        assert!(compare_toon_ordered(&expected, &reordered).is_ok());
        assert!(
            compare_toon_ordered(
                &serde_json::json!([[{"a":1,"b":2},{"a":3,"b":4}]]),
                &serde_json::json!([[{"b":2,"a":1},{"b":4,"a":3}]])
            )
            .is_err()
        );
        assert!(
            compare_toon_ordered(
                &serde_json::json!([{"x":{"a":1,"b":2},"y":{"a":3,"b":4}},{}]),
                &serde_json::json!([{"x":{"b":2,"a":1},"y":{"b":4,"a":3}},{}])
            )
            .is_err()
        );
        let root_table = serde_json::json!([{"a":{"x":1,"y":2},"b":3},{"a":{"x":4,"y":5},"b":6}]);
        let root_table_reordered =
            serde_json::json!([{"b":3,"a":{"y":2,"x":1}},{"b":6,"a":{"y":5,"x":4}}]);
        assert!(compare_toon_ordered(&root_table, &root_table_reordered).is_ok());
        let field_table = serde_json::json!({"rows":[{"a":1,"b":2},{"a":3,"b":4}]});
        let field_table_reordered = serde_json::json!({"rows":[{"b":2,"a":1},{"b":4,"a":3}]});
        assert!(compare_toon_ordered(&field_table, &field_table_reordered).is_ok());
        let outer_reordered: Value = serde_json::from_str(
            r#"{"before":{"right":1,"left":0},"rows":[{"id":1,"meta":{"x":2,"y":3}},{"id":2,"meta":{"x":4,"y":5}}],"after":9}"#,
        )
        .unwrap();
        assert!(compare_ordered(&expected, &outer_reordered).is_err());
        let rows_reordered = serde_json::json!({"before":{"left":0,"right":1},"rows":[{"id":2,"meta":{"x":4,"y":5}},{"id":1,"meta":{"x":2,"y":3}}],"after":9});
        assert!(compare_ordered(&expected, &rows_reordered).is_err());

        let keyed_expected: Value =
            serde_json::from_str(r#"{"alice":{"name":"A","age":1},"bob":{"name":"B","age":2}}"#)
                .unwrap();
        let keyed_reordered: Value =
            serde_json::from_str(r#"{"alice":{"age":1,"name":"A"},"bob":{"age":2,"name":"B"}}"#)
                .unwrap();
        assert!(compare_ordered(&keyed_expected, &keyed_reordered).is_err());
        assert!(compare_toon_ordered(&keyed_expected, &keyed_reordered).is_ok());
        let keyed_outer_reordered: Value =
            serde_json::from_str(r#"{"bob":{"name":"B","age":2},"alice":{"name":"A","age":1}}"#)
                .unwrap();
        assert!(compare_ordered(&keyed_expected, &keyed_outer_reordered).is_err());
        assert!(
            compare_ordered(
                &serde_json::json!({"rows":[{"id":1},{"id":2}]}),
                &serde_json::json!({"rows":[{"id":"1"},{"id":2}]})
            )
            .is_err()
        );
    }

    fn digest(input: &str) -> [u8; 32] {
        stream_semantic_digest(Cursor::new(input.as_bytes())).expect("semantic digest")
    }
    fn toon_digest(input: &str) -> [u8; 32] {
        super::stream_semantic_digest_mode(Cursor::new(input.as_bytes()), true)
            .expect("TOON semantic digest")
    }
    #[test]
    fn stream_digest_preserves_nested_object_array_and_result_order() {
        let original = r#"{"first":[{"a":34.0,"b":[1,{"x":true,"y":null}]}],"second":2}
{"next":3}"#;
        assert_eq!(
            digest(original),
            digest(
                r#"{"first":[{"a":34,"b":[1,{"x":true,"y":null}]}],"second":2}
{"next":3}"#
            )
        );
        assert_ne!(
            digest(original),
            digest(
                r#"{"second":2,"first":[{"a":34,"b":[1,{"x":true,"y":null}]}]}
{"next":3}"#
            )
        );
        assert_ne!(
            digest(original),
            digest(
                r#"{"first":[{"a":34,"b":[1,{"y":null,"x":true}]}],"second":2}
{"next":3}"#
            )
        );
        assert_ne!(digest("1\n2"), digest("2\n1"));
        assert_ne!(digest("[1,2]"), digest("[2,1]"));
    }

    #[test]
    fn toon_stream_digest_normalizes_only_eligible_recursive_headers() {
        let source = r#"{"before":0,"rows":[{"id":1,"meta":{"x":2,"y":3}},{"id":2,"meta":{"y":5,"x":4}}],"after":9}"#;
        let toon = r#"{"before":0,"rows":[{"meta":{"y":3,"x":2},"id":1},{"meta":{"y":5,"x":4},"id":2}],"after":9}"#;
        assert_eq!(toon_digest(source), toon_digest(toon));
        assert_ne!(
            toon_digest(r#"{"before":0,"after":9}"#),
            toon_digest(r#"{"after":9,"before":0}"#)
        );
        assert_ne!(
            toon_digest(r#"{"rows":[{"id":1,"meta":[1]},{"id":2,"meta":[2]}]}"#),
            toon_digest(r#"{"rows":[{"meta":[1],"id":1},{"meta":[2],"id":2}]}"#)
        );
        assert_ne!(
            toon_digest(r#"{"rows":[{"id":1},{"id":2}]}"#),
            toon_digest(r#"{"rows":[{"id":2},{"id":1}]}"#)
        );

        let keyed = r#"{"users":{"alice":{"id":1,"name":"A"},"bob":{"id":2,"name":"B"}}}"#;
        let keyed_reordered =
            r#"{"users":{"alice":{"name":"A","id":1},"bob":{"name":"B","id":2}}}"#;
        let keyed_entries_reordered =
            r#"{"users":{"bob":{"id":2,"name":"B"},"alice":{"id":1,"name":"A"}}}"#;
        assert_eq!(toon_digest(keyed), toon_digest(keyed_reordered));
        assert_ne!(toon_digest(keyed), toon_digest(keyed_entries_reordered));
        assert_ne!(
            toon_digest(r#"{"rows":[{"id":"1"}]}"#),
            toon_digest(r#"{"rows":[{"id":1}]}"#)
        );
        assert_ne!(
            toon_digest(r#"{"items":[[{"id":1,"name":"A"},{"id":2,"name":"B"}]]}"#),
            toon_digest(r#"{"items":[[{"name":"A","id":1},{"name":"B","id":2}]]}"#)
        );
    }

    #[test]
    fn stream_digest_normalizes_exact_large_and_decimal_numbers() {
        assert_eq!(
            digest("123456789012345678901234567890.000e2"),
            digest("12345678901234567890123456789000")
        );
        assert_eq!(digest("-0.00e100"), digest("0"));
        assert_ne!(digest("9007199254740993"), digest("9007199254740992"));
        assert_ne!(
            digest("0.1234567890123456789012345678901"),
            digest("0.1234567890123456789012345678902")
        );
    }

    #[test]
    fn stream_digest_keeps_real_private_number_keys_as_object_members() {
        let object = r#"{"$serde_json::private::Number":"34.0"}"#;
        assert_ne!(digest(object), digest("34.0"));
        assert_ne!(
            digest(r#"{"nested":[{"$serde_json::private::Number":"1"}]}"#),
            digest(r#"{"nested":[1]}"#)
        );
        assert_ne!(
            digest(object),
            digest(r#"{"$serde_json::private::Number":"35.0"}"#)
        );
    }

    #[test]
    fn stream_digest_rejects_invalid_json_and_duplicate_keys() {
        for invalid in [
            "",
            " \n ",
            "[\x0c1]",
            "true\x0cfalse",
            r#"{"x":1,}"#,
            "[1,]",
            "1e",
            "01",
            "[1] trailing",
            r#"{"a":1,"a":2}"#,
            r#"{"a":1,"\u0061":2}"#,
        ] {
            assert!(
                stream_semantic_digest(Cursor::new(invalid.as_bytes())).is_err(),
                "unexpectedly accepted {invalid:?}"
            );
        }
    }
}
