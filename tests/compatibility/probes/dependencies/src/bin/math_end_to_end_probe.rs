#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sha2::{Digest, Sha256};

const PINNED_MACOS_JQ_SHA256: &str =
    "a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603";
const PINNED_LINUX_JQ_SHA256: &str =
    "136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97";
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MAX_TIMEOUT_MS: u64 = 60_000;

struct Sample {
    label: &'static str,
    value: f64,
}

struct Function {
    name: &'static str,
    query: &'static str,
    inputs: &'static [Sample],
    apply: fn(f64) -> f64,
}

struct Config {
    jq: PathBuf,
    tq: PathBuf,
    output: PathBuf,
    timeout: Duration,
}

struct ProcessResult {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

struct RuntimeLibraryHash {
    path: &'static str,
    sha256: String,
}

struct OwnedTempDir {
    path: PathBuf,
}

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn probe_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message.into(),
    ))
}

const ACOS_INPUTS: &[Sample] = &[
    Sample {
        label: "-2",
        value: -2.0,
    },
    Sample {
        label: "-1",
        value: -1.0,
    },
    Sample {
        label: "-0.9999999999999999",
        value: -0.9999999999999999,
    },
    Sample {
        label: "-0.5",
        value: -0.5,
    },
    Sample {
        label: "-0",
        value: 0.0,
    },
    Sample {
        label: "0",
        value: 0.0,
    },
    Sample {
        label: "0.5",
        value: 0.5,
    },
    Sample {
        label: "0.9999999999999999",
        value: 0.9999999999999999,
    },
    Sample {
        label: "1",
        value: 1.0,
    },
    Sample {
        label: "2",
        value: 2.0,
    },
    Sample {
        label: "nan",
        value: f64::NAN,
    },
    Sample {
        label: "infinite",
        value: f64::INFINITY,
    },
    Sample {
        label: "-infinite",
        value: f64::NEG_INFINITY,
    },
];

const EXP_INPUTS: &[Sample] = &[
    Sample {
        label: "-infinite",
        value: f64::NEG_INFINITY,
    },
    Sample {
        label: "-1000",
        value: -1000.0,
    },
    Sample {
        label: "-745",
        value: -745.0,
    },
    Sample {
        label: "-744",
        value: -744.0,
    },
    Sample {
        label: "-709",
        value: -709.0,
    },
    Sample {
        label: "-100",
        value: -100.0,
    },
    Sample {
        label: "-1",
        value: -1.0,
    },
    Sample {
        label: "-0",
        value: 0.0,
    },
    Sample {
        label: "0",
        value: 0.0,
    },
    Sample {
        label: "1",
        value: 1.0,
    },
    Sample {
        label: "10",
        value: 10.0,
    },
    Sample {
        label: "100",
        value: 100.0,
    },
    Sample {
        label: "709",
        value: 709.0,
    },
    Sample {
        label: "710",
        value: 710.0,
    },
    Sample {
        label: "1000",
        value: 1000.0,
    },
    Sample {
        label: "infinite",
        value: f64::INFINITY,
    },
    Sample {
        label: "nan",
        value: f64::NAN,
    },
];

const ERFC_INPUTS: &[Sample] = &[
    Sample {
        label: "-infinite",
        value: f64::NEG_INFINITY,
    },
    Sample {
        label: "-30",
        value: -30.0,
    },
    Sample {
        label: "-10",
        value: -10.0,
    },
    Sample {
        label: "-2",
        value: -2.0,
    },
    Sample {
        label: "-1",
        value: -1.0,
    },
    Sample {
        label: "-0",
        value: 0.0,
    },
    Sample {
        label: "0",
        value: 0.0,
    },
    Sample {
        label: "0.5",
        value: 0.5,
    },
    Sample {
        label: "1",
        value: 1.0,
    },
    Sample {
        label: "2",
        value: 2.0,
    },
    Sample {
        label: "5",
        value: 5.0,
    },
    Sample {
        label: "10",
        value: 10.0,
    },
    Sample {
        label: "20",
        value: 20.0,
    },
    Sample {
        label: "26",
        value: 26.0,
    },
    Sample {
        label: "27",
        value: 27.0,
    },
    Sample {
        label: "30",
        value: 30.0,
    },
    Sample {
        label: "infinite",
        value: f64::INFINITY,
    },
    Sample {
        label: "nan",
        value: f64::NAN,
    },
];

