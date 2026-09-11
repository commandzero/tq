//! Attach retained native control evidence without claiming clock-resolution accuracy.

use std::{fmt::Write as _, fs, path::Path};

use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use tq_test_support::benchmark::{
    LaunchIsolationEvidence, MeasurementProtocol, WorkerIdentity, collect_environment,
    collector_source_sha256,
};

#[derive(Deserialize)]
struct Summary {
    metadata: Metadata,
    calibration_linkage: Linkage,
    sampler_calibration_linkage: Option<Linkage>,
    noop_isolation: Option<NoopIsolation>,
    allocation_delta_checks: Vec<AllocationDeltaCheck>,
    short_burst_check: ShortBurstCheck,
    worker_isolation: Option<WorkerIsolationEvidence>,
    validation_failures: Vec<String>,
    cases: Vec<Control>,
}

#[derive(Deserialize)]
struct Metadata {
    repetitions: usize,
    page_size_bytes: u64,
}

#[derive(Deserialize)]
struct Linkage {
    machine_identity: String,
    collector_source_sha256: String,
    runner_build_profile: String,
    measurement_protocol: MeasurementProtocol,
    conservative_observed_duration_error_bound_micros: Option<u128>,
    status: String,
}

#[derive(Deserialize)]
struct Control {
    case: String,
    repetitions: usize,
    native_duration_error_bound_micros: Option<u128>,
    rss_limit_bytes: Option<u64>,
    rss_comparisons_within_tolerance: Option<usize>,
    cpu_comparisons_within_tolerance: Option<usize>,
}

#[derive(Deserialize)]
struct NoopIsolation {
    within_tolerance: bool,
}

#[derive(Deserialize)]
struct AllocationDeltaCheck {
    case: String,
    within_tolerance: bool,
}

#[derive(Deserialize)]
struct ShortBurstCheck {
    observed: bool,
    status: String,
}

#[derive(Deserialize)]
struct WorkerAllocationControl {
    coordinator_allocation_bytes: u64,
    repetitions: usize,
    native_rss_median_bytes: u64,
    independent_time_rss_median_bytes: Option<u64>,
    rss_comparisons_within_tolerance: usize,
    cpu_comparisons_within_tolerance: usize,
    max_delta_from_zero_bytes: u64,
    tolerance_bytes: u64,
}

#[derive(Deserialize)]
struct WorkerIsolationEvidence {
    collector_source_sha256: String,
    coordinator_controls: Vec<WorkerAllocationControl>,
    residual_floor_bytes: u64,
    prepared_stdin_bytes: u64,
    prepared_stdin_repetitions: usize,
    prepared_stdin_verified: bool,
    high_low_request_repetitions: usize,
    high_low_request_verified: bool,
    independent_time_repetitions: usize,
    independent_time_verified: bool,
}

pub(super) struct TimingCalibration {
    protocol: MeasurementProtocol,
    observed_bound_micros: u64,
    summary_sha256: String,
}

impl TimingCalibration {
    pub(super) fn load(path: &Path) -> Result<Vec<Self>, String> {
        if cfg!(debug_assertions) {
            return Err("timing calibration requires a release-built campaign driver".to_owned());
        }
        let bytes = fs::read(path).map_err(|error| format!("read timing calibration: {error}"))?;
        Self::from_bytes(
            &bytes,
            &collect_environment("native-validation").machine_identity,
            &collector_source_sha256(),
        )
    }

    fn from_bytes(bytes: &[u8], machine: &str, collector: &str) -> Result<Vec<Self>, String> {
        let summary: Summary = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid timing calibration summary: {error}"))?;
        std::iter::once(&summary.calibration_linkage)
            .chain(summary.sampler_calibration_linkage.as_ref())
            .map(|linkage| Self::from_linkage(&summary, linkage, bytes, machine, collector))
            .collect()
    }

