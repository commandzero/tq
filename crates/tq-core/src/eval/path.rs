//! Allocation-aware persistent path updates for evaluator values.

use std::sync::Arc;

use indexmap::IndexMap;

use crate::{PathComponent, Value, VmError, VmLimits};

use super::{resource, runtime, type_error};

/// Replaces or creates a value at `components` without unchecked indexed
/// growth.
///
/// The output-byte check is deliberately a lower-bound admission check for
/// each resulting container, not a claim that heap capacity equals encoded
/// output bytes. A container's entries plus its two delimiters are enough to
/// reject unbounded indexed growth before allocation while retaining ordinary
/// existing-array updates under the default limit.
#[allow(
    clippy::too_many_lines,
    reason = "iterative bounded reconstruction keeps admission and cloning rules together"
)]
pub(super) fn replace_or_create_bounded(
    root: &Value,
    components: &[PathComponent],
    replacement: Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    if components.len() > limits.path_stack {
        return Err(resource("path-stack"));
    }

    let mut ancestors = Vec::new();
    ancestors
        .try_reserve_exact(components.len())
        .map_err(|_| resource("path-stack"))?;
    let null = Value::Null;
    let mut current = root;

    for component in components {
        charge()?;
        match (component, current) {
            (PathComponent::Key(key), Value::Object(object)) => {
                let target = object
                    .len()
                    .checked_add(usize::from(!object.contains_key(key)))
                    .ok_or_else(|| resource("output-bytes"))?;
                ensure_container_lower_bound(target, limits.output_bytes)?;
                ancestors.push((current, component.clone()));
                current = object.get(key).unwrap_or(&null);
            }
            (PathComponent::Key(_), Value::Null) => {
                ensure_container_lower_bound(1, limits.output_bytes)?;
                ancestors.push((current, component.clone()));
                current = &null;
            }
            (PathComponent::Key(_), value) => {
                return Err(type_error("object assignment", value));
            }
            (PathComponent::Index(index), Value::Array(values)) => {
                let required = index
                    .checked_add(1)
                    .ok_or_else(|| resource("output-bytes"))?;
                ensure_container_lower_bound(values.len().max(required), limits.output_bytes)?;
                ancestors.push((current, component.clone()));
                current = values.get(*index).unwrap_or(&null);
            }
            (PathComponent::Index(index), Value::Null) => {
                let required = index
                    .checked_add(1)
                    .ok_or_else(|| resource("output-bytes"))?;
                ensure_container_lower_bound(required, limits.output_bytes)?;
                ancestors.push((current, component.clone()));
                current = &null;
            }
            (PathComponent::Index(_), value) => {
                return Err(type_error("array assignment", value));
            }
        }
    }

    let mut rebuilt = replacement;
    while let Some((parent, component)) = ancestors.pop() {
        charge()?;
        rebuilt = match component {
            PathComponent::Key(key) => {
                let mut object = IndexMap::new();
                let capacity = match parent {
                    Value::Object(values) => values
                        .len()
                        .checked_add(usize::from(!values.contains_key(&key)))
                        .ok_or_else(|| resource("output-bytes"))?,
                    Value::Null => 1,
                    value => return Err(type_error("object assignment", value)),
                };
                object
                    .try_reserve(capacity)
                    .map_err(|_| resource("output-bytes"))?;
                match parent {
                    Value::Object(values) => {
                        for (existing_key, existing_value) in values.iter() {
                            charge()?;
                            object.insert(Arc::clone(existing_key), existing_value.clone());
                        }
                    }
                    Value::Null => {}
                    value => return Err(type_error("object assignment", value)),
                }
                charge()?;
                object.insert(key, rebuilt);
                Value::object(object)
            }
            PathComponent::Index(index) => {
                let existing = match parent {
                    Value::Array(values) => values.len(),
                    Value::Null => 0,
                    value => return Err(type_error("array assignment", value)),
                };
                let required = index
                    .checked_add(1)
                    .ok_or_else(|| resource("output-bytes"))?;
                let target = existing.max(required);
                let mut values = Vec::new();
                values
                    .try_reserve_exact(target)
                    .map_err(|_| resource("output-bytes"))?;
                match parent {
                    Value::Array(existing_values) => {
                        for value in existing_values.iter() {
                            charge()?;
                            values.push(value.clone());
                        }
                    }
                    Value::Null => {}
                    value => return Err(type_error("array assignment", value)),
                }
                while values.len() < target {
                    charge()?;
                    values.push(Value::Null);
                }
                values[index] = rebuilt;
                Value::array(values)
            }
        };
    }
    Ok(rebuilt)
}

