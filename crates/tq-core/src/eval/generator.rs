//! Pull-driven, bounded generators used by the managed evaluator.
//!
//! The evaluator owns argument/filter evaluation.  This module only turns
//! already-materialized values into an iterator state, so a caller can stop
//! after one result without materializing an unbounded generator.

use std::sync::Arc;

use crate::{Number, Object, Path, PathComponent, Value, VmError, VmLimits};

use super::{
    invalid, path::replace_or_create_bounded, path_value, resource, scalar::bounded_path,
    type_error,
};

/// A generator whose next result can be pulled independently.
pub(super) enum GeneratorState {
    Range(RangeState),
    Combinations(CombinationsState),
    ToStream(ToStreamState),
}

/// Starts a supported generator from already-evaluated arguments.
///
/// `argument_arity` is the source-call arity, which is distinct from the
/// number of values produced by one generator argument.  In particular,
/// `combinations/1` can receive zero or many evaluated count values.
/// `Ok(None)` means that `name` is not a generator handled by this module.
pub(super) fn start(
    name: &str,
    input: &Value,
    arguments: &[Value],
    argument_arity: usize,
    limits: VmLimits,
) -> Result<Option<GeneratorState>, VmError> {
    match name {
        "range" => start_range(arguments, limits).map(|state| Some(GeneratorState::Range(state))),
        "combinations" => start_combinations(input, arguments, argument_arity, limits)
            .map(|state| Some(GeneratorState::Combinations(state))),
        "tostream" if argument_arity == 0 && arguments.is_empty() => Ok(Some(
            GeneratorState::ToStream(ToStreamState::new(input.clone(), limits)?),
        )),
        "tostream" => Err(invalid("tostream does not accept arguments")),
        _ => Ok(None),
    }
}