    fn from_linkage(
        summary: &Summary,
        linkage: &Linkage,
        bytes: &[u8],
        machine: &str,
        collector: &str,
    ) -> Result<Self, String> {
        if !summary.validation_failures.is_empty()
            || linkage.status != "verified"
            || summary.metadata.repetitions < 20
        {
            return Err("timing calibration is failed, incomplete, or undersampled".to_owned());
        }
        if linkage.machine_identity != machine {
            return Err("timing calibration belongs to a different host configuration".to_owned());
        }
        if linkage.collector_source_sha256 != collector {
            return Err("timing calibration belongs to different collector sources".to_owned());
        }
        if linkage.runner_build_profile != "release" {
            return Err("timing calibration must use the release collector".to_owned());
        }
        let protocol = worker_calibration_protocol(summary, linkage, bytes)?;
        if !summary
            .noop_isolation
            .as_ref()
            .is_some_and(|isolation| isolation.within_tolerance)
        {
            return Err("timing calibration lacks successful noop isolation".to_owned());
        }
        if summary.metadata.page_size_bytes == 0 {
            return Err("timing calibration has an invalid zero page size".to_owned());
        }
        if !summary.short_burst_check.observed || summary.short_burst_check.status != "verified" {
            return Err("timing calibration lacks a verified short allocation burst".to_owned());
        }
        let required_allocations = ["burst-4m", "burst-16m", "threads-4m-x2", "threads-2m-x4"];
        if summary.allocation_delta_checks.len() != required_allocations.len()
            || required_allocations.iter().any(|name| {
                !summary
                    .allocation_delta_checks
                    .iter()
                    .any(|check| check.case == *name && check.within_tolerance)
            })
        {
            return Err("timing calibration lacks successful allocation delta checks".to_owned());
        }
        let mut observed_bound = 0;
        let sampled = protocol.rss_poll_interval_micros.is_some();
        let prefix = if sampled { "sampler-" } else { "" };
        for name in [
            "noop",
            "known-duration-20ms",
            "known-duration-100ms",
            "known-duration-250ms",
        ] {
            let control = summary
                .cases
                .iter()
                .find(|control| control.case == format!("{prefix}{name}"))
                .ok_or_else(|| format!("timing calibration lacks {prefix}{name}"))?;
            if control.rss_limit_bytes.is_some() != sampled {
                return Err(format!(
                    "timing calibration {prefix}{name} has different instrumentation"
                ));
            }
            if control.repetitions < summary.metadata.repetitions {
                return Err(format!(
                    "timing calibration {name} has fewer than {} samples",
                    summary.metadata.repetitions
                ));
            }
            if control.rss_comparisons_within_tolerance != Some(control.repetitions)
                || control.cpu_comparisons_within_tolerance != Some(control.repetitions)
            {
                return Err(format!(
                    "timing calibration {prefix}{name} has failed RSS/CPU counter comparisons"
                ));
            }
            if name != "noop" {
                observed_bound =
                    observed_bound.max(control.native_duration_error_bound_micros.ok_or_else(
                        || format!("timing calibration {name} lacks a control bound"),
                    )?);
            }
        }
        if observed_bound == 0
            || linkage.conservative_observed_duration_error_bound_micros != Some(observed_bound)
        {
            return Err("timing calibration bound does not match retained controls".to_owned());
        }
        let observed_bound_micros = u64::try_from(observed_bound)
            .map_err(|_| "timing calibration bound overflows report units".to_owned())?;
        let summary_sha256 = summary_sha256(bytes);
        Ok(Self {
            protocol,
            observed_bound_micros,
            summary_sha256,
        })
    }

    pub(super) fn apply(&self, protocol: &mut MeasurementProtocol) -> Result<(), String> {
        if !same_raw_launch_contract(protocol, &self.protocol) {
            return Err("sample protocol differs from timing calibration; instrumented runs require their own controls".to_owned());
        }
        protocol
            .isolation_evidence
            .clone_from(&self.protocol.isolation_evidence);
        protocol.validated_accuracy_micros = Some(self.observed_bound_micros);
        write!(protocol.timing_method,
            "; observed known-duration control bound including spawn and sleep scheduling, not a universal accuracy guarantee; calibration SHA-256 {}",
            self.summary_sha256,
        ).expect("write calibration reference");
        Ok(())
    }

    pub(super) fn matches(&self, protocol: &MeasurementProtocol) -> bool {
        same_raw_launch_contract(protocol, &self.protocol)
    }
}

fn same_raw_launch_contract(left: &MeasurementProtocol, right: &MeasurementProtocol) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.validated_accuracy_micros = None;
    right.validated_accuracy_micros = None;
    left.isolation_evidence = None;
    right.isolation_evidence = None;
    left == right
}

