//! Bounded materialization helpers for jq's SQL-shaped built-ins.
//!
//! Filter evaluation and pull control stay in the evaluator dispatcher.  This
//! module owns only the small materialized pieces needed by `INDEX` and the
//! buffered `JOIN` form so that retained rows and keys have explicit quotas.

use std::sync::Arc;

use crate::{Object, Value, VmError, VmLimits, format};

use super::{access_index, collection, resource};

/// Builds the object produced by `INDEX`, preserving jq's stringified keys
/// and last-row-wins behavior for duplicate keys.
pub(super) struct SqlIndexBuilder {
    index: Object,
    limits: VmLimits,
    retained_key_bytes: usize,
}

impl SqlIndexBuilder {
    /// Starts an empty bounded index.
    #[must_use]
    pub(super) fn new(limits: VmLimits) -> Self {
        Self {
            index: Object::new(),
            limits,
            retained_key_bytes: 0,
        }
    }

    /// Adds one row under one jq-stringified key.
    pub(super) fn insert(
        &mut self,
        key: &Value,
        row: Value,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<(), VmError> {
        charge()?;
        charge_value_tree(key, self.limits, charge)?;
        let key = format::text(key, self.limits.output_bytes)?;
        charge()?;
        if !self.index.contains_key(&key) {
            let next = self
                .index
                .len()
                .checked_add(1)
                .ok_or_else(|| resource("output-bytes"))?;
            let key_bytes = self
                .retained_key_bytes
                .checked_add(key.len())
                .ok_or_else(|| resource("output-bytes"))?;
            ensure_index_lower_bound(next, key_bytes, self.limits.output_bytes)?;
            self.index
                .try_reserve(1)
                .map_err(|_| resource("output-bytes"))?;
            self.retained_key_bytes = key_bytes;
        }
        self.index.insert(key, row);
        Ok(())
    }

    /// Finishes the index as an immutable runtime object.
    #[must_use]
    pub(super) fn finish(self) -> Value {
        Value::object(self.index)
    }
}

/// Collects buffered `[row, lookup]` pairs for `JOIN`/2.
pub(super) struct SqlPairCollector {
    pairs: Vec<Value>,
    limits: VmLimits,
}

impl SqlPairCollector {
    /// Starts an empty bounded pair collection.
    #[must_use]
    pub(super) fn new(limits: VmLimits) -> Self {
        Self {
            pairs: Vec::new(),
            limits,
        }
    }

    /// Retains one pair after checking output quota and allocation failure.
    pub(super) fn push(
        &mut self,
        row: Value,
        matched: Value,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<(), VmError> {
        charge()?;
        let next = self
            .pairs
            .len()
            .checked_add(1)
            .ok_or_else(|| resource("output-bytes"))?;
        ensure_container_lower_bound(next, self.limits.output_bytes)?;
        self.pairs
            .try_reserve(1)
            .map_err(|_| resource("output-bytes"))?;
        let mut pair = Vec::new();
        pair.try_reserve_exact(2)
            .map_err(|_| resource("output-bytes"))?;
        pair.push(row);
        pair.push(matched);
        self.pairs.push(Value::array(pair));
        Ok(())
    }

    /// Finishes the buffered pairs as one array.
    #[must_use]
    pub(super) fn finish(self) -> Value {
        Value::array(self.pairs)
    }
}

/// Performs a typed jq lookup for JOIN keys; unlike INDEX keys, it does not
/// stringify the key before accessing an object or array.
pub(super) fn typed_lookup(index: &Value, key: &Value) -> Result<Value, VmError> {
    access_index(index, key)
}

enum EqualFrame<'a> {
    Compare(&'a Value, &'a Value),
    Array {
        left: &'a Arc<[Value]>,
        right: &'a Arc<[Value]>,
        next: usize,
    },
    Object {
        left: &'a Arc<Object>,
        right: &'a Arc<Object>,
        next: usize,
    },
}

fn push_equal_frame<'a>(
    frames: &mut Vec<EqualFrame<'a>>,
    frame: EqualFrame<'a>,
    limits: VmLimits,
) -> Result<(), VmError> {
    if frames.len() >= limits.value_stack {
        return Err(resource("value-stack"));
    }
    frames
        .try_reserve(1)
        .map_err(|_| resource("output-bytes"))?;
    frames.push(frame);
    Ok(())
}

