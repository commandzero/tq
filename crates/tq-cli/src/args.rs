//! Dependency-light jq-shaped command parser with pre-input validation.

use std::{collections::BTreeSet, ffi::OsString, fmt::Write as _, path::PathBuf};

use thiserror::Error;
use tq_formats::{InputFormat, JsonIndent, OutputFormat, ToonFraming};
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
    /// Print stable build and capability information.
    BuildConfiguration,
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
    /// Entire UTF-8 file contents.
    RawFile,
    /// Ordered JSON texts collected into an array.
    SlurpFile,
}

/// Parser for argv values consumed after `--args` or `--jsonargs`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositionalArgumentKind {
    /// Preserve each argv value as a string.
    String,
    /// Decode each argv value as one JSON text.
    Json,
}

/// JSON color selection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorMode {
    /// Use terminal/environment policy at the process boundary.
    #[default]
    Auto,
    /// Emit tq's stable ANSI palette.
    Always,
    /// Never emit ANSI escapes.
    Never,
}

/// Ambient capability policy for library callers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent ambient authorities remain explicit and denyable"
)]
pub struct CapabilityPolicy {
    /// Permit filter, input, argument, and report file access.
    pub filesystem: bool,
    /// Permit ambient environment inspection.
    pub environment: bool,
    /// Permit ambient terminal inspection and explicit color.
    pub terminal: bool,
    /// Permit clock, local-timezone, and input-source metadata inspection.
    pub platform: bool,
}

impl Default for CapabilityPolicy {
    fn default() -> Self {
        Self {
            filesystem: true,
            environment: true,
            terminal: true,
            platform: true,
        }
    }
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
    /// Maximum values in one hybrid preparation batch.
    pub hybrid_batch_values: usize,
    /// Maximum hybrid worker batches simultaneously in flight.
    pub hybrid_in_flight_batches: usize,
    /// Maximum estimated hybrid worker bytes simultaneously in flight.
    pub hybrid_in_flight_bytes: usize,
    /// Maximum elements in one parallel selected-decode batch.
    pub decode_batch_values: usize,
    /// Target maximum encoded bytes in one parallel selected-decode batch.
    pub decode_batch_bytes: usize,
    /// Maximum selected-decode batches simultaneously in flight.
    pub decode_in_flight_batches: usize,
    /// Maximum selected-decode source bytes simultaneously in flight.
    pub decode_in_flight_bytes: usize,
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
            hybrid_batch_values: 16 * 1024,
            hybrid_in_flight_batches: 4,
            hybrid_in_flight_bytes: 8 * 1024 * 1024,
            decode_batch_values: 4 * 1024,
            decode_batch_bytes: 2 * 1024 * 1024,
            decode_in_flight_batches: 32,
            decode_in_flight_bytes: 64 * 1024 * 1024,
            spool_bytes: 8 * 1024 * 1024 * 1024,
        }
    }
}

/// Internal execution selection for differential tests and benchmarks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[doc(hidden)]
pub enum ExecutionOverride {
    /// Use analyzed automatic plan selection.
    #[default]
    Automatic,
    /// Force retained document execution for differential tests and benchmarks.
    Document,
}