/// Reads a path while charging each component and enforcing the managed path
/// depth.  Missing object keys and array indices follow jq's `getpath`
/// behavior and produce `null`; a missing ancestor therefore remains a valid
/// target for a later bounded update.
pub(super) fn getpath_bounded(
    root: &Value,
    components: &[PathComponent],
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    if components.len() > limits.path_stack {
        return Err(resource("path-stack"));
    }

    let mut current = root.clone();
    for component in components {
        charge()?;
        current = match (component, &current) {
            (PathComponent::Key(key), Value::Object(values)) => {
                values.get(key).cloned().unwrap_or(Value::Null)
            }
            (PathComponent::Index(index), Value::Array(values)) => {
                values.get(*index).cloned().unwrap_or(Value::Null)
            }
            (PathComponent::Key(_) | PathComponent::Index(_), Value::Null) => Value::Null,
            (_, value) => return Err(type_error("index", value)),
        };
    }
    Ok(current)
}

/// Deletes a path with fallible, charged reconstruction.
///
/// Unlike the legacy recursive helper, this keeps the ancestor stack in a
/// bounded vector and reserves every rebuilt container before copying values.
/// Missing paths are no-ops, matching jq's deletion behavior.
pub(super) fn delete_path_bounded(
    root: &Value,
    components: &[PathComponent],
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    charge()?;
    if components.len() > limits.path_stack {
        return Err(resource("path-stack"));
    }
    if components.is_empty() {
        return Ok(Value::Null);
    }

    let mut ancestors = Vec::new();
    ancestors
        .try_reserve_exact(components.len())
        .map_err(|_| resource("path-stack"))?;
    let mut current = root;
    for component in components {
        charge()?;
        match (component, current) {
            (PathComponent::Key(key), Value::Object(values)) => {
                let Some(child) = values.get(key) else {
                    return Ok(root.clone());
                };
                ancestors.push((current, component.clone()));
                current = child;
            }
            (PathComponent::Index(index), Value::Array(values)) => {
                let Some(child) = values.get(*index) else {
                    return Ok(root.clone());
                };
                ancestors.push((current, component.clone()));
                current = child;
            }
            (PathComponent::Key(_) | PathComponent::Index(_), Value::Null) => {
                return Ok(root.clone());
            }
            (_, value) => return Err(type_error("index", value)),
        }
    }

    // Every non-empty path has one ancestor frame at this point.
    let (parent, last_component) = ancestors
        .pop()
        .expect("non-empty path has a final ancestor");
    let mut rebuilt = delete_from_parent(parent, &last_component, limits, charge)?;

    while let Some((parent, component)) = ancestors.pop() {
        charge()?;
        rebuilt = replace_child_bounded(parent, &component, rebuilt, limits, charge)?;
    }
    Ok(rebuilt)
}

/// Replaces an array slice, charging both materialization and ancestor
/// reconstruction. jq requires the replacement to be an array; its elements
/// are spliced into the selected range.
pub(super) fn replace_slice_bounded(
    root: &Value,
    components: &[PathComponent],
    start: usize,
    end: usize,
    replacement: Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let target = getpath_bounded(root, components, limits, charge)?;
    let Value::Array(values) = target else {
        return Err(type_error("slice", &target));
    };
    let Value::Array(replacement_values) = replacement else {
        return Err(runtime(
            "A slice of an array can only be assigned another array".to_owned(),
        ));
    };
    let replacement_len = replacement_values.len();
    let prefix_len = start.min(values.len());
    let suffix_len = values.len().saturating_sub(end);
    let output_len = prefix_len
        .checked_add(replacement_len)
        .and_then(|length| length.checked_add(suffix_len))
        .ok_or_else(|| resource("output-bytes"))?;
    ensure_container_lower_bound(output_len, limits.output_bytes)?;

    charge()?;
    let mut updated = Vec::new();
    updated
        .try_reserve_exact(output_len)
        .map_err(|_| resource("output-bytes"))?;
    for value in values.iter().take(start) {
        charge()?;
        updated.push(value.clone());
    }
    for value in replacement_values.iter() {
        charge()?;
        updated.push(value.clone());
    }
    for value in values.iter().skip(end) {
        charge()?;
        updated.push(value.clone());
    }
    replace_or_create_bounded(root, components, Value::array(updated), limits, charge)
}