/// Performs jq equality without recursive native-stack growth.
#[allow(
    clippy::too_many_lines,
    reason = "the iterative equality state machine keeps composite admission in one bounded loop"
)]
pub(super) fn equal_bounded(
    left: &Value,
    right: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<bool, VmError> {
    let mut frames = Vec::new();
    push_equal_frame(&mut frames, EqualFrame::Compare(left, right), limits)?;
    while let Some(frame) = frames.pop() {
        charge()?;
        match frame {
            EqualFrame::Compare(left, right) => match (left, right) {
                (Value::Array(left), Value::Array(right)) => {
                    if Arc::ptr_eq(left, right) {
                        continue;
                    }
                    if left.len() != right.len() {
                        return Ok(false);
                    }
                    push_equal_frame(
                        &mut frames,
                        EqualFrame::Array {
                            left,
                            right,
                            next: 0,
                        },
                        limits,
                    )?;
                }
                (Value::Object(left), Value::Object(right)) => {
                    if Arc::ptr_eq(left, right) {
                        continue;
                    }
                    if left.len() != right.len() {
                        return Ok(false);
                    }
                    push_equal_frame(
                        &mut frames,
                        EqualFrame::Object {
                            left,
                            right,
                            next: 0,
                        },
                        limits,
                    )?;
                }
                (Value::String(left), Value::String(right)) => {
                    charge_string_bytes(left, charge)?;
                    charge_string_bytes(right, charge)?;
                    if left != right {
                        return Ok(false);
                    }
                }
                _ => {
                    if !collection::jq_equal(left, right) {
                        return Ok(false);
                    }
                }
            },
            EqualFrame::Array { left, right, next } => {
                if let Some(left_value) = left.get(next) {
                    let right_value = right
                        .get(next)
                        .expect("array lengths were checked before traversal");
                    push_equal_frame(
                        &mut frames,
                        EqualFrame::Array {
                            left,
                            right,
                            next: next.saturating_add(1),
                        },
                        limits,
                    )?;
                    push_equal_frame(
                        &mut frames,
                        EqualFrame::Compare(left_value, right_value),
                        limits,
                    )?;
                }
            }
            EqualFrame::Object { left, right, next } => {
                if let Some((key, left_value)) = left.get_index(next) {
                    charge_string_bytes(key, charge)?;
                    let Some(right_value) = right.get(key) else {
                        return Ok(false);
                    };
                    push_equal_frame(
                        &mut frames,
                        EqualFrame::Object {
                            left,
                            right,
                            next: next.saturating_add(1),
                        },
                        limits,
                    )?;
                    push_equal_frame(
                        &mut frames,
                        EqualFrame::Compare(left_value, right_value),
                        limits,
                    )?;
                }
            }
        }
    }
    Ok(true)
}

/// Constructs a streaming JOIN pair with bounded allocation.
pub(super) fn pair_bounded(
    row: Value,
    matched: Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    let lower_bound = 2_usize
        .checked_add(2)
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    let mut pair = Vec::new();
    pair.try_reserve_exact(2)
        .map_err(|_| resource("output-bytes"))?;
    pair.push(row);
    pair.push(matched);
    Ok(Value::array(pair))
}

fn ensure_index_lower_bound(
    entries: usize,
    key_bytes: usize,
    output_limit: usize,
) -> Result<(), VmError> {
    let lower_bound = entries
        .checked_add(2)
        .and_then(|size| size.checked_add(key_bytes))
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

fn ensure_container_lower_bound(entries: usize, output_limit: usize) -> Result<(), VmError> {
    let lower_bound = entries
        .checked_add(2)
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

/// Charges a key's composite nodes before the formatter traverses it.  The
/// stack is iterative and fallibly grown, with `value_stack` limiting active
/// frame depth rather than the flat width of one admitted input value.
fn charge_value_tree(
    value: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    let mut pending = Vec::new();
    push_frame(&mut pending, ChargeFrame::Value(value), limits)?;
    while let Some(frame) = pending.pop() {
        charge()?;
        match frame {
            ChargeFrame::Value(value) => match value {
                Value::Array(values) => {
                    push_frame(&mut pending, ChargeFrame::Array(values, 0), limits)?;
                }
                Value::Object(values) => {
                    push_frame(&mut pending, ChargeFrame::Object(values, 0), limits)?;
                }
                Value::String(value) => charge_string_bytes(value, charge)?,
                Value::Null | Value::Bool(_) | Value::Number(_) => {}
            },
            ChargeFrame::Array(values, next) => {
                if let Some(child) = values.get(next) {
                    push_frame(
                        &mut pending,
                        ChargeFrame::Array(values, next.saturating_add(1)),
                        limits,
                    )?;
                    push_frame(&mut pending, ChargeFrame::Value(child), limits)?;
                }
            }
            ChargeFrame::Object(values, next) => {
                if let Some((key, child)) = values.get_index(next) {
                    charge_string_bytes(key, charge)?;
                    push_frame(
                        &mut pending,
                        ChargeFrame::Object(values, next.saturating_add(1)),
                        limits,
                    )?;
                    push_frame(&mut pending, ChargeFrame::Value(child), limits)?;
                }
            }
        }
    }
    Ok(())
}

enum ChargeFrame<'a> {
    Value(&'a Value),
    Array(&'a [Value], usize),
    Object(&'a Object, usize),
}

fn push_frame<'a>(
    frames: &mut Vec<ChargeFrame<'a>>,
    frame: ChargeFrame<'a>,
    limits: VmLimits,
) -> Result<(), VmError> {
    if frames.len() >= limits.value_stack {
        return Err(resource("value-stack"));
    }
    frames
        .try_reserve(1)
        .map_err(|_| resource("output-bytes"))?;
    frames.push(frame);
    Ok(())
}

fn charge_string_bytes(
    value: &str,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    for _ in value.as_bytes().chunks(4096) {
        charge()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use indexmap::IndexMap;

    use super::*;

    fn number(value: usize) -> Value {
        Value::Number(crate::Number::parse(&value.to_string()).expect("fixture integer"))
    }

    fn flat_array(length: usize) -> Value {
        Value::array((0..length).map(number).collect::<Vec<_>>())
    }

    fn nested_array(depth: usize) -> Value {
        let mut value = Value::Null;
        for _ in 0..depth {
            value = Value::array(vec![value]);
        }
        value
    }

    fn ok_charge() -> impl FnMut() -> Result<(), VmError> {
        || Ok(())
    }

    #[test]
    fn index_builder_stringifies_and_replaces_duplicate_keys() {
        let mut builder = SqlIndexBuilder::new(VmLimits::default());
        let mut charge = ok_charge();
        builder
            .insert(&number(1), Value::string("first"), &mut charge)
            .unwrap();
        builder
            .insert(&Value::string("1"), Value::string("last"), &mut charge)
            .unwrap();
        assert_eq!(
            builder.finish(),
            Value::object(IndexMap::from([(Arc::from("1"), Value::string("last"))]))
        );
    }

    #[test]
    fn index_builder_rejects_new_keys_before_unbounded_growth() {
        let mut builder = SqlIndexBuilder::new(VmLimits {
            output_bytes: 2,
            ..VmLimits::default()
        });
        let mut charge = ok_charge();
        let error = builder
            .insert(&Value::string("a"), Value::Null, &mut charge)
            .expect_err("object lower bound includes delimiters");
        assert!(matches!(
            error,
            VmError::Resource {
                resource: "output-bytes"
            }
        ));

        let mut builder = SqlIndexBuilder::new(VmLimits {
            output_bytes: 4,
            ..VmLimits::default()
        });
        let mut charge = ok_charge();
        builder
            .insert(&Value::string("a"), Value::Null, &mut charge)
            .expect("the lower-bound quota admits one key");
    }

    #[test]
    fn index_builder_bounds_cumulative_distinct_key_bytes() {
        let mut builder = SqlIndexBuilder::new(VmLimits {
            output_bytes: 7,
            ..VmLimits::default()
        });
        let mut charge = ok_charge();
        builder
            .insert(&Value::string("aa"), Value::Null, &mut charge)
            .expect("the first retained key fits");
        let error = builder
            .insert(&Value::string("bb"), Value::Null, &mut charge)
            .expect_err("distinct retained key bytes must share the quota");
        assert!(matches!(
            error,
            VmError::Resource {
                resource: "output-bytes"
            }
        ));

        builder
            .insert(&Value::string("aa"), Value::Bool(true), &mut charge)
            .expect("duplicate replacement does not increase retained key bytes");
    }

    #[test]
    fn index_builder_observes_charge_failures() {
        let mut builder = SqlIndexBuilder::new(VmLimits::default());
        let mut charge = || {
            Err(VmError::Resource {
                resource: "vm-steps",
            })
        };
        let error = builder
            .insert(&Value::string("a"), Value::Null, &mut charge)
            .expect_err("index insertion must checkpoint before formatting");
        assert!(matches!(
            error,
            VmError::Resource {
                resource: "vm-steps"
            }
        ));
    }

    #[test]
    fn composite_key_walk_bounds_depth_without_capping_flat_width() {
        let mut builder = SqlIndexBuilder::new(VmLimits {
            value_stack: 2,
            output_bytes: 100_000,
            ..VmLimits::default()
        });
        let mut steps = 0usize;
        let mut charge = || {
            steps = steps.saturating_add(1);
            Ok(())
        };
        builder
            .insert(&flat_array(5_000), Value::Null, &mut charge)
            .expect("flat key width is not a value-stack depth cap");
        assert!(steps >= 5_000);

        let mut builder = SqlIndexBuilder::new(VmLimits {
            value_stack: 2,
            output_bytes: 100,
            ..VmLimits::default()
        });
        let mut charge = ok_charge();
        let error = builder
            .insert(&nested_array(4), Value::Null, &mut charge)
            .expect_err("nested key depth must be bounded");
        assert!(matches!(
            error,
            VmError::Resource {
                resource: "value-stack"
            }
        ));
    }

    #[test]
    fn composite_key_walk_charges_huge_strings_before_formatting() {
        let mut builder = SqlIndexBuilder::new(VmLimits {
            output_bytes: 8,
            ..VmLimits::default()
        });
        let mut steps = 0usize;
        let mut charge = || {
            steps = steps.saturating_add(1);
            Ok(())
        };
        let error = builder
            .insert(&Value::string("x".repeat(9_000)), Value::Null, &mut charge)
            .expect_err("formatter output quota rejects the huge key");
        assert!(matches!(
            error,
            VmError::Resource {
                resource: "output-bytes"
            }
        ));
        assert!(steps >= 4, "insert, node, and string chunks were charged");
    }

    #[test]
    fn pair_collector_bounds_retained_pairs_and_lookup_stays_typed() {
        let mut collector = SqlPairCollector::new(VmLimits {
            output_bytes: 3,
            ..VmLimits::default()
        });
        let mut charge = ok_charge();
        collector
            .push(Value::string("row"), Value::Null, &mut charge)
            .expect("the lower-bound quota admits one pair");
        assert_eq!(
            collector.finish(),
            Value::from_json(serde_json::json!([["row", null]])).unwrap()
        );

        let index = Value::object(IndexMap::from([(Arc::from("1"), Value::string("object"))]));
        assert_eq!(
            typed_lookup(&index, &Value::string("1")).unwrap(),
            Value::string("object")
        );
        assert!(typed_lookup(&index, &number(1)).is_err());
    }

    #[test]
    fn pair_collector_rejects_quota_and_charge_exhaustion() {
        let mut collector = SqlPairCollector::new(VmLimits {
            output_bytes: 2,
            ..VmLimits::default()
        });
        let mut charge = ok_charge();
        let error = collector
            .push(Value::Null, Value::Null, &mut charge)
            .expect_err("pair lower-bound quota must reject before retention");
        assert!(matches!(
            error,
            VmError::Resource {
                resource: "output-bytes"
            }
        ));

        let mut collector = SqlPairCollector::new(VmLimits::default());
        let mut charge = || {
            Err(VmError::Resource {
                resource: "vm-steps",
            })
        };
        let error = collector
            .push(Value::Null, Value::Null, &mut charge)
            .expect_err("pair collection must checkpoint before allocation");
        assert!(matches!(
            error,
            VmError::Resource {
                resource: "vm-steps"
            }
        ));
    }
}
