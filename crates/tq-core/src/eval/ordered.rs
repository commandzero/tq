//! Bounded ordering, grouping, and parallel collection operations.

use super::direct::one_error;
use super::managed_work::{KeyedCollectionMode, ManagedKeyedValue};
use super::origins::{OriginToken, fresh_origin};
use super::value_ops::{jq_compare, type_error};
use super::{Outcomes, resource, runtime};
use crate::{Value, VmError, VmLimits, collection};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use rayon::slice::ParallelSliceMut;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, mpsc};

// Below this size, thread-pool startup and scheduling cost more than the work.
pub(super) const PARALLEL_SORT_THRESHOLD: usize = 16 * 1024;
pub(super) const PARALLEL_REDUCTION_THRESHOLD: usize = 64 * 1024;
pub(super) fn fold_values(
    input: &Value,
    initial: Option<Value>,
    apply: fn(&Value, &Value) -> Result<Value, VmError>,
) -> Outcomes {
    let values = match input {
        Value::Array(values) => values.iter().collect::<Vec<_>>(),
        Value::Object(values) => values.values().collect::<Vec<_>>(),
        value => return one_error(type_error("add", value)),
    };
    let mut iterator = values.into_iter();
    let mut result = if let Some(initial) = initial {
        initial
    } else if let Some(first) = iterator.next() {
        first.clone()
    } else {
        return vec![Ok(Value::Null)];
    };
    for value in iterator {
        match apply(&result, value) {
            Ok(next) => result = next,
            Err(error) => return one_error(error),
        }
    }
    vec![Ok(result)]
}

pub(super) fn extrema(input: &Value, maximum: bool) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error(if maximum { "max" } else { "min" }, input));
    };
    let selected = if values.len() >= PARALLEL_REDUCTION_THRESHOLD {
        if maximum {
            values
                .par_iter()
                .enumerate()
                .max_by(|left, right| {
                    jq_compare(left.1, right.1).then_with(|| left.0.cmp(&right.0))
                })
                .map(|(_, value)| value)
        } else {
            values
                .par_iter()
                .enumerate()
                .min_by(|left, right| {
                    jq_compare(left.1, right.1).then_with(|| left.0.cmp(&right.0))
                })
                .map(|(_, value)| value)
        }
    } else if maximum {
        values.iter().max()
    } else {
        values.iter().min()
    };
    vec![Ok(selected.cloned().unwrap_or(Value::Null))]
}

pub(super) fn sort_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("sort", input));
    };
    let mut values = values.to_vec();
    stable_sort_values(&mut values);
    vec![Ok(Value::array(values))]
}

pub(super) fn unique_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("unique", input));
    };
    let mut values = values.to_vec();
    stable_sort_values(&mut values);
    values.dedup_by(|left, right| collection::jq_equal(left, right));
    vec![Ok(Value::array(values))]
}

/// Stably orders values using the shared thresholded parallel sort policy.
pub fn stable_sort_values(values: &mut [Value]) {
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort_by(jq_compare);
    } else {
        values.sort_by(jq_compare);
    }
}

/// Number of workers in the shared Rayon execution pool.
#[must_use]
pub fn parallel_worker_count() -> usize {
    rayon::current_num_threads()
}

/// High-water observations from an overlapping stable-sort preparation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StableSortPipelineObservations {
    /// Number of independently sorted input runs.
    pub batches: usize,
    /// Maximum worker batches simultaneously in flight.
    pub in_flight_batches: usize,
    /// Maximum estimated bytes simultaneously in flight.
    pub in_flight_bytes: usize,
}

/// Bounded producer for stable sorted runs backed by the shared Rayon pool.
pub struct StableSortPipeline {
    batch: Vec<Value>,
    batch_bytes: usize,
    batch_values_limit: usize,
    batch_bytes_limit: usize,
    in_flight_batches_limit: usize,
    in_flight_bytes_limit: usize,
    in_flight_batches: usize,
    in_flight_bytes: usize,
    next_ordinal: usize,
    sender: Sender<(usize, usize, Vec<Value>, bool)>,
    receiver: Receiver<(usize, usize, Vec<Value>, bool)>,
    runs: BTreeMap<usize, Vec<Value>>,
    observations: StableSortPipelineObservations,
    cancellation: Option<Arc<AtomicBool>>,
}