const TGAMMA_INPUTS: &[Sample] = &[
    Sample {
        label: "-infinite",
        value: f64::NEG_INFINITY,
    },
    Sample {
        label: "-10",
        value: -10.0,
    },
    Sample {
        label: "-2.5",
        value: -2.5,
    },
    Sample {
        label: "-1",
        value: -1.0,
    },
    Sample {
        label: "-0.5",
        value: -0.5,
    },
    Sample {
        label: "-0",
        value: 0.0,
    },
    Sample {
        label: "0",
        value: 0.0,
    },
    Sample {
        label: r#"("-0.0"|tonumber)"#,
        value: -0.0,
    },
    Sample {
        label: "0.5",
        value: 0.5,
    },
    Sample {
        label: "1",
        value: 1.0,
    },
    Sample {
        label: "1.5",
        value: 1.5,
    },
    Sample {
        label: "2",
        value: 2.0,
    },
    Sample {
        label: "10",
        value: 10.0,
    },
    Sample {
        label: "100",
        value: 100.0,
    },
    Sample {
        label: "171",
        value: 171.0,
    },
    Sample {
        label: "172",
        value: 172.0,
    },
    Sample {
        label: "infinite",
        value: f64::INFINITY,
    },
    Sample {
        label: "nan",
        value: f64::NAN,
    },
];

const Y0_INPUTS: &[Sample] = &[
    Sample {
        label: "-1",
        value: -1.0,
    },
    Sample {
        label: "0",
        value: 0.0,
    },
    Sample {
        label: "1e-300",
        value: 1e-300,
    },
    Sample {
        label: "1e-320",
        value: 1e-320,
    },
    Sample {
        label: "1e-10",
        value: 1e-10,
    },
    Sample {
        label: "0.5",
        value: 0.5,
    },
    Sample {
        label: "0.9999999999999999",
        value: 0.9999999999999999,
    },
    Sample {
        label: "1",
        value: 1.0,
    },
    Sample {
        label: "2",
        value: 2.0,
    },
    Sample {
        label: "10",
        value: 10.0,
    },
    Sample {
        label: "1e10",
        value: 1e10,
    },
    Sample {
        label: "infinite",
        value: f64::INFINITY,
    },
    Sample {
        label: "nan",
        value: f64::NAN,
    },
];

const YN_INPUTS: &[Sample] = Y0_INPUTS;

fn yn_zero(value: f64) -> f64 {
    libm::yn(0, value)
}

fn functions() -> [Function; 6] {
    [
        Function {
            name: "acos",
            query: "acos",
            inputs: ACOS_INPUTS,
            apply: libm::acos,
        },
        Function {
            name: "exp",
            query: "exp",
            inputs: EXP_INPUTS,
            apply: libm::exp,
        },
        Function {
            name: "erfc",
            query: "erfc",
            inputs: ERFC_INPUTS,
            apply: libm::erfc,
        },
        Function {
            name: "tgamma",
            query: "tgamma",
            inputs: TGAMMA_INPUTS,
            apply: libm::tgamma,
        },
        Function {
            name: "y0",
            query: "y0",
            inputs: Y0_INPUTS,
            apply: libm::y0,
        },
        Function {
            name: "yn",
            query: "yn(0; .)",
            inputs: YN_INPUTS,
            apply: yn_zero,
        },
    ]
}

fn expected_jq_sha256() -> Result<&'static str, Box<dyn Error>> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Ok(PINNED_MACOS_JQ_SHA256)
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        Ok(PINNED_LINUX_JQ_SHA256)
    } else {
        Err(probe_error(
            "math end-to-end probe supports only pinned aarch64 macOS or x86_64 GNU/Linux hosts",
        ))
    }
}

const LINUX_RUNTIME_LIBRARIES: &[(&str, &str)] = &[
    (
        "/usr/lib64/libjq.so.1",
        "5de9294d71f67b22f56889348dc424c45ae2167065d5e766eea30ff8d3b011ce",
    ),
    (
        "/usr/lib64/libonig.so.5",
        "73a1423e3d1c5f8d1f076376cbbfcf8e4f9dbb2a683121bdfd9353eb714ab7f9",
    ),
    (
        "/usr/lib64/libc.so.6",
        "01cccbe278d898add05282986a9346d4dda4b3d2b84bc496f9c04c66016528db",
    ),
    (
        "/usr/lib64/libm.so.6",
        "6f7365a58fc0778dbc8869d70cdd4822ab93d879c29e84bb491d447961735e56",
    ),
    (
        "/lib64/ld-linux-x86-64.so.2",
        "f0949392f14253241540dfeb152f5bd3cbd75f8a3e4f8d3fabc286a2daa27a68",
    ),
];

