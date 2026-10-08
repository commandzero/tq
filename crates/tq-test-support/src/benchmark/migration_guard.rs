//! Frozen, correctness-gated local TOON migration wall-time comparisons.
//!
//! This is deliberately not a calibrated benchmark/publication approval. Its
//! MAD ratio band is a dispersion heuristic, not a confidence interval.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::Write as _,
    fs::{self, File},
    io::{self, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    BenchmarkInvocation, CorrectnessDecision, CorrectnessObservation, CorrectnessPayload,
    EnvironmentManifest, MeasuredOutcome, MeasuredStatus, OutputContractKind, collect_environment,
    correctness_gate, measure_process_uninstrumented, normalize_correctness_run, semantic_digest,
};
use crate::{
    compatibility::{ExecutableConfig, ProcessStatus, ToolIdentity, ToolKind, discover_tool},
    corpus::ArtifactIdentity,
};

/// Candidate/baseline median-time ratio hard breach threshold.
pub const THRESHOLD: f64 = 1.20;
/// Paired measurements in the initial round.
pub const INITIAL_PAIRS: usize = 3;
/// Paired measurements in a confirmation round.
pub const CONFIRMATION_PAIRS: usize = 7;
/// Records in each generated workload input.
pub const WORKLOAD_ROWS: usize = 32_768;
const PAYLOAD_BYTES: usize = 96;
const OUTPUT_LIMIT: u64 = 128 * 1024 * 1024;
const GENERATION: &str = "toon-4-1-fixed-matrix-v1";
const BASE_REVISION: &str = "286b5f6690bc1bee260cba007f3953c52178bf68";
const REVIEW_THRESHOLD: f64 = 1.10;
const REVIEW_NOTE: &str =
    "Observed median wall-time regression exceeds 10%; deferred for user review (20% hard limit).";

/// Fallible result type used by migration-guard operations.
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
/// Build provenance required for comparing a candidate with the frozen baseline.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildManifest {
    /// Forty-character hexadecimal commit identifier for the source tree.
    pub source_revision: String,
    /// SHA-256 hex digest identifying the source contents used for this build.
    pub source_sha256: String,
    /// Rust compiler/toolchain identity.
    pub toolchain: String,
    /// Compilation target triple.
    pub target: String,
    /// Build profile; accepted guards require `release`.
    pub profile: String,
    /// Enabled Cargo features; accepted guards require only `default`.
    pub features: Vec<String>,
    /// Allocator identity/configuration used by the executable.
    pub allocator: String,
    /// Runtime thread policy, NOT the host logical CPU count.
    pub threads: String,
    /// Optimization settings used for compilation.
    pub optimization: String,
}

impl BuildManifest {
    fn validate(&self) -> Result<()> {
        if self.profile != "release"
            || self.features != ["default"]
            || self.source_revision.len() != 40
            || !self
                .source_revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || !valid_hash(&self.source_sha256)
            || [
                &self.toolchain,
                &self.target,
                &self.allocator,
                &self.threads,
                &self.optimization,
            ]
            .iter()
            .any(|s| s.trim().is_empty())
        {
            return Err(
                "requires explicit release/default build and source/toolchain/runtime metadata"
                    .into(),
            );
        }
        let architecture = if std::env::consts::ARCH == "aarch64" {
            "aarch64"
        } else {
            std::env::consts::ARCH
        };
        let os = match std::env::consts::OS {
            "macos" => "darwin",
            other => other,
        };
        if !self.target.contains(architecture) || !self.target.contains(os) {
            return Err("build target is not this native host".into());
        }
        Ok(())
    }

    fn comparable(&self, other: &Self) -> bool {
        self.toolchain == other.toolchain
            && self.target == other.target
            && self.profile == other.profile
            && self.features == other.features
            && self.allocator == other.allocator
            && self.threads == other.threads
            && self.optimization == other.optimization
    }
}

/// Outcome assigned to one correctness-gated workload.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    /// Evidence satisfies the comparison policy.
    Passed,
    /// A confirmed performance breach or observed correctness/process failure.
    Failed,
    /// Evidence is invalid, noisy, or otherwise insufficient for a decision.
    Inconclusive,
    /// The campaign stopped before producing all required evidence.
    Incomplete,
}

/// Returns 0 only when every expected row passes, 1 for any failure, otherwise 2.
///
/// # Arguments
/// * `decisions` — observed workload decisions.
/// * `expected_rows` — required workload count.
#[must_use]
pub fn exit_code(decisions: &[Decision], expected_rows: usize) -> i32 {
    if decisions.contains(&Decision::Failed) {
        1
    } else if decisions.len() != expected_rows
        || decisions.is_empty()
        || decisions.iter().any(|d| *d != Decision::Passed)
    {
        2
    } else {
        0
    }
}

/// Per-round timing statistics and threshold classification.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RoundSummary {
    /// Median baseline duration in microseconds.
    pub baseline_median_micros: f64,
    /// Median candidate duration in microseconds.
    pub candidate_median_micros: f64,
    /// Baseline median absolute deviation in microseconds.
    pub baseline_mad_micros: f64,
    /// Candidate median absolute deviation in microseconds.
    pub candidate_mad_micros: f64,
    /// Candidate median divided by baseline median.
    pub ratio: f64,
    /// Nonblocking review note when the observed median exceeds the advisory threshold.
    pub review_note: Option<Cow<'static, str>>,
    /// Heuristic ratio range derived from median ± MAD; not a confidence interval.
    pub ratio_band: Option<[f64; 2]>,
    /// Whether the median ratio exceeds [`THRESHOLD`].
    pub breach: bool,
    /// Whether the dispersion band straddles the threshold or cannot be computed.
    pub noisy: bool,
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        f64::midpoint(values[middle - 1], values[middle])
    } else {
        values[middle]
    }
}

fn expected_band(
    baseline_median: f64,
    baseline_mad: f64,
    candidate_median: f64,
    candidate_mad: f64,
) -> Option<[f64; 2]> {
    let baseline_lower = baseline_median - baseline_mad;
    (baseline_lower > 0.0).then(|| {
        [
            (candidate_median - candidate_mad).max(0.0) / (baseline_median + baseline_mad),
            (candidate_median + candidate_mad) / baseline_lower,
        ]
    })
}

