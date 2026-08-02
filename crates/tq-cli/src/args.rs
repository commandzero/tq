//! Dependency-light jq-shaped command parser with pre-input validation.

use std::{collections::BTreeSet, ffi::OsString, path::PathBuf};

use thiserror::Error;
use tq_formats::{InputFormat, OutputFormat, ToonFraming};
use tq_toon::{Delimiter, KeyFolding, WriterConfig};

/// Top-level CLI action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    /// Compile and run a filter.
    Run(Box<RunOptions>),
    /// Print generated compatibility coverage.
    Compatibility,
    /// Print stable help.
    Help,
    /// Print version targets and revision.
    Version,
}

/// Query source selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilterSource {
    /// Positional query text.
    Inline(String),
    /// Query loaded from a file.
    File(PathBuf),
}

/// Explain report encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplainFormat {
    /// Human-readable report.
    Human,
    /// Machine-readable JSON report.
    Json,
}

/// External argument parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalArgumentKind {
    /// Literal string.
    String,
    /// JSON value.
    Json,
    /// TOON value.
    Toon,
}

/// One CLI variable declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalArgument {
    /// Variable name without `$`.
    pub name: String,
    /// Unparsed CLI value.
    pub value: String,
    /// Parser selected by the option.
    pub kind: ExternalArgumentKind,
}

/// Coherent untrusted-input, evaluation, and output resource envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    /// Maximum bytes read from one input source.
    pub input_bytes: u64,
    /// Maximum structured nesting depth.
    pub depth: usize,
    /// Maximum bytes in one scalar/key token.
    pub token_bytes: usize,
    /// Maximum bytes in one physical TOON line.
    pub line_bytes: usize,
    /// Maximum format-detection lookahead bytes.
    pub lookahead_bytes: usize,
    /// Maximum VM work per input document or stream record.
    pub vm_steps: u64,
    /// Maximum emitted results across the invocation.
    pub results: u64,
    /// Maximum bytes written to stdout.
    pub output_bytes: u64,
    /// Maximum in-memory unknown-array preparation bytes.
    pub preparation_memory_bytes: usize,
    /// Maximum disk-backed unknown-array spool bytes.
    pub spool_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            input_bytes: 2 * 1024 * 1024 * 1024,
            depth: 256,
            token_bytes: 8 * 1024 * 1024,
            line_bytes: 16 * 1024 * 1024,
            lookahead_bytes: 64 * 1024,
            vm_steps: 10_000_000,
            results: 100_000_000,
            output_bytes: 8 * 1024 * 1024 * 1024,
            preparation_memory_bytes: 8 * 1024 * 1024,
            spool_bytes: 8 * 1024 * 1024 * 1024,
        }
    }
}

/// Validated run configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent jq-compatible CLI switches remain observable after validation"
)]
pub struct RunOptions {
    /// Query source.
    pub filter: FilterSource,
    /// Ordered paths, with `-` denoting stdin.
    pub files: Vec<PathBuf>,
    /// Input parser override/detection mode.
    pub input_format: InputFormat,
    /// Structured output syntax.
    pub output_format: OutputFormat,
    /// TOON framing.
    pub framing: ToonFraming,
    /// Raw string output.
    pub raw_output: bool,
    /// Suppress raw-output separators.
    pub join_output: bool,
    /// Run once with null and do not consume structured input.
    pub null_input: bool,
    /// Supply physical input lines as strings.
    pub raw_input: bool,
    /// Collect inputs into one array/string.
    pub slurp: bool,
    /// Request jq path/value stream input.
    pub stream: bool,
    /// Turn stream parse failures into values.
    pub stream_errors: bool,
    /// Track jq-compatible last-result status.
    pub exit_status: bool,
    /// TOON strictness.
    pub strict: bool,
    /// JSON pretty-print mode.
    pub pretty_json: bool,
    /// Canonical TOON writer controls.
    pub toon_writer: WriterConfig,
    /// External variables in declaration order.
    pub arguments: Vec<ExternalArgument>,
    /// Optional analysis report.
    pub explain: Option<ExplainFormat>,
    /// Optional trace entry cap.
    pub trace_limit: usize,
    /// Optional machine report file.
    pub report_file: Option<PathBuf>,
    /// Invocation resource envelope.
    pub limits: ResourceLimits,
}