fn verify_runtime_libraries() -> Result<Vec<RuntimeLibraryHash>, Box<dyn Error>> {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Ok(Vec::new());
    }
    LINUX_RUNTIME_LIBRARIES
        .iter()
        .map(|(path, expected)| {
            let digest = sha256(Path::new(path)).map_err(|error| {
                probe_error(format!("required Linux jq runtime library {path} is unavailable: {error}"))
            })?;
            if digest != *expected {
                return Err(probe_error(format!(
                    "Linux jq runtime library {path} has unexpected SHA-256: expected {expected}, got {digest}"
                )));
            }
            Ok(RuntimeLibraryHash {
                path,
                sha256: digest,
            })
        })
        .collect()
}

fn usage() -> &'static str {
    "usage: math_end_to_end_probe --jq ABSOLUTE_PINNED_JQ --tq ABSOLUTE_IMMUTABLE_TQ --output ABSOLUTE_TOON [--timeout-ms N]"
}

fn parse_config() -> Result<Config, Box<dyn Error>> {
    let mut jq = None;
    let mut tq = None;
    let mut output = None;
    let mut timeout_ms = DEFAULT_TIMEOUT_MS;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--jq" => {
                jq = Some(PathBuf::from(
                    arguments.next().ok_or_else(|| probe_error(usage()))?,
                ));
            }
            "--tq" => {
                tq = Some(PathBuf::from(
                    arguments.next().ok_or_else(|| probe_error(usage()))?,
                ));
            }
            "--output" => {
                output = Some(PathBuf::from(
                    arguments.next().ok_or_else(|| probe_error(usage()))?,
                ));
            }
            "--timeout-ms" => {
                timeout_ms = arguments
                    .next()
                    .ok_or_else(|| probe_error(usage()))?
                    .parse()?;
                if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
                    return Err(probe_error(format!(
                        "timeout must be between 1 and {MAX_TIMEOUT_MS} ms"
                    )));
                }
            }
            "--help" | "-h" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            _ => {
                return Err(probe_error(format!(
                    "unknown argument {argument:?}; {}",
                    usage()
                )));
            }
        }
    }

    let jq = jq.ok_or_else(|| probe_error(usage()))?;
    let tq = tq.ok_or_else(|| probe_error(usage()))?;
    let output = output.ok_or_else(|| probe_error(usage()))?;
    for (name, path) in [("jq", &jq), ("tq", &tq), ("output", &output)] {
        if !path.is_absolute() {
            return Err(probe_error(format!(
                "{name} path must be absolute: {}",
                path.display()
            )));
        }
    }
    Ok(Config {
        jq: canonical_executable(&jq, "jq")?,
        tq: canonical_executable(&tq, "tq")?,
        output,
        timeout: Duration::from_millis(timeout_ms),
    })
}