fn valid_summary(summary: &RoundSummary) -> bool {
    let values = [
        summary.baseline_median_micros,
        summary.candidate_median_micros,
        summary.baseline_mad_micros,
        summary.candidate_mad_micros,
        summary.ratio,
    ];
    if values.iter().any(|value| !value.is_finite())
        || summary.baseline_median_micros < 1.0
        || summary.baseline_median_micros > 9_007_199_254_740_992.0
        || summary.candidate_median_micros < 1.0
        || summary.candidate_median_micros > 9_007_199_254_740_992.0
        || summary.baseline_mad_micros < 0.0
        || summary.baseline_mad_micros > 9_007_199_254_740_992.0
        || summary.candidate_mad_micros < 0.0
        || summary.candidate_mad_micros > 9_007_199_254_740_992.0
    {
        return false;
    }
    let ratio = summary.candidate_median_micros / summary.baseline_median_micros;
    let band = expected_band(
        summary.baseline_median_micros,
        summary.baseline_mad_micros,
        summary.candidate_median_micros,
        summary.candidate_mad_micros,
    );
    let band_is_valid = match (summary.ratio_band, band) {
        (None, None) => true,
        (Some([lower, upper]), Some([expected_lower, expected_upper])) => {
            lower.is_finite()
                && upper.is_finite()
                && lower.to_bits() == expected_lower.to_bits()
                && upper.to_bits() == expected_upper.to_bits()
        }
        _ => false,
    };
    band_is_valid
        && summary.ratio.to_bits() == ratio.to_bits()
        && summary.breach == (ratio > THRESHOLD)
        && summary.review_note.is_some() == (ratio > REVIEW_THRESHOLD)
        && summary.noisy
            == band.is_none_or(|[lower, upper]| lower <= THRESHOLD && upper > THRESHOLD)
}

/// Summarizes timing samples, expressed in microseconds, when both sides have the required count.
/// Returns `None` for invalid durations or a count mismatch. MAD bands describe dispersion,
/// not confidence intervals.
#[must_use]
pub fn summarize_round(
    baseline: &[u128],
    candidate: &[u128],
    required: usize,
) -> Option<RoundSummary> {
    if required == 0
        || baseline.len() != required
        || candidate.len() != required
        || baseline
            .iter()
            .chain(candidate)
            .any(|t| *t == 0 || *t > (1_u128 << 53))
    {
        return None;
    }
    let convert = |value: &u128| {
        let high = u32::try_from(*value >> 32).ok()?;
        let low = u32::try_from(*value & u128::from(u32::MAX)).ok()?;
        Some(f64::from(high) * 4_294_967_296.0 + f64::from(low))
    };
    let mut b: Vec<_> = baseline.iter().map(convert).collect::<Option<_>>()?;
    let mut c: Vec<_> = candidate.iter().map(convert).collect::<Option<_>>()?;
    let bm = median(&mut b);
    let cm = median(&mut c);
    let mut bd: Vec<_> = b.iter().map(|t| (t - bm).abs()).collect();
    let mut cd: Vec<_> = c.iter().map(|t| (t - cm).abs()).collect();
    let bmad = median(&mut bd);
    let cmad = median(&mut cd);
    let band = expected_band(bm, bmad, cm, cmad);
    let ratio = cm / bm;
    let noisy = band.is_none_or(|[lower, upper]| lower <= THRESHOLD && upper > THRESHOLD);
    Some(RoundSummary {
        baseline_median_micros: bm,
        candidate_median_micros: cm,
        baseline_mad_micros: bmad,
        candidate_mad_micros: cmad,
        ratio,
        ratio_band: band,
        breach: ratio > THRESHOLD,
        review_note: (ratio > REVIEW_THRESHOLD).then_some(Cow::Borrowed(REVIEW_NOTE)),
        noisy,
    })
}

/// A breach or threshold ambiguity must receive a separate seven-pair round.
#[must_use]
pub fn needs_confirmation(initial: Option<&RoundSummary>) -> bool {
    initial.is_none_or(|summary| !valid_summary(summary) || summary.breach || summary.noisy)
}

/// Applies the fixed decision policy to initial and optional confirmation summaries.
#[must_use]
pub fn decide_rounds(
    initial: Option<&RoundSummary>,
    confirmation: Option<&RoundSummary>,
) -> Decision {
    if initial.is_some_and(|summary| !valid_summary(summary))
        || confirmation.is_some_and(|summary| !valid_summary(summary))
    {
        return Decision::Inconclusive;
    }
    let Some(initial) = initial else {
        return Decision::Inconclusive;
    };
    if !needs_confirmation(Some(initial)) {
        return Decision::Passed;
    }
    let Some(confirmation) = confirmation else {
        return Decision::Incomplete;
    };
    if initial.ratio_band.is_none() || confirmation.noisy || initial.breach != confirmation.breach {
        return Decision::Inconclusive;
    }
    if confirmation.breach {
        if confirmation
            .ratio_band
            .is_some_and(|[lower, _]| lower > THRESHOLD)
        {
            Decision::Failed
        } else {
            Decision::Inconclusive
        }
    } else if initial.noisy {
        Decision::Inconclusive
    } else {
        Decision::Passed
    }
}

/// Alternates execution order: even pair indices run the baseline first.
#[must_use]
pub fn baseline_first(pair_index: usize) -> bool {
    pair_index.is_multiple_of(2)
}

/// Identity and generation metadata for the fixed input artifacts.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InputManifest {
    /// Input-manifest schema version.
    pub schema_version: u32,
    /// Generated records in each workload source.
    pub rows: usize,
    /// Fixed payload bytes per generated record.
    pub payload_bytes: usize,
    /// Generator revision identifying the frozen data matrix.
    pub generation: String,
    /// Frozen artifact relative paths, sizes, and SHA-256 digests.
    pub artifacts: Vec<ArtifactIdentity>,
}

/// One workload's semantic source, compared representations, and execution contract.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Workload {
    /// Stable identifier in the fixed workload matrix.
    pub id: String,
    /// Source artifact whose decoded semantic value is checked.
    pub semantic_source: String,
    /// Baseline-side input artifact.
    pub baseline_input: String,
    /// Candidate-side input artifact.
    pub candidate_input: String,
    /// Whether input bytes are identical or the representation changes end to end.
    pub comparison: String,
    /// CLI arguments used for correctness and timed invocations.
    pub args: Vec<String>,
    /// Required execution plan, when the workload constrains it.
    pub expected_plan: Option<String>,
    /// Whether correctness evidence must demonstrate spool use.
    pub require_spool: bool,
}