impl StableSortPipeline {
    /// Creates a finite run queue. Every limit must be non-zero.
    #[must_use]
    pub fn new(
        batch_values_limit: usize,
        batch_bytes_limit: usize,
        in_flight_batches_limit: usize,
        in_flight_bytes_limit: usize,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            batch: Vec::with_capacity(batch_values_limit.max(1)),
            batch_bytes: 0,
            batch_values_limit: batch_values_limit.max(1),
            batch_bytes_limit: batch_bytes_limit.max(1),
            in_flight_batches_limit: in_flight_batches_limit.max(1),
            in_flight_bytes_limit: in_flight_bytes_limit.max(1),
            in_flight_batches: 0,
            in_flight_bytes: 0,
            next_ordinal: 0,
            sender,
            receiver,
            runs: BTreeMap::new(),
            observations: StableSortPipelineObservations::default(),
            cancellation: None,
        }
    }

    /// Adds cooperative cancellation shared with decoding and evaluation.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: Arc<AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Adds one producer result, blocking only when the finite worker queue is full.
    ///
    /// # Errors
    ///
    /// Returns a stable resource name when one value exceeds the byte envelope or
    /// a worker disconnects.
    pub fn push(&mut self, value: Value, estimated_bytes: usize) -> Result<(), &'static str> {
        self.check_cancellation()?;
        if estimated_bytes > self.in_flight_bytes_limit {
            return Err("hybrid-in-flight-bytes");
        }
        if !self.batch.is_empty()
            && (self.batch.len() >= self.batch_values_limit
                || self.batch_bytes.saturating_add(estimated_bytes) > self.batch_bytes_limit)
        {
            self.dispatch()?;
        }
        self.batch.push(value);
        self.batch_bytes = self.batch_bytes.saturating_add(estimated_bytes);
        if self.batch.len() >= self.batch_values_limit || self.batch_bytes >= self.batch_bytes_limit
        {
            self.dispatch()?;
        }
        Ok(())
    }

    /// Drains every run and performs deterministic pairwise stable merges.
    ///
    /// # Errors
    ///
    /// Returns a stable resource name if a worker disconnects.
    pub fn finish(mut self) -> Result<(Vec<Value>, StableSortPipelineObservations), &'static str> {
        self.check_cancellation()?;
        self.dispatch()?;
        while self.in_flight_batches != 0 {
            self.receive_one()?;
        }
        let runs = self.runs.into_values().collect::<Vec<_>>();
        Ok((
            merge_sorted_runs(runs, self.cancellation.as_ref())?,
            self.observations,
        ))
    }

    fn dispatch(&mut self) -> Result<(), &'static str> {
        self.check_cancellation()?;
        if self.batch.is_empty() {
            return Ok(());
        }
        let bytes = self.batch_bytes;
        while self.in_flight_batches >= self.in_flight_batches_limit
            || self.in_flight_bytes.saturating_add(bytes) > self.in_flight_bytes_limit
        {
            self.receive_one()?;
        }
        let ordinal = self.next_ordinal;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        let mut values =
            std::mem::replace(&mut self.batch, Vec::with_capacity(self.batch_values_limit));
        self.batch_bytes = 0;
        let sender = self.sender.clone();
        let cancellation = self.cancellation.clone();
        rayon::spawn_fifo(move || {
            if !cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                stable_sort_values(&mut values);
            }
            let cancelled = cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed));
            let _ = sender.send((ordinal, bytes, values, cancelled));
        });
        self.in_flight_batches = self.in_flight_batches.saturating_add(1);
        self.in_flight_bytes = self.in_flight_bytes.saturating_add(bytes);
        self.observations.batches = self.observations.batches.saturating_add(1);
        self.observations.in_flight_batches = self
            .observations
            .in_flight_batches
            .max(self.in_flight_batches);
        self.observations.in_flight_bytes =
            self.observations.in_flight_bytes.max(self.in_flight_bytes);
        Ok(())
    }

    fn receive_one(&mut self) -> Result<(), &'static str> {
        self.check_cancellation()?;
        let (ordinal, bytes, values, worker_cancelled) =
            self.receiver.recv().map_err(|_| "hybrid-sort-worker")?;
        if worker_cancelled {
            return Err("interrupted");
        }
        self.check_cancellation()?;
        self.in_flight_batches = self.in_flight_batches.saturating_sub(1);
        self.in_flight_bytes = self.in_flight_bytes.saturating_sub(bytes);
        self.runs.insert(ordinal, values);
        Ok(())
    }

    fn check_cancellation(&self) -> Result<(), &'static str> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err("interrupted")
        } else {
            Ok(())
        }
    }
}