fn canonical_executable(path: &Path, name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let canonical = fs::canonicalize(path)?;
    if !canonical.is_file() {
        return Err(probe_error(format!(
            "{name} is not a file: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn fresh_home() -> Result<OwnedTempDir, Box<dyn Error>> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| probe_error(format!("system clock is before UNIX_EPOCH: {error}")))?
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("tq-math-end-to-end-{}-{stamp}", std::process::id()));
    fs::create_dir(&path)?;
    Ok(OwnedTempDir { path })
}

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let digest = Sha256::digest(fs::read(path)?);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn run_process(
    executable: &Path,
    arguments: &[&str],
    timeout: Duration,
    home: &Path,
) -> Result<ProcessResult, Box<dyn Error>> {
    let mut child = Command::new(executable)
        .args(arguments)
        .env_clear()
        .env("HOME", home)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("TZ", "UTC")
        .env("RAYON_NUM_THREADS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            child.kill()?;
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let output = child.wait_with_output()?;
    Ok(ProcessResult {
        status: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
        timed_out,
    })
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("JSON string quoting cannot fail")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn projected(value: f64) -> Option<f64> {
    if value.is_nan() {
        None
    } else if value.is_infinite() {
        Some(value.signum() * f64::MAX)
    } else {
        Some(value)
    }
}

fn json_value(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn ordered_bits(value: f64) -> u64 {
    let bits = value.to_bits();
    if bits >> 63 == 0 {
        bits | (1 << 63)
    } else {
        !bits
    }
}

fn ulp_distance(left: Option<f64>, right: Option<f64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) if left.is_finite() && right.is_finite() => {
            Some(ordered_bits(left).abs_diff(ordered_bits(right)))
        }
        _ => None,
    }
}

fn parse_output(
    process: &ProcessResult,
    function: &Function,
    executable_name: &str,
    issues: &mut Vec<String>,
) -> Option<Vec<Value>> {
    if process.timed_out {
        issues.push(format!("{executable_name}/{} timed out", function.name));
    }
    if process.status != Some(0) {
        issues.push(format!(
            "{executable_name}/{} exited with {:?}",
            function.name, process.status
        ));
    }
    if !process.stderr.is_empty() {
        issues.push(format!(
            "{executable_name}/{} emitted stderr",
            function.name
        ));
    }
    match serde_json::from_slice::<Value>(&process.stdout) {
        Ok(Value::Array(values)) => {
            if values.len() != function.inputs.len() {
                issues.push(format!(
                    "{executable_name}/{} returned {} results, expected {}",
                    function.name,
                    values.len(),
                    function.inputs.len()
                ));
            }
            for (index, value) in values.iter().enumerate() {
                if !value.is_null() && !value.is_number() {
                    issues.push(format!(
                        "{executable_name}/{} result {index} is neither a number nor null: {value:?}",
                        function.name
                    ));
                }
            }
            Some(values)
        }
        Ok(other) => {
            issues.push(format!(
                "{executable_name}/{} returned non-array JSON {other:?}",
                function.name
            ));
            None
        }
        Err(error) => {
            issues.push(format!(
                "{executable_name}/{} returned invalid JSON: {error}",
                function.name
            ));
            None
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let config = parse_config()?;
    let parent = config
        .output
        .parent()
        .ok_or_else(|| probe_error("output path has no parent directory"))?;
    if !parent.is_dir() {
        return Err(probe_error(format!(
            "output parent does not exist: {}",
            parent.display()
        )));
    }
    let mut output_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config.output)?;
    let expected_jq_sha256 = expected_jq_sha256()?;
    let runtime_before = verify_runtime_libraries()?;
    let jq_sha256 = sha256(&config.jq)?;
    if jq_sha256 != expected_jq_sha256 {
        return Err(probe_error(format!(
            "--jq is not the pinned jq 1.8.1 executable for this target: expected {expected_jq_sha256}, got {jq_sha256}"
        )));
    }
    let tq_sha256 = sha256(&config.tq)?;
    let home = fresh_home()?;
    let mut report = String::new();
    writeln!(report, "schema_version: 1")?;
    writeln!(report, "probe: math-end-to-end")?;
    writeln!(report, "target:")?;
    writeln!(report, "  arch: {}", quote(std::env::consts::ARCH))?;
    writeln!(report, "  os: {}", quote(std::env::consts::OS))?;
    writeln!(report, "  locale: \"C\"")?;
    writeln!(report, "  timezone: \"UTC\"")?;
    writeln!(report, "  timeout_ms: {}", config.timeout.as_millis())?;
    writeln!(report, "tools:")?;
    writeln!(report, "  jq:")?;
    writeln!(
        report,
        "    path: {}",
        quote(&config.jq.display().to_string())
    )?;
    writeln!(report, "    sha256: {}", quote(&jq_sha256))?;
    writeln!(
        report,
        "    expected_sha256: {}",
        quote(expected_jq_sha256)
    )?;
    writeln!(
        report,
        "  runtime_libraries_before[{}]:",
        runtime_before.len()
    )?;
    for library in &runtime_before {
        writeln!(
            report,
            "    - path: {}",
            quote(library.path)
        )?;
        writeln!(report, "      sha256: {}", quote(&library.sha256))?;
    }
    writeln!(report, "  tq:")?;
    writeln!(
        report,
        "    path: {}",
        quote(&config.tq.display().to_string())
    )?;
    writeln!(report, "    sha256: {}", quote(&tq_sha256))?;
    writeln!(report, "  dependencies:")?;
    writeln!(report, "    libm: \"0.2.16\"")?;
    writeln!(report, "    serde_json: \"1.0.151\"")?;
    writeln!(report, "functions[6]:")?;

    let mut issues = Vec::new();
    let mut total_inputs = 0usize;
    let mut exact_samples = 0usize;
    let mut differing_samples = 0usize;
    let mut process_runs = 0usize;
    let mut max_ulps = Vec::new();

    for function in functions() {
        total_inputs += function.inputs.len();
        let query = format!(
            "[{}] | map({})",
            function
                .inputs
                .iter()
                .map(|sample| sample.label)
                .collect::<Vec<_>>()
                .join(","),
            function.query
        );
        let arguments = ["--null-input", "--compact-output", query.as_str()];
        let jq = run_process(&config.jq, &arguments, config.timeout, &home.path)?;
        let tq = run_process(&config.tq, &arguments, config.timeout, &home.path)?;
        process_runs += 2;
        let jq_values = parse_output(&jq, &function, "jq", &mut issues);
        let tq_values = parse_output(&tq, &function, "tq", &mut issues);
        let mut function_max_ulp = 0u64;
        writeln!(report, "  - name: {}", function.name)?;
        writeln!(report, "    query: {}", quote(&query))?;
        writeln!(report, "    input_count: {}", function.inputs.len())?;
        write_process(&mut report, "jq", &jq)?;
        write_process(&mut report, "tq", &tq)?;
        writeln!(report, "    samples[{}]:", function.inputs.len())?;
        for (index, sample) in function.inputs.iter().enumerate() {
            let jq_value = jq_values
                .as_ref()
                .and_then(|values| values.get(index))
                .and_then(Value::as_f64);
            let tq_value = tq_values
                .as_ref()
                .and_then(|values| values.get(index))
                .and_then(Value::as_f64);
            let jq_text = jq_values
                .as_ref()
                .and_then(|values| values.get(index))
                .map_or_else(|| "<missing>".to_owned(), json_text);
            let tq_text = tq_values
                .as_ref()
                .and_then(|values| values.get(index))
                .map_or_else(|| "<missing>".to_owned(), json_text);
            let safe = projected((function.apply)(sample.value));
            let jq_ulp = ulp_distance(jq_value, safe);
            let tq_ulp = ulp_distance(tq_value, safe);
            let tq_vs_jq = ulp_distance(tq_value, jq_value);
            if jq_text == tq_text {
                exact_samples += 1;
            } else {
                differing_samples += 1;
            }
            if let Some(distance) = tq_vs_jq {
                function_max_ulp = function_max_ulp.max(distance);
            }
            writeln!(report, "      - label: {}", quote(sample.label))?;
            writeln!(report, "        jq: {}", quote(&jq_text))?;
            writeln!(report, "        tq: {}", quote(&tq_text))?;
            writeln!(report, "        libm: {}", quote(&json_value(safe)))?;
            write_optional_number(&mut report, "        jq_libm_ulp", jq_ulp)?;
            write_optional_number(&mut report, "        tq_libm_ulp", tq_ulp)?;
            write_optional_number(&mut report, "        tq_jq_ulp", tq_vs_jq)?;
        }
        max_ulps.push((function.name, function_max_ulp));
    }

    let post_run_jq_sha256 = sha256(&config.jq)?;
    let post_run_tq_sha256 = sha256(&config.tq)?;
    let runtime_after = match verify_runtime_libraries() {
        Ok(libraries) => libraries,
        Err(error) => {
            issues.push(error.to_string());
            Vec::new()
        }
    };
    if post_run_jq_sha256 != jq_sha256 {
        issues.push(format!(
            "jq executable changed during probe: before {jq_sha256}, after {post_run_jq_sha256}"
        ));
    }
    if post_run_tq_sha256 != tq_sha256 {
        issues.push(format!(
            "tq executable changed during probe: before {tq_sha256}, after {post_run_tq_sha256}"
        ));
    }
    if runtime_after.len() != runtime_before.len()
        || runtime_after
            .iter()
            .zip(&runtime_before)
            .any(|(after, before)| after.path != before.path || after.sha256 != before.sha256)
    {
        issues.push("Linux jq runtime library identity changed during probe".to_owned());
    }
    writeln!(report, "runtime_libraries_after[{}]:", runtime_after.len())?;
    for library in &runtime_after {
        writeln!(report, "  - path: {}", quote(library.path))?;
        writeln!(report, "    sha256: {}", quote(&library.sha256))?;
    }

    writeln!(report, "summary:")?;
    writeln!(report, "  functions: 6")?;
    writeln!(report, "  inputs: {total_inputs}")?;
    writeln!(report, "  query_process_runs: {process_runs}")?;
    writeln!(report, "  exact_samples: {exact_samples}")?;
    writeln!(report, "  differing_samples: {differing_samples}")?;
    writeln!(report, "  post_run_sha256:")?;
    writeln!(report, "    jq: {}", quote(&post_run_jq_sha256))?;
    writeln!(report, "    tq: {}", quote(&post_run_tq_sha256))?;
    writeln!(report, "  max_tq_jq_ulp:")?;
    for (name, distance) in max_ulps {
        writeln!(report, "    {name}: {distance}")?;
    }
    if !issues.is_empty() {
        writeln!(report, "  issues[{}]:", issues.len())?;
        for issue in &issues {
            writeln!(report, "    - {}", quote(issue))?;
        }
    } else {
        writeln!(report, "  issues[0]:")?;
    }
    output_file.write_all(report.as_bytes())?;
    if issues.is_empty() {
        Ok(())
    } else {
        Err(probe_error(format!(
            "probe wrote {} with {} issue(s)",
            config.output.display(),
            issues.len()
        )))
    }
}

fn write_process(
    report: &mut String,
    name: &str,
    process: &ProcessResult,
) -> Result<(), Box<dyn Error>> {
    writeln!(report, "    {name}:")?;
    writeln!(report, "      status: {}", optional_status(process.status))?;
    writeln!(report, "      timed_out: {}", process.timed_out)?;
    writeln!(report, "      stdout_hex: {}", quote(&hex(&process.stdout)))?;
    writeln!(report, "      stderr_hex: {}", quote(&hex(&process.stderr)))?;
    Ok(())
}

fn optional_status(status: Option<i32>) -> String {
    status.map_or_else(|| "null".to_owned(), |status| status.to_string())
}

fn write_optional_number(
    report: &mut String,
    field: &str,
    value: Option<u64>,
) -> Result<(), Box<dyn Error>> {
    writeln!(
        report,
        "{field}: {}",
        value.map_or_else(|| "null".to_owned(), |value| value.to_string())
    )?;
    Ok(())
}

fn json_text(value: &Value) -> String {
    serde_json::to_string(value).expect("JSON values from jq/tq must serialize")
}

fn main() {
    if let Err(error) = run() {
        eprintln!("math_end_to_end_probe: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_INPUTS: &[Sample] = &[Sample {
        label: "1",
        value: 1.0,
    }];

    #[test]
    fn projected_preserves_finite_and_projects_nonfinite() {
        assert_eq!(projected(1.5), Some(1.5));
        assert_eq!(projected(f64::NAN), None);
        assert_eq!(projected(f64::INFINITY), Some(f64::MAX));
        assert_eq!(projected(f64::NEG_INFINITY), Some(-f64::MAX));
        assert_eq!(projected(-0.0).unwrap().to_bits(), (-0.0f64).to_bits());
    }

    #[test]
    fn ulp_distance_reports_adjacent_finite_values_only() {
        let next = f64::from_bits(1.0f64.to_bits() + 1);
        assert_eq!(ulp_distance(Some(1.0), Some(next)), Some(1));
        assert_eq!(ulp_distance(Some(f64::NAN), Some(1.0)), None);
        assert_eq!(ulp_distance(None, Some(1.0)), None);
    }

    #[test]
    fn parse_output_rejects_non_number_non_null_values() {
        let process = ProcessResult {
            status: Some(0),
            stdout: br#"[1,"bad"]
"#
            .to_vec(),
            stderr: Vec::new(),
            timed_out: false,
        };
        let function = Function {
            name: "test",
            query: "exp",
            inputs: TEST_INPUTS,
            apply: libm::exp,
        };
        let mut issues = Vec::new();
        let values = parse_output(&process, &function, "probe", &mut issues);
        assert!(values.is_some());
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("neither a number nor null"))
        );
    }
}