fn workload(
    id: &str,
    inputs: (&str, &str, &str),
    formats: (&str, &str),
    query: &str,
    plan: Option<&str>,
    spool: bool,
) -> Workload {
    let (source, b, c) = inputs;
    let (input, output) = formats;
    Workload {
        id: id.into(),
        semantic_source: source.into(),
        baseline_input: b.into(),
        candidate_input: c.into(),
        comparison: if b == c {
            "identical-input-bytes"
        } else {
            "end-to-end-representation-migration"
        }
        .into(),
        args: [
            "-M",
            "--input-format",
            input,
            "--output-format",
            output,
            "--prepare-memory-bytes",
            if spool { "262144" } else { "67108864" },
            "--max-spool-bytes",
            "268435456",
            query,
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        expected_plan: plan.map(str::to_owned),
        require_spool: spool,
    }
}

/// Returns the fixed workload matrix used for preparation and campaigns.
#[must_use]
pub fn matrix() -> Vec<Workload> {
    vec![
        workload(
            "flat-json-control",
            ("flat.json", "flat.json", "flat.json"),
            ("json", "json"),
            ".",
            None,
            false,
        ),
        workload(
            "decode-flat",
            ("flat.json", "flat.toon", "flat.toon"),
            ("toon", "json"),
            ".",
            None,
            false,
        ),
        workload(
            "decode-nested",
            (
                "nested.json",
                "nested-baseline.toon",
                "nested-candidate.toon",
            ),
            ("toon", "json"),
            ".",
            None,
            false,
        ),
        workload(
            "decode-keyed",
            ("keyed.json", "keyed-baseline.toon", "keyed-candidate.toon"),
            ("toon", "json"),
            ".",
            None,
            false,
        ),
        workload(
            "document-flat",
            ("flat.json", "flat.json", "flat.json"),
            ("json", "toon"),
            "[.] | .[0]",
            Some("blocking-document"),
            false,
        ),
        workload(
            "transcode-flat",
            ("flat.json", "flat.json", "flat.json"),
            ("json", "toon"),
            ".",
            Some("transcode"),
            false,
        ),
        workload(
            "transcode-nested",
            ("nested.json", "nested.json", "nested.json"),
            ("json", "toon"),
            ".",
            Some("transcode"),
            false,
        ),
        workload(
            "transcode-keyed",
            ("keyed.json", "keyed.json", "keyed.json"),
            ("json", "toon"),
            ".",
            Some("transcode"),
            false,
        ),
        workload(
            "transcode-late",
            ("late.json", "late.json", "late.json"),
            ("json", "toon"),
            ".",
            Some("transcode"),
            false,
        ),
        workload(
            "spool-flat",
            ("flat.json", "flat.json", "flat.json"),
            ("json", "toon"),
            ".",
            Some("transcode"),
            true,
        ),
    ]
}

/// Generates format-independent fixed semantic sources and explicit legacy/4.1 representations.
/// No product writer is used to silently change a frozen workload after migration.
#[must_use]
///
/// # Panics
/// Panics only if the fixed JSON values or in-memory string writes fail to serialize.
pub fn generated_inputs() -> Vec<(String, Vec<u8>)> {
    use std::fmt::Write as _;
    let payload = "x".repeat(PAYLOAD_BYTES);
    let flat: Vec<Value> = (0..WORKLOAD_ROWS)
        .map(|i| json!({"id":i,"active":i%2==0,"payload":payload}))
        .collect();
    let nested: Vec<Value> = (0..WORKLOAD_ROWS).map(|i| json!({"id":i,"customer":{"name":format!("user-{i}"),"country":"US"},"payload":payload})).collect();
    let mut keyed = serde_json::Map::new();
    for (i, row) in nested.iter().enumerate() {
        keyed.insert(format!("k{i}"), row.clone());
    }
    let mut late = nested.clone();
    late[WORKLOAD_ROWS - 1]["customer"] = Value::Null;
    let mut files = Vec::new();
    for (name, value) in [
        ("flat.json", Value::Array(flat)),
        ("nested.json", Value::Array(nested)),
        ("keyed.json", Value::Object(keyed)),
        ("late.json", Value::Array(late)),
    ] {
        let mut bytes = serde_json::to_vec(&value).expect("fixed JSON values serialize");
        bytes.push(b'\n');
        files.push((name.to_owned(), bytes));
    }
    let mut flat_toon = format!("[{WORKLOAD_ROWS}]{{id,active,payload}}:\n");
    let mut nested_old = format!("[{WORKLOAD_ROWS}]:\n");
    let mut nested_new = format!("[{WORKLOAD_ROWS}]{{id,customer{{name,country}},payload}}:\n");
    let mut keyed_old = String::new();
    let mut keyed_new = format!("[{WORKLOAD_ROWS}:]{{id,customer{{name,country}},payload}}:\n");
    for i in 0..WORKLOAD_ROWS {
        writeln!(flat_toon, "  {i},{},{payload}", i % 2 == 0).expect("string write");
        writeln!(nested_old, "  - id: {i}\n    customer:\n      name: user-{i}\n      country: US\n    payload: {payload}").expect("string write");
        writeln!(nested_new, "  {i},user-{i},US,{payload}").expect("string write");
        writeln!(keyed_old, "k{i}:\n  id: {i}\n  customer:\n    name: user-{i}\n    country: US\n  payload: {payload}").expect("string write");
        writeln!(keyed_new, "  k{i}: {i},user-{i},US,{payload}").expect("string write");
    }
    for (name, text) in [
        ("flat.toon", flat_toon),
        ("nested-baseline.toon", nested_old),
        ("nested-candidate.toon", nested_new),
        ("keyed-baseline.toon", keyed_old),
        ("keyed-candidate.toon", keyed_new),
    ] {
        files.push((name.into(), text.into_bytes()));
    }
    files
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn digest_hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(result, "{byte:02x}").expect("string write");
    }
    result
}
fn identity(name: &str, bytes: &[u8]) -> ArtifactIdentity {
    ArtifactIdentity {
        path: name.into(),
        bytes: bytes.len() as u64,
        sha256: digest_hex(&Sha256::digest(bytes)),
    }
}

