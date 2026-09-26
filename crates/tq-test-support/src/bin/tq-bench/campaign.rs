//! Campaign execution: row checkpoints survive deadlines, cancellation, and later failures.

use super::{
    Options, PreparedCampaign, calibration::TimingCalibration, corpus_identity, deadline::Deadline,
    format_name, invocation, plan_rows, policy, validate_rows_against_plan, write_report,
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};
use tq_test_support::{
    benchmark::{
        BenchmarkAdapter, BenchmarkCampaignReport, BenchmarkCase, BenchmarkFinalStatus,
        BenchmarkOutcome, BenchmarkRow, BenchmarkRunnerError, BenchmarkSampling, BenchmarkTool,
        CampaignExecution, Comparability, MeasureError, RegressionGate, collect_environment,
        is_correctness_output_limit, load_benchmark_catalog, normalize_correctness_run,
        populate_reference_ratios, run_correctness_limit_probe, run_gated_row, unsupported_row,
    },
    compatibility::{ToolIdentity, ToolKind},
};

type Error = Box<dyn std::error::Error>;

struct Campaign<'a> {
    report: BenchmarkCampaignReport,
    output: &'a Path,
    calibrations: &'a [TimingCalibration],
    started: Instant,
}

impl Campaign<'_> {
    fn checkpoint(&mut self) -> Result<(), Error> {
        if let Some(execution) = &mut self.report.execution {
            execution.elapsed_seconds = self.started.elapsed().as_secs_f64();
        }
        if self
            .report
            .execution
            .as_ref()
            .is_some_and(|execution| execution.sampling == "quick")
            && self.report.final_status != BenchmarkFinalStatus::Incomplete
        {
            let status = self.report.final_status;
            self.report.final_status = BenchmarkFinalStatus::Incomplete;
            self.report.execution.as_mut().expect("execution").complete = false;
            let result = write_report(self.output, &self.report);
            self.report.execution.as_mut().expect("execution").complete = true;
            self.report.final_status = status;
            result
        } else {
            write_report(self.output, &self.report)
        }
    }

    fn interrupt(&mut self, context: &str, reason: impl std::fmt::Display) -> Result<(), Error> {
        let mut detail = format!("{context}: {reason}");
        if detail.len() > 2048 {
            let mut end = 2048;
            while !detail.is_char_boundary(end) {
                end -= 1;
            }
            detail.truncate(end);
        }
        eprintln!("tq-bench: {detail}");
        self.report
            .execution
            .as_mut()
            .expect("execution metadata")
            .interruptions
            .push(detail);
        self.checkpoint()
    }

    fn record(&mut self, mut row: BenchmarkRow) -> Result<(), Error> {
        if self.report.profile != "quick"
            && self
                .report
                .execution
                .as_ref()
                .is_none_or(|execution| execution.sampling != "quick")
        {
            eprintln!(
                "tq-bench: workload={} dataset={} adapter={} outcome={:?}",
                row.case_id, row.source_id, row.adapter_id, row.outcome
            );
        }
        let calibration_result = (|| -> Result<(), Error> {
            for sample in row.samples.iter_mut().chain(&mut row.instrumented_samples) {
                if !self.calibrations.is_empty() {
                    let protocol = sample
                        .measurement_protocol
                        .as_mut()
                        .ok_or("measured sample has no protocol for timing calibration")?;
                    self.calibrations
                        .iter()
                        .find(|calibration| calibration.matches(protocol))
                        .ok_or("no matching timing calibration for sample instrumentation")?
                        .apply(protocol)?;
                }
            }
            Ok(())
        })();
        // Retain the completed row even if its calibration evidence is rejected.
        self.report.cases.push(row);
        self.checkpoint()?;
        calibration_result
    }
}