impl GeneratorState {
    /// Pulls one result, or `None` when the generator is exhausted.
    pub(super) fn next(
        &mut self,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<Value>, VmError> {
        match self {
            Self::Range(state) => state.next(charge),
            Self::Combinations(state) => state.next(charge),
            Self::ToStream(state) => state.next(charge),
        }
    }
}

pub(super) struct RangeState {
    current: f64,
    end: f64,
    step: f64,
    literal_current: Option<Number>,
    finished: bool,
}

fn start_range(arguments: &[Value], limits: VmLimits) -> Result<RangeState, VmError> {
    if arguments.len() > limits.fork_stack {
        return Err(resource("fork-stack"));
    }
    let mut numbers = Vec::new();
    if numbers.try_reserve(arguments.len()).is_err() {
        return Err(resource("fork-stack"));
    }
    for argument in arguments {
        let Value::Number(number) = argument else {
            return Err(type_error("range", argument));
        };
        numbers.push(number.clone());
    }
    let (current, end, step, literal_current) = match numbers.as_slice() {
        [end] => (0.0, end.as_f64(), 1.0, None),
        [start, end] => (start.as_f64(), end.as_f64(), 1.0, Some(start.clone())),
        [start, end, step] => (
            start.as_f64(),
            end.as_f64(),
            step.as_f64(),
            Some(start.clone()),
        ),
        _ => return Err(invalid("range arity")),
    };
    Ok(RangeState {
        current,
        end,
        step,
        literal_current,
        // A zero increment is empty.  A NaN increment emits the initial value
        // only when jq's descending comparison accepts it, then becomes
        // exhausted after the first update.  Ordinary no-progress steps
        // remain charge-bounded by `next`.
        finished: step == 0.0,
    })
}

impl RangeState {
    fn next(
        &mut self,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<Value>, VmError> {
        charge()?;
        if self.finished {
            return Ok(None);
        }

        let in_range = if self.step.is_nan() {
            !self.current.is_nan() && (self.end.is_nan() || self.current > self.end)
        } else if self.step > 0.0 {
            // `range(0; nan)` is an unbounded positive generator in jq.  It
            // remains safe here because results are pulled one at a time.
            self.current.is_nan() || self.end.is_nan() || self.current < self.end
        } else if self.step < 0.0 {
            !self.current.is_nan() && (self.end.is_nan() || self.current > self.end)
        } else {
            false
        };
        if !in_range {
            self.finished = true;
            return Ok(None);
        }

        let value = Value::Number(
            self.literal_current
                .take()
                .unwrap_or_else(|| Number::from_runtime_f64(self.current)),
        );
        if self.step.is_nan() {
            self.finished = true;
            return Ok(Some(value));
        }
        let next = self.current + self.step;
        if next.to_bits() != self.current.to_bits() {
            self.current = next;
        }
        Ok(Some(value))
    }
}

pub(super) struct CombinationsState {
    dimensions: Vec<Arc<[Value]>>,
    indices: Vec<usize>,
    current: Vec<Value>,
    started: bool,
    done: bool,
    limits: VmLimits,
}

fn start_combinations(
    input: &Value,
    arguments: &[Value],
    argument_arity: usize,
    limits: VmLimits,
) -> Result<CombinationsState, VmError> {
    if argument_arity > 1 {
        return Err(invalid("combinations arity"));
    }
    if argument_arity == 0 && !arguments.is_empty() {
        return Err(invalid("combinations argument result mismatch"));
    }
    let mut dimensions = Vec::new();
    if argument_arity == 1 {
        let mut base = None;
        for argument in arguments {
            let Value::Number(number) = argument else {
                return Err(type_error("combinations", argument));
            };
            let count = number.as_f64().ceil();
            if !count.is_finite() {
                return Err(resource("fork-stack"));
            }
            if count <= 0.0 {
                // combinations(0) has the one zero-width product.
                continue;
            }
            let count = count
                .to_string()
                .parse::<usize>()
                .map_err(|_| resource("fork-stack"))?;
            let total = dimensions
                .len()
                .checked_add(count)
                .ok_or_else(|| resource("fork-stack"))?;
            if total > limits.fork_stack {
                return Err(resource("fork-stack"));
            }
            if dimensions.try_reserve(count).is_err() {
                return Err(resource("fork-stack"));
            }
            let values = if let Some(values) = base.as_ref() {
                Arc::clone(values)
            } else {
                let values = iterable_values(input, limits)?;
                base = Some(Arc::clone(&values));
                values
            };
            for _ in 0..count {
                dimensions.push(Arc::clone(&values));
            }
        }
    } else {
        let Value::Array(values) = input else {
            return Err(type_error("combinations", input));
        };
        if values.len() > limits.fork_stack {
            return Err(resource("fork-stack"));
        }
        if dimensions.try_reserve(values.len()).is_err() {
            return Err(resource("fork-stack"));
        }
        for value in values.iter() {
            let dimension = iterable_values(value, limits)?;
            if dimension.is_empty() {
                dimensions.push(dimension);
                break;
            }
            dimensions.push(dimension);
        }
    }

    let mut indices = Vec::new();
    let mut current = Vec::new();
    if indices.try_reserve(dimensions.len()).is_err()
        || current.try_reserve(dimensions.len()).is_err()
    {
        return Err(resource("fork-stack"));
    }
    Ok(CombinationsState {
        dimensions,
        indices,
        current,
        started: false,
        done: false,
        limits,
    })
}

fn iterable_values(value: &Value, limits: VmLimits) -> Result<Arc<[Value]>, VmError> {
    match value {
        Value::Array(values) => Ok(Arc::clone(values)),
        Value::Object(values) => {
            if values.len() > limits.fork_stack {
                return Err(resource("fork-stack"));
            }
            let mut output = Vec::new();
            if output.try_reserve(values.len()).is_err() {
                return Err(resource("fork-stack"));
            }
            output.extend(values.values().cloned());
            Ok(output.into())
        }
        _ => Err(type_error("combinations", value)),
    }
}

impl CombinationsState {
    fn next(
        &mut self,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<Value>, VmError> {
        charge()?;
        if self.done {
            return Ok(None);
        }
        if self.dimensions.is_empty() {
            self.done = true;
            return Ok(Some(Value::array(Vec::new())));
        }
        if self.dimensions.iter().any(|dimension| dimension.is_empty()) {
            self.done = true;
            return Ok(None);
        }

        if self.started {
            let mut position = self.dimensions.len();
            loop {
                if position == 0 {
                    self.done = true;
                    return Ok(None);
                }
                position -= 1;
                self.indices[position] = self.indices[position].saturating_add(1);
                if self.indices[position] < self.dimensions[position].len() {
                    self.current[position] =
                        self.dimensions[position][self.indices[position]].clone();
                    break;
                }
                self.indices[position] = 0;
                self.current[position] = self.dimensions[position][0].clone();
            }
        } else {
            self.indices.resize(self.dimensions.len(), 0);
            self.current
                .extend(self.dimensions.iter().map(|dimension| dimension[0].clone()));
            self.started = true;
        }
        if self.current.len() > self.limits.fork_stack {
            self.done = true;
            return Err(resource("fork-stack"));
        }
        Ok(Some(Value::array(self.current.clone())))
    }
}

enum StreamFrame {
    Visit {
        value: Value,
        path: Vec<PathComponent>,
    },
    Array {
        values: Arc<[Value]>,
        path: Vec<PathComponent>,
        next: usize,
        last_child: Option<Vec<PathComponent>>,
    },
    Object {
        values: Arc<Object>,
        path: Vec<PathComponent>,
        next: usize,
        last_child: Option<Vec<PathComponent>>,
    },
    Close {
        path: Vec<PathComponent>,
    },
}

pub(super) struct ToStreamState {
    frames: Vec<StreamFrame>,
    limits: VmLimits,
}

impl ToStreamState {
    fn new(input: Value, limits: VmLimits) -> Result<Self, VmError> {
        let mut frames = Vec::new();
        if frames.try_reserve(1).is_err() {
            return Err(resource("fork-stack"));
        }
        frames.push(StreamFrame::Visit {
            value: input,
            path: Vec::new(),
        });
        Ok(Self { frames, limits })
    }