fn worker_calibration_protocol(
    summary: &Summary,
    linkage: &Linkage,
    bytes: &[u8],
) -> Result<MeasurementProtocol, String> {
    let worker_isolation = summary
        .worker_isolation
        .as_ref()
        .ok_or_else(|| "timing calibration lacks retained worker isolation evidence".to_owned())?;
    let worker = linkage
        .measurement_protocol
        .worker
        .as_ref()
        .ok_or_else(|| "timing calibration lacks worker identity".to_owned())?;
    let isolation = linkage
        .measurement_protocol
        .isolation_evidence
        .as_ref()
        .ok_or_else(|| "timing calibration lacks launch-isolation evidence".to_owned())?;
    if !worker_is_valid(worker, &linkage.collector_source_sha256) || !isolation_is_valid(isolation)
    {
        return Err(
            "timing calibration has invalid worker identity or isolation evidence".to_owned(),
        );
    }
    if !worker_isolation_passes(worker_isolation, &summary.metadata, linkage) {
        return Err("timing calibration lacks passing worker isolation controls".to_owned());
    }
    let zero_control = worker_isolation
        .coordinator_controls
        .iter()
        .find(|control| control.coordinator_allocation_bytes == 0)
        .ok_or_else(|| "timing calibration lacks zero-allocation worker control".to_owned())?;
    let max_parent_delta = worker_isolation
        .coordinator_controls
        .iter()
        .map(|control| control.max_delta_from_zero_bytes)
        .max()
        .unwrap_or(0);
    let max_tolerance = worker_isolation
        .coordinator_controls
        .iter()
        .map(|control| control.tolerance_bytes)
        .max()
        .unwrap_or(0);
    if isolation.control_peak_rss_bytes != zero_control.native_rss_median_bytes
        || isolation.max_parent_delta_bytes != max_parent_delta
        || isolation.tolerance_bytes != max_tolerance
    {
        return Err(
            "timing calibration isolation summary does not match retained controls".to_owned(),
        );
    }
    let mut protocol = linkage.measurement_protocol.clone();
    let evidence = protocol
        .isolation_evidence
        .as_mut()
        .expect("launch-isolation evidence was checked above");
    // This field is an output reference, not a digest of a JSON document that
    // contains itself. Resolve it from the retained summary bytes here.
    evidence.summary_sha256 = summary_sha256(bytes);
    Ok(protocol)
}

fn worker_is_valid(worker: &WorkerIdentity, collector_source: &str) -> bool {
    !worker.executable_sha256.trim().is_empty()
        && !worker.launch_protocol.trim().is_empty()
        && worker.collector_source_sha256 == collector_source
}

fn summary_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("write digest to string");
            hex
        })
}

fn isolation_is_valid(isolation: &LaunchIsolationEvidence) -> bool {
    isolation.control_peak_rss_bytes > 0
        && isolation.tolerance_bytes > 0
        && isolation.max_parent_delta_bytes <= isolation.tolerance_bytes
}

fn worker_isolation_passes(
    evidence: &WorkerIsolationEvidence,
    metadata: &Metadata,
    linkage: &Linkage,
) -> bool {
    const EXPECTED_ALLOCATIONS: [u64; 3] = [0, 32 * 1024 * 1024, 128 * 1024 * 1024];
    const MIN_PREPARED_STDIN_BYTES: u64 = 1 << 20;
    let repetitions = metadata.repetitions;

    if evidence.collector_source_sha256 != linkage.collector_source_sha256
        || metadata.page_size_bytes == 0
        || evidence.residual_floor_bytes == 0
        || evidence.prepared_stdin_bytes < MIN_PREPARED_STDIN_BYTES
        || evidence.prepared_stdin_repetitions < repetitions
        || !evidence.prepared_stdin_verified
        || evidence.high_low_request_repetitions < repetitions
        || !evidence.high_low_request_verified
        || evidence.independent_time_repetitions < repetitions
        || !evidence.independent_time_verified
        || evidence.coordinator_controls.len() != EXPECTED_ALLOCATIONS.len()
    {
        return false;
    }

    let Some(zero_control) = evidence
        .coordinator_controls
        .iter()
        .find(|control| control.coordinator_allocation_bytes == 0)
    else {
        return false;
    };
    let zero_rss = zero_control.native_rss_median_bytes;
    if evidence.residual_floor_bytes != zero_rss {
        return false;
    }

    EXPECTED_ALLOCATIONS.iter().all(|expected| {
        evidence
            .coordinator_controls
            .iter()
            .find(|control| control.coordinator_allocation_bytes == *expected)
            .is_some_and(|control| {
                control.repetitions >= repetitions
                    && control.native_rss_median_bytes > 0
                    && control
                        .independent_time_rss_median_bytes
                        .is_some_and(|rss| {
                            rss > 0
                                && rss_comparison_tolerance(
                                    control.native_rss_median_bytes,
                                    rss,
                                    metadata.page_size_bytes,
                                )
                                .is_some_and(|bound| {
                                    control.native_rss_median_bytes.abs_diff(rss) <= bound
                                })
                        })
                    && control.rss_comparisons_within_tolerance == control.repetitions
                    && control.cpu_comparisons_within_tolerance == control.repetitions
                    && control.tolerance_bytes >= metadata.page_size_bytes
                    && noop_tolerance(
                        zero_rss,
                        control.native_rss_median_bytes,
                        metadata.page_size_bytes,
                    )
                    .is_some_and(|bound| control.tolerance_bytes <= bound)
                    && control.max_delta_from_zero_bytes <= control.tolerance_bytes
                    && control.native_rss_median_bytes.abs_diff(zero_rss)
                        <= control.max_delta_from_zero_bytes
            })
    })
}