/// Validated run configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent jq-compatible CLI switches remain observable after validation"
)]
pub struct RunOptions {
    /// Internal differential-test and benchmark plan override.
    #[doc(hidden)]
    pub execution_override: ExecutionOverride,
    /// Query source.
    pub filter: FilterSource,
    /// Ordered paths, with `-` denoting stdin.
    pub files: Vec<PathBuf>,
    /// Explicit jq module search roots, in command-line order.
    pub module_paths: Vec<PathBuf>,
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
    /// Separate raw outputs with NUL and reject NUL-containing strings.
    pub raw_output0: bool,
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
    /// Pass a source through byte-for-byte when structured parsing rejects it.
    pub proxy_on_error: bool,
    /// Track jq-compatible last-result status.
    pub exit_status: bool,
    /// TOON strictness.
    pub strict: bool,
    /// JSON pretty-print mode.
    pub pretty_json: bool,
    /// JSON indentation style.
    pub json_indent: JsonIndent,
    /// Escape all non-ASCII JSON output.
    pub ascii_output: bool,
    /// Recursively sort object keys before encoding.
    pub sort_keys: bool,
    /// JSON ANSI color policy.
    pub color: ColorMode,
    /// Flush after every complete emitted result.
    pub unbuffered: bool,
    /// Platform binary-output request.
    pub binary: bool,
    /// Query may inspect an explicitly admitted environment snapshot.
    pub allow_environment: bool,
    /// Query may inspect clock, timezone, and input-source metadata.
    pub allow_platform: bool,
    /// Canonical TOON writer controls.
    pub toon_writer: WriterConfig,
    /// External variables in declaration order.
    pub arguments: Vec<ExternalArgument>,
    /// Positional `$ARGS` parser, when selected.
    pub positional_argument_kind: Option<PositionalArgumentKind>,
    /// Ordered argv values exposed through `$ARGS.positional`.
    pub positional_arguments: Vec<String>,
    /// Optional analysis report.
    pub explain: Option<ExplainFormat>,
    /// Optional trace entry cap.
    pub trace_limit: usize,
    /// Optional machine report file.
    pub report_file: Option<PathBuf>,
    /// Invocation resource envelope.
    pub limits: ResourceLimits,
    /// Library-controlled ambient integration policy.
    pub capability_policy: CapabilityPolicy,
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

#[derive(Clone, Copy)]
struct OptionSpec {
    short: Option<char>,
    syntax: &'static str,
    value: bool,
    description: &'static str,
}

const OPTION_REGISTRY: &[OptionSpec] = &[
    OptionSpec {
        short: Some('i'),
        syntax: "-i, --input-format FORMAT",
        value: true,
        description: "select auto, TOON, YAML, JSON, JSON Lines, or TOON sequence input",
    },
    OptionSpec {
        short: Some('o'),
        syntax: "-o, --output-format FORMAT",
        value: true,
        description: "select TOON, YAML, JSON, or JSON Lines output",
    },
    OptionSpec {
        short: Some('n'),
        syntax: "-n, --null-input",
        value: false,
        description: "run once with null input",
    },
    OptionSpec {
        short: Some('R'),
        syntax: "-R, --raw-input",
        value: false,
        description: "read physical lines as strings",
    },
    OptionSpec {
        short: Some('s'),
        syntax: "-s, --slurp",
        value: false,
        description: "collect ordered inputs and run once",
    },
    OptionSpec {
        short: Some('c'),
        syntax: "-c, --compact-output",
        value: false,
        description: "emit compact JSON",
    },
    OptionSpec {
        short: Some('r'),
        syntax: "-r, --raw-output",
        value: false,
        description: "emit strings without JSON quotes",
    },
    OptionSpec {
        short: None,
        syntax: "--raw-output0",
        value: false,
        description: "emit NUL-separated raw outputs",
    },
    OptionSpec {
        short: Some('j'),
        syntax: "-j, --join-output",
        value: false,
        description: "emit raw output without separators",
    },
    OptionSpec {
        short: Some('a'),
        syntax: "-a, --ascii-output",
        value: false,
        description: "escape non-ASCII JSON output",
    },
    OptionSpec {
        short: Some('S'),
        syntax: "-S, --sort-keys",
        value: false,
        description: "sort object keys recursively",
    },
    OptionSpec {
        short: Some('C'),
        syntax: "-C, --color-output",
        value: false,
        description: "force stable ANSI JSON color",
    },
    OptionSpec {
        short: Some('M'),
        syntax: "-M, --monochrome-output",
        value: false,
        description: "disable ANSI color",
    },
    OptionSpec {
        short: None,
        syntax: "--tab",
        value: false,
        description: "indent JSON with tabs",
    },
    OptionSpec {
        short: None,
        syntax: "--indent N",
        value: true,
        description: "indent structured output",
    },
    OptionSpec {
        short: None,
        syntax: "--unbuffered",
        value: false,
        description: "flush after every output",
    },
    OptionSpec {
        short: None,
        syntax: "--allow-environment",
        value: false,
        description: "permit env to inspect a redaction-safe process snapshot",
    },
    OptionSpec {
        short: None,
        syntax: "--allow-platform",
        value: false,
        description: "permit clock, timezone, and input metadata built-ins",
    },
    OptionSpec {
        short: None,
        syntax: "--stream",
        value: false,
        description: "read path/value events",
    },
    OptionSpec {
        short: None,
        syntax: "--stream-errors",
        value: false,
        description: "report stream parse errors as values",
    },
    OptionSpec {
        short: Some('x'),
        syntax: "-x, --proxy-on-error",
        value: false,
        description: "pass through sources rejected by structured parsing",
    },
    OptionSpec {
        short: None,
        syntax: "--seq",
        value: false,
        description: "emit TOON Text Sequence frames",
    },
    OptionSpec {
        short: Some('f'),
        syntax: "-f, --from-file FILE",
        value: true,
        description: "load filter from a file",
    },
    OptionSpec {
        short: Some('L'),
        syntax: "-L, --library-path DIR",
        value: true,
        description: "add an explicit jq module search path",
    },
    OptionSpec {
        short: None,
        syntax: "--arg NAME VALUE",
        value: true,
        description: "bind a string variable",
    },
    OptionSpec {
        short: None,
        syntax: "--argjson NAME JSON",
        value: true,
        description: "bind a JSON variable",
    },
    OptionSpec {
        short: None,
        syntax: "--argtoon NAME TOON",
        value: true,
        description: "bind a TOON variable",
    },
    OptionSpec {
        short: None,
        syntax: "--slurpfile NAME FILE",
        value: true,
        description: "bind JSON texts as an array",
    },
    OptionSpec {
        short: None,
        syntax: "--rawfile NAME FILE",
        value: true,
        description: "bind complete UTF-8 file text",
    },
    OptionSpec {
        short: None,
        syntax: "--args",
        value: false,
        description: "bind remaining argv strings",
    },
    OptionSpec {
        short: None,
        syntax: "--jsonargs",
        value: false,
        description: "bind remaining argv JSON texts",
    },
    OptionSpec {
        short: Some('e'),
        syntax: "-e, --exit-status",
        value: false,
        description: "derive status from the last result",
    },
    OptionSpec {
        short: Some('b'),
        syntax: "-b, --binary",
        value: false,
        description: "request binary-safe platform output",
    },
    OptionSpec {
        short: Some('V'),
        syntax: "-V, --version",
        value: false,
        description: "print version targets",
    },
    OptionSpec {
        short: None,
        syntax: "--build-configuration",
        value: false,
        description: "print stable build capabilities",
    },
    OptionSpec {
        short: Some('h'),
        syntax: "-h, --help",
        value: false,
        description: "print this generated help",
    },
];

/// Produces stable help from the same admitted option registry used for short
/// cluster validation.
#[must_use]
pub fn generated_help() -> String {
    let mut help = String::from(
        "tq - jq-compatible queries over TOON, YAML, JSON, and JSON Lines\n\n\
Usage: tq [OPTIONS] [FILTER [FILE...]]\n       tq [OPTIONS] -f FILE [INPUT...]\n       tq compatibility\n\nOptions:\n",
    );
    for option in OPTION_REGISTRY {
        let _ = writeln!(help, "  {:<31} {}", option.syntax, option.description);
    }
    help.push_str(
        "\nFormats: -i, --input-format auto|toon|yaml|json|jsonl|toon-seq\n\
         -o, --output-format toon|yaml|json|jsonl, --toon-sequence-input, --unframed\n\
TOON:    --delimiter comma|tab|pipe, --fold-keys, --flatten-depth N, --non-strict\n\
Reports: --explain, --explain-json, --trace, --trace-limit N, --report-file FILE\n\
Limits:  --max-input-bytes N, --max-depth N, --max-token-bytes N,\n\
         --max-line-bytes N, --max-lookahead-bytes N, --max-vm-steps N,\n\
         --max-results N, --max-output-bytes N, --prepare-memory-bytes N,\n\
         --hybrid-batch-values N, --hybrid-in-flight-batches N,\n\
         --hybrid-in-flight-bytes N, --decode-batch-values N,\n\
         --decode-batch-bytes N, --decode-in-flight-batches N,\n\
         --decode-in-flight-bytes N, --max-spool-bytes N\n",
    );
    help
}

/// Parses a jq-shaped argument vector excluding `argv[0]`.
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
    parse_args_with_policy(arguments, CapabilityPolicy::default())
}