fn merge_sorted_runs(
    mut runs: Vec<Vec<Value>>,
    cancellation: Option<&Arc<AtomicBool>>,
) -> Result<Vec<Value>, &'static str> {
    while runs.len() > 1 {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("interrupted");
        }
        let mut pairs = Vec::with_capacity(runs.len().div_ceil(2));
        let mut iterator = runs.into_iter();
        while let Some(left) = iterator.next() {
            pairs.push((left, iterator.next()));
        }
        runs = if let Some(cancellation) = cancellation {
            pairs
                .into_par_iter()
                .map(|(left, right)| match right {
                    Some(right) => stable_merge_cancellable(left, right, cancellation),
                    None => Ok(left),
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            pairs
                .into_par_iter()
                .map(|(left, right)| match right {
                    Some(right) => stable_merge(left, right),
                    None => left,
                })
                .collect()
        };
    }
    Ok(runs.pop().unwrap_or_default())
}

fn stable_merge_cancellable(
    left: Vec<Value>,
    right: Vec<Value>,
    cancellation: &AtomicBool,
) -> Result<Vec<Value>, &'static str> {
    let mut merged = Vec::with_capacity(left.len().saturating_add(right.len()));
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    let mut comparisons_until_check = 0usize;
    while let (Some(left_value), Some(right_value)) = (left.peek(), right.peek()) {
        if comparisons_until_check == 0 {
            if cancellation.load(Ordering::Relaxed) {
                return Err("interrupted");
            }
            comparisons_until_check = 16 * 1024;
        }
        comparisons_until_check = comparisons_until_check.saturating_sub(1);
        if jq_compare(left_value, right_value).is_gt() {
            merged.push(right.next().expect("right value was peeked"));
        } else {
            merged.push(left.next().expect("left value was peeked"));
        }
    }
    merged.extend(left);
    merged.extend(right);
    Ok(merged)
}

fn stable_merge(left: Vec<Value>, right: Vec<Value>) -> Vec<Value> {
    let mut merged = Vec::with_capacity(left.len().saturating_add(right.len()));
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    while let (Some(left_value), Some(right_value)) = (left.peek(), right.peek()) {
        if jq_compare(left_value, right_value).is_gt() {
            merged.push(right.next().expect("right value was peeked"));
        } else {
            merged.push(left.next().expect("left value was peeked"));
        }
    }
    merged.extend(left);
    merged.extend(right);
    merged
}

pub(super) fn sort_by_cached_key(values: &mut [(Value, Value)]) {
    // `sort_by` and Rayon `par_sort_by` are stable, so equal keys retain the
    // order in which jq produced them.
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort_by(|left, right| jq_compare(&left.0, &right.0));
    } else {
        values.sort_by(|left, right| jq_compare(&left.0, &right.0));
    }
}

fn sort_managed_keyed_values(values: &mut [ManagedKeyedValue]) {
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort_by(|left, right| jq_compare(&left.key, &right.key));
    } else {
        values.sort_by(|left, right| jq_compare(&left.key, &right.key));
    }
}

