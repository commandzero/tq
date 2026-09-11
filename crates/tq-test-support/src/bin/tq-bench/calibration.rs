//! Attach retained native control evidence without claiming clock-resolution accuracy.

use std::{fmt::Write as _, fs, path::Path};

use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use tq_test_support::benchmark::{
    MeasurementProtocol, collect_environment, collector_source_sha256,
};

#[derive(Deserialize)]
struct Summary {
    metadata: Metadata,
    calibration_linkage: Linkage,
    sampler_calibration_linkage: Option<Linkage>,
    noop_isolation: Option<NoopIsolation>,
    allocation_delta_checks: Vec<AllocationDeltaCheck>,
    short_burst_check: ShortBurstCheck,
    validation_failures: Vec<String>,
    cases: Vec<Control>,
}

#[derive(Deserialize)]
struct Metadata {
    repetitions: usize,
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
        if !summary
            .noop_isolation
            .as_ref()
            .is_some_and(|isolation| isolation.within_tolerance)
        {
            return Err("timing calibration lacks successful noop isolation".to_owned());
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
        let sampled = linkage
            .measurement_protocol
            .rss_poll_interval_micros
            .is_some();
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
            if control.repetitions < 20 {
                return Err(format!(
                    "timing calibration {name} has fewer than 20 samples"
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
        let summary_sha256 =
            Sha256::digest(bytes)
                .iter()
                .fold(String::with_capacity(64), |mut hex, byte| {
                    write!(hex, "{byte:02x}").expect("write digest to string");
                    hex
                });
        Ok(Self {
            protocol: linkage.measurement_protocol.clone(),
            observed_bound_micros,
            summary_sha256,
        })
    }

    pub(super) fn apply(&self, protocol: &mut MeasurementProtocol) -> Result<(), String> {
        if protocol != &self.protocol {
            return Err("sample protocol differs from timing calibration; instrumented runs require their own controls".to_owned());
        }
        protocol.validated_accuracy_micros = Some(self.observed_bound_micros);
        write!(protocol.timing_method,
            "; observed known-duration control bound including spawn and sleep scheduling, not a universal accuracy guarantee; calibration SHA-256 {}",
            self.summary_sha256,
        ).expect("write calibration reference");
        Ok(())
    }

    pub(super) fn matches(&self, protocol: &MeasurementProtocol) -> bool {
        protocol == &self.protocol
    }
}

#[cfg(test)]
mod tests {
    use super::TimingCalibration;

    fn summary() -> serde_json::Value {
        serde_json::json!({
            "metadata": {"repetitions": 20},
            "calibration_linkage": {
                "machine_identity": "host", "collector_source_sha256": "collector",
                "runner_build_profile": "release", "status": "verified",
                "measurement_protocol": {
                    "timing_method": "native", "input_delivery": "file", "rss_scope": "child",
                    "exit_poll_interval_micros": 100, "rss_poll_interval_micros": null,
                    "validated_accuracy_micros": null
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
        calibration.apply(&mut protocol).unwrap();
        assert_eq!(protocol.validated_accuracy_micros, Some(3000));
        assert!(
            protocol
                .timing_method
                .contains("not a universal accuracy guarantee")
        );
        assert!(protocol.timing_method.contains(&calibration.summary_sha256));
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