pub(super) fn initial_report(
    options: &Options,
    prepared: &PreparedCampaign,
    tools: &BTreeMap<BenchmarkTool, ToolIdentity>,
    planned_rows: usize,
) -> BenchmarkCampaignReport {
    let corpus = prepared
        .datasets
        .iter()
        .flat_map(|dataset| {
            dataset
                .formats
                .iter()
                .map(|(format, (_, artifact))| corpus_identity(dataset, format, artifact))
        })
        .collect();
    BenchmarkCampaignReport {
        schema_version: 1,
        campaign_id: jiff::Timestamp::now().to_string(),
        suite: options.suite.clone(),
        profile: options.profile.clone(),
        environment: collect_environment(if cfg!(debug_assertions) {
            "debug-benchmark"
        } else {
            "release-benchmark"
        }),
        corpus,
        tools: tools.values().cloned().collect(),
        cases: Vec::new(),
        comparability: Comparability::default(),
        regression_gate: RegressionGate::default(),
        final_status: BenchmarkFinalStatus::Incomplete,
        execution: Some(CampaignExecution {
            mode: options.mode.label().to_owned(),
            sampling: if options.max_samples.is_some() {
                "explicit"
            } else {
                options.sampling.label()
            }
            .to_owned(),
            instrument_rss: options.instrument_rss,
            campaign_budget_seconds: options.campaign_budget_seconds,
            case_budget_seconds: options.case_budget_seconds,
            planned_rows,
            elapsed_seconds: 0.0,
            complete: false,
            interruptions: Vec::new(),
        }),
    }
}

fn configure_case(case: &mut BenchmarkCase, options: &Options) {
    options.sampling.apply(case);
    if let Some(samples) = options.max_samples {
        case.sampling = BenchmarkSampling {
            warmups: usize::from(samples > 1),
            small: samples,
            medium: samples,
            large: samples,
        };
    }
    if let Some(timeout) = options.timeout_seconds {
        case.timeout_seconds = timeout;
    }
    if let Some(limit) = options.rss_limit_bytes {
        case.limits.rss_bytes = Some(
            case.limits
                .rss_bytes
                .map_or(limit, |catalog| catalog.min(limit)),
        );
    }
}

pub(super) fn execute(
    options: &Options,
    prepared: &PreparedCampaign,
    tools: &BTreeMap<BenchmarkTool, ToolIdentity>,
    calibrations: &[TimingCalibration],
    deadline: &Deadline,
    started: Instant,
    root: &Path,
) -> Result<BenchmarkCampaignReport, Error> {
    let mut catalog = load_benchmark_catalog(&root.join("benchmarks/cases"))?;
    policy::select_adapters(&mut catalog.cases, &options.selected_adapters, options.mode)?;
    let planned = plan_rows(&catalog.cases, &options.selected_cases, &prepared.datasets)?;
    if planned.is_empty() {
        return Err("selected benchmark plan has no rows".into());
    }
    let report = initial_report(options, prepared, tools, planned.len());
    let mut campaign = Campaign {
        report,
        output: &options.output,
        calibrations,
        started,
    };
    campaign.checkpoint()?;
    if options.is_quick() || options.profile == "standard" {
        eprintln!("tq-bench: report checkpoint: {}", options.output.display());
    }
    for mut case in catalog.cases {
        if !options.selected_cases.is_empty() && !options.selected_cases.contains(&case.id) {
            continue;
        }
        if deadline.cancelled() {
            break;
        }
        configure_case(&mut case, options);
        let case_deadline = begin_case(&case, options, deadline)?;
        let outcome = run_case(
            &case,
            prepared,
            tools,
            &case_deadline,
            options.instrument_rss,
            &mut campaign,
        );
        if let Err(error) = outcome {
            if deadline.cancelled() {
                break;
            }
            if case_deadline.expired() && clean_cancellation(&error) {
                campaign.interrupt(&case.id, "case wall-clock budget exhausted")?;
            } else {
                campaign.interrupt(&case.id, error)?;
                // Infrastructure failure can leave cleanup pending; never time another
                // case while child ownership or the collector is uncertain.
                break;
            }
        } else if case_deadline.expired() {
            campaign.interrupt(&case.id, "case wall-clock budget exhausted")?;
        }
    }
    if deadline.cancelled() {
        campaign.interrupt(
            "campaign",
            if deadline.expired() {
                "campaign wall-clock budget exhausted"
            } else {
                "cancelled by signal"
            },
        )?;
    }
    if let Err(error) = campaign.report.validate_authoritative_rss() {
        campaign.interrupt("measurement evidence", error)?;
    }
    let execution = campaign
        .report
        .execution
        .as_mut()
        .expect("execution metadata");
    execution.complete =
        execution.interruptions.is_empty() && campaign.report.cases.len() == planned.len();
    if execution.complete {
        validate_rows_against_plan(&planned, &campaign.report.cases)?;
        campaign.report.final_status = if campaign.report.cases.iter().any(|row| {
            !matches!(
                row.outcome,
                BenchmarkOutcome::Timed | BenchmarkOutcome::Unsupported
            )
        }) {
            BenchmarkFinalStatus::ObservedFailures
        } else {
            BenchmarkFinalStatus::Passed
        };
    }
    populate_reference_ratios(
        &mut campaign.report.cases,
        &[
            "jq-json",
            "yq-json",
            "yq-yaml",
            "jq-json-seq",
            "yq-csv",
            "yq-tsv",
        ],
    );
    campaign.checkpoint()?;
    Ok(campaign.report)
}