fn file_identity(path: &Path) -> Result<ArtifactIdentity> {
    use std::io::Read as _;
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        bytes = bytes
            .checked_add(u64::try_from(read)?)
            .ok_or("file length overflow")?;
    }
    Ok(ArtifactIdentity {
        path: path.display().to_string(),
        bytes,
        sha256: digest_hex(&hasher.finalize()),
    })
}

/// A requested report path cannot replace frozen inputs or either executable.
pub const REPORT_PATH_CONFLICT: &str =
    "report path conflicts with a frozen input or executable; report was not written";

fn resolved_path(path: &Path) -> Result<std::path::PathBuf> {
    if path.exists() {
        return Ok(fs::canonicalize(path)?);
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = absolute
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = absolute.file_name().ok_or("report path must name a file")?;
    Ok(fs::canonicalize(parent)?.join(name))
}

fn report_path_conflicts(
    report_path: &Path,
    directory: &Path,
    frozen: &FrozenBaseline,
    candidate: &Path,
) -> Result<bool> {
    let report_path = resolved_path(report_path)?;
    let mut protected = vec![
        directory.join("baseline.json"),
        directory.join("inputs.json"),
        directory.join("prepare-report.json"),
        frozen.binary.path.clone(),
    ];
    protected.extend(
        frozen
            .inputs
            .artifacts
            .iter()
            .map(|artifact| directory.join(&artifact.path)),
    );
    if let Ok(candidate) = resolved_path(candidate) {
        protected.push(candidate);
    }
    for path in protected {
        if resolved_path(&path)? == report_path {
            return Ok(true);
        }
    }
    Ok(false)
}
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(path).map_err(|error| error.error)?;
    Ok(())
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_reader(File::open(path)?)?)
}
fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temp, value)?;
    temp.write_all(b"\n")?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    Ok(())
}

/// Existing inputs are accepted only when byte-identical to the fixed generation.
/// # Errors
/// Returns an error if the directory or immutable generated inputs cannot be read or written,
/// or if existing artifacts differ from the fixed generation.
pub fn freeze_inputs(directory: &Path) -> Result<InputManifest> {
    fs::create_dir_all(directory)?;
    let mut artifacts = Vec::new();
    for (name, bytes) in generated_inputs() {
        let path = directory.join(&name);
        if path.exists() {
            if fs::read(&path)? != bytes {
                return Err(format!("frozen input changed: {name}").into());
            }
        } else {
            write_new(&path, &bytes)?;
        }
        artifacts.push(identity(&name, &bytes));
    }
    let manifest = InputManifest {
        schema_version: 1,
        rows: WORKLOAD_ROWS,
        payload_bytes: PAYLOAD_BYTES,
        generation: GENERATION.into(),
        artifacts,
    };
    let path = directory.join("inputs.json");
    if path.exists() {
        if read_json::<InputManifest>(&path)? != manifest {
            return Err("frozen input manifest changed".into());
        }
    } else {
        write_new(&path, &serde_json::to_vec_pretty(&manifest)?)?;
    }
    Ok(manifest)
}

/// Verifies existing immutable artifacts without recreating missing inputs.
/// # Errors
/// Returns an error if the manifest or any immutable input is missing, unreadable, or differs.
pub fn verify_inputs(directory: &Path, expected: &InputManifest) -> Result<()> {
    let manifest: InputManifest = read_json(&directory.join("inputs.json"))?;
    if &manifest != expected
        || manifest.schema_version != 1
        || manifest.rows != WORKLOAD_ROWS
        || manifest.payload_bytes != PAYLOAD_BYTES
        || manifest.generation != GENERATION
    {
        return Err("input manifest differs from frozen generation".into());
    }
    let files = generated_inputs();
    if files.len() != manifest.artifacts.len() {
        return Err("missing or extra input identities".into());
    }
    for ((name, generated), artifact) in files.into_iter().zip(&manifest.artifacts) {
        if identity(&name, &generated) != *artifact
            || identity(&name, &fs::read(directory.join(&name))?) != *artifact
        {
            return Err(format!("missing, changed, or incompatible input: {name}").into());
        }
    }
    Ok(())
}

fn runtime_environment() -> BTreeMap<String, String> {
    std::env::vars()
        .filter(|(key, _)| {
            key.starts_with("MALLOC_")
                || key.starts_with("MIMALLOC_")
                || key.starts_with("RAYON_")
                || key.starts_with("OMP_")
                || matches!(
                    key.as_str(),
                    "RUSTFLAGS"
                        | "RUST_MIN_STACK"
                        | "LANG"
                        | "LC_ALL"
                        | "LC_NUMERIC"
                        | "TZ"
                        | "TQ_BENCH_FORCE_DOCUMENT"
                )
        })
        .collect()
}
fn identify(executable: &Path, directory: &Path) -> Result<ToolIdentity> {
    discover_tool(
        ToolKind::Tq,
        &ExecutableConfig {
            tq: Some(executable.to_owned()),
            ..ExecutableConfig::default()
        },
        directory,
    )?
    .ok_or_else(|| "missing executable identity".into())
}

/// Correctness evidence recorded before performance timing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CorrectnessEvidence {
    /// SHA-256 hex digest of the ordered semantic result sequence.
    pub semantic_sha256: String,
    /// Number of semantic results in the checked output.
    pub result_count: u64,
    /// Untimed execution report supporting correctness and resource checks.
    pub run_report: Value,
}

/// Immutable baseline identity and evidence approved for later comparisons.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FrozenBaseline {
    /// Frozen-baseline schema version.
    pub schema_version: u32,
    /// Build provenance for the baseline executable.
    pub build: BuildManifest,
    /// Discovered identity of the copied baseline executable.
    pub binary: ToolIdentity,
    /// Host and build environment recorded during preparation.
    pub environment: EnvironmentManifest,
    /// Captured runtime-affecting environment variables.
    pub runtime_environment: BTreeMap<String, String>,
    /// Frozen generated input manifest.
    pub inputs: InputManifest,
    /// Fixed workloads used to establish baseline evidence.
    pub workloads: Vec<Workload>,
    /// Correctness evidence, in workload order.
    pub correctness: Vec<CorrectnessEvidence>,
}