fn delete_from_parent(
    parent: &Value,
    component: &PathComponent,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match (component, parent) {
        (PathComponent::Key(key), Value::Object(values)) => {
            let target = values
                .len()
                .checked_sub(1)
                .ok_or_else(|| resource("output-bytes"))?;
            ensure_container_lower_bound(target, limits.output_bytes)?;
            let mut updated = IndexMap::new();
            updated
                .try_reserve(target)
                .map_err(|_| resource("output-bytes"))?;
            for (existing_key, value) in values.iter() {
                charge()?;
                if existing_key != key {
                    updated.insert(Arc::clone(existing_key), value.clone());
                }
            }
            Ok(Value::object(updated))
        }
        (PathComponent::Index(index), Value::Array(values)) => {
            let target = values
                .len()
                .checked_sub(1)
                .ok_or_else(|| resource("output-bytes"))?;
            ensure_container_lower_bound(target, limits.output_bytes)?;
            let mut updated = Vec::new();
            updated
                .try_reserve_exact(target)
                .map_err(|_| resource("output-bytes"))?;
            for (position, value) in values.iter().enumerate() {
                charge()?;
                if position != *index {
                    updated.push(value.clone());
                }
            }
            Ok(Value::array(updated))
        }
        (_, value) => Err(type_error("index", value)),
    }
}

