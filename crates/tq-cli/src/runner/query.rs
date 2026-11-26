//! Load query source, module roots, and external argument values.

use super::input::{open_path, read_limited, toon_decoder_config};
use super::{AMBIENT_ENVIRONMENT, AMBIENT_PLATFORM, INPUT_FILENAME, INPUT_LINE_NUMBER, RunError};
use crate::{
    CliError, ColorMode, ExternalArgumentKind, FilterSource, PositionalArgumentKind, RunOptions,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tq_core::{Number, Value};
use tq_formats::{FormatError, InputFormat, decode_json, decode_toon};

pub(super) fn validate_capability_policy(options: &RunOptions) -> Result<(), RunError> {
    let uses_filesystem = matches!(&options.filter, FilterSource::File(_))
        || options.files.iter().any(|path| path != Path::new("-"))
        || !options.module_paths.is_empty()
        || options.report_file.is_some()
        || options.arguments.iter().any(|argument| {
            matches!(
                argument.kind,
                ExternalArgumentKind::RawFile | ExternalArgumentKind::SlurpFile
            )
        });
    if uses_filesystem && !options.capability_policy.filesystem {
        return Err(CliError::Incompatible(
            "filesystem access is disabled by capability policy".to_owned(),
        )
        .into());
    }
    if options.color == ColorMode::Always && !options.capability_policy.terminal {
        return Err(CliError::Incompatible(
            "--color-output is disabled by terminal capability policy".to_owned(),
        )
        .into());
    }
    if options.allow_environment && !options.capability_policy.environment {
        return Err(CliError::Incompatible(
            "environment access is disabled by capability policy".to_owned(),
        )
        .into());
    }
    if options.allow_platform && !options.capability_policy.platform {
        return Err(CliError::Incompatible(
            "platform access is disabled by capability policy".to_owned(),
        )
        .into());
    }
    Ok(())
}
#[allow(
    clippy::type_complexity,
    reason = "filter loading returns the query and optional startup source together"
)]
pub(super) fn load_filter(
    options: &RunOptions,
) -> Result<(String, Vec<u8>, Option<(String, Vec<u8>)>), RunError> {
    let (identity, query) = match &options.filter {
        FilterSource::Inline(query) => ("<command-line>".to_owned(), query.as_bytes().to_vec()),
        FilterSource::File(path) => {
            let identity = path.display().to_string();
            (
                identity.clone(),
                read_limited(open_path(path)?, options.limits.input_bytes, &identity)?,
            )
        }
    };
    let startup_source = if options.allow_environment
        && options.capability_policy.environment
        && options.capability_policy.filesystem
    {
        if let Some(home) = std::env::var_os("HOME") {
            let startup = PathBuf::from(home).join(".jq");
            if startup.is_file() {
                let startup_identity = startup.display().to_string();
                let startup = read_limited(
                    open_path(&startup)?,
                    options.limits.input_bytes,
                    &startup_identity,
                )?;
                let total = startup.len().saturating_add(1).saturating_add(query.len());
                if u64::try_from(total).unwrap_or(u64::MAX) > options.limits.input_bytes {
                    return Err(RunError::ResourceSource {
                        identity: startup_identity,
                        resource: "input-bytes",
                    });
                }
                Some((startup_identity, startup))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    Ok((identity, query, startup_source))
}

#[derive(Default)]
pub(super) struct AmbientModuleRoots {
    pub(super) home: Option<PathBuf>,
    pub(super) library_path: Option<std::ffi::OsString>,
}

pub(super) fn module_roots(options: &RunOptions, query_name: &str) -> Vec<PathBuf> {
    if !options.capability_policy.filesystem {
        return Vec::new();
    }
    let ambient = (options.allow_environment && options.capability_policy.environment).then(|| {
        AmbientModuleRoots {
            home: std::env::var_os("HOME").map(PathBuf::from),
            library_path: std::env::var_os("JQ_LIBRARY_PATH"),
        }
    });
    module_roots_with_ambient(options, query_name, ambient)
}

pub(super) fn module_roots_with_ambient(
    options: &RunOptions,
    query_name: &str,
    ambient: Option<AmbientModuleRoots>,
) -> Vec<PathBuf> {
    let ambient = (options.allow_environment && options.capability_policy.environment)
        .then_some(ambient)
        .flatten();
    let origin = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let home = ambient.as_ref().and_then(|ambient| ambient.home.as_ref());
    let substitute = |path: &Path| {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix("$ORIGIN/") {
            return origin
                .as_ref()
                .map_or_else(|| path.to_owned(), |origin| origin.join(rest));
        }
        if let Some(rest) = text.strip_prefix("~/") {
            return home
                .as_ref()
                .map_or_else(|| path.to_owned(), |home| home.join(rest));
        }
        path.to_owned()
    };

    if !options.module_paths.is_empty() {
        return options
            .module_paths
            .iter()
            .map(|path| substitute(path))
            .collect();
    }

    let mut roots = Vec::new();
    if let FilterSource::File(path) = &options.filter {
        if let Some(parent) = path.parent() {
            roots.push(parent.to_path_buf());
        }
    } else if query_name == "<command-line>" {
        roots.push(PathBuf::from("."));
    }
    if let Some(paths) = ambient
        .as_ref()
        .and_then(|ambient| ambient.library_path.as_deref())
    {
        roots.extend(std::env::split_paths(&paths));
    }
    if let Some(home) = &home {
        roots.push(home.join(".jq"));
    }
    if let Some(origin) = origin.clone() {
        roots.push(origin.join("../lib/jq"));
        roots.push(origin.join("../lib"));
    }
    // Ambient jq roots are optional: a missing HOME or installation path must
    // not make ordinary filters fail during module-root canonicalization.
    // Explicit `-L` roots returned above remain strict and are canonicalized
    // by the resolver so misspellings are still diagnosed.
    roots
        .into_iter()
        .map(|path| substitute(&path))
        .filter(|path| path.is_dir())
        .collect()
}

pub(super) fn parse_external_arguments(
    options: &RunOptions,
) -> Result<BTreeMap<Arc<str>, Value>, RunError> {
    let mut values = BTreeMap::new();
    let mut named = tq_core::Object::new();
    for argument in &options.arguments {
        validate_external_argument_name(&argument.name)?;
        let value = match argument.kind {
            ExternalArgumentKind::String => Value::string(argument.value.as_str()),
            ExternalArgumentKind::Json => {
                decode_json_argument(argument.value.as_bytes(), "--argjson")?
            }
            ExternalArgumentKind::Toon => {
                let config = toon_decoder_config(options);
                decode_toon(argument.value.as_bytes(), "--argtoon", config)?
                    .pop()
                    .ok_or_else(|| RunError::Unsupported("--argtoon produced no value".to_owned()))?
                    .value
            }
            ExternalArgumentKind::RawFile => {
                let bytes = read_limited(
                    open_path(Path::new(&argument.value))?,
                    options.limits.input_bytes,
                    &argument.value,
                )?;
                Value::string(String::from_utf8_lossy(&bytes).into_owned())
            }
            ExternalArgumentKind::SlurpFile => {
                let bytes = read_limited(
                    open_path(Path::new(&argument.value))?,
                    options.limits.input_bytes,
                    &argument.value,
                )?;
                let documents =
                    decode_json(&bytes, &argument.value).map_err(|error| match error {
                        FormatError::Parse { .. } => RunError::Cli(CliError::Usage(format!(
                            "--slurpfile '{}' requires valid JSON values",
                            argument.value
                        ))),
                        other => RunError::Input(other),
                    })?;
                Value::array(
                    documents
                        .into_iter()
                        .map(|document| document.value)
                        .collect::<Vec<_>>(),
                )
            }
        };
        named.insert(Arc::from(argument.name.as_str()), value.clone());
        values.insert(Arc::from(argument.name.as_str()), value);
    }
    let positional = options
        .positional_arguments
        .iter()
        .map(|argument| match argument.kind {
            PositionalArgumentKind::String => Ok(Value::string(argument.value.as_str())),
            PositionalArgumentKind::Json => {
                decode_json_argument(argument.value.as_bytes(), "--jsonargs")
            }
        })
        .collect::<Result<Vec<_>, RunError>>()?;
    let arguments = tq_core::Object::from_iter([
        (Arc::from("named"), Value::object(named)),
        (Arc::from("positional"), Value::array(positional)),
    ]);
    values.insert(Arc::from("ARGS"), Value::object(arguments));
    if options.allow_environment && options.capability_policy.environment {
        let environment = std::env::vars_os()
            .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .map(|(name, value)| (Arc::from(name), Value::string(value)))
            .collect::<tq_core::Object>();
        let environment = Value::object(environment);
        values.insert(Arc::from(AMBIENT_ENVIRONMENT), environment.clone());
        values.insert(Arc::from("ENV"), environment);
    }
    values.insert(
        Arc::from(AMBIENT_PLATFORM),
        Value::Bool(options.allow_platform && options.capability_policy.platform),
    );
    let input_filename = if options.null_input {
        "<null-input>".to_owned()
    } else {
        options
            .files
            .first()
            .filter(|path| *path != Path::new("-"))
            .map_or_else(|| "<stdin>".to_owned(), |path| path.display().to_string())
    };
    values.insert(Arc::from(INPUT_FILENAME), Value::string(input_filename));
    values.insert(
        Arc::from(INPUT_LINE_NUMBER),
        Value::Number(
            Number::parse(if options.null_input { "0" } else { "1" })
                .expect("an admitted source line number"),
        ),
    );
    Ok(values)
}

fn decode_json_argument(bytes: &[u8], option: &str) -> Result<Value, RunError> {
    decode_single_json(bytes, option).map_err(|error| match error {
        RunError::Input(FormatError::Parse { .. }) => RunError::Cli(CliError::Usage(format!(
            "{option} requires one valid JSON value"
        ))),
        other => other,
    })
}

fn validate_external_argument_name(name: &str) -> Result<(), RunError> {
    if name.starts_with("__tq_") {
        return Err(CliError::Usage(
            "external variable names beginning with '__tq_' are reserved".to_owned(),
        )
        .into());
    }
    Ok(())
}

fn decode_single_json(bytes: &[u8], identity: &str) -> Result<Value, RunError> {
    let mut documents = decode_json(bytes, identity)?;
    if documents.len() != 1 {
        return Err(RunError::Input(FormatError::Parse {
            format: InputFormat::Json,
            message: format!("{identity} requires exactly one JSON value"),
        }));
    }
    Ok(documents.pop().expect("one document").value)
}
