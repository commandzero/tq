//! Render explain output and write execution reports.

use super::RunError;
use super::automatic::RetentionObservations;
use super::detection::DetectionObservation;
use crate::{ExplainFormat, RunOptions};
use std::fs;
use std::io::Write;
use std::path::Path;
use tq_core::{
    Analysis, Analyzed, PlanKind, Query, TranscodeInput, TranscodeProof, parallel_worker_count,
};
use tq_formats::{InputFormat, NativeFormat};
use tq_toon::PreparationObservations;

const JSON_DUPLICATE_LIMITATION: &str = "JSON duplicates reject the current streamed record; use a document plan for last-value normalization";

fn duplicate_key_limitation(proof: Option<TranscodeProof>) -> Option<&'static str> {
    proof
        .is_some_and(|proof| proof.input == TranscodeInput::Json)
        .then_some(JSON_DUPLICATE_LIMITATION)
}

#[allow(
    clippy::too_many_lines,
    reason = "human and JSON plan explanations stay aligned in one function"
)]
pub(super) fn write_explain(
    format: ExplainFormat,
    options: &RunOptions,
    analyzed: &Query<Analyzed>,
    analysis: &Analysis,
    stderr: &mut impl Write,
) -> Result<(), RunError> {
    let capabilities = analyzed.capabilities();
    let plan = analysis.selected_plan;
    let staged_toon_stream = options.stream && !options.strict;
    let retained = if staged_toon_stream {
        "when input is non-strict TOON, complete documents are staged for last-write-wins before semantic event projection"
    } else {
        match plan {
            PlanKind::Transcode => {
                "whole object/array candidates and recursive schemas in a bounded preparation arena; spills only as needed"
            }
            PlanKind::Events if capabilities.fold_state => {
                "decoder frames, current path, one event value, and one fold accumulator"
            }
            PlanKind::Events => "decoder frames, current path, and one scalar event value",
            PlanKind::WholeInput => "all input documents",
            PlanKind::Blocking => "one document plus blocking operator state",
            PlanKind::Subtree => "one selected complete subtree",
            PlanKind::HybridBlocking => {
                "bounded decoder state plus cardinality-proportional projected collection and blocking state"
            }
            PlanKind::Document => "one complete document",
        }
    };
    let spool_policy = if plan == PlanKind::Transcode {
        "whole object/array shape preparation remains in memory within its budget and spills only as required, subject to the spool limit"
    } else {
        "not used by this plan"
    };
    let detection = input_format_name(options.input_format);
    match format {
        ExplainFormat::Human => {
            stderr.write_all(analyzed.explain().as_bytes())?;
            writeln!(stderr, "plan: {plan}")?;
            writeln!(stderr, "input-detection: {detection}")?;
            writeln!(stderr, "retained-working-set: {retained}")?;
            writeln!(stderr, "blocking: {}", capabilities.blocking)?;
            writeln!(stderr, "spool-policy: {spool_policy}")?;
            if let Some(proof) = &analysis.transcode_proof {
                writeln!(stderr, "identity-proof: semantic-identity")?;
                writeln!(stderr, "duplicate-policy: {:?}", proof.duplicate_policy)?;
                if let Some(limitation) = duplicate_key_limitation(Some(*proof)) {
                    writeln!(stderr, "duplicate-key-limitation: {limitation}")?;
                }
                writeln!(stderr, "commitment-mode: {:?}", proof.commitment)?;
            }
            if let Some(proof) = &analysis.stream_proof {
                writeln!(
                    stderr,
                    "required-path-prefix: {:?}",
                    proof.required_path_prefix
                )?;
                writeln!(stderr, "subtree-complete: {}", proof.subtree_complete)?;
                writeln!(stderr, "value-escapes: {}", proof.value_escapes)?;
                writeln!(stderr, "retention-high-water: available in --report-file")?;
            }
            if let Some(proof) = &analysis.hybrid_proof {
                writeln!(
                    stderr,
                    "collection-boundary: {}..{}",
                    proof.collection.start, proof.collection.end
                )?;
                writeln!(
                    stderr,
                    "blocking-cause: {}..{}",
                    proof.blocking_cause.start, proof.blocking_cause.end
                )?;
                writeln!(stderr, "hybrid-preparation: {:?}", proof.preparation)?;
                let (eligible, reason) = parallel_selected_decode_explain(options, analysis);
                writeln!(
                    stderr,
                    "parallel-selected-decode: {} ({reason})",
                    if eligible {
                        "eligible"
                    } else {
                        "serial-fallback"
                    }
                )?;
            }
            for rewrite in &analysis.optimizer_rewrites {
                writeln!(
                    stderr,
                    "optimizer-rewrite: {} at {}..{}",
                    rewrite.name, rewrite.span.start, rewrite.span.end
                )?;
            }
            if let Some(rejection) = &analysis.stream_rejection {
                writeln!(stderr, "stream-rejection: {rejection}")?;
            }
            if let Some(rejection) = &analysis.transcode_rejection {
                writeln!(stderr, "transcode-rejection: {rejection}")?;
            }
            writeln!(
                stderr,
                "limits: input={} depth={} token={} line={} lookahead={} vm-steps={} results={} output={} prepare-memory={} hybrid-batch-values={} hybrid-in-flight-batches={} hybrid-in-flight-bytes={} decode-batch-values={} decode-batch-bytes={} decode-in-flight-batches={} decode-in-flight-bytes={} spool={}",
                options.limits.input_bytes,
                options.limits.depth,
                options.limits.token_bytes,
                options.limits.line_bytes,
                options.limits.lookahead_bytes,
                options.limits.vm_steps,
                options.limits.results,
                options.limits.output_bytes,
                options.limits.preparation_memory_bytes,
                options.limits.hybrid_batch_values,
                options.limits.hybrid_in_flight_batches,
                options.limits.hybrid_in_flight_bytes,
                options.limits.decode_batch_values,
                options.limits.decode_batch_bytes,
                options.limits.decode_in_flight_batches,
                options.limits.decode_in_flight_bytes,
                options.limits.spool_bytes,
            )?;
        }
        ExplainFormat::Json => {
            let mut report = analyzed.explain_json();
            report["execution"] = serde_json::json!({
                "plan": plan.to_string(),
            "input_projection": if options.stream { "jq-stream" } else { "documents" },
                "input_detection": detection,
                "retained_working_set": retained,
                "blocking": capabilities.blocking,
                "spool_policy": spool_policy,
                "preparation_policy": if staged_toon_stream {
                    "when selected input is non-strict TOON, stage each complete document to resolve last-write-wins before EventProjector; other inputs use their selected stream route"
                } else if plan == PlanKind::Transcode {
                    "stage whole object/array candidates and recursive schemas; spill as required by preparation-memory budget"
                } else {
                    "not used by this plan"
                },
                "proof": if plan == PlanKind::Transcode {
                    serde_json::to_value(analysis.transcode_proof)?
                } else {
                    serde_json::to_value(&analysis.stream_proof)?
                },
                "stream_rejection": analysis.stream_rejection,
                "hybrid_proof": analysis.hybrid_proof,
                "parallel_selected_decode": {
                    "eligible": parallel_selected_decode_explain(options, analysis).0,
                    "reason": parallel_selected_decode_explain(options, analysis).1,
                    "worker_count": parallel_worker_count(),
                },
                "optimizer_rewrites": analysis.optimizer_rewrites,
                "transcode_rejection": analysis.transcode_rejection,
                "identity_proof": analysis.transcode_proof.map(|_| "semantic-identity"),
                "duplicate_policy": analysis.transcode_proof.map(|proof| proof.duplicate_policy),
                "duplicate_key_limitation": duplicate_key_limitation(analysis.transcode_proof),
                "commitment_mode": analysis.transcode_proof.map(|proof| proof.commitment),
                "high_water": {
                    "available_in_report": true
                },
                "limits": {
                    "input_bytes": options.limits.input_bytes,
                    "depth": options.limits.depth,
                    "token_bytes": options.limits.token_bytes,
                    "line_bytes": options.limits.line_bytes,
            "frame_bytes": options.limits.frame_bytes,
            "fields": options.limits.fields,
                    "lookahead_bytes": options.limits.lookahead_bytes,
                    "vm_steps": options.limits.vm_steps,
                    "results": options.limits.results,
                    "output_bytes": options.limits.output_bytes,
                    "preparation_memory_bytes": options.limits.preparation_memory_bytes,
                    "hybrid_batch_values": options.limits.hybrid_batch_values,
                    "hybrid_in_flight_batches": options.limits.hybrid_in_flight_batches,
                    "hybrid_in_flight_bytes": options.limits.hybrid_in_flight_bytes,
                    "decode_batch_values": options.limits.decode_batch_values,
                    "decode_batch_bytes": options.limits.decode_batch_bytes,
                    "decode_in_flight_batches": options.limits.decode_in_flight_batches,
                    "decode_in_flight_bytes": options.limits.decode_in_flight_bytes,
                    "spool_bytes": options.limits.spool_bytes,
                }
            });
            serde_json::to_writer_pretty(&mut *stderr, &report)?;
            stderr.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn parallel_selected_decode_explain<'a>(
    options: &RunOptions,
    analysis: &'a Analysis,
) -> (bool, &'a str) {
    if analysis.selected_plan != PlanKind::HybridBlocking {
        return (
            false,
            analysis
                .stream_rejection
                .as_deref()
                .unwrap_or("plan-is-not-hybrid-blocking"),
        );
    }
    if parallel_worker_count() <= 1 {
        return (false, "one-worker-configured");
    }
    if !matches!(options.input_format, InputFormat::Auto | InputFormat::Json) {
        return (false, "input-format-is-not-json-document");
    }
    if analysis.hybrid_proof.is_none() {
        return (false, "hybrid-plan-has-no-static-prefix-proof");
    }
    // Automatic JSON roots are transactional: selected values and effects are
    // staged until the decoder commits the containing root.  The current
    // executor therefore cannot safely use the old cross-root parallel
    // collector even when the static prefix would otherwise qualify.
    (false, "root-lifecycle-requires-serial-commit")
}

fn input_format_name(format: InputFormat) -> String {
    match NativeFormat::from_input(format) {
        None => "auto:toon/json/yaml-bounded-probe".to_owned(),
        Some(native) => format!("override:{}", native.report_name()),
    }
}

pub(super) const fn concrete_input_format_name(format: InputFormat) -> &'static str {
    match NativeFormat::from_input(format) {
        None => "auto",
        Some(native) => native.report_name(),
    }
}
pub(super) fn write_report(
    path: &Path,
    observations: &[tq_core::VmObservations],
    results: usize,
    output_bytes: u64,
    options: &RunOptions,
    plan: PlanKind,
    execution: ReportExecution<'_>,
) -> Result<(), RunError> {
    let staged_toon_stream = options.stream && !options.strict;
    let ReportExecution {
        analysis,
        retention,
        resource_outcome,
    } = execution;
    let report = serde_json::json!({
        "schema_version": 1,
        "documents": observations.len(),
        "results": results,
        "output_bytes": output_bytes,
        "execution": {
            "plan": plan.to_string(),
            "input_projection": if options.stream { "jq-stream" } else { "documents" },
            "proof": analysis.stream_proof,
            "hybrid_proof": analysis.hybrid_proof,
            "optimizer_rewrites": analysis.optimizer_rewrites,
            "stream_rejection": analysis.stream_rejection,
            "retained_working_set": if staged_toon_stream {
                "when input is non-strict TOON, complete documents are staged for last-write-wins before semantic event projection"
            } else {
                match plan {
                    PlanKind::Transcode => "whole object/array candidates and recursive schemas in bounded preparation",
                    PlanKind::Events => "decoder events",
                    PlanKind::Subtree => "selected subtree",
                    PlanKind::HybridBlocking => "projected collection and blocking state",
                    PlanKind::Document => "document",
                    PlanKind::WholeInput => "whole input",
                    PlanKind::Blocking => "document and blocking state",
                }
            },
            "retention_high_water": {
                "bytes": retention.bytes_high_water,
                "depth": retention.depth_high_water,
                "decoder_depth": retention.decoder_depth_high_water,
                "completed_subtrees": retention.completed_subtrees,
                "retained_result_count": retention.retained_results_high_water,
                "retained_estimated_bytes": retention.retained_bytes_high_water,
                "blocking_state": if plan == PlanKind::HybridBlocking { "projected-collection-and-blocking-suffix" } else { "none" },
                "sort_runs": retention.sort_runs,
                "in_flight_batches": retention.in_flight_batches_high_water,
                "in_flight_bytes": retention.in_flight_bytes_high_water,
                "worker_count": parallel_worker_count(),
                "parallel_decode_batches": retention.decode_batches,
                "parallel_decode_in_flight_batches": retention.decode_in_flight_batches_high_water,
                "parallel_decode_in_flight_bytes": retention.decode_in_flight_bytes_high_water,
                "parallel_decode_reordered_batches": retention.decode_reordered_batches_high_water,
                "parallel_decode_active": retention.decode_batches > 0,
                "runtime_spool_fixed_io_bytes": retention.runtime_spool_fixed_io_bytes_high_water,
                "root_staging": {
                    "encoded_bytes_high_water": retention.root_staging_encoded_bytes_high_water,
                    "memory_bytes_high_water": retention.root_staging_memory_bytes_high_water,
                    "index_bytes_high_water": retention.root_staging_index_bytes_high_water,
                    "spool_bytes_written": retention.root_staging_spool_bytes_written_high_water,
                    "fixed_io_bytes_high_water": retention.runtime_spool_fixed_io_bytes_high_water,
                },
                "root_materialized": if staged_toon_stream {
                    None
                } else {
                    Some(!matches!(
                        plan,
                        PlanKind::Events | PlanKind::Subtree | PlanKind::HybridBlocking | PlanKind::Transcode
                    ))
                },
            },
            "resource_outcome": resource_outcome,
        },
        "limits": {
            "input_bytes": options.limits.input_bytes,
            "depth": options.limits.depth,
            "token_bytes": options.limits.token_bytes,
            "line_bytes": options.limits.line_bytes,
            "frame_bytes": options.limits.frame_bytes,
            "fields": options.limits.fields,
            "lookahead_bytes": options.limits.lookahead_bytes,
            "vm_steps": options.limits.vm_steps,
            "results": options.limits.results,
            "output_bytes": options.limits.output_bytes,
            "preparation_memory_bytes": options.limits.preparation_memory_bytes,
            "hybrid_batch_values": options.limits.hybrid_batch_values,
            "hybrid_in_flight_batches": options.limits.hybrid_in_flight_batches,
            "hybrid_in_flight_bytes": options.limits.hybrid_in_flight_bytes,
            "decode_batch_values": options.limits.decode_batch_values,
            "decode_batch_bytes": options.limits.decode_batch_bytes,
            "decode_in_flight_batches": options.limits.decode_in_flight_batches,
            "decode_in_flight_bytes": options.limits.decode_in_flight_bytes,
            "spool_bytes": options.limits.spool_bytes,
        },
        "observations": observations.iter().map(|item| serde_json::json!({
            "value_stack_high_water": item.value_stack_high_water,
            "call_stack_high_water": item.call_stack_high_water,
            "path_stack_high_water": item.path_stack_high_water,
            "fork_stack_high_water": item.fork_stack_high_water,
            "steps": item.steps,
            "results": item.results,
        })).collect::<Vec<_>>()
    });
    fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct TranscodeReportExecution<'a> {
    pub(super) documents: u64,
    pub(super) output_bytes: u64,
    pub(super) observations: PreparationObservations,
    pub(super) resource_outcome: &'a str,
}

pub(super) fn write_transcode_report(
    path: &Path,
    options: &RunOptions,
    analysis: &Analysis,
    detections: &[DetectionObservation],
    execution: TranscodeReportExecution<'_>,
) -> Result<(), RunError> {
    let TranscodeReportExecution {
        documents,
        output_bytes,
        observations,
        resource_outcome,
    } = execution;
    let proof = analysis
        .transcode_proof
        .expect("transcode report carries a selected proof");
    let detection = detections
        .iter()
        .map(|item| {
            serde_json::json!({
                "identity": item.identity,
                "selected_input_format": concrete_input_format_name(item.selected),
                "lookahead_bytes": item.lookahead_bytes,
                "commitment_bytes": item.commitment_bytes,
                "rejections": item.rejections.iter().map(|(format, reason)| serde_json::json!({
                    "format": concrete_input_format_name(*format),
                    "reason": reason,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "schema_version": 1,
        "documents": documents,
        "results": documents,
        "output_bytes": output_bytes,
        "execution": {
            "plan": "transcode",
            "selected_input_format": concrete_input_format_name(match proof.input {
                TranscodeInput::Json => InputFormat::Json,
                TranscodeInput::Toon => InputFormat::Toon,
            }),
            "input_format_source": if options.input_format == InputFormat::Auto { "detected" } else { "override" },
            "detection": detection,
            "proof": analysis.transcode_proof,
            "transcode_rejection": analysis.transcode_rejection,
            "duplicate_key_limitation": duplicate_key_limitation(analysis.transcode_proof),
            "retained_working_set": "whole object/array candidates and recursive schemas in a bounded preparation arena; spills only as needed within configured limits",
            "preparation_policy": "replay complete candidate object/array bodies after recursive schema and layout selection; use spool only when memory preparation limits require it",
            "spooled": observations.spool_bytes_written > 0,
            "commitment_mode": proof.commitment,
            "input_stage_bytes_written": 0,
            "input_stage_bytes_replayed": 0,
            "preparation_high_water_bytes": observations.memory_high_water_bytes,
            "preparation_nesting_high_water": observations.nesting_high_water,
            "array_preparations": observations.array_preparations,
            "object_preparations": observations.object_preparations,
            "spool_bytes_written": observations.spool_bytes_written,
            "spool_bytes_replayed": observations.spool_bytes_replayed,
            "prepared_output_bytes": observations.output_bytes,
            "resource_outcome": resource_outcome,
        },
        "limits": {
            "input_bytes": options.limits.input_bytes,
            "depth": options.limits.depth,
            "token_bytes": options.limits.token_bytes,
            "line_bytes": options.limits.line_bytes,
            "frame_bytes": options.limits.frame_bytes,
            "fields": options.limits.fields,
            "lookahead_bytes": options.limits.lookahead_bytes,
            "results": options.limits.results,
            "output_bytes": options.limits.output_bytes,
            "preparation_memory_bytes": options.limits.preparation_memory_bytes,
            "hybrid_batch_values": options.limits.hybrid_batch_values,
            "hybrid_in_flight_batches": options.limits.hybrid_in_flight_batches,
            "hybrid_in_flight_bytes": options.limits.hybrid_in_flight_bytes,
            "decode_batch_values": options.limits.decode_batch_values,
            "decode_batch_bytes": options.limits.decode_batch_bytes,
            "decode_in_flight_batches": options.limits.decode_in_flight_batches,
            "decode_in_flight_bytes": options.limits.decode_in_flight_bytes,
            "spool_bytes": options.limits.spool_bytes,
        },
        "observations": [],
    });
    fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct ReportExecution<'a> {
    pub(super) analysis: &'a Analysis,
    pub(super) retention: RetentionObservations,
    pub(super) resource_outcome: &'a str,
}