    fn next(
        &mut self,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<Value>, VmError> {
        loop {
            charge()?;
            let Some(frame) = self.frames.pop() else {
                return Ok(None);
            };
            match frame {
                StreamFrame::Visit { value, path } => {
                    if path.len() > self.limits.path_stack {
                        return Err(resource("path-stack"));
                    }
                    match value {
                        Value::Array(values) if !values.is_empty() => {
                            self.push_frame(StreamFrame::Array {
                                values,
                                path,
                                next: 0,
                                last_child: None,
                            })?;
                        }
                        Value::Object(values) if !values.is_empty() => {
                            self.push_frame(StreamFrame::Object {
                                values,
                                path,
                                next: 0,
                                last_child: None,
                            })?;
                        }
                        value => {
                            let leaf =
                                Value::array(vec![path_value(&Path::new(path.clone())), value]);
                            return Ok(Some(leaf));
                        }
                    }
                }
                StreamFrame::Array {
                    values,
                    path,
                    next,
                    last_child,
                } => {
                    if next < values.len() {
                        let child_path =
                            extend_path(&path, PathComponent::Index(next), &self.limits)?;
                        self.push_frame(StreamFrame::Array {
                            values: Arc::clone(&values),
                            path: path.clone(),
                            next: next.saturating_add(1),
                            last_child: Some(child_path.clone()),
                        })?;
                        self.push_frame(StreamFrame::Visit {
                            value: values[next].clone(),
                            path: child_path,
                        })?;
                    } else {
                        self.push_close(last_child.unwrap_or(path))?;
                    }
                }
                StreamFrame::Object {
                    values,
                    path,
                    next,
                    last_child,
                } => {
                    if let Some((key, value)) = values.get_index(next) {
                        let child_path =
                            extend_path(&path, PathComponent::Key(Arc::clone(key)), &self.limits)?;
                        self.push_frame(StreamFrame::Object {
                            values: Arc::clone(&values),
                            path: path.clone(),
                            next: next.saturating_add(1),
                            last_child: Some(child_path.clone()),
                        })?;
                        self.push_frame(StreamFrame::Visit {
                            value: value.clone(),
                            path: child_path,
                        })?;
                    } else {
                        self.push_close(last_child.unwrap_or(path))?;
                    }
                }
                StreamFrame::Close { path } => {
                    return Ok(Some(Value::array(vec![path_value(&Path::new(path))])));
                }
            }
        }
    }

    fn push_frame(&mut self, frame: StreamFrame) -> Result<(), VmError> {
        if self.frames.len() >= self.limits.fork_stack {
            return Err(resource("fork-stack"));
        }
        if self.frames.try_reserve(1).is_err() {
            return Err(resource("fork-stack"));
        }
        self.frames.push(frame);
        Ok(())
    }

    fn push_close(&mut self, path: Vec<PathComponent>) -> Result<(), VmError> {
        self.push_frame(StreamFrame::Close { path })
    }
}

/// Pull-fed state for rebuilding values from `tostream` events.
///
/// The managed dispatcher owns the stream-producing expression and calls
/// [`Self::feed`] once for each event.  Keeping only the partially rebuilt
/// root means a caller can stop after the first completed value without
/// collecting the remainder of the stream.
pub(super) struct FromStreamState {
    root: Option<Value>,
}

impl FromStreamState {
    pub(super) const fn new() -> Self {
        Self { root: None }
    }