/// Parses arguments under an explicit ambient integration policy.
///
/// # Errors
///
/// Returns stable unsupported, usage, value, incompatibility, and policy
/// failures before any input is consumed.
#[allow(
    clippy::too_many_lines,
    reason = "single-pass CLI parsing keeps positional and option ordering deterministic"
)]
pub fn parse_args_with_policy<I, S>(
    arguments: I,
    capability_policy: CapabilityPolicy,
) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let tokens = arguments
        .into_iter()
        .map(|value| {
            value
                .into()
                .into_string()
                .map_err(|_| CliError::Usage("arguments must be valid UTF-8".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut tokens = expand_short_options(tokens)?.into_iter();
    let mut inline_filter = None;
    let mut filter_file = None;
    let mut files = Vec::new();
    let mut module_paths = Vec::new();
    let mut input_format = InputFormat::Auto;
    let mut output_format = OutputFormat::Toon;
    let mut framing = ToonFraming::Sequence;
    let mut raw_output = false;
    let mut join_output = false;
    let mut raw_output0 = false;
    let mut null_input = false;
    let mut raw_input = false;
    let mut slurp = false;
    let mut stream = false;
    let mut stream_errors = false;
    let mut proxy_on_error = false;
    let mut exit_status = false;
    let mut strict = true;
    let mut pretty_json = true;
    let mut pretty_explicit = false;
    let mut json_indent = JsonIndent::default();
    let mut indent_explicit = false;
    let mut tab_explicit = false;
    let mut ascii_output = false;
    let mut sort_keys = false;
    let mut color = ColorMode::Auto;
    let mut unbuffered = false;
    let mut binary = false;
    let mut allow_environment = false;
    let mut allow_platform = false;
    let mut toon_writer = WriterConfig::default();
    let mut external = Vec::new();
    let mut external_names = BTreeSet::new();
    let mut positional_argument_kind = None;
    let mut positional_arguments = Vec::new();
    let mut explain = None;
    let mut trace_limit = 0;
    let mut report_file = None;
    let mut limits = ResourceLimits::default();
    let mut positional_only = false;

    while let Some(token) = tokens.next() {
        if positional_argument_kind.is_some() && (inline_filter.is_some() || filter_file.is_some())
        {
            positional_arguments.push(token);
            continue;
        }
        if positional_only {
            positional(
                &mut inline_filter,
                &mut files,
                &mut positional_arguments,
                positional_argument_kind,
                token,
                filter_file.is_some(),
            );
            continue;
        }
        match token.as_str() {
            "--" => positional_only = true,
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--build-configuration" => return Ok(Command::BuildConfiguration),
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
            "-i" | "--input-format" => {
                input_format = parse_input(&token, next_value(&mut tokens, &token)?)?;
            }
            "-o" | "--output-format" => {
                output_format = parse_output(&token, next_value(&mut tokens, &token)?)?;
            }
            "--seq" => framing = ToonFraming::Sequence,
            "--toon-sequence-input" => input_format = InputFormat::ToonSequence,
            "--unframed" => framing = ToonFraming::Unframed,
            "-r" | "--raw-output" => raw_output = true,
            "--raw-output0" => {
                raw_output = true;
                raw_output0 = true;
                join_output = false;
            }
            "-j" | "--join-output" => {
                raw_output = true;
                join_output = true;
                raw_output0 = false;
            }
            "-a" | "--ascii-output" => ascii_output = true,
            "-S" | "--sort-keys" => sort_keys = true,
            "-C" | "--color-output" => color = ColorMode::Always,
            "-M" | "--monochrome-output" => color = ColorMode::Never,
            "--tab" => {
                json_indent = JsonIndent::Tabs;
                tab_explicit = true;
            }
            "--unbuffered" => unbuffered = true,
            "--allow-environment" => allow_environment = true,
            "--allow-platform" => allow_platform = true,
            "-b" | "--binary" => binary = true,
            "-n" | "--null-input" => null_input = true,
            "-R" | "--raw-input" => raw_input = true,
            "-s" | "--slurp" => slurp = true,
            "--stream" => stream = true,
            "--stream-errors" => {
                stream = true;
                stream_errors = true;
            }
            "-x" | "--proxy-on-error" => proxy_on_error = true,
            "-e" | "--exit-status" => exit_status = true,
            "--non-strict" => strict = false,
            "-c" | "--compact-output" => pretty_json = false,
            "--pretty-output" => {
                pretty_json = true;
                pretty_explicit = true;
            }
            "--indent" => {
                let value = next_value(&mut tokens, &token)?;
                let indent: u8 = value.parse().map_err(|_| CliError::InvalidValue {
                    option: token.clone(),
                    value: value.clone(),
                })?;
                if indent > 7 {
                    return Err(CliError::InvalidValue {
                        option: token,
                        value,
                    });
                }
                toon_writer.indent_size = usize::from(indent);
                json_indent = JsonIndent::Spaces(indent);
                indent_explicit = true;
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
            "--slurpfile" => parse_external(
                &mut tokens,
                &mut external,
                &mut external_names,
                ExternalArgumentKind::SlurpFile,
                &token,
            )?,
            "--rawfile" => parse_external(
                &mut tokens,
                &mut external,
                &mut external_names,
                ExternalArgumentKind::RawFile,
                &token,
            )?,
            "--args" => positional_argument_kind = Some(PositionalArgumentKind::String),
            "--jsonargs" => positional_argument_kind = Some(PositionalArgumentKind::Json),
            "-L" | "--library-path" => {
                module_paths.push(PathBuf::from(next_value(&mut tokens, &token)?));
            }
            "--argfile" | "--run-tests" => return Err(CliError::Unsupported(token)),
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
            "--hybrid-batch-values" => {
                limits.hybrid_batch_values = parse_limit(&mut tokens, &token)?;
            }
            "--hybrid-in-flight-batches" => {
                limits.hybrid_in_flight_batches = parse_limit(&mut tokens, &token)?;
            }
            "--hybrid-in-flight-bytes" => {
                limits.hybrid_in_flight_bytes = parse_limit(&mut tokens, &token)?;
            }
            "--decode-batch-values" => {
                limits.decode_batch_values = parse_limit(&mut tokens, &token)?;
            }
            "--decode-batch-bytes" => {
                limits.decode_batch_bytes = parse_limit(&mut tokens, &token)?;
            }
            "--decode-in-flight-batches" => {
                limits.decode_in_flight_batches = parse_limit(&mut tokens, &token)?;
            }
            "--decode-in-flight-bytes" => {
                limits.decode_in_flight_bytes = parse_limit(&mut tokens, &token)?;
            }
            "--max-spool-bytes" => limits.spool_bytes = parse_limit(&mut tokens, &token)?,
            "-" => positional(
                &mut inline_filter,
                &mut files,
                &mut positional_arguments,
                positional_argument_kind,
                token,
                filter_file.is_some(),
            ),
            value if value.starts_with('-') => return Err(CliError::Unsupported(token)),
            _ => positional(
                &mut inline_filter,
                &mut files,
                &mut positional_arguments,
                positional_argument_kind,
                token,
                filter_file.is_some(),
            ),
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
        (None, None) => FilterSource::Inline(".".to_owned()),
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
    if proxy_on_error && stream_errors {
        return Err(CliError::Incompatible(
            "--proxy-on-error cannot be combined with --stream-errors".to_owned(),
        ));
    }
    if output_format == OutputFormat::JsonLines {
        if pretty_explicit || indent_explicit || tab_explicit {
            return Err(CliError::Incompatible(
                "JSON Lines output is compact and cannot use pretty, indent, or tab controls"
                    .to_owned(),
            ));
        }
        if raw_output || join_output || color == ColorMode::Always {
            return Err(CliError::Incompatible(
                "JSON Lines output cannot use raw, joined, or forced-color output".to_owned(),
            ));
        }
        pretty_json = false;
        if color == ColorMode::Auto {
            color = ColorMode::Never;
        }
    }
    let mut json_compatible_writer = WriterConfig::default();
    if let JsonIndent::Spaces(indent) = json_indent {
        json_compatible_writer.indent_size = usize::from(indent);
    }
    if matches!(output_format, OutputFormat::Json | OutputFormat::JsonLines)
        && (toon_writer != json_compatible_writer || framing == ToonFraming::Unframed)
    {
        return Err(CliError::Incompatible(
            "TOON output options cannot be applied to JSON or JSON Lines output".to_owned(),
        ));
    }
    if !matches!(output_format, OutputFormat::Json | OutputFormat::JsonLines) && !pretty_json {
        return Err(CliError::Incompatible(
            "--compact-output applies only to JSON or JSON Lines output".to_owned(),
        ));
    }
    if !matches!(output_format, OutputFormat::Json | OutputFormat::JsonLines) && ascii_output {
        return Err(CliError::Incompatible(
            "--ascii-output applies only to JSON or JSON Lines output".to_owned(),
        ));
    }
    if output_format != OutputFormat::Json && color == ColorMode::Always {
        return Err(CliError::Incompatible(
            "color controls apply only to JSON output".to_owned(),
        ));
    }
    if output_format != OutputFormat::Json && json_indent == JsonIndent::Tabs {
        return Err(CliError::Incompatible(
            "--tab applies only to JSON output".to_owned(),
        ));
    }
    if output_format == OutputFormat::Yaml && indent_explicit {
        return Err(CliError::Incompatible(
            "--indent is fixed for YAML flow output".to_owned(),
        ));
    }
    if color == ColorMode::Always && !capability_policy.terminal {
        return Err(CliError::Incompatible(
            "--color-output is disabled by terminal capability policy".to_owned(),
        ));
    }
    if allow_environment && !capability_policy.environment {
        return Err(CliError::Incompatible(
            "environment access is disabled by capability policy".to_owned(),
        ));
    }
    if allow_platform && !capability_policy.platform {
        return Err(CliError::Incompatible(
            "platform access is disabled by capability policy".to_owned(),
        ));
    }
    if (raw_output || join_output) && framing == ToonFraming::Unframed {
        return Err(CliError::Incompatible(
            "raw output has its own separators and cannot be --unframed".to_owned(),
        ));
    }
    let uses_filesystem = matches!(&filter, FilterSource::File(_))
        || files.iter().any(|path| path != std::path::Path::new("-"))
        || !module_paths.is_empty()
        || report_file.is_some()
        || external.iter().any(|argument| {
            matches!(
                argument.kind,
                ExternalArgumentKind::RawFile | ExternalArgumentKind::SlurpFile
            )
        });
    if uses_filesystem && !capability_policy.filesystem {
        return Err(CliError::Incompatible(
            "filesystem access is disabled by capability policy".to_owned(),
        ));
    }

    Ok(Command::Run(Box::new(RunOptions {
        execution_override: ExecutionOverride::Automatic,
        filter,
        files,
        module_paths,
        input_format,
        output_format,
        framing,
        raw_output,
        join_output,
        raw_output0,
        null_input,
        raw_input,
        slurp,
        stream,
        stream_errors,
        proxy_on_error,
        exit_status,
        strict,
        pretty_json,
        json_indent,
        ascii_output,
        sort_keys,
        color,
        unbuffered,
        binary,
        allow_environment,
        allow_platform,
        toon_writer,
        arguments: external,
        positional_argument_kind,
        positional_arguments,
        explain,
        trace_limit,
        report_file,
        limits,
        capability_policy,
    })))
}

fn expand_short_options(tokens: Vec<String>) -> Result<Vec<String>, CliError> {
    let mut expanded = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token == "-" || token == "--" || !token.starts_with('-') || token.starts_with("--") {
            expanded.push(token);
            continue;
        }
        let mut characters = token[1..].char_indices().peekable();
        if characters.peek().is_none() {
            expanded.push(token);
            continue;
        }
        for (offset, short) in characters {
            let Some(spec) = OPTION_REGISTRY
                .iter()
                .find(|option| option.short == Some(short))
            else {
                return Err(CliError::Unsupported(format!("-{short}")));
            };
            expanded.push(format!("-{short}"));
            if spec.value {
                let value_start = offset + short.len_utf8();
                if value_start < token.len() - 1 {
                    expanded.push(token[1 + value_start..].to_owned());
                }
                break;
            }
        }
    }
    Ok(expanded)
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
    positional_arguments: &mut Vec<String>,
    positional_argument_kind: Option<PositionalArgumentKind>,
    token: String,
    has_filter_file: bool,
) {
    if filter.is_none() && !has_filter_file {
        *filter = Some(token);
    } else if positional_argument_kind.is_some() {
        positional_arguments.push(token);
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
        "jsonl" | "ndjson" => Ok(InputFormat::JsonLines),
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
        "jsonl" | "ndjson" => Ok(OutputFormat::JsonLines),
        "yaml" | "yml" => Ok(OutputFormat::Yaml),
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

    use super::{
        CapabilityPolicy, CliError, ColorMode, Command, FilterSource, generated_help, parse_args,
        parse_args_with_policy,
    };

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

        let Command::Run(run) = parse_args(Vec::<&str>::new()).unwrap() else {
            panic!("run command")
        };
        assert_eq!(run.filter, FilterSource::Inline(".".to_owned()));
        assert!(run.files.is_empty());
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
            "-x",
            ".",
        ])
        .unwrap() else {
            panic!("run command")
        };
        assert_eq!(run.input_format, InputFormat::Yaml);
        assert_eq!(run.output_format, OutputFormat::Json);
        assert_eq!(run.framing, ToonFraming::Sequence);
        assert!(run.raw_output && run.slurp && run.exit_status && run.proxy_on_error);
    }

    #[test]
    fn parses_short_format_options_with_separate_or_attached_values() {
        let Command::Run(run) = parse_args(["-i", "yaml", "-ojson", "."]).unwrap() else {
            panic!("run command")
        };
        assert_eq!(run.input_format, InputFormat::Yaml);
        assert_eq!(run.output_format, OutputFormat::Json);

        for alias in ["jsonl", "ndjson"] {
            let Command::Run(run) = parse_args(["-i", alias, "-o", alias, "."]).unwrap() else {
                panic!("run command")
            };
            assert_eq!(run.input_format, InputFormat::JsonLines);
            assert_eq!(run.output_format, OutputFormat::JsonLines);
            assert!(!run.pretty_json);
            assert_eq!(run.color, ColorMode::Never);
        }

        let Command::Run(run) =
            parse_args(["-ijsonl", "-ondjson", "-acSM", "--unbuffered", "."]).unwrap()
        else {
            panic!("run command")
        };
        assert_eq!(run.input_format, InputFormat::JsonLines);
        assert_eq!(run.output_format, OutputFormat::JsonLines);
        assert!(run.ascii_output && run.sort_keys);
        assert!(run.unbuffered);
    }

    #[test]
    fn json_lines_rejects_output_modes_that_break_record_framing() {
        for options in [
            &["-o", "jsonl", "--pretty-output", "."][..],
            &["--indent", "4", "-o", "jsonl", "."][..],
            &["-o", "ndjson", "--tab", "."][..],
            &["-r", "-o", "jsonl", "."][..],
            &["-o", "jsonl", "-j", "."][..],
            &["-C", "-o", "jsonl", "."][..],
        ] {
            assert!(matches!(
                parse_args(options),
                Err(CliError::Incompatible(_))
            ));
        }
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
        assert!(parse_args(["--proxy-on-error", "--stream-errors", "."]).is_err());
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

    #[test]
    fn registry_expands_short_clusters_and_generates_admitted_help() {
        let Command::Run(run) = parse_args(["--output-format", "json", "-nacSM", "."]).unwrap()
        else {
            panic!("run command")
        };
        assert!(run.null_input && run.ascii_output && run.sort_keys);
        assert!(!run.pretty_json);
        assert_eq!(run.color, ColorMode::Never);

        let help = generated_help();
        for option in [
            "--raw-output0",
            "--slurpfile",
            "--jsonargs",
            "--unbuffered",
            "--proxy-on-error",
        ] {
            assert!(help.contains(option));
        }
    }

    #[test]
    fn ambient_integrations_obey_library_capability_policy() {
        let denied_files = CapabilityPolicy {
            filesystem: false,
            ..CapabilityPolicy::default()
        };
        assert!(matches!(
            parse_args_with_policy([".", "input.json"], denied_files),
            Err(CliError::Incompatible(message)) if message.contains("filesystem")
        ));
        assert!(matches!(
            parse_args_with_policy(["-L", "modules", "."], denied_files),
            Err(CliError::Incompatible(message)) if message.contains("filesystem")
        ));

        let denied_terminal = CapabilityPolicy {
            terminal: false,
            ..CapabilityPolicy::default()
        };
        assert!(matches!(
            parse_args_with_policy(["--output-format", "json", "-C", "."], denied_terminal),
            Err(CliError::Incompatible(message)) if message.contains("terminal")
        ));

        let denied_environment = CapabilityPolicy {
            environment: false,
            ..CapabilityPolicy::default()
        };
        assert!(matches!(
            parse_args_with_policy(["--allow-environment", "env"], denied_environment),
            Err(CliError::Incompatible(message)) if message.contains("environment")
        ));

        let denied_platform = CapabilityPolicy {
            platform: false,
            ..CapabilityPolicy::default()
        };
        assert!(matches!(
            parse_args_with_policy(["--allow-platform", "now"], denied_platform),
            Err(CliError::Incompatible(message)) if message.contains("platform")
        ));

        let Command::Run(run) = parse_args(["--allow-environment", "--allow-platform", "."])
            .expect("ambient admission flags")
        else {
            panic!("run command")
        };
        assert!(run.allow_environment && run.allow_platform);
    }
}