fn clean_cancellation(error: &Error) -> bool {
    match error.downcast_ref::<BenchmarkRunnerError>() {
        Some(BenchmarkRunnerError::Measure(MeasureError::Cancelled)) => true,
        Some(BenchmarkRunnerError::Measure(MeasureError::Collection {
            source,
            exit_code,
            signal,
            ..
        })) => {
            matches!(source.as_ref(), MeasureError::Cancelled)
                && (exit_code.is_some() || signal.is_some())
        }
        _ => false,
    }
}

fn run_case(
    case: &BenchmarkCase,
    prepared: &PreparedCampaign,
    tools: &BTreeMap<BenchmarkTool, ToolIdentity>,
    deadline: &Deadline,
    instrument_rss: bool,
    campaign: &mut Campaign<'_>,
) -> Result<(), Error> {
    for dataset in prepared.datasets.iter().filter(|dataset| {
        case.dataset_selector.tiers.contains(&dataset.tier)
            && super::family_matches(case.dataset_selector.family, dataset)
    }) {
        if deadline.cancelled() {
            return Err(BenchmarkRunnerError::Measure(MeasureError::Cancelled).into());
        }
        let reference_adapter = case
            .adapters
            .iter()
            .find(|adapter| adapter.id == case.output_contract.reference_adapter)
            .ok_or("reference adapter missing from selected case")?;
        let reference_invocation = invocation(
            case,
            reference_adapter,
            dataset,
            &tools[&reference_adapter.tool],
            deadline.flag(),
        )?;
        let reference = match normalize_correctness_run(
            &reference_invocation,
            tool_kind(reference_adapter),
            case.output_contract.kind,
        ) {
            Ok(reference) => Some(reference),
            Err(error) if is_correctness_output_limit(&error) => None,
            Err(error) => return Err(error.into()),
        };
        for adapter in &case.adapters {
            if deadline.cancelled() {
                return Err(BenchmarkRunnerError::Measure(MeasureError::Cancelled).into());
            }
            let identity = dataset
                .formats
                .get(format_name(adapter.input_format))
                .ok_or("missing prepared representation")?;
            let corpus = corpus_identity(dataset, format_name(adapter.input_format), &identity.1);
            let invocation = invocation(
                case,
                adapter,
                dataset,
                &tools[&adapter.tool],
                deadline.flag(),
            )?;
            let row = if !adapter.applicable {
                unsupported_row(case, adapter, &corpus, dataset.tier, &invocation)
            } else if let Some(reference) = &reference {
                run_gated_row(
                    case,
                    adapter,
                    &corpus,
                    dataset.tier,
                    &invocation,
                    reference,
                    instrument_rss,
                )?
            } else {
                run_correctness_limit_probe(case, adapter, &corpus, dataset.tier, &invocation)?
            };
            campaign.record(row)?;
        }
    }
    Ok(())
}

const fn tool_kind(adapter: &BenchmarkAdapter) -> ToolKind {
    match adapter.tool {
        BenchmarkTool::Jq => ToolKind::Jq,
        BenchmarkTool::Yq => ToolKind::Yq,
        BenchmarkTool::Tq => ToolKind::Tq,
    }
}

fn begin_case(
    case: &BenchmarkCase,
    options: &Options,
    parent: &Deadline,
) -> Result<Deadline, std::io::Error> {
    let duration = Duration::from_secs(options.case_budget_seconds);
    let duration = if options.is_quick() {
        duration
            .min(parent.remaining())
            .min(tq_test_support::benchmark::quick::remaining_work_budget().unwrap_or(duration))
    } else {
        duration
    };
    let deadline = Deadline::start(
        Arc::new(AtomicBool::new(false)),
        Some(Arc::clone(parent.flag())),
        duration,
    )?;
    if !options.is_quick() {
        eprintln!(
            "tq-bench: starting {} (case budget {}s)",
            case.id, options.case_budget_seconds
        );
    }
    Ok(deadline)
}