    /// Consumes one encoded stream event.
    ///
    /// A root leaf is returned immediately.  A container is returned when
    /// its one-component close event arrives.  Incomplete state is retained
    /// between calls, and an end-of-input without a close event is not
    /// flushed implicitly (matching jq's `fromstream` behavior).
    pub(super) fn feed(
        &mut self,
        event: &Value,
        limits: VmLimits,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<Value>, VmError> {
        charge()?;
        let (path, value) = bounded_stream_event(event, limits, charge)?;

        match value {
            Some(value) if path.is_empty() => Ok(Some(value)),
            Some(value) => {
                let base = self.root.take().unwrap_or(Value::Null);
                self.root = Some(replace_or_create_bounded(
                    &base, &path, value, limits, charge,
                )?);
                Ok(None)
            }
            None if path.len() == 1 => Ok(self.root.take()),
            None => Ok(None),
        }
    }
}

/// Decodes a `tostream` event without first cloning an unchecked path.
///
/// The public stream-event decoder is also used by legacy eager operators and
/// intentionally has no VM limit parameter.  Pull-fed `fromstream` must
/// validate the event shape, then run the bounded path admission check before
/// reserving or cloning path components.
fn bounded_stream_event(
    event: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(Vec<PathComponent>, Option<Value>), VmError> {
    let Value::Array(parts) = event else {
        return Err(super::runtime("stream event must be an array".to_owned()));
    };
    if !(1..=2).contains(&parts.len()) {
        return Err(super::runtime(
            "stream event must contain a path and optional value".to_owned(),
        ));
    }
    let path = bounded_path(&parts[0], limits, charge)?;
    Ok((path, parts.get(1).cloned()))
}

/// Pulls one event through `truncate_stream`'s path projection.
///
/// `None` means the event is at or above the removed prefix and therefore
/// produces no truncated event.  Every retained event produces exactly one
/// value, including close events, so callers can preserve stream cardinality
/// without first collecting the source.
pub(super) fn truncate_stream_event(
    event: &Value,
    count_value: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Option<Value>, VmError> {
    charge()?;
    let zero = Value::Number(Number::parse("0").expect("zero is a valid index"));
    let path_value = super::access_index(event, &zero)?;
    let (path_length, path_len, path_kind) = truncate_path_shape(&path_value, charge)?;
    let retain = match count_value {
        Value::Number(number) if number.as_f64().is_nan() => true,
        count => path_length > *count,
    };
    if !retain {
        return Ok(None);
    }
    let start = match path_kind {
        TruncatePathKind::Null | TruncatePathKind::Number | TruncatePathKind::Object => 0,
        TruncatePathKind::Array | TruncatePathKind::String => {
            let Some(start) = truncate_start(count_value, path_len)? else {
                return Ok(None);
            };
            start
        }
    };
    let path = truncate_path_value(&path_value, path_kind, start, limits, charge)?;
    let parts = match event {
        Value::Array(parts) => Some(parts),
        Value::Null => None,
        _ => return Ok(None),
    };
    let record_len = parts.map_or(1, |parts| parts.len()).max(1);
    if record_len > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    charge()?;
    let mut record = Vec::new();
    record
        .try_reserve_exact(record_len)
        .map_err(|_| resource("output-bytes"))?;
    record.push(path);
    if let Some(parts) = parts {
        for part in parts.iter().skip(1) {
            charge()?;
            record.push(part.clone());
        }
    }
    Ok(Some(Value::array(record)))
}

#[derive(Clone, Copy)]
enum TruncatePathKind {
    Null,
    Number,
    Object,
    Array,
    String,
}

fn truncate_path_shape(
    value: &Value,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(Value, usize, TruncatePathKind), VmError> {
    match value {
        Value::Null => Ok((super::number_usize(0)?, 0, TruncatePathKind::Null)),
        Value::Number(number) => {
            let length = number.as_f64().abs();
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "finite nonnegative lengths use Rust's saturating float-to-usize conversion"
            )]
            let path_len = if length.is_finite() {
                length as usize
            } else {
                usize::MAX
            };
            Ok((super::length(value)?, path_len, TruncatePathKind::Number))
        }
        Value::Object(values) => Ok((
            super::number_usize(values.len())?,
            values.len(),
            TruncatePathKind::Object,
        )),
        Value::Bool(_) => Err(super::type_error("length", value)),
        Value::Array(values) => Ok((
            super::number_usize(values.len())?,
            values.len(),
            TruncatePathKind::Array,
        )),
        Value::String(value) => {
            let mut length = 0usize;
            for _character in value.chars() {
                charge()?;
                length = length
                    .checked_add(1)
                    .ok_or_else(|| super::resource("output-bytes"))?;
            }
            Ok((
                super::number_usize(length)?,
                length,
                TruncatePathKind::String,
            ))
        }
    }
}

fn truncate_path_value(
    value: &Value,
    kind: TruncatePathKind,
    start: usize,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match (kind, value) {
        (TruncatePathKind::Null, Value::Null) => Ok(Value::Null),
        (TruncatePathKind::Number, value @ Value::Number(_))
        | (TruncatePathKind::Object, value @ Value::Object(_)) => {
            Err(super::type_error("slice", value))
        }
        (TruncatePathKind::Array, Value::Array(values)) => {
            let retained = values.len().saturating_sub(start);
            if retained > limits.path_stack {
                return Err(resource("path-stack"));
            }
            let mut path = Vec::new();
            path.try_reserve_exact(retained)
                .map_err(|_| resource("path-stack"))?;
            for (index, value) in values.iter().enumerate() {
                charge()?;
                if index >= start {
                    path.push(value.clone());
                }
            }
            Ok(Value::array(path))
        }
        (TruncatePathKind::String, Value::String(value)) => {
            let mut retained = String::new();
            for (index, character) in value.chars().enumerate() {
                charge()?;
                if index < start {
                    continue;
                }
                let next_len = retained
                    .len()
                    .checked_add(character.len_utf8())
                    .ok_or_else(|| resource("output-bytes"))?;
                if next_len > limits.output_bytes {
                    return Err(resource("output-bytes"));
                }
                retained
                    .try_reserve(character.len_utf8())
                    .map_err(|_| resource("output-bytes"))?;
                retained.push(character);
            }
            Ok(Value::string(retained))
        }
        _ => Err(super::invalid("truncate stream path shape changed")),
    }
}

fn truncate_start(value: &Value, path_len: usize) -> Result<Option<usize>, VmError> {
    match value {
        Value::Null => Ok(Some(0)),
        Value::Bool(_) => Err(super::runtime(
            "Array/string slice indices must be integers".to_owned(),
        )),
        Value::Number(number) => {
            let count = number.as_f64();
            #[allow(
                clippy::cast_precision_loss,
                reason = "path lengths are bounded runtime indices compared with jq's floating count"
            )]
            let path_len_as_float = path_len as f64;
            if count.is_nan() || path_len_as_float > count {
                Ok(Some(super::slice_bounds(path_len, Some(count), None).0))
            } else {
                Ok(None)
            }
        }
        Value::String(_) | Value::Array(_) | Value::Object(_) => Ok(None),
    }
}

fn extend_path(
    prefix: &[PathComponent],
    component: PathComponent,
    limits: &VmLimits,
) -> Result<Vec<PathComponent>, VmError> {
    if prefix.len() >= limits.path_stack {
        return Err(resource("path-stack"));
    }
    let mut path = Vec::new();
    if path.try_reserve(prefix.len().saturating_add(1)).is_err() {
        return Err(resource("path-stack"));
    }
    path.extend_from_slice(prefix);
    path.push(component);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: f64) -> Value {
        Value::Number(Number::from_runtime_f64(value))
    }