fn noop_tolerance(zero_rss: u64, control_rss: u64, page_size: u64) -> Option<u64> {
    zero_rss
        .max(control_rss)
        .checked_div(4)
        .zip(page_size.checked_mul(8))
        .map(|(relative, page)| relative.max(page))
}

fn rss_comparison_tolerance(native_rss: u64, independent_rss: u64, page_size: u64) -> Option<u64> {
    native_rss
        .max(independent_rss)
        .checked_div(4)
        .zip(page_size.checked_mul(4))
        .map(|(relative, page)| relative.max(page))
}

#[cfg(test)]
mod tests {
    use super::TimingCalibration;

    fn summary() -> serde_json::Value {
        serde_json::json!({
            "metadata": {"repetitions": 20, "page_size_bytes": 4096},
            "calibration_linkage": {
                "machine_identity": "host", "collector_source_sha256": "collector",
                "runner_build_profile": "release", "status": "verified",
                "measurement_protocol": {
                    "timing_method": "native", "input_delivery": "file", "rss_scope": "child",
                    "exit_poll_interval_micros": 100, "rss_poll_interval_micros": null,
                    "validated_accuracy_micros": null,
                    "worker": {
                        "executable_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "launch_protocol": "worker-v1",
                        "collector_source_sha256": "collector"
                    },
                    "isolation_evidence": {
                        "summary_sha256": "summary",
                        "control_peak_rss_bytes": 2_000_000,
                        "max_parent_delta_bytes": 4_096,
                        "tolerance_bytes": 32_768
                    }
                },
                "conservative_observed_duration_error_bound_micros": 3000
            },
            "noop_isolation": {"within_tolerance": true},
            "allocation_delta_checks": [
                {"case": "burst-4m", "within_tolerance": true},
                {"case": "burst-16m", "within_tolerance": true},
                {"case": "threads-4m-x2", "within_tolerance": true},
                {"case": "threads-2m-x4", "within_tolerance": true}
            ],
            "short_burst_check": {"observed": true, "status": "verified"},
            "worker_isolation": {
                "collector_source_sha256": "collector",
                "coordinator_controls": [
                    {
                        "coordinator_allocation_bytes": 0,
                        "repetitions": 20,
                        "native_rss_median_bytes": 2_000_000,
                        "independent_time_rss_median_bytes": 2_000_000,
                        "rss_comparisons_within_tolerance": 20,
                        "cpu_comparisons_within_tolerance": 20,
                        "max_delta_from_zero_bytes": 0,
                        "tolerance_bytes": 32_768
                    },
                    {
                        "coordinator_allocation_bytes": 33_554_432,
                        "repetitions": 20,
                        "native_rss_median_bytes": 2_000_000,
                        "independent_time_rss_median_bytes": 2_000_000,
                        "rss_comparisons_within_tolerance": 20,
                        "cpu_comparisons_within_tolerance": 20,
                        "max_delta_from_zero_bytes": 4096,
                        "tolerance_bytes": 32_768
                    },
                    {
                        "coordinator_allocation_bytes": 134_217_728,
                        "repetitions": 20,
                        "native_rss_median_bytes": 2_000_000,
                        "independent_time_rss_median_bytes": 2_000_000,
                        "rss_comparisons_within_tolerance": 20,
                        "cpu_comparisons_within_tolerance": 20,
                        "max_delta_from_zero_bytes": 4096,
                        "tolerance_bytes": 32_768
                    }
                ],
                "residual_floor_bytes": 2_000_000,
                "prepared_stdin_bytes": 2_097_152,
                "prepared_stdin_repetitions": 20,
                "prepared_stdin_verified": true,
                "high_low_request_repetitions": 20,
                "high_low_request_verified": true,
                "independent_time_repetitions": 20,
                "independent_time_verified": true
            },
            "validation_failures": [],
            "cases": [
                {"case": "noop", "repetitions": 20, "rss_comparisons_within_tolerance": 20, "cpu_comparisons_within_tolerance": 20},
                {"case": "known-duration-20ms", "repetitions": 20, "native_duration_error_bound_micros": 1000, "rss_comparisons_within_tolerance": 20, "cpu_comparisons_within_tolerance": 20},
                {"case": "known-duration-100ms", "repetitions": 20, "native_duration_error_bound_micros": 2000, "rss_comparisons_within_tolerance": 20, "cpu_comparisons_within_tolerance": 20},
                {"case": "known-duration-250ms", "repetitions": 20, "native_duration_error_bound_micros": 3000, "rss_comparisons_within_tolerance": 20, "cpu_comparisons_within_tolerance": 20}
            ]
        })
    }