struct Deadline {
    started: Instant,
    budget: Duration,
    cancelled: Arc<AtomicBool>,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
    signals: Vec<signal_hook::SigId>,
}
impl Deadline {
    fn new(seconds: u64) -> Result<Self> {
        if seconds == 0 {
            return Err("deadline must be positive".into());
        }
        let started = Instant::now();
        let budget = Duration::from_secs(seconds);
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut signals = Vec::new();
        for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
            match signal_hook::flag::register(signal, Arc::clone(&cancelled)) {
                Ok(id) => signals.push(id),
                Err(error) => {
                    for id in signals {
                        signal_hook::low_level::unregister(id);
                    }
                    return Err(error.into());
                }
            }
        }
        let (stop, receiver) = mpsc::channel();
        let flag = Arc::clone(&cancelled);
        let worker = match thread::Builder::new()
            .name("toon-migration-deadline".into())
            .spawn(move || {
                if matches!(
                    receiver.recv_timeout(budget),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    flag.store(true, Ordering::Release);
                }
            }) {
            Ok(worker) => worker,
            Err(error) => {
                for id in signals.drain(..) {
                    signal_hook::low_level::unregister(id);
                }
                return Err(error.into());
            }
        };
        Ok(Self {
            started,
            budget,
            cancelled,
            stop,
            worker: Some(worker),
            signals,
        })
    }
    fn remaining(&self) -> Result<Duration> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err("campaign interrupted or deadline exhausted".into());
        }
        self.budget
            .checked_sub(self.started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| "campaign deadline exhausted".into())
    }
}
impl Drop for Deadline {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        for id in self.signals.drain(..) {
            signal_hook::low_level::unregister(id);
        }
    }
}
fn invocation(
    binary: &Path,
    workload: &Workload,
    input: Vec<u8>,
    directory: &Path,
    deadline: &Deadline,
) -> Result<BenchmarkInvocation> {
    Ok(BenchmarkInvocation {
        cancellation: Some(Arc::clone(&deadline.cancelled)),
        executable: binary.into(),
        args: workload.args.clone(),
        stdin: input,
        current_dir: Some(directory.into()),
        timeout: deadline.remaining()?,
        output_limit: OUTPUT_LIMIT,
        rss_limit: None,
        retain_output: false,
    })
}

/// Checks the same ordered semantic/process contract, allowing changed canonical TOON bytes.
/// # Errors
/// Returns an error when the observation violates the ordered semantic/process contract.
pub fn check_observation(expected: &Value, observation: &CorrectnessObservation) -> Result<()> {
    let reference = CorrectnessObservation {
        payload: CorrectnessPayload::SemanticSequence(semantic_digest([expected])?),
        process_status: ProcessStatus::Exited,
        exit_code: Some(0),
        error_class: None,
    };
    match correctness_gate(
        OutputContractKind::SemanticSequence,
        &reference,
        Ok(observation),
    ) {
        CorrectnessDecision::Passed => Ok(()),
        other => Err(format!("shared correctness contract failed: {other:?}").into()),
    }
}

/// Report-file evidence is captured outside every timed repetition.
/// # Errors
/// Returns an error when execution evidence violates the required plan or spool contract.
pub fn check_run_report(workload: &Workload, report: &Value) -> Result<()> {
    if let Some(plan) = &workload.expected_plan
        && report.pointer("/execution/plan").and_then(Value::as_str) != Some(plan)
    {
        return Err(format!("{} did not select required {plan} plan", workload.id).into());
    }
    if workload.require_spool {
        let memory_limit = workload
            .args
            .windows(2)
            .find(|pair| pair[0] == "--prepare-memory-bytes")
            .and_then(|pair| pair[1].parse::<u64>().ok())
            .filter(|limit| *limit > 0)
            .ok_or("spool workload omitted a positive preparation memory limit")?;
        let high_water = report
            .pointer("/execution/preparation_high_water_bytes")
            .and_then(Value::as_u64);
        let spool_written = report
            .pointer("/execution/spool_bytes_written")
            .and_then(Value::as_u64);
        let spool_replayed = report
            .pointer("/execution/spool_bytes_replayed")
            .and_then(Value::as_u64);
        let array_preparations = report
            .pointer("/execution/array_preparations")
            .and_then(Value::as_u64);
        if high_water.is_none_or(|bytes| bytes == 0 || bytes > memory_limit)
            || spool_written.is_none_or(|bytes| bytes <= memory_limit)
            || spool_replayed.is_none_or(|bytes| bytes == 0)
            || array_preparations.is_none_or(|count| count == 0)
        {
            return Err("spool workload did not report bounded memory and actual array disk preparation/replay".into());
        }
    }
    Ok(())
}

fn correctness(
    binary: &Path,
    workload: &Workload,
    candidate: bool,
    directory: &Path,
    deadline: &Deadline,
) -> Result<CorrectnessEvidence> {
    let input_name = if candidate {
        &workload.candidate_input
    } else {
        &workload.baseline_input
    };
    let mut request = invocation(
        binary,
        workload,
        fs::read(directory.join(input_name))?,
        directory,
        deadline,
    )?;
    let report_file = tempfile::NamedTempFile::new_in(directory)?;
    // A unique empty path prevents a stale report from satisfying the contract.
    request.args.splice(
        0..0,
        [
            "--report-file".into(),
            report_file.path().display().to_string(),
        ],
    );
    let observation =
        normalize_correctness_run(&request, ToolKind::Tq, OutputContractKind::SemanticSequence)?;
    deadline.remaining()?;
    let expected: Value = read_json(&directory.join(&workload.semantic_source))?;
    check_observation(&expected, &observation)?;
    let report: Value = read_json(report_file.path())?;
    check_run_report(workload, &report)?;
    let CorrectnessPayload::SemanticSequence(digest) = observation.payload else {
        unreachable!("semantic gate checked payload")
    };
    let evidence = CorrectnessEvidence {
        semantic_sha256: hex_digest(&digest.sha256),
        result_count: digest.result_count,
        run_report: report,
    };
    deadline.remaining()?;
    Ok(evidence)
}
fn hex_digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").expect("string write");
        s
    })
}