    fn pull_all(mut state: GeneratorState) -> Result<Vec<Value>, VmError> {
        let mut output = Vec::new();
        let mut charge = || Ok(());
        while let Some(value) = state.next(&mut charge)? {
            output.push(value);
            if output.len() > 32 {
                return Err(VmError::Resource {
                    resource: "vm-steps",
                });
            }
        }
        Ok(output)
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the range state-machine test keeps its pull, progress, and quota cases together"
    )]
    fn range_is_pull_driven_and_handles_no_progress() {
        let state = start(
            "range",
            &Value::Null,
            &[number(0.0), number(3.0), number(0.0)],
            3,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap(), [] as [Value; 0]);

        let state = start(
            "range",
            &Value::Null,
            &[number(0.0), number(3.0), number(f64::NAN)],
            3,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap(), [] as [Value; 0]);

        let state = start(
            "range",
            &Value::Null,
            &[number(0.0), number(f64::NAN)],
            2,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut state = state;
        let mut charge = || Ok(());
        let first = state.next(&mut charge).unwrap().unwrap();
        let second = state.next(&mut charge).unwrap().unwrap();
        assert_eq!(first.to_string(), "0");
        assert_eq!(second.to_string(), "1");

        let state = start(
            "range",
            &Value::Null,
            &[number(f64::NAN), number(3.0)],
            2,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut state = state;
        let mut charge = || Ok(());
        assert!(matches!(
            state.next(&mut charge).unwrap(),
            Some(Value::Number(number)) if number.as_f64().is_nan()
        ));
        assert!(matches!(
            state.next(&mut charge).unwrap(),
            Some(Value::Number(number)) if number.as_f64().is_nan()
        ));

        let state = start(
            "range",
            &Value::Null,
            &[number(3.0), number(f64::NAN), number(-1.0)],
            3,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut state = state;
        let mut charge = || Ok(());
        assert_eq!(state.next(&mut charge).unwrap().unwrap().to_string(), "3");
        assert_eq!(state.next(&mut charge).unwrap().unwrap().to_string(), "2");

        let state = start(
            "range",
            &Value::Null,
            &[number(3.0), number(f64::NAN), number(f64::NAN)],
            3,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut state = state;
        let mut charge = || Ok(());
        assert_eq!(state.next(&mut charge).unwrap().unwrap().to_string(), "3");
        assert!(state.next(&mut charge).unwrap().is_none());

        let state = start(
            "range",
            &Value::Null,
            &[number(f64::NAN), number(3.0), number(-1.0)],
            3,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut state = state;
        let mut charge = || Ok(());
        assert!(state.next(&mut charge).unwrap().is_none());

        let state = start(
            "range",
            &Value::Null,
            &[
                Value::Number(Number::parse("1.00").expect("literal start")),
                number(3.0),
            ],
            2,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut state = state;
        let mut charge = || Ok(());
        assert_eq!(
            state.next(&mut charge).unwrap().unwrap().to_string(),
            "1.00"
        );
        assert_eq!(state.next(&mut charge).unwrap().unwrap().to_string(), "2");

        let state = start(
            "range",
            &Value::Null,
            &[number(3.0), number(0.0), number(-1.0)],
            3,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap().len(), 3);

        let current: f64 = 1.0e20;
        let end = f64::from_bits(current.to_bits().saturating_add(1));
        let state = start(
            "range",
            &Value::Null,
            &[number(current), number(end), number(1.0)],
            3,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut state = state;
        let mut remaining = 2usize;
        let mut charge = || {
            if remaining == 0 {
                Err(VmError::Resource {
                    resource: "vm-steps",
                })
            } else {
                remaining -= 1;
                Ok(())
            }
        };
        assert!(state.next(&mut charge).unwrap().is_some());
        assert!(state.next(&mut charge).unwrap().is_some());
        assert_eq!(
            state.next(&mut charge),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
    }

    #[test]
    fn combinations_covers_zero_width_and_empty_dimensions() {
        let input = Value::array(vec![
            Value::array(vec![number(1.0), number(2.0)]),
            Value::array(vec![Value::string("a"), Value::string("b")]),
        ]);
        let state = start("combinations", &input, &[], 0, VmLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(pull_all(state).unwrap().len(), 4);

        let object = Value::object(indexmap::IndexMap::from([
            (Arc::from("a"), number(1.0)),
            (Arc::from("b"), number(2.0)),
        ]));
        let state = start(
            "combinations",
            &object,
            &[number(2.0)],
            1,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap().len(), 4);

        let state = start(
            "combinations",
            &Value::array(vec![number(1.0), number(2.0)]),
            &[number(1.0), number(2.0)],
            1,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap().len(), 8);

        let state = start(
            "combinations",
            &Value::array(vec![number(1.0)]),
            &[number(0.0)],
            1,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap(), vec![Value::array(Vec::new())]);

        let state = start(
            "combinations",
            &Value::Null,
            &[number(-1.0)],
            1,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap(), vec![Value::array(Vec::new())]);

        let state = start(
            "combinations",
            &Value::array(vec![Value::array(Vec::new())]),
            &[],
            0,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap(), [] as [Value; 0]);

        let state = start(
            "combinations",
            &Value::array(vec![Value::array(Vec::new()), Value::Null]),
            &[],
            0,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap(), [] as [Value; 0]);

        let state = start("combinations", &Value::Null, &[], 1, VmLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(pull_all(state).unwrap(), vec![Value::array(Vec::new())]);
        assert!(start("combinations", &Value::Null, &[], 0, VmLimits::default(),).is_err());
    }

    #[test]
    fn tostream_emits_leaves_and_container_closes_without_recursion() {
        let input = Value::array(vec![Value::object(indexmap::IndexMap::from([(
            Arc::from("a"),
            Value::array(Vec::new()),
        )]))]);
        let state = start("tostream", &input, &[], 0, VmLimits::default())
            .unwrap()
            .unwrap();
        let output = pull_all(state).unwrap();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0].to_string(), "[[0,\"a\"],[]]");
        assert_eq!(output[1].to_string(), "[[0,\"a\"]]");
        assert_eq!(output[2].to_string(), "[[0]]");

        let state = start(
            "tostream",
            &Value::array(vec![number(1.0), number(2.0)]),
            &[],
            0,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let output = pull_all(state).unwrap();
        assert_eq!(
            output.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["[[0],1]", "[[1],2]", "[[1]]"]
        );

        let object = Value::object(indexmap::IndexMap::from([
            (Arc::from("a"), number(1.0)),
            (Arc::from("b"), number(2.0)),
        ]));
        let state = start("tostream", &object, &[], 0, VmLimits::default())
            .unwrap()
            .unwrap();
        let output = pull_all(state).unwrap();
        assert_eq!(
            output.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["[[\"a\"],1]", "[[\"b\"],2]", "[[\"b\"]]"]
        );

        let state = start(
            "tostream",
            &Value::array(vec![Value::array(vec![number(1.0), number(2.0)])]),
            &[],
            0,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let output = pull_all(state).unwrap();
        assert_eq!(
            output.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["[[0,0],1]", "[[0,1],2]", "[[0,1]]", "[[0]]"]
        );

        let state = start(
            "tostream",
            &Value::array(vec![Value::array(Vec::new()), number(1.0)]),
            &[],
            0,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let output = pull_all(state).unwrap();
        assert_eq!(
            output.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["[[0],[]]", "[[1],1]", "[[1]]"]
        );

        let object = Value::object(indexmap::IndexMap::from([
            (Arc::from("a"), Value::array(Vec::new())),
            (Arc::from("b"), number(1.0)),
        ]));
        let state = start("tostream", &object, &[], 0, VmLimits::default())
            .unwrap()
            .unwrap();
        let output = pull_all(state).unwrap();
        assert_eq!(
            output.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["[[\"a\"],[]]", "[[\"b\"],1]", "[[\"b\"]]"]
        );

        let state = start(
            "tostream",
            &Value::array(Vec::new()),
            &[],
            0,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(pull_all(state).unwrap()[0].to_string(), "[[],[]]");
    }

    #[test]
    fn fromstream_rebuilds_only_on_matching_root_close() {
        let mut state = FromStreamState::new();
        let mut charge = || Ok(());
        let limits = VmLimits::default();
        let event = |path: Vec<Value>, value: Option<Value>| {
            let mut parts = vec![Value::array(path)];
            if let Some(value) = value {
                parts.push(value);
            }
            Value::array(parts)
        };

        assert_eq!(
            state
                .feed(
                    &event(vec![number(0.0)], Some(number(1.0))),
                    limits,
                    &mut charge,
                )
                .unwrap(),
            None
        );
        assert_eq!(
            state
                .feed(
                    &event(vec![number(1.0)], Some(number(2.0))),
                    limits,
                    &mut charge,
                )
                .unwrap(),
            None
        );
        assert_eq!(
            state
                .feed(&event(vec![number(1.0)], None), limits, &mut charge)
                .unwrap()
                .expect("root close emits the rebuilt array")
                .to_string(),
            "[1,2]"
        );
        let root_leaf = event(Vec::new(), Some(Value::string("leaf")));
        assert_eq!(
            state.feed(&root_leaf, limits, &mut charge).unwrap(),
            Some(Value::string("leaf"))
        );
        let empty_root = event(Vec::new(), Some(Value::array(Vec::new())));
        assert_eq!(
            state.feed(&empty_root, limits, &mut charge).unwrap(),
            Some(Value::array(Vec::new()))
        );
        assert!(state.feed(&Value::Null, limits, &mut charge).is_err());

        let mut partial = FromStreamState::new();
        partial
            .feed(
                &event(vec![number(0.0)], Some(number(1.0))),
                limits,
                &mut charge,
            )
            .unwrap();
        assert!(partial.feed(&Value::Null, limits, &mut charge).is_err());
        assert_eq!(
            partial
                .feed(&event(vec![number(0.0)], None), limits, &mut charge)
                .unwrap()
                .expect("partial root survives malformed event")
                .to_string(),
            "[1]"
        );
    }

    #[test]
    fn fromstream_keeps_nested_state_and_does_not_flush_incomplete_roots() {
        let mut state = FromStreamState::new();
        let mut charge = || Ok(());
        let limits = VmLimits::default();
        let events = [
            (vec![number(0.0), number(0.0)], Some(number(1.0))),
            (vec![number(0.0), number(0.0)], None),
            (vec![number(0.0), number(1.0)], Some(number(2.0))),
            (vec![number(0.0), number(1.0)], None),
            (vec![number(0.0)], None),
        ];
        let mut output = None;
        for (path, value) in events {
            let mut parts = vec![Value::array(path)];
            if let Some(value) = value {
                parts.push(value);
            }
            output = state
                .feed(&Value::array(parts), limits, &mut charge)
                .unwrap()
                .or(output);
        }
        assert_eq!(output.expect("nested root close").to_string(), "[[1,2]]");

        let mut incomplete = FromStreamState::new();
        assert_eq!(
            incomplete
                .feed(
                    &Value::array(vec![Value::array(vec![number(0.0)]), number(1.0)]),
                    limits,
                    &mut charge,
                )
                .unwrap(),
            None
        );
    }

    #[test]
    fn fromstream_checks_charge_and_path_limits() {
        let mut state = FromStreamState::new();
        let limits = VmLimits {
            path_stack: 1,
            ..VmLimits::default()
        };
        let mut cancelled = || {
            Err(VmError::Resource {
                resource: "cancelled",
            })
        };
        let event = Value::array(vec![Value::array(vec![number(0.0)]), number(1.0)]);
        assert_eq!(
            state.feed(&event, limits, &mut cancelled),
            Err(VmError::Resource {
                resource: "cancelled"
            })
        );

        let mut charge = || Ok(());
        let too_deep = Value::array(vec![
            Value::array(vec![number(0.0), number(0.0)]),
            number(1.0),
        ]);
        assert_eq!(
            state.feed(&too_deep, limits, &mut charge),
            Err(VmError::Resource {
                resource: "path-stack"
            })
        );
    }

    #[test]
    fn fromstream_rejects_deep_paths_before_component_charges() {
        let mut state = FromStreamState::new();
        let limits = VmLimits {
            path_stack: 1,
            ..VmLimits::default()
        };
        let event = Value::array(vec![
            Value::array((0..8).map(|_| number(0.0)).collect::<Vec<_>>()),
            number(1.0),
        ]);
        let mut charges = 0;
        let mut charge = || {
            charges += 1;
            Ok(())
        };

        assert_eq!(
            state.feed(&event, limits, &mut charge),
            Err(VmError::Resource {
                resource: "path-stack"
            })
        );
        assert_eq!(charges, 1, "only the event pull should be charged");
    }

    #[test]
    fn fromstream_reconstructs_a_path_at_the_depth_limit() {
        let mut state = FromStreamState::new();
        let limits = VmLimits {
            path_stack: 1,
            ..VmLimits::default()
        };
        let mut charge = || Ok(());
        let event = |path: Vec<Value>, value: Option<Value>| {
            let mut parts = vec![Value::array(path)];
            if let Some(value) = value {
                parts.push(value);
            }
            Value::array(parts)
        };

        assert_eq!(
            state
                .feed(
                    &event(vec![number(0.0)], Some(number(7.0))),
                    limits,
                    &mut charge,
                )
                .unwrap(),
            None
        );
        assert_eq!(
            state
                .feed(&event(vec![number(0.0)], None), limits, &mut charge)
                .unwrap()
                .expect("matching root close emits the rebuilt array")
                .to_string(),
            "[7]"
        );
    }

    #[test]
    fn truncate_stream_event_projects_each_retained_event() {
        let count = number(1.0);
        let limits = VmLimits::default();
        let mut charge = || Ok(());
        let value_event = Value::array(vec![
            Value::array(vec![number(1.0), number(0.0)]),
            Value::string("b"),
        ]);
        let close_event = Value::array(vec![Value::array(vec![number(1.0), number(0.0)])]);
        assert_eq!(
            truncate_stream_event(&value_event, &count, limits, &mut charge)
                .unwrap()
                .expect("retained value event")
                .to_string(),
            "[[0],\"b\"]"
        );
        assert_eq!(
            truncate_stream_event(&close_event, &count, limits, &mut charge)
                .unwrap()
                .expect("retained close event")
                .to_string(),
            "[[0]]"
        );

        let shallow = Value::array(vec![Value::array(vec![number(0.0)]), number(1.0)]);
        assert_eq!(
            truncate_stream_event(&shallow, &count, limits, &mut charge).unwrap(),
            None
        );
    }

    #[test]
    fn truncate_stream_event_checks_count_path_and_charge_limits() {
        let event = Value::array(vec![
            Value::array(vec![number(0.0), number(0.0), number(0.0)]),
            number(1.0),
        ]);
        let count = number(1.0);
        let limits = VmLimits {
            path_stack: 1,
            ..VmLimits::default()
        };
        let mut charge = || Ok(());
        assert_eq!(
            truncate_stream_event(&event, &count, limits, &mut charge),
            Err(VmError::Resource {
                resource: "path-stack"
            })
        );

        let mut cancelled = || {
            Err(VmError::Resource {
                resource: "cancelled",
            })
        };
        let short = Value::array(vec![Value::array(vec![number(0.0)]), number(1.0)]);
        assert_eq!(
            truncate_stream_event(&short, &count, VmLimits::default(), &mut cancelled),
            Err(VmError::Resource {
                resource: "cancelled"
            })
        );

        let invalid_count = Value::Bool(false);
        let mut charge = || Ok(());
        assert!(
            truncate_stream_event(&short, &invalid_count, VmLimits::default(), &mut charge)
                .is_err()
        );

        let ignored_count = Value::string("one");
        assert_eq!(
            truncate_stream_event(&short, &ignored_count, VmLimits::default(), &mut charge)
                .expect("non-numeric count is an empty stream"),
            None
        );
    }

    #[test]
    fn generator_charge_is_checked_between_results() {
        let state = start(
            "range",
            &Value::Null,
            &[number(0.0), number(3.0)],
            2,
            VmLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut remaining = 1usize;
        let mut state = state;
        let mut charge = || {
            if remaining == 0 {
                Err(VmError::Resource {
                    resource: "vm-steps",
                })
            } else {
                remaining -= 1;
                Ok(())
            }
        };
        assert!(state.next(&mut charge).unwrap().is_some());
        assert_eq!(
            state.next(&mut charge),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
    }
}