    fn load(value: &serde_json::Value) -> Result<TimingCalibration, String> {
        TimingCalibration::from_bytes(&serde_json::to_vec(value).unwrap(), "host", "collector")
            .map(|mut calibrations| calibrations.remove(0))
    }

    #[test]
    fn retained_controls_attach_bound_and_content_identity() {
        let calibration = load(&summary()).unwrap();
        let mut protocol = calibration.protocol.clone();
        protocol.isolation_evidence = None;
        calibration.apply(&mut protocol).unwrap();
        assert_eq!(protocol.validated_accuracy_micros, Some(3000));
        assert_eq!(
            protocol.isolation_evidence.as_ref().unwrap().summary_sha256,
            calibration.summary_sha256
        );
        assert!(
            protocol
                .timing_method
                .contains("not a universal accuracy guarantee")
        );
        assert!(protocol.timing_method.contains(&calibration.summary_sha256));
    }

    #[test]
    fn raw_worker_protocol_matches_but_stale_worker_does_not() {
        let calibration = load(&summary()).unwrap();
        let mut raw = calibration.protocol.clone();
        raw.isolation_evidence = None;
        assert!(calibration.matches(&raw));

        let mut stale = raw.clone();
        stale.worker.as_mut().unwrap().executable_sha256 = "stale".to_owned();
        assert!(!calibration.matches(&stale));
        assert!(calibration.apply(&mut stale).is_err());
    }

    #[test]
    fn failed_foreign_debug_or_stale_evidence_is_rejected() {
        for (field, replacement) in [
            ("machine_identity", "other"),
            ("collector_source_sha256", "old"),
            ("runner_build_profile", "debug"),
            ("status", "unverified"),
        ] {
            let mut value = summary();
            value["calibration_linkage"][field] = replacement.into();
            assert!(load(&value).is_err());
        }
        let mut value = summary();
        value["validation_failures"] = serde_json::json!(["RSS mismatch"]);
        assert!(load(&value).is_err());
    }

    #[test]
    fn missing_controls_and_invented_precision_are_rejected() {
        let mut value = summary();
        value["cases"][1]["repetitions"] = 1.into();
        assert!(load(&value).is_err());
        let mut value = summary();
        value["calibration_linkage"]["conservative_observed_duration_error_bound_micros"] =
            1.into();
        assert!(load(&value).is_err());
    }

    #[test]
    fn verified_legacy_summary_without_worker_proof_is_rejected() {
        let mut value = summary();
        value.as_object_mut().unwrap().remove("worker_isolation");
        assert!(load(&value).is_err());

        let mut value = summary();
        value["calibration_linkage"]["measurement_protocol"]
            .as_object_mut()
            .unwrap()
            .remove("worker");
        assert!(load(&value).is_err());

        let mut value = summary();
        value["calibration_linkage"]["measurement_protocol"]
            .as_object_mut()
            .unwrap()
            .remove("isolation_evidence");
        assert!(load(&value).is_err());
    }