/// Freeze only once. Reruns must use `run`; preparation never replaces an approved baseline.
/// # Errors
/// Returns an error if the baseline, inputs, correctness evidence, or deadline is invalid.
pub fn prepare(
    baseline: &Path,
    directory: &Path,
    build: BuildManifest,
    deadline_seconds: u64,
) -> Result<FrozenBaseline> {
    let deadline = Deadline::new(deadline_seconds)?;
    deadline.remaining()?;
    build.validate()?;
    if build.source_revision != BASE_REVISION {
        return Err("baseline must identify the reconciled clean PR68 commit".into());
    }
    if runtime_environment().contains_key("TQ_BENCH_FORCE_DOCUMENT") {
        return Err("global document override invalidates transcode matrix".into());
    }
    fs::create_dir_all(directory)?;
    let directory = fs::canonicalize(directory)?;
    if resolved_path(baseline)? == resolved_path(&directory.join("prepare-report.json"))? {
        return Err("baseline executable path conflicts with the preparation checkpoint".into());
    }
    deadline.remaining()?;
    if directory.join("baseline.json").exists() {
        return Err("baseline already frozen; refusing to replace it".into());
    }
    let inputs = freeze_inputs(&directory)?;
    deadline.remaining()?;
    let copied = directory.join(format!("baseline{}", std::env::consts::EXE_SUFFIX));
    let binary = if copied.exists() {
        deadline.remaining()?;
        let source_identity = file_identity(baseline)?;
        let copied_identity = file_identity(&copied)?;
        if source_identity.bytes != copied_identity.bytes
            || source_identity.sha256 != copied_identity.sha256
        {
            return Err(
                "existing interrupted baseline copy differs from the requested executable".into(),
            );
        }
        deadline.remaining()?;
        let binary = identify(&copied, &directory)?;
        deadline.remaining()?;
        binary
    } else {
        let mut source = File::open(baseline)?;
        let mut copied_file = tempfile::NamedTempFile::new_in(&directory)?;
        io::copy(&mut source, copied_file.as_file_mut())?;
        copied_file.as_file().sync_all()?;
        copied_file
            .as_file()
            .set_permissions(fs::metadata(baseline)?.permissions())?;
        deadline.remaining()?;
        copied_file
            .persist_noclobber(&copied)
            .map_err(|error| error.error)?;
        deadline.remaining()?;
        let binary = identify(&copied, &directory)?;
        deadline.remaining()?;
        binary
    };
    let workloads = matrix();
    let mut evidence = Vec::new();
    for workload in &workloads {
        let result = correctness(&copied, workload, false, &directory, &deadline);
        save_json(
            &directory.join("prepare-report.json"),
            &json!({
                "binary": binary, "build": build, "inputs": inputs, "workloads": workloads,
                "completed_correctness": evidence, "current_row": workload.id,
                "error": result.as_ref().err().map(ToString::to_string), "elapsed_seconds": deadline.started.elapsed().as_secs_f64()
            }),
        )?;
        evidence.push(result?);
    }
    deadline.remaining()?;
    let frozen = FrozenBaseline {
        schema_version: 1,
        build,
        binary,
        environment: collect_environment("release"),
        runtime_environment: runtime_environment(),
        inputs,
        workloads,
        correctness: evidence,
    };
    deadline.remaining()?;
    write_new(
        &directory.join("baseline.json"),
        &serde_json::to_vec_pretty(&frozen)?,
    )?;
    deadline.remaining()?;
    Ok(frozen)
}

/// One baseline/candidate measurement pair and its execution order.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PairedSample {
    /// Zero-based index within the round.
    pub pair_index: usize,
    /// Whether this pair executed the baseline before the candidate.
    pub baseline_first: bool,
    /// Baseline process outcome, absent if it did not complete.
    pub baseline: Option<MeasuredOutcome>,
    /// Candidate process outcome, absent if it did not complete.
    pub candidate: Option<MeasuredOutcome>,
}
/// Correctness, timing, and decision evidence for one workload.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GuardRow {
    /// Workload definition from the immutable matrix.
    pub workload: Workload,
    /// Baseline correctness/resource evidence, if completed.
    pub baseline_correctness: Option<CorrectnessEvidence>,
    /// Candidate correctness/resource evidence, if completed.
    pub candidate_correctness: Option<CorrectnessEvidence>,
    /// Untimed warm-up outcomes, baseline followed by candidate.
    pub warmup: Vec<MeasuredOutcome>,
    /// Initial paired measurements.
    pub initial: Vec<PairedSample>,
    /// Additional paired measurements required by initial evidence.
    pub confirmation: Vec<PairedSample>,
    /// Summary of initial measurements, when complete.
    pub initial_summary: Option<RoundSummary>,
    /// Summary of confirmation measurements, when complete.
    pub confirmation_summary: Option<RoundSummary>,
    /// Workload outcome under the fixed decision policy.
    pub decision: Decision,
    /// Row-specific failure or incompleteness explanation.
    pub diagnostic: Option<String>,
}
/// Full campaign evidence; this guard is bounded and explicitly uncalibrated.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GuardReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Profile used for the campaign.
    pub profile: String,
    /// False because this guard is not a calibrated benchmark.
    pub calibrated: bool,
    /// Comparison scope represented by this report.
    pub scope: String,
    /// Candidate/baseline median ratio threshold.
    pub threshold: f64,
    /// Nonblocking candidate/baseline median ratio review threshold.
    pub review_threshold: f64,
    /// Description of the MAD-based dispersion heuristic.
    pub dispersion_policy: String,
    /// Campaign deadline budget in seconds.
    pub deadline_seconds: u64,
    /// Campaign elapsed wall time in seconds.
    pub elapsed_seconds: f64,
    /// Identity of the frozen baseline manifest used for this run.
    pub frozen_manifest: ArtifactIdentity,
    /// Frozen baseline build, environment, inputs, workloads, and evidence.
    pub frozen: FrozenBaseline,
    /// Candidate build provenance supplied for the campaign.
    pub candidate_build: BuildManifest,
    /// Discovered candidate executable identity, if available.
    pub candidate: Option<ToolIdentity>,
    /// Environment identity observed for this campaign.
    pub environment: EnvironmentManifest,
    /// Captured runtime-affecting environment variables.
    pub runtime_environment: BTreeMap<String, String>,
    /// Per-workload evidence, preserving completed rows if interrupted.
    pub rows: Vec<GuardRow>,
    /// Campaign-level failure or incompleteness explanation.
    pub diagnostic: Option<String>,
    /// Process status convention: 0 pass, 1 failure, 2 incomplete/inconclusive.
    pub exit_code: i32,
}

