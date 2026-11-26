//! Release work owned by a failed branch and locate its recovery continuation.

use super::managed_paths::{PathWork, path_candidate_with_aliases, schedule_bind_alternative};
use super::managed_work::{
    GeneratorContinuation, GeneratorTask, GeneratorWork, task_owned_by_assignment_boundary,
    task_owned_by_bind_alternative, task_owned_by_catch, task_owned_by_label,
    task_owned_by_optional, task_owned_by_path_collection, task_owned_by_pull_consumer,
    task_owned_by_sql,
};
use super::structured_paths::is_retryable_alternative_error;
use super::{UserFrames, catch_value, is_catchable_error};
use crate::{Bytecode, Path, VmError, VmLimits, VmObservations};
use std::sync::atomic::AtomicBool;

pub(super) struct ErrorUnwind<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) limits: VmLimits,
    pub(super) cancellation: Option<&'a AtomicBool>,
    pub(super) stop: &'a AtomicBool,
    pub(super) pending: &'a mut Vec<GeneratorTask>,
    pub(super) observations: &'a mut VmObservations,
}

impl ErrorUnwind<'_> {
    #[cold]
    pub(super) fn recover(
        self,
        mut frames: UserFrames,
        mut continuations: Vec<GeneratorContinuation>,
        mut error: VmError,
    ) -> Option<VmError> {
        let Self {
            bytecode,
            limits,
            cancellation,
            stop,
            pending,
            observations,
        } = self;
        while let Some(continuation) = continuations.pop() {
            let path_catch = matches!(&continuation, GeneratorContinuation::PathCatch { .. });
            match continuation {
                GeneratorContinuation::OptionalBoundary { boundary }
                    if is_catchable_error(&error) =>
                {
                    pending.retain(|task| !task_owned_by_optional(task, boundary));
                    return None;
                }
                GeneratorContinuation::IgnoreError if is_catchable_error(&error) => {
                    return None;
                }
                GeneratorContinuation::Catch {
                    boundary,
                    node,
                    origin,
                    environment,
                }
                | GeneratorContinuation::PathCatch {
                    boundary,
                    node,
                    origin,
                    environment,
                } if is_catchable_error(&error) => {
                    pending.retain(|task| !task_owned_by_catch(task, boundary));
                    let Some(node) = node else {
                        return None;
                    };
                    let work = GeneratorWork {
                        node,
                        input: catch_value(&error),
                        origin,
                        environment,
                        frames,
                        continuations,
                    };
                    pending.push(recovery_task(bytecode, work, path_catch));
                    return None;
                }
                GeneratorContinuation::BindAlternativeBody {
                    boundary,
                    patterns,
                    next,
                    body,
                    value,
                    input,
                    environment,
                } => {
                    pending.retain(|task| !task_owned_by_bind_alternative(task, boundary));
                    if !is_retryable_alternative_error(&error) {
                        continue;
                    }
                    if next >= patterns.len() {
                        continue;
                    }
                    match schedule_bind_alternative(
                        bytecode,
                        patterns,
                        next,
                        value,
                        input,
                        environment,
                        body,
                        frames.clone(),
                        continuations.clone(),
                        boundary,
                        pending,
                        limits,
                        cancellation,
                        stop,
                        observations,
                        false,
                        Path::root(),
                    ) {
                        Ok(()) => {
                            return None;
                        }
                        Err(next_error) => error = next_error,
                    }
                }
                GeneratorContinuation::PathBindAlternativeBody {
                    boundary,
                    patterns,
                    next,
                    body,
                    value,
                    input,
                    prefix,
                    environment,
                } => {
                    pending.retain(|task| !task_owned_by_bind_alternative(task, boundary));
                    if !is_retryable_alternative_error(&error) {
                        continue;
                    }
                    if next >= patterns.len() {
                        continue;
                    }
                    match schedule_bind_alternative(
                        bytecode,
                        patterns,
                        next,
                        value,
                        input,
                        environment,
                        body,
                        frames.clone(),
                        continuations.clone(),
                        boundary,
                        pending,
                        limits,
                        cancellation,
                        stop,
                        observations,
                        true,
                        prefix,
                    ) {
                        Ok(()) => {
                            return None;
                        }
                        Err(next_error) => error = next_error,
                    }
                }
                GeneratorContinuation::AssignmentUpdateRhs { boundary, .. } => {
                    pending.retain(|task| !task_owned_by_assignment_boundary(task, boundary));
                }
                GeneratorContinuation::PullConsumer { state } => {
                    pending
                        .retain(|task| !task_owned_by_pull_consumer(task, state.borrow().boundary));
                }
                GeneratorContinuation::PathCollect { boundary, .. } => {
                    pending.retain(|task| !task_owned_by_path_collection(task, boundary));
                }
                GeneratorContinuation::SqlInRight { boundary, .. }
                | GeneratorContinuation::SqlInLeft { boundary, .. }
                | GeneratorContinuation::SqlIndexRow { boundary, .. }
                | GeneratorContinuation::SqlIndexKey { boundary, .. }
                | GeneratorContinuation::SqlJoinIndex { boundary, .. }
                | GeneratorContinuation::SqlJoinRow { boundary, .. }
                | GeneratorContinuation::SqlJoinKey { boundary, .. }
                | GeneratorContinuation::SqlJoinCallback { boundary } => {
                    pending.retain(|task| !task_owned_by_sql(task, boundary));
                }
                GeneratorContinuation::Label(symbol) => {
                    if matches!(error, VmError::Break { label } if label == symbol) {
                        pending.retain(|task| !task_owned_by_label(task, symbol));
                        return None;
                    }
                }
                GeneratorContinuation::ReturnUser { frames: caller, .. } => frames = caller,
                GeneratorContinuation::AccessField(_)
                | GeneratorContinuation::Iterate
                | GeneratorContinuation::Pipe { .. }
                | GeneratorContinuation::BinaryLeft { .. }
                | GeneratorContinuation::BinaryRight { .. }
                | GeneratorContinuation::Unary(_)
                | GeneratorContinuation::ArrayItem(_)
                | GeneratorContinuation::ArrayUpdateItem { .. }
                | GeneratorContinuation::ObjectItem { .. }
                | GeneratorContinuation::Select { .. }
                | GeneratorContinuation::HasArgument { .. }
                | GeneratorContinuation::SortKeyItem(_)
                | GeneratorContinuation::AlternativeItem(_)
                | GeneratorContinuation::PredicateItem { .. }
                | GeneratorContinuation::PredicateCondition { .. }
                | GeneratorContinuation::PullCount { .. }
                | GeneratorContinuation::FromStreamItem(_)
                | GeneratorContinuation::TruncateStreamItem { .. }
                | GeneratorContinuation::AddItem(_)
                | GeneratorContinuation::DebugItem
                | GeneratorContinuation::FromEntries
                | GeneratorContinuation::Bind { .. }
                | GeneratorContinuation::BindAlternatives { .. }
                | GeneratorContinuation::Conditional { .. }
                | GeneratorContinuation::AccessIndex { .. }
                | GeneratorContinuation::SliceBase { .. }
                | GeneratorContinuation::SliceStart { .. }
                | GeneratorContinuation::SliceEnd { .. }
                | GeneratorContinuation::ApplyIndex { .. }
                | GeneratorContinuation::ObjectKey { .. }
                | GeneratorContinuation::ObjectValue { .. }
                | GeneratorContinuation::FoldInitial { .. }
                | GeneratorContinuation::FoldItem { .. }
                | GeneratorContinuation::FoldUpdate { .. }
                | GeneratorContinuation::PathField(_)
                | GeneratorContinuation::PathIndex { .. }
                | GeneratorContinuation::PathIndexValue { .. }
                | GeneratorContinuation::PathIterate { .. }
                | GeneratorContinuation::PathPipe { .. }
                | GeneratorContinuation::PathJoin(_)
                | GeneratorContinuation::PathSelect(_)
                | GeneratorContinuation::PathOrigin
                | GeneratorContinuation::PathGetPath { .. }
                | GeneratorContinuation::FreshOrigin
                | GeneratorContinuation::PathFilter { .. }
                | GeneratorContinuation::PathFilterResult(_)
                | GeneratorContinuation::PathConditional { .. }
                | GeneratorContinuation::PathCatch { .. }
                | GeneratorContinuation::PathBindAlternatives { .. }
                | GeneratorContinuation::PathAliases(_)
                | GeneratorContinuation::PathBind { .. }
                | GeneratorContinuation::PathBindValue { .. }
                | GeneratorContinuation::PathSliceBase { .. }
                | GeneratorContinuation::PathSliceStart { .. }
                | GeneratorContinuation::PathSliceEnd { .. }
                | GeneratorContinuation::PathUserReturn { .. }
                | GeneratorContinuation::PathUserArgument { .. }
                | GeneratorContinuation::AssignmentPath(_)
                | GeneratorContinuation::AssignmentRhs(_)
                | GeneratorContinuation::Interpolate { .. }
                | GeneratorContinuation::HaltError { .. }
                | GeneratorContinuation::UserArgument { .. }
                | GeneratorContinuation::RecurseChild { .. }
                | GeneratorContinuation::RecurseCondition { .. }
                | GeneratorContinuation::LoopCondition { .. }
                | GeneratorContinuation::LoopUpdate { .. }
                | GeneratorContinuation::RepeatItem
                | GeneratorContinuation::LimitCount { .. }
                | GeneratorContinuation::LimitItem { .. }
                | GeneratorContinuation::BuiltinArguments { .. }
                | GeneratorContinuation::RegexArguments { .. }
                | GeneratorContinuation::RegexReplacementItem { .. }
                | GeneratorContinuation::Raise
                | GeneratorContinuation::OptionalBoundary { .. }
                | GeneratorContinuation::IgnoreError
                | GeneratorContinuation::Catch { .. } => {}
            }
        }
        Some(error)
    }
}

fn recovery_task(bytecode: &Bytecode, work: GeneratorWork, path_catch: bool) -> GeneratorTask {
    if path_catch && path_candidate_with_aliases(bytecode, work.node, &work.continuations) {
        GeneratorTask::PathEval(PathWork {
            node: work.node,
            input: work.input,
            origin: work.origin,
            prefix: Path::root(),
            environment: work.environment,
            frames: work.frames,
            continuations: work.continuations,
        })
    } else {
        GeneratorTask::Eval(work)
    }
}