pub(super) fn ensure_managed_container_size(
    length: usize,
    limits: VmLimits,
) -> Result<(), VmError> {
    let lower_bound = length
        .checked_add(2)
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

fn ensure_managed_group_projection_size(
    groups: usize,
    members: usize,
    limits: VmLimits,
) -> Result<(), VmError> {
    let group_delimiters = groups
        .checked_mul(2)
        .ok_or_else(|| resource("output-bytes"))?;
    let lower_bound = 2_usize
        .checked_add(group_delimiters)
        .and_then(|size| size.checked_add(members))
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

pub(super) fn project_managed_keyed_values(
    mode: KeyedCollectionMode,
    mut keyed_values: Vec<ManagedKeyedValue>,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
    next_origin: &mut u64,
) -> Result<(Option<OriginToken>, Value), VmError> {
    if matches!(
        mode,
        KeyedCollectionMode::Sort | KeyedCollectionMode::Unique | KeyedCollectionMode::Group
    ) {
        sort_managed_keyed_values(&mut keyed_values);
    }
    match mode {
        KeyedCollectionMode::Sort => {
            ensure_managed_container_size(keyed_values.len(), limits)?;
            let mut output = Vec::new();
            output
                .try_reserve_exact(keyed_values.len())
                .map_err(|_| resource("output-bytes"))?;
            for entry in keyed_values {
                charge()?;
                output.push(entry.value);
            }
            Ok((Some(fresh_origin(next_origin)?), Value::array(output)))
        }
        KeyedCollectionMode::Unique => {
            keyed_values.dedup_by(|left, right| collection::jq_equal(&left.key, &right.key));
            ensure_managed_container_size(keyed_values.len(), limits)?;
            let mut output = Vec::new();
            output
                .try_reserve_exact(keyed_values.len())
                .map_err(|_| resource("output-bytes"))?;
            for entry in keyed_values {
                charge()?;
                output.push(entry.value);
            }
            Ok((Some(fresh_origin(next_origin)?), Value::array(output)))
        }
        KeyedCollectionMode::Group => {
            let mut group_count = 0_usize;
            let mut previous: Option<&Value> = None;
            for entry in &keyed_values {
                if previous.is_none_or(|key| !collection::jq_equal(key, &entry.key)) {
                    group_count = group_count
                        .checked_add(1)
                        .ok_or_else(|| resource("output-bytes"))?;
                    previous = Some(&entry.key);
                }
            }
            ensure_managed_group_projection_size(group_count, keyed_values.len(), limits)?;
            let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
            groups
                .try_reserve_exact(group_count)
                .map_err(|_| resource("output-bytes"))?;
            for entry in keyed_values {
                charge()?;
                let starts_new_group = groups
                    .last()
                    .is_none_or(|(key, _)| !collection::jq_equal(key, &entry.key));
                if starts_new_group {
                    groups.push((entry.key, Vec::new()));
                }
                let (_, values) = groups.last_mut().expect("group was created");
                let value_count = values
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| resource("output-bytes"))?;
                ensure_managed_container_size(value_count, limits)?;
                values
                    .try_reserve(1)
                    .map_err(|_| resource("output-bytes"))?;
                values.push(entry.value);
            }
            ensure_managed_container_size(groups.len(), limits)?;
            let mut output = Vec::new();
            output
                .try_reserve_exact(groups.len())
                .map_err(|_| resource("output-bytes"))?;
            for (_, values) in groups {
                charge()?;
                output.push(Value::array(values));
            }
            Ok((Some(fresh_origin(next_origin)?), Value::array(output)))
        }
        KeyedCollectionMode::Min | KeyedCollectionMode::Max => {
            let maximum = matches!(mode, KeyedCollectionMode::Max);
            let selected = keyed_values.into_iter().try_fold(None, |selected, entry| {
                charge()?;
                let replace = selected.as_ref().is_none_or(|current: &ManagedKeyedValue| {
                    let ordering = jq_compare(&entry.key, &current.key);
                    if maximum {
                        ordering.is_gt() || ordering.is_eq()
                    } else {
                        ordering.is_lt()
                    }
                });
                Ok::<_, VmError>(if replace { Some(entry) } else { selected })
            })?;
            match selected {
                Some(entry) => Ok((entry.origin, entry.value)),
                None => Ok((Some(fresh_origin(next_origin)?), Value::Null)),
            }
        }
    }
}

pub(super) fn reverse(input: &Value) -> Outcomes {
    match input {
        Value::Array(values) => vec![Ok(Value::array(
            values.iter().rev().cloned().collect::<Vec<_>>(),
        ))],
        Value::String(value) => vec![Ok(Value::string(value.chars().rev().collect::<String>()))],
        value => one_error(type_error("reverse", value)),
    }
}

pub(super) fn flatten(input: &Value, depth: Option<i64>) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("flatten", input));
    };
    if depth.is_some_and(|depth| depth < 0) {
        return Err(runtime("flatten depth cannot be negative".to_owned()));
    }
    let mut output = Vec::new();
    let mut stack = values
        .iter()
        .rev()
        .cloned()
        .map(|value| (value, depth))
        .collect::<Vec<_>>();
    while let Some((value, depth)) = stack.pop() {
        match value {
            Value::Array(values) if depth != Some(0) => {
                let next_depth = depth.map(|depth| depth - 1);
                stack.extend(
                    values
                        .iter()
                        .rev()
                        .cloned()
                        .map(|value| (value, next_depth)),
                );
            }
            value => output.push(value),
        }
    }
    Ok(Value::array(output))
}