fn valid_sample(sample: &MeasuredOutcome) -> bool {
    sample.status == MeasuredStatus::Exited
        && sample.exit_code == Some(0)
        && sample.signal.is_none()
        && sample.wall_time_micros > 0
        && sample.output_bytes > 0
        && sample.stderr_bytes == 0
}
fn round_summary(pairs: &[PairedSample], required: usize) -> Option<RoundSummary> {
    let b: Option<Vec<_>> = pairs
        .iter()
        .map(|p| p.baseline.as_ref().map(|o| o.wall_time_micros))
        .collect();
    let c: Option<Vec<_>> = pairs
        .iter()
        .map(|p| p.candidate.as_ref().map(|o| o.wall_time_micros))
        .collect();
    summarize_round(&b?, &c?, required)
}
fn measure(
    binary: &Path,
    workload: &Workload,
    input: &[u8],
    directory: &Path,
    deadline: &Deadline,
) -> Result<MeasuredOutcome> {
    let request = invocation(binary, workload, input.to_vec(), directory, deadline)?;
    Ok(measure_process_uninstrumented(&request)?)
}

fn measure_pairs(
    row: &mut GuardRow,
    confirmation: bool,
    b: &Path,
    c: &Path,
    inputs: (&[u8], &[u8]),
    directory: &Path,
    deadline: &Deadline,
) -> Result<()> {
    let count = if confirmation {
        CONFIRMATION_PAIRS
    } else {
        INITIAL_PAIRS
    };
    for offset in 0..count {
        let index = if confirmation {
            INITIAL_PAIRS + offset
        } else {
            offset
        };
        let mut pair = PairedSample {
            pair_index: index,
            baseline_first: baseline_first(index),
            baseline: None,
            candidate: None,
        };
        let mut error = None;
        for is_baseline in [pair.baseline_first, !pair.baseline_first] {
            let result = measure(
                if is_baseline { b } else { c },
                &row.workload,
                if is_baseline { inputs.0 } else { inputs.1 },
                directory,
                deadline,
            );
            match result {
                Ok(sample) => {
                    if !valid_sample(&sample) {
                        error = Some(format!(
                            "invalid timed outcome: {:?} exit {:?}",
                            sample.status, sample.exit_code
                        ));
                    }
                    if is_baseline {
                        pair.baseline = Some(sample);
                    } else {
                        pair.candidate = Some(sample);
                    }
                }
                Err(e) => error = Some(e.to_string()),
            }
            if error.is_some() {
                break;
            }
        }
        if confirmation {
            row.confirmation.push(pair);
        } else {
            row.initial.push(pair);
        }
        if let Some(error) = error {
            return Err(error.into());
        }
    }
    Ok(())
}

fn run_row(
    row: &mut GuardRow,
    b: &Path,
    c: &Path,
    directory: &Path,
    deadline: &Deadline,
) -> Result<()> {
    row.baseline_correctness = Some(correctness(b, &row.workload, false, directory, deadline)?);
    row.candidate_correctness = Some(correctness(c, &row.workload, true, directory, deadline)?);
    let bi = fs::read(directory.join(&row.workload.baseline_input))?;
    let ci = fs::read(directory.join(&row.workload.candidate_input))?;
    for (binary, input) in [(b, bi.as_slice()), (c, ci.as_slice())] {
        let sample = measure(binary, &row.workload, input, directory, deadline)?;
        let valid = valid_sample(&sample);
        row.warmup.push(sample);
        if !valid {
            return Err("warmup did not satisfy process/output contract".into());
        }
    }
    measure_pairs(row, false, b, c, (&bi, &ci), directory, deadline)?;
    row.initial_summary = round_summary(&row.initial, INITIAL_PAIRS);
    if needs_confirmation(row.initial_summary.as_ref()) {
        measure_pairs(row, true, b, c, (&bi, &ci), directory, deadline)?;
        row.confirmation_summary = round_summary(&row.confirmation, CONFIRMATION_PAIRS);
    }
    let samples: Vec<_> = row
        .warmup
        .iter()
        .chain(
            row.initial
                .iter()
                .chain(&row.confirmation)
                .flat_map(|p| p.baseline.iter().chain(&p.candidate)),
        )
        .collect();
    let protocol = &samples[0].measurement_protocol;
    if protocol.worker.is_none() || samples.iter().any(|s| &s.measurement_protocol != protocol) {
        return Err("native measurement/worker protocols are missing or incompatible".into());
    }
    // Per-version byte counts may differ, but a timed invocation must reproduce
    // its own checked output length, never crash silently into a smaller result.
    let bw = row
        .baseline_correctness
        .as_ref()
        .and_then(|e| e.run_report["output_bytes"].as_u64())
        .ok_or("baseline correctness report omitted output size")?;
    let cw = row
        .candidate_correctness
        .as_ref()
        .and_then(|e| e.run_report["output_bytes"].as_u64())
        .ok_or("candidate correctness report omitted output size")?;
    if row.warmup[0].output_bytes != bw || row.warmup[1].output_bytes != cw {
        return Err("warmup output differs from correctness capture size".into());
    }
    if row.initial.iter().chain(&row.confirmation).any(|p| {
        p.baseline.as_ref().is_none_or(|s| s.output_bytes != bw)
            || p.candidate.as_ref().is_none_or(|s| s.output_bytes != cw)
    }) {
        return Err("timed output byte count changed within a binary".into());
    }
    row.decision = decide_rounds(
        row.initial_summary.as_ref(),
        row.confirmation_summary.as_ref(),
    );
    Ok(())
}

fn identify_candidate(
    candidate: &Path,
    directory: &Path,
    report: &GuardReport,
    deadline: &Deadline,
) -> Result<ToolIdentity> {
    deadline.remaining()?;
    report.candidate_build.validate()?;
    report.frozen.build.validate()?;
    if report.frozen.schema_version != 1
        || report.frozen.build.source_revision != BASE_REVISION
        || report.frozen.workloads != matrix()
        || report.frozen.correctness.len() != matrix().len()
        || !report.frozen.build.comparable(&report.candidate_build)
        || report.frozen.environment.machine_identity != report.environment.machine_identity
        || report.frozen.runtime_environment != report.runtime_environment
    {
        return Err(
            "incompatible frozen baseline, matrix, build, host, or runtime settings".into(),
        );
    }
    if report
        .runtime_environment
        .contains_key("TQ_BENCH_FORCE_DOCUMENT")
    {
        return Err("global document override invalidates transcode matrix".into());
    }
    verify_inputs(directory, &report.frozen.inputs)?;
    deadline.remaining()?;
    let baseline = identify(&report.frozen.binary.path, directory)?;
    deadline.remaining()?;
    if baseline != report.frozen.binary {
        return Err("baseline executable or runtime library identity changed".into());
    }
    let candidate = identify(candidate, directory)?;
    deadline.remaining()?;
    if candidate.build_features != baseline.build_features
        || candidate.runtime_libraries != baseline.runtime_libraries
    {
        return Err("candidate runtime libraries or observed features differ".into());
    }
    Ok(candidate)
}

