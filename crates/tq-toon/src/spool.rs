//! Bounded unknown-length array preparation with secure disk transition.

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use thiserror::Error;
use tq_core::Value;

use crate::{WriterConfig, writer};

static NEXT_SPOOL: AtomicU64 = AtomicU64::new(0);

/// Unknown-length array buffering and spool limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayPreparationConfig {
    /// Encoded bytes retained before disk transition.
    pub memory_threshold_bytes: usize,
    /// Maximum total framed bytes allowed in a spool.
    pub maximum_spool_bytes: u64,
    /// Directory that owns temporary spool files.
    pub spool_directory: PathBuf,
    /// Whether disk transition is permitted.
    pub allow_spool: bool,
}

impl Default for ArrayPreparationConfig {
    fn default() -> Self {
        Self {
            memory_threshold_bytes: 8 * 1024 * 1024,
            maximum_spool_bytes: 8 * 1024 * 1024 * 1024,
            spool_directory: std::env::temp_dir(),
            allow_spool: true,
        }
    }
}

/// Array preparation or replay failure.
#[derive(Debug, Error)]
pub enum SpoolError {
    /// Spooling was required but disabled.
    #[error("array preparation exceeded memory threshold and spooling is disabled")]
    Disabled,
    /// Configured disk limit would be exceeded.
    #[error("array spool exceeds configured byte limit")]
    Limit,
    /// Temporary-file or output I/O failed.
    #[error("array spool I/O failed: {0}")]
    Io(#[from] io::Error),
    /// An internal spool record could not be decoded.
    #[error("array spool record is invalid: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Prepared unknown-length array that retains values in memory or a private
/// length-framed temporary file, never both after transition.
#[derive(Debug)]
pub struct PreparedArray {
    config: ArrayPreparationConfig,
    memory: Vec<Vec<u8>>,
    memory_bytes: usize,
    spool: Option<Spool>,
    count: u64,
    framed_bytes: u64,
    layout: Layout,
}

#[derive(Debug)]
struct Spool {
    file: File,
    path: PathBuf,
}

impl Drop for Spool {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[derive(Clone, Debug)]
enum Layout {
    Empty,
    Scalars,
    Tabular(Vec<Arc<str>>),
    Expanded,
}

impl PreparedArray {
    /// Creates an empty unknown-length array preparation.
    #[must_use]
    pub fn new(config: ArrayPreparationConfig) -> Self {
        Self {
            config,
            memory: Vec::new(),
            memory_bytes: 0,
            spool: None,
            count: 0,
            framed_bytes: 0,
            layout: Layout::Empty,
        }
    }

    /// Adds one value while enforcing memory and disk limits.
    ///
    /// # Errors
    ///
    /// Returns serialization, temporary-file, disabled-spool, or limit errors.
    pub fn push(&mut self, value: &Value) -> Result<(), SpoolError> {
        let encoded = serde_json::to_vec(value)?;
        let framed = u64::try_from(encoded.len())
            .unwrap_or(u64::MAX)
            .saturating_add(8);
        if self.framed_bytes.saturating_add(framed) > self.config.maximum_spool_bytes {
            return Err(SpoolError::Limit);
        }
        if self.spool.is_none()
            && self.memory_bytes.saturating_add(encoded.len() + 8)
                > self.config.memory_threshold_bytes
        {
            self.transition_to_disk()?;
        }
        if let Some(spool) = &mut self.spool {
            write_record(&mut spool.file, &encoded)?;
        } else {
            self.memory_bytes = self.memory_bytes.saturating_add(encoded.len() + 8);
            self.memory.push(encoded);
        }
        self.update_layout(value);
        self.count = self.count.saturating_add(1);
        self.framed_bytes = self.framed_bytes.saturating_add(framed);
        Ok(())
    }

    /// Number of prepared values.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.count
    }

    /// Whether no values have been prepared.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Whether preparation transitioned to disk.
    #[must_use]
    pub const fn spooled(&self) -> bool {
        self.spool.is_some()
    }

    /// Current private spool path, exposed for observability and tests.
    #[must_use]
    pub fn spool_path(&self) -> Option<&Path> {
        self.spool.as_ref().map(|spool| spool.path.as_path())
    }

    /// Replays the prepared array as canonical root TOON without rebuilding it.
    ///
    /// # Errors
    ///
    /// Returns spool decode/read or output failures.
    pub fn write_to<W: Write>(
        &mut self,
        mut output: W,
        writer_config: WriterConfig,
    ) -> Result<(), SpoolError> {
        let layout = self.layout.clone();
        write!(
            output,
            "[{}{}]",
            self.count,
            delimiter_suffix(writer_config)
        )?;
        match layout {
            Layout::Empty => output.write_all(b":")?,
            Layout::Scalars => {
                output.write_all(b": ")?;
                let mut index = 0_usize;
                self.for_each_value(|value| {
                    if index != 0 {
                        write!(output, "{}", delimiter_character(writer_config))?;
                    }
                    output
                        .write_all(writer::encode_array_scalar(value, writer_config).as_bytes())?;
                    index += 1;
                    Ok(())
                })?;
            }
            Layout::Tabular(fields) => {
                output.write_all(b"{")?;
                for (index, field) in fields.iter().enumerate() {
                    if index != 0 {
                        write!(output, "{}", delimiter_character(writer_config))?;
                    }
                    output.write_all(writer::render_key(field).as_bytes())?;
                }
                output.write_all(b"}:")?;
                self.for_each_value(|value| {
                    let Value::Object(object) = value else {
                        unreachable!("layout tracked during preparation")
                    };
                    output.write_all(b"\n")?;
                    output.write_all(" ".repeat(writer_config.indent_size).as_bytes())?;
                    output.write_all(
                        writer::encode_tabular_row(object, &fields, writer_config).as_bytes(),
                    )?;
                    Ok(())
                })?;
            }
            Layout::Expanded => {
                output.write_all(b":")?;
                self.for_each_value(|value| {
                    output.write_all(b"\n")?;
                    let item = writer::encode_list_item(value, writer_config);
                    for (line_index, line) in item.lines().enumerate() {
                        if line_index != 0 {
                            output.write_all(b"\n")?;
                        }
                        output.write_all(" ".repeat(writer_config.indent_size).as_bytes())?;
                        output.write_all(line.as_bytes())?;
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    fn update_layout(&mut self, value: &Value) {
        self.layout = match &self.layout {
            Layout::Empty | Layout::Scalars if scalar(value) => Layout::Scalars,
            Layout::Empty => tabular_schema(value).map_or(Layout::Expanded, Layout::Tabular),
            Layout::Tabular(fields) if matches_schema(value, fields) => {
                Layout::Tabular(fields.clone())
            }
            Layout::Expanded | Layout::Scalars | Layout::Tabular(_) => Layout::Expanded,
        };
    }

    fn transition_to_disk(&mut self) -> Result<(), SpoolError> {
        if !self.config.allow_spool {
            return Err(SpoolError::Disabled);
        }
        let mut spool = create_spool(&self.config.spool_directory)?;
        for record in &self.memory {
            write_record(&mut spool.file, record)?;
        }
        self.memory.clear();
        self.memory_bytes = 0;
        self.spool = Some(spool);
        Ok(())
    }

    fn for_each_value(
        &mut self,
        mut consume: impl FnMut(&Value) -> Result<(), SpoolError>,
    ) -> Result<(), SpoolError> {
        if let Some(spool) = &mut self.spool {
            spool.file.flush()?;
            spool.file.seek(SeekFrom::Start(0))?;
            loop {
                let mut length = [0_u8; 8];
                if spool.file.read(&mut length[..1])? == 0 {
                    break;
                }
                spool.file.read_exact(&mut length[1..])?;
                let length =
                    usize::try_from(u64::from_le_bytes(length)).map_err(|_| SpoolError::Limit)?;
                let mut bytes = vec![0; length];
                spool.file.read_exact(&mut bytes)?;
                let value = serde_json::from_slice(&bytes)?;
                consume(&value)?;
            }
        } else {
            for bytes in &self.memory {
                let value = serde_json::from_slice(bytes)?;
                consume(&value)?;
            }
        }
        Ok(())
    }
}

fn write_record(mut writer: impl Write, bytes: &[u8]) -> Result<(), io::Error> {
    writer.write_all(&(bytes.len() as u64).to_le_bytes())?;
    writer.write_all(bytes)
}

fn create_spool(directory: &Path) -> Result<Spool, io::Error> {
    for _ in 0..128 {
        let id = NEXT_SPOOL.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(".tq-spool-{}-{id}", std::process::id()));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok(Spool { file, path }),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate unique tq spool",
    ))
}

fn scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn tabular_schema(value: &Value) -> Option<Vec<Arc<str>>> {
    let Value::Object(object) = value else {
        return None;
    };
    if object.is_empty() || object.values().any(|value| !scalar(value)) {
        None
    } else {
        Some(object.keys().cloned().collect())
    }
}

fn matches_schema(value: &Value, fields: &[Arc<str>]) -> bool {
    let Value::Object(object) = value else {
        return false;
    };
    object.len() == fields.len()
        && fields.iter().all(|field| object.contains_key(field))
        && object.values().all(scalar)
}

fn delimiter_suffix(config: WriterConfig) -> &'static str {
    match config.delimiter {
        crate::Delimiter::Comma => "",
        crate::Delimiter::Tab => "\t",
        crate::Delimiter::Pipe => "|",
    }
}

fn delimiter_character(config: WriterConfig) -> char {
    match config.delimiter {
        crate::Delimiter::Comma => ',',
        crate::Delimiter::Tab => '\t',
        crate::Delimiter::Pipe => '|',
    }
}

#[cfg(test)]
mod tests {
    use tq_core::Value;

    use super::{ArrayPreparationConfig, PreparedArray, SpoolError};
    use crate::WriterConfig;

    #[test]
    fn threshold_transition_preserves_tabular_schema_and_cleans_up() {
        let directory = tempfile::tempdir().unwrap();
        let mut prepared = PreparedArray::new(ArrayPreparationConfig {
            memory_threshold_bytes: 1,
            maximum_spool_bytes: 1024 * 1024,
            spool_directory: directory.path().to_owned(),
            allow_spool: true,
        });
        for json in [r#"{"id":1,"name":"Ada"}"#, r#"{"name":"Bob","id":2}"#] {
            prepared
                .push(&serde_json::from_str::<Value>(json).unwrap())
                .unwrap();
        }
        assert!(prepared.spooled());
        let path = prepared.spool_path().unwrap().to_owned();
        let mut output = Vec::new();
        prepared
            .write_to(&mut output, WriterConfig::default())
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "[2]{id,name}:\n  1,Ada\n  2,Bob"
        );
        drop(prepared);
        assert!(!path.exists());
    }

    #[test]
    fn later_schema_change_falls_back_without_losing_prior_values() {
        let mut prepared = PreparedArray::new(ArrayPreparationConfig::default());
        prepared
            .push(&serde_json::from_str::<Value>(r#"{"id":1}"#).unwrap())
            .unwrap();
        prepared.push(&Value::Bool(true)).unwrap();
        let mut output = Vec::new();
        prepared
            .write_to(&mut output, WriterConfig::default())
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "[2]:\n  - id: 1\n  - true"
        );
    }

    #[test]
    fn disabled_and_limited_spools_fail_before_output() {
        let mut disabled = PreparedArray::new(ArrayPreparationConfig {
            memory_threshold_bytes: 0,
            allow_spool: false,
            ..ArrayPreparationConfig::default()
        });
        assert!(matches!(
            disabled.push(&Value::Null),
            Err(SpoolError::Disabled)
        ));

        let mut limited = PreparedArray::new(ArrayPreparationConfig {
            maximum_spool_bytes: 1,
            ..ArrayPreparationConfig::default()
        });
        assert!(matches!(limited.push(&Value::Null), Err(SpoolError::Limit)));
    }
}