/// Stable CLI parse/validation failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CliError {
    /// An option is not recognized in the MVP.
    #[error("unsupported option '{0}'")]
    Unsupported(String),
    /// Required option value is absent.
    #[error("option '{0}' requires a value")]
    MissingValue(String),
    /// Option value is invalid.
    #[error("invalid value '{value}' for option '{option}'")]
    InvalidValue {
        /// Option spelling.
        option: String,
        /// Rejected value.
        value: String,
    },
    /// Positional/filter selection is invalid.
    #[error("{0}")]
    Usage(String),
    /// Options are individually valid but incompatible.
    #[error("incompatible options: {0}")]
    Incompatible(String),
}

/// Parses a jq-shaped argument vector excluding argv[0].
///
/// # Errors
///
/// Returns stable unsupported, usage, value, and incompatibility failures.
#[allow(
    clippy::too_many_lines,
    reason = "single-pass CLI parsing keeps positional and option ordering deterministic"
)]
pub fn parse_args<I, S>(arguments: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut tokens = arguments
        .into_iter()
        .map(|value| {
            value
                .into()
                .into_string()
                .map_err(|_| CliError::Usage("arguments must be valid UTF-8".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter();
    let mut inline_filter = None;
    let mut filter_file = None;
    let mut files = Vec::new();
    let mut input_format = InputFormat::Auto;
    let mut output_format = OutputFormat::Toon;
    let mut framing = ToonFraming::Sequence;
    let mut raw_output = false;
    let mut join_output = false;
    let mut null_input = false;
    let mut raw_input = false;
    let mut slurp = false;
    let mut stream = false;
    let mut stream_errors = false;
    let mut exit_status = false;
    let mut strict = true;
    let mut pretty_json = true;
    let mut toon_writer = WriterConfig::default();
    let mut external = Vec::new();
    let mut external_names = BTreeSet::new();
    let mut explain = None;
    let mut trace_limit = 0;
    let mut report_file = None;
    let mut limits = ResourceLimits::default();
    let mut positional_only = false;

    while let Some(token) = tokens.next() {
        if positional_only {
            positional(&mut inline_filter, &mut files, token, filter_file.is_some());
            continue;
        }
        match token.as_str() {
            "--" => positional_only = true,
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "compatibility" if inline_filter.is_none() && filter_file.is_none() => {
                return Ok(Command::Compatibility);
            }
            "-f" | "--from-file" => {
                let value = next_value(&mut tokens, &token)?;
                if filter_file.replace(PathBuf::from(value)).is_some() {
                    return Err(CliError::Usage(
                        "filter file may be specified once".to_owned(),
                    ));
                }
            }
            "--input-format" => {
                input_format = parse_input(&token, next_value(&mut tokens, &token)?)?;
            }
            "--output-format" => {
                output_format = parse_output(&token, next_value(&mut tokens, &token)?)?;
            }
            "--seq" => framing = ToonFraming::Sequence,
            "--toon-sequence-input" => input_format = InputFormat::ToonSequence,
            "--unframed" => framing = ToonFraming::Unframed,
            "-r" | "--raw-output" => raw_output = true,
            "-j" | "--join-output" => {
                raw_output = true;
                join_output = true;
            }
            "-n" | "--null-input" => null_input = true,
            "-R" | "--raw-input" => raw_input = true,
            "-s" | "--slurp" => slurp = true,
            "--stream" => stream = true,
            "--stream-errors" => {
                stream = true;
                stream_errors = true;
            }
            "-e" | "--exit-status" => exit_status = true,
            "--non-strict" => strict = false,
            "-c" | "--compact-output" => pretty_json = false,
            "--pretty-output" => pretty_json = true,
            "--indent" => {
                let value = next_value(&mut tokens, &token)?;
                toon_writer.indent_size = value.parse().map_err(|_| CliError::InvalidValue {
                    option: token.clone(),
                    value,
                })?;
            }
            "--delimiter" => {
                let value = next_value(&mut tokens, &token)?;
                toon_writer.delimiter = match value.as_str() {
                    "comma" | "," => Delimiter::Comma,
                    "tab" | "\\t" => Delimiter::Tab,
                    "pipe" | "|" => Delimiter::Pipe,
                    _ => {
                        return Err(CliError::InvalidValue {
                            option: token,
                            value,
                        });
                    }
                };
            }
            "--fold-keys" => toon_writer.key_folding = KeyFolding::Safe,
            "--flatten-depth" => {
                let value = next_value(&mut tokens, &token)?;
                toon_writer.flatten_depth = value.parse().map_err(|_| CliError::InvalidValue {
                    option: token.clone(),
                    value,
                })?;
            }
            "--arg" => parse_external(
                &mut tokens,
                &mut external,
                &mut external_names,
                ExternalArgumentKind::String,
                &token,
            )?,
            "--argjson" => parse_external(
                &mut tokens,
                &mut external,
                &mut external_names,
                ExternalArgumentKind::Json,
                &token,
            )?,
            "--argtoon" => parse_external(
                &mut tokens,
                &mut external,
                &mut external_names,
                ExternalArgumentKind::Toon,
                &token,
            )?,
            "--explain" => explain = Some(ExplainFormat::Human),
            "--explain-json" => explain = Some(ExplainFormat::Json),
            "--trace" => trace_limit = 256,
            "--trace-limit" => {
                let value = next_value(&mut tokens, &token)?;
                trace_limit = value.parse().map_err(|_| CliError::InvalidValue {
                    option: token.clone(),
                    value,
                })?;
            }
            "--report-file" => {
                report_file = Some(PathBuf::from(next_value(&mut tokens, &token)?));
            }
            "--max-input-bytes" => limits.input_bytes = parse_limit(&mut tokens, &token)?,
            "--max-depth" => limits.depth = parse_limit(&mut tokens, &token)?,
            "--max-token-bytes" => limits.token_bytes = parse_limit(&mut tokens, &token)?,
            "--max-line-bytes" => limits.line_bytes = parse_limit(&mut tokens, &token)?,
            "--max-lookahead-bytes" => {
                limits.lookahead_bytes = parse_limit(&mut tokens, &token)?;
            }
            "--max-vm-steps" => limits.vm_steps = parse_limit(&mut tokens, &token)?,
            "--max-results" => limits.results = parse_limit(&mut tokens, &token)?,
            "--max-output-bytes" => limits.output_bytes = parse_limit(&mut tokens, &token)?,
            "--prepare-memory-bytes" => {
                limits.preparation_memory_bytes = parse_limit(&mut tokens, &token)?;
            }
            "--max-spool-bytes" => limits.spool_bytes = parse_limit(&mut tokens, &token)?,
            "-" => positional(&mut inline_filter, &mut files, token, filter_file.is_some()),
            value if value.starts_with('-') => return Err(CliError::Unsupported(token)),
            _ => positional(&mut inline_filter, &mut files, token, filter_file.is_some()),
        }
    }

    if inline_filter.is_some() && filter_file.is_some() {
        return Err(CliError::Usage(
            "positional FILTER conflicts with --from-file".to_owned(),
        ));
    }
    let filter = match (inline_filter, filter_file) {
        (Some(filter), None) => FilterSource::Inline(filter),
        (None, Some(path)) => FilterSource::File(path),
        (None, None) => return Err(CliError::Usage("missing FILTER or --from-file".to_owned())),
        (Some(_), Some(_)) => unreachable!("checked above"),
    };
    if raw_input && input_format != InputFormat::Auto {
        return Err(CliError::Incompatible(
            "--raw-input cannot be combined with --input-format".to_owned(),
        ));
    }
    if stream && matches!(input_format, InputFormat::Yaml) {
        return Err(CliError::Incompatible(
            "--stream requires TOON/JSON event input; YAML is document-at-a-time".to_owned(),
        ));
    }
    if output_format == OutputFormat::Json
        && (toon_writer != WriterConfig::default() || framing == ToonFraming::Unframed)
    {
        return Err(CliError::Incompatible(
            "TOON output options cannot be applied to JSON output".to_owned(),
        ));
    }
    if output_format == OutputFormat::Toon && !pretty_json {
        return Err(CliError::Incompatible(
            "--compact-output applies only to JSON output".to_owned(),
        ));
    }
    if (raw_output || join_output) && framing == ToonFraming::Unframed {
        return Err(CliError::Incompatible(
            "raw output has its own separators and cannot be --unframed".to_owned(),
        ));
    }

    Ok(Command::Run(Box::new(RunOptions {
        filter,
        files,
        input_format,
        output_format,
        framing,
        raw_output,
        join_output,
        null_input,
        raw_input,
        slurp,
        stream,
        stream_errors,
        exit_status,
        strict,
        pretty_json,
        toon_writer,
        arguments: external,
        explain,
        trace_limit,
        report_file,
        limits,
    })))
}

fn parse_limit<T: std::str::FromStr>(
    tokens: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<T, CliError> {
    let value = next_value(tokens, option)?;
    value.parse().map_err(|_| CliError::InvalidValue {
        option: option.to_owned(),
        value,
    })
}

fn positional(
    filter: &mut Option<String>,
    files: &mut Vec<PathBuf>,
    token: String,
    has_filter_file: bool,
) {
    if filter.is_none() && !has_filter_file {
        *filter = Some(token);
    } else {
        files.push(PathBuf::from(token));
    }
}

fn next_value(tokens: &mut impl Iterator<Item = String>, option: &str) -> Result<String, CliError> {
    tokens
        .next()
        .ok_or_else(|| CliError::MissingValue(option.to_owned()))
}

fn parse_input(option: &str, value: String) -> Result<InputFormat, CliError> {
    match value.as_str() {
        "auto" => Ok(InputFormat::Auto),
        "toon" => Ok(InputFormat::Toon),
        "yaml" | "yml" => Ok(InputFormat::Yaml),
        "json" => Ok(InputFormat::Json),
        "toon-seq" | "toon-sequence" => Ok(InputFormat::ToonSequence),
        _ => Err(CliError::InvalidValue {
            option: option.to_owned(),
            value,
        }),
    }
}

fn parse_output(option: &str, value: String) -> Result<OutputFormat, CliError> {
    match value.as_str() {
        "toon" => Ok(OutputFormat::Toon),
        "json" => Ok(OutputFormat::Json),
        "yaml" | "yml" => Err(CliError::Unsupported(
            "YAML output is deferred; select toon or json".to_owned(),
        )),
        _ => Err(CliError::InvalidValue {
            option: option.to_owned(),
            value,
        }),
    }
}

fn parse_external(
    tokens: &mut impl Iterator<Item = String>,
    arguments: &mut Vec<ExternalArgument>,
    names: &mut BTreeSet<String>,
    kind: ExternalArgumentKind,
    option: &str,
) -> Result<(), CliError> {
    let name = next_value(tokens, option)?;
    if !valid_variable(&name) {
        return Err(CliError::InvalidValue {
            option: option.to_owned(),
            value: name,
        });
    }
    if !names.insert(name.clone()) {
        return Err(CliError::Usage(format!(
            "external variable '{name}' is declared more than once"
        )));
    }
    let value = next_value(tokens, option)?;
    arguments.push(ExternalArgument { name, value, kind });
    Ok(())
}

fn valid_variable(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use tq_formats::{InputFormat, OutputFormat, ToonFraming};

    use super::{CliError, Command, FilterSource, parse_args};

    #[test]
    fn parses_jq_shape_files_stdin_and_filter_file() {
        let Command::Run(run) = parse_args([".name", "a.toon", "-"]).unwrap() else {
            panic!("run command")
        };
        assert_eq!(run.filter, FilterSource::Inline(".name".to_owned()));
        assert_eq!(run.files.len(), 2);

        let Command::Run(run) = parse_args(["-f", "query.tq", "input.json"]).unwrap() else {
            panic!("run command")
        };
        assert_eq!(run.filter, FilterSource::File("query.tq".into()));
    }

    #[test]
    fn parses_formats_framing_and_core_modes() {
        let Command::Run(run) = parse_args([
            "--input-format",
            "yaml",
            "--output-format",
            "json",
            "-c",
            "-r",
            "-s",
            "-e",
            ".",
        ])
        .unwrap() else {
            panic!("run command")
        };
        assert_eq!(run.input_format, InputFormat::Yaml);
        assert_eq!(run.output_format, OutputFormat::Json);
        assert_eq!(run.framing, ToonFraming::Sequence);
        assert!(run.raw_output && run.slurp && run.exit_status);
    }

    #[test]
    fn validates_conflicts_missing_values_and_unsupported_options() {
        assert!(matches!(
            parse_args(["-f", "q", "-f", "other"]),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(
            parse_args(["--input-format"]),
            Err(CliError::MissingValue(_))
        ));
        assert!(matches!(
            parse_args(["--wat", "."]),
            Err(CliError::Unsupported(_))
        ));
        assert!(parse_args(["--stream", "--input-format", "yaml", "."]).is_err());
        assert!(parse_args(["-c", "."]).is_err());
    }

    #[test]
    fn help_version_and_compatibility_are_commands() {
        assert_eq!(parse_args(["--help"]).unwrap(), Command::Help);
        assert_eq!(parse_args(["--version"]).unwrap(), Command::Version);
        assert_eq!(
            parse_args(["compatibility"]).unwrap(),
            Command::Compatibility
        );
    }
}