fn verify_final_identities(
    directory: &Path,
    candidate: &ToolIdentity,
    report: &GuardReport,
    deadline: &Deadline,
) -> Result<()> {
    let frozen_bytes = fs::read(directory.join("baseline.json"))?;
    let current_manifest = identity("baseline.json", &frozen_bytes);
    deadline.remaining()?;
    let current_baseline = identify(&report.frozen.binary.path, directory)?;
    deadline.remaining()?;
    let current_candidate = identify(&candidate.path, directory)?;
    deadline.remaining()?;
    let same_host =
        collect_environment("release").machine_identity == report.environment.machine_identity;
    deadline.remaining()?;
    if current_manifest != report.frozen_manifest
        || current_baseline != report.frozen.binary
        || current_candidate != *candidate
        || !same_host
    {
        return Err("binary/runtime/host identities changed during campaign".into());
    }
    verify_inputs(directory, &report.frozen.inputs)?;
    deadline.remaining()?;
    Ok(())
}

/// Replays the immutable fixed plan. Every failure/deadline preserves all completed rows.
/// # Errors
/// Returns an error if report publication fails; campaign failures are recorded in the report.
pub fn run(
    candidate: &Path,
    directory: &Path,
    candidate_build: BuildManifest,
    report_path: &Path,
    deadline_seconds: u64,
) -> Result<GuardReport> {
    let deadline = Deadline::new(deadline_seconds)?;
    deadline.remaining()?;
    let directory = fs::canonicalize(directory)?;
    let frozen_bytes = fs::read(directory.join("baseline.json"))?;
    let frozen: FrozenBaseline = serde_json::from_slice(&frozen_bytes)?;
    deadline.remaining()?;
    let mut report = GuardReport { schema_version: 1, profile: "focused-migration-repeated".into(), calibrated: false,
        scope: "same-native-host local migration only; no calibrated publication or all-platform acceptance".into(),
        threshold: THRESHOLD, review_threshold: REVIEW_THRESHOLD, dispersion_policy: "median +/- MAD ratio band; dispersion heuristic, NOT confidence interval".into(),
        deadline_seconds, elapsed_seconds: 0.0, frozen_manifest: identity("baseline.json", &frozen_bytes),
        frozen, candidate_build, candidate: None, environment: collect_environment("release"),
        runtime_environment: runtime_environment(), rows: matrix().into_iter().map(|workload| GuardRow {
            workload, baseline_correctness: None, candidate_correctness: None, warmup: Vec::new(), initial: Vec::new(),
            confirmation: Vec::new(), initial_summary: None, confirmation_summary: None,
            decision: Decision::Incomplete, diagnostic: None,
        }).collect(), diagnostic: None, exit_code: 2 };
    if report_path_conflicts(report_path, &directory, &report.frozen, candidate)? {
        report.diagnostic = Some(REPORT_PATH_CONFLICT.into());
        report.elapsed_seconds = deadline.started.elapsed().as_secs_f64();
        return Ok(report);
    }
    save_json(report_path, &report)?;
    let identities = identify_candidate(candidate, &directory, &report, &deadline);
    match identities {
        Err(error) => {
            report.diagnostic = Some(error.to_string());
            report.exit_code = if deadline.remaining().is_err() { 2 } else { 1 };
        }
        Ok(candidate_identity) => {
            report.candidate = Some(candidate_identity.clone());
            let baseline = report.frozen.binary.path.clone();
            for index in 0..report.rows.len() {
                if let Err(error) = deadline.remaining() {
                    report.diagnostic = Some(format!("{error}; missing rows retained"));
                    break;
                }
                let result = run_row(
                    &mut report.rows[index],
                    &baseline,
                    &candidate_identity.path,
                    &directory,
                    &deadline,
                );
                if let Err(error) = result {
                    report.rows[index].diagnostic = Some(error.to_string());
                    report.rows[index].decision = if deadline.remaining().is_err() {
                        Decision::Incomplete
                    } else {
                        Decision::Failed
                    };
                }
                report.elapsed_seconds = deadline.started.elapsed().as_secs_f64();
                report.exit_code = exit_code(
                    &report.rows.iter().map(|r| r.decision).collect::<Vec<_>>(),
                    matrix().len(),
                );
                save_json(report_path, &report)?;
            }
            if let Err(error) = deadline.remaining() {
                report.diagnostic = Some(format!("{error}; final identity check skipped"));
                if report.exit_code == 0 {
                    report.exit_code = 2;
                }
            } else {
                // Recheck immutable identities without allowing an error to leave a
                // previously checkpointed passing report behind.
                let final_identities =
                    verify_final_identities(&directory, &candidate_identity, &report, &deadline);
                if let Err(error) = final_identities {
                    report.diagnostic = Some(error.to_string());
                    let observed_failure = report
                        .rows
                        .iter()
                        .any(|row| row.decision == Decision::Failed);
                    report.exit_code = if deadline.remaining().is_err() && !observed_failure {
                        2
                    } else {
                        1
                    };
                }
            }
        }
    }
    report.elapsed_seconds = deadline.started.elapsed().as_secs_f64();
    if deadline.remaining().is_err() && report.exit_code == 0 {
        report
            .diagnostic
            .get_or_insert_with(|| "campaign deadline exhausted before report completion".into());
        report.exit_code = 2;
    }
    save_json(report_path, &report)?;
    if deadline.remaining().is_err() && report.exit_code == 0 {
        report.elapsed_seconds = deadline.started.elapsed().as_secs_f64();
        report.diagnostic = Some("campaign deadline exhausted while saving final report".into());
        report.exit_code = 2;
        save_json(report_path, &report)?;
    }
    Ok(report)
}