fn replace_child_bounded(
    parent: &Value,
    component: &PathComponent,
    replacement: Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let mut replacement = Some(replacement);
    match (component, parent) {
        (PathComponent::Key(key), Value::Object(values)) => {
            ensure_container_lower_bound(values.len(), limits.output_bytes)?;
            let mut updated = IndexMap::new();
            updated
                .try_reserve(values.len())
                .map_err(|_| resource("output-bytes"))?;
            for (existing_key, value) in values.iter() {
                charge()?;
                if existing_key == key {
                    updated.insert(
                        Arc::clone(existing_key),
                        replacement.take().expect("ancestor key occurs once"),
                    );
                } else {
                    updated.insert(Arc::clone(existing_key), value.clone());
                }
            }
            Ok(Value::object(updated))
        }
        (PathComponent::Index(index), Value::Array(values)) => {
            ensure_container_lower_bound(values.len(), limits.output_bytes)?;
            if *index >= values.len() {
                return Err(resource("path-stack"));
            }
            let mut updated = Vec::new();
            updated
                .try_reserve_exact(values.len())
                .map_err(|_| resource("output-bytes"))?;
            for (position, value) in values.iter().enumerate() {
                charge()?;
                updated.push(if position == *index {
                    replacement.take().expect("ancestor index occurs once")
                } else {
                    value.clone()
                });
            }
            Ok(Value::array(updated))
        }
        (_, value) => Err(type_error("index", value)),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: usize) -> Value {
        Value::Number(crate::Number::parse(&value.to_string()).expect("fixture integer"))
    }

    #[test]
    fn indexed_growth_is_rejected_before_allocation() {
        let mut charge = || Ok(());
        let root = Value::Null;
        let path = [PathComponent::Index(100)];
        assert_eq!(
            replace_or_create_bounded(
                &root,
                &path,
                number(1),
                VmLimits {
                    output_bytes: 32,
                    ..VmLimits::default()
                },
                &mut charge,
            ),
            Err(VmError::Resource {
                resource: "output-bytes"
            })
        );
    }

    #[test]
    fn ordinary_nested_replacement_matches_existing_shape() {
        let mut charge = || Ok(());
        let root = Value::Null;
        let path = [PathComponent::Index(0), PathComponent::Key(Arc::from("x"))];
        let output =
            replace_or_create_bounded(&root, &path, number(1), VmLimits::default(), &mut charge)
                .unwrap();
        assert_eq!(output.to_string(), r#"[{"x":1}]"#);
    }

    #[test]
    fn existing_array_can_be_updated_with_default_limits() {
        let mut charge = || Ok(());
        let root = Value::array(vec![number(1), number(2)]);
        let path = [PathComponent::Index(1)];
        let output =
            replace_or_create_bounded(&root, &path, number(3), VmLimits::default(), &mut charge)
                .unwrap();
        assert_eq!(output.to_string(), "[1,3]");
    }

    #[test]
    fn charge_is_checked_before_each_copy_and_growth() {
        let mut remaining = 2_usize;
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
        let root = Value::Null;
        let path = [PathComponent::Index(1)];
        assert_eq!(
            replace_or_create_bounded(&root, &path, number(1), VmLimits::default(), &mut charge,),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
    }

    #[test]
    fn charge_can_fail_during_existing_copy_without_mutating_root() {
        let root = Value::array(vec![number(1), number(2), number(3)]);
        let original = root.to_string();
        let path = [PathComponent::Index(1)];
        let mut remaining = 4_usize;
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
        assert_eq!(
            replace_or_create_bounded(&root, &path, number(4), VmLimits::default(), &mut charge,),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
        assert_eq!(root.to_string(), original);
    }

    #[test]
    fn charge_can_fail_during_synthesized_growth_without_mutating_root() {
        let root = Value::Null;
        let path = [PathComponent::Index(2)];
        let mut remaining = 4_usize;
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
        assert_eq!(
            replace_or_create_bounded(&root, &path, number(4), VmLimits::default(), &mut charge,),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
        assert_eq!(root, Value::Null);
    }

    #[test]
    fn indexed_overflow_is_rejected_without_allocation() {
        let root = Value::Null;
        let path = [PathComponent::Index(usize::MAX)];
        let mut charge = || Ok(());
        assert_eq!(
            replace_or_create_bounded(&root, &path, number(1), VmLimits::default(), &mut charge,),
            Err(VmError::Resource {
                resource: "output-bytes"
            })
        );
    }

    #[test]
    fn object_order_and_unaffected_value_sharing_are_preserved() {
        let untouched = Value::array(vec![number(2)]);
        let root = Value::object(IndexMap::from([
            (Arc::from("a"), Value::array(vec![number(1)])),
            (Arc::from("b"), untouched.clone()),
        ]));
        let path = [PathComponent::Key(Arc::from("a")), PathComponent::Index(0)];
        let mut charge = || Ok(());
        let output =
            replace_or_create_bounded(&root, &path, number(3), VmLimits::default(), &mut charge)
                .unwrap();
        assert_eq!(output.to_string(), r#"{"a":[3],"b":[2]}"#);
        let Value::Object(values) = output else {
            panic!("replacement should preserve object shape");
        };
        let Value::Array(output_untouched) = values.get("b").expect("b key") else {
            panic!("b should remain an array");
        };
        let Value::Object(input_values) = &root else {
            panic!("input should remain an object");
        };
        let Value::Array(input_untouched) = input_values.get("b").expect("b key") else {
            panic!("b should remain an array");
        };
        assert!(Arc::ptr_eq(output_untouched, input_untouched));
        assert_eq!(
            values.keys().map(AsRef::as_ref).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(input_untouched.len(), 1);
    }

    #[test]
    fn sequential_indexed_growth_obeys_total_container_bound() {
        let mut state = crate::eval::generator::FromStreamState::new();
        let limits = VmLimits {
            output_bytes: 32,
            ..VmLimits::default()
        };
        let mut charge = || Ok(());
        let mut error = None;
        for index in 0..40 {
            let event = Value::array(vec![Value::array(vec![number(index)]), number(index)]);
            if let Err(result) = state.feed(&event, limits, &mut charge) {
                error = Some(result);
                break;
            }
        }
        assert_eq!(
            error,
            Some(VmError::Resource {
                resource: "output-bytes"
            })
        );
    }

    #[test]
    fn sequential_object_growth_obeys_total_container_bound() {
        let mut state = crate::eval::generator::FromStreamState::new();
        let limits = VmLimits {
            output_bytes: 32,
            ..VmLimits::default()
        };
        let mut charge = || Ok(());
        let mut error = None;
        for index in 0..40 {
            let event = Value::array(vec![
                Value::array(vec![Value::string(format!("k{index}"))]),
                number(index),
            ]);
            if let Err(result) = state.feed(&event, limits, &mut charge) {
                error = Some(result);
                break;
            }
        }
        assert_eq!(
            error,
            Some(VmError::Resource {
                resource: "output-bytes"
            })
        );
    }

    #[test]
    fn bounded_delete_handles_empty_and_missing_paths() {
        let root = Value::object(IndexMap::from([
            (Arc::from("a"), number(1)),
            (Arc::from("b"), number(2)),
        ]));
        let mut charge = || Ok(());
        assert_eq!(
            delete_path_bounded(&root, &[], VmLimits::default(), &mut charge).unwrap(),
            Value::Null
        );

        let missing = [PathComponent::Key(Arc::from("missing"))];
        let output =
            delete_path_bounded(&root, &missing, VmLimits::default(), &mut charge).unwrap();
        assert_eq!(output.to_string(), root.to_string());
    }

    #[test]
    fn bounded_delete_shifts_arrays_and_preserves_object_order() {
        let root = Value::object(IndexMap::from([
            (
                Arc::from("a"),
                Value::array(vec![number(1), number(2), number(3)]),
            ),
            (Arc::from("b"), number(4)),
        ]));
        let path = [PathComponent::Key(Arc::from("a")), PathComponent::Index(1)];
        let mut charge = || Ok(());
        let output = delete_path_bounded(&root, &path, VmLimits::default(), &mut charge).unwrap();
        assert_eq!(output.to_string(), r#"{"a":[1,3],"b":4}"#);

        let path = [PathComponent::Key(Arc::from("a"))];
        let output = delete_path_bounded(&root, &path, VmLimits::default(), &mut charge).unwrap();
        assert_eq!(output.to_string(), r#"{"b":4}"#);
    }

    #[test]
    fn bounded_delete_reports_a_mid_copy_budget_failure() {
        let root = Value::object(IndexMap::from([
            (Arc::from("a"), number(1)),
            (Arc::from("b"), number(2)),
            (Arc::from("c"), number(3)),
        ]));
        let original = root.to_string();
        let path = [PathComponent::Key(Arc::from("b"))];
        let mut remaining = 3_usize;
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
        assert_eq!(
            delete_path_bounded(&root, &path, VmLimits::default(), &mut charge),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
        assert_eq!(root.to_string(), original);
    }

    #[test]
    fn bounded_slice_supports_insertion_and_replacement() {
        let root = Value::array(vec![number(1), number(2), number(3)]);
        let mut charge = || Ok(());
        let inserted = replace_slice_bounded(
            &root,
            &[],
            1,
            1,
            Value::array(vec![number(8), number(9)]),
            VmLimits::default(),
            &mut charge,
        )
        .unwrap();
        assert_eq!(inserted.to_string(), "[1,8,9,2,3]");

        let path = [PathComponent::Key(Arc::from("values"))];
        let root = Value::object(IndexMap::from([(
            Arc::from("values"),
            Value::array(vec![number(1), number(2), number(3)]),
        )]));
        let replaced = replace_slice_bounded(
            &root,
            &path,
            1,
            2,
            Value::array(vec![number(7)]),
            VmLimits::default(),
            &mut charge,
        )
        .unwrap();
        assert_eq!(replaced.to_string(), r#"{"values":[1,7,3]}"#);
    }

    #[test]
    fn bounded_slice_checks_budget_during_copy() {
        let root = Value::array(vec![number(1), number(2), number(3)]);
        let original = root.to_string();
        let mut remaining = 4_usize;
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
        assert_eq!(
            replace_slice_bounded(
                &root,
                &[],
                1,
                2,
                Value::array(vec![number(7)]),
                VmLimits::default(),
                &mut charge,
            ),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );
        assert_eq!(root.to_string(), original);
    }

    #[test]
    fn bounded_slice_handles_extreme_indices_without_overflow() {
        let root = Value::array(vec![number(1)]);
        let mut charge = || Ok(());
        let output = replace_slice_bounded(
            &root,
            &[],
            usize::MAX,
            usize::MAX,
            Value::array(vec![number(2)]),
            VmLimits::default(),
            &mut charge,
        )
        .unwrap();
        assert_eq!(output.to_string(), "[1,2]");
    }

    #[test]
    fn bounded_slice_rejects_non_array_replacements() {
        let root = Value::array(vec![number(1), number(2)]);
        for replacement in [
            number(7),
            Value::Null,
            Value::object(IndexMap::from([(Arc::from("x"), number(1))])),
        ] {
            let mut charge = || Ok(());
            let error = replace_slice_bounded(
                &root,
                &[],
                0,
                1,
                replacement,
                VmLimits::default(),
                &mut charge,
            )
            .expect_err("slice assignment must require an array");
            assert!(matches!(
                error,
                VmError::Runtime { message }
                    if message.as_ref() == "A slice of an array can only be assigned another array"
            ));
        }
    }
}