    #[test]
    fn required_native_evidence_gates_calibration() {
        let mut value = summary();
        value["short_burst_check"]["observed"] = false.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["allocation_delta_checks"][0]["within_tolerance"] = false.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["noop_isolation"]["within_tolerance"] = false.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["cases"][1]["rss_comparisons_within_tolerance"] = 19.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["cases"][1]["cpu_comparisons_within_tolerance"] = 19.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["calibration_linkage"]["measurement_protocol"]["isolation_evidence"]["max_parent_delta_bytes"] =
            32_769.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["calibration_linkage"]["measurement_protocol"]["isolation_evidence"]["control_peak_rss_bytes"] =
            1.into();
        assert!(load(&value).is_err());
    }

    #[test]
    fn worker_evidence_derives_pass_from_controls_and_flags() {
        for (field, replacement) in [
            ("prepared_stdin_verified", false.into()),
            ("high_low_request_verified", false.into()),
            ("independent_time_verified", false.into()),
        ] {
            let mut value = summary();
            value["worker_isolation"][field] = replacement;
            assert!(load(&value).is_err(), "worker flag {field} was trusted");
        }

        let mut value = summary();
        value["worker_isolation"]["coordinator_controls"][1]["coordinator_allocation_bytes"] =
            1.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["worker_isolation"]["coordinator_controls"][1]["rss_comparisons_within_tolerance"] =
            19.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["worker_isolation"]["coordinator_controls"][1]["independent_time_rss_median_bytes"] =
            serde_json::Value::Null;
        assert!(load(&value).is_err());

        let mut value = summary();
        value["worker_isolation"]["residual_floor_bytes"] = 0.into();
        assert!(load(&value).is_err());
    }

    #[test]
    fn worker_identity_must_match_the_linked_collector() {
        let mut value = summary();
        value["calibration_linkage"]["measurement_protocol"]["worker"]["collector_source_sha256"] =
            "old".into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["calibration_linkage"]["measurement_protocol"]["worker"]["launch_protocol"] =
            "".into();
        assert!(load(&value).is_err());
    }

    #[test]
    fn worker_controls_reject_zero_page_size_or_wrong_residual_floor() {
        let mut value = summary();
        value["metadata"]["page_size_bytes"] = 0.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["worker_isolation"]["residual_floor_bytes"] = 1.into();
        assert!(load(&value).is_err());
    }

    #[test]
    fn worker_controls_reject_loose_tolerance_or_rss_mismatch() {
        let mut value = summary();
        value["worker_isolation"]["coordinator_controls"][1]["tolerance_bytes"] = 500_001.into();
        assert!(load(&value).is_err());

        let mut value = summary();
        value["worker_isolation"]["coordinator_controls"][1]["independent_time_rss_median_bytes"] =
            100_000_000.into();
        assert!(load(&value).is_err());
    }

    #[test]
    fn worker_controls_use_configured_repetition_count() {
        let mut value = summary();
        value["metadata"]["repetitions"] = 21.into();
        assert!(load(&value).is_err());
    }

    #[test]
    fn default_calibration_cannot_validate_instrumented_samples() {
        let calibration = load(&summary()).unwrap();
        let mut protocol = calibration.protocol.clone();
        protocol.rss_poll_interval_micros = Some(25_000);
        assert!(calibration.apply(&mut protocol).is_err());
    }

    #[test]
    fn sampler_evidence_uses_its_own_controls_and_bound() {
        let mut value = summary();
        let mut linkage = value["calibration_linkage"].clone();
        linkage["measurement_protocol"]["rss_poll_interval_micros"] = 25_000.into();
        linkage["conservative_observed_duration_error_bound_micros"] = 4000.into();
        value["sampler_calibration_linkage"] = linkage;
        let controls = value["cases"].as_array().unwrap().clone();
        for mut control in controls {
            control["case"] = format!("sampler-{}", control["case"].as_str().unwrap()).into();
            control["rss_limit_bytes"] = 134_217_728.into();
            if control["native_duration_error_bound_micros"].is_number() {
                control["native_duration_error_bound_micros"] = 4000.into();
            }
            value["cases"].as_array_mut().unwrap().push(control);
        }
        let calibrations = TimingCalibration::from_bytes(
            &serde_json::to_vec(&value).unwrap(),
            "host",
            "collector",
        )
        .unwrap();
        assert_eq!(calibrations.len(), 2);
        assert_eq!(calibrations[0].observed_bound_micros, 3000);
        assert_eq!(calibrations[1].observed_bound_micros, 4000);
        value["cases"][7]["rss_limit_bytes"] = serde_json::Value::Null;
        assert!(
            TimingCalibration::from_bytes(
                &serde_json::to_vec(&value).unwrap(),
                "host",
                "collector"
            )
            .is_err()
        );
    }
}
