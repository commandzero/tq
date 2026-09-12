//! Bounded path-result state shared by callback-driven path built-ins.
//!
//! This module deliberately does not evaluate filters. The managed generator
//! evaluates a callback and submits only its selected path components here;
//! these helpers own retained-path admission and persistent reconstruction.

use crate::{Path, PathComponent, Value, VmError, VmLimits};

use super::{compare_paths_for_deletion, path, resource};

/// Bounded retained path results produced by a callback path expression.
pub(super) struct PathAccumulator {
    paths: Vec<Path>,
    retained_lower_bound: usize,
}

impl PathAccumulator {
    /// Starts an empty retained path set.
    pub(super) const fn new() -> Self {
        Self {
            paths: Vec::new(),
            retained_lower_bound: 0,
        }
    }

    /// Retains one callback-selected path after bounded admission.
    ///
    /// Components are copied only after the depth and retained-result checks;
    /// both the path storage and its component storage use fallible reserve.
    /// The caller supplies the charge so managed cancellation and step limits
    /// remain owned by the generator.
    pub(super) fn push(
        &mut self,
        components: &[PathComponent],
        limits: VmLimits,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<(), VmError> {
        if components.len() > limits.path_stack {
            return Err(resource("path-stack"));
        }
        // This is deliberately a lower bound rather than an allocator-size
        // estimate: one unit accounts for the path itself and one for each
        // retained component.  It bounds aggregate materialization without
        // confusing callback result width with pending VM forks.
        let lower_bound = components
            .len()
            .checked_add(1)
            .ok_or_else(|| resource("output-bytes"))?;
        let retained = self
            .retained_lower_bound
            .checked_add(lower_bound)
            .ok_or_else(|| resource("output-bytes"))?;
        if retained > limits.output_bytes {
            return Err(resource("output-bytes"));
        }
        charge()?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(components.len())
            .map_err(|_| resource("output-bytes"))?;
        for component in components {
            charge()?;
            owned.push(component.clone());
        }
        self.paths
            .try_reserve(1)
            .map_err(|_| resource("output-bytes"))?;
        self.paths.push(Path::new(owned));
        self.retained_lower_bound = retained;
        Ok(())
    }

    /// Returns callback paths in their original order, consuming the state.
    pub(super) fn into_paths(self) -> Vec<Path> {
        self.paths
    }

    /// Returns paths in jq's deletion order, removing exact duplicates.
    pub(super) fn into_delete_paths(
        mut self,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Vec<Path>, VmError> {
        fallible_sort_paths(&mut self.paths, charge)?;

        let mut retained = 0;
        for examined in 0..self.paths.len() {
            let duplicate = if retained == 0 {
                false
            } else {
                charge()?;
                self.paths[retained - 1] == self.paths[examined]
            };
            if !duplicate {
                if retained != examined {
                    self.paths.swap(retained, examined);
                }
                retained += 1;
            }
        }
        self.paths.truncate(retained);
        Ok(self.paths)
    }
}

/// Sorts paths without allocation while allowing every comparison to fail.
///
/// `slice::sort*` cannot propagate a fallible comparator and would require a
/// sentinel ordering after cancellation.  That violates the comparator's
/// total-order contract, so deletion uses a bounded in-place heapsort.
fn fallible_sort_paths(
    paths: &mut [Path],
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    let len = paths.len();
    if len < 2 {
        return Ok(());
    }

    for start in (0..(len / 2)).rev() {
        sift_down(paths, start, len, charge)?;
    }
    for end in (1..len).rev() {
        paths.swap(0, end);
        sift_down(paths, 0, end, charge)?;
    }
    Ok(())
}

fn sift_down(
    paths: &mut [Path],
    start: usize,
    end: usize,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    let mut root = start;
    loop {
        let child = root
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| resource("output-bytes"))?;
        if child >= end {
            return Ok(());
        }
        let mut candidate = child;
        if child + 1 < end {
            charge()?;
            if compare_paths_for_deletion(&paths[candidate], &paths[child + 1])
                == std::cmp::Ordering::Less
            {
                candidate = child + 1;
            }
        }
        charge()?;
        if compare_paths_for_deletion(&paths[root], &paths[candidate]) != std::cmp::Ordering::Less {
            return Ok(());
        }
        paths.swap(root, candidate);
        root = candidate;
    }
}

/// Deletes callback-selected paths in jq's descending-index order.
pub(super) fn delete_paths_bounded(
    root: &Value,
    paths: PathAccumulator,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let paths = paths.into_delete_paths(charge)?;
    for path in &paths {
        if path.components().len() > limits.path_stack {
            return Err(resource("path-stack"));
        }
    }
    let mut deleted = root.clone();
    for path in paths {
        deleted = path::delete_path_bounded(&deleted, path.components(), limits, charge)?;
    }
    Ok(deleted)
}

/// Builds `pick` output in callback order.
///
/// The bounded path read supplies `null` for missing keys or indices, matching
/// jq's `pick` behavior before the bounded persistent update creates the
/// selected shape in the result.  Type mismatches remain errors.
pub(super) fn pick_paths_bounded(
    input: &Value,
    paths: PathAccumulator,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let mut picked = Value::Null;
    for path in paths.into_paths() {
        let value = path::getpath_bounded(input, path.components(), limits, charge)?;
        picked =
            path::replace_or_create_bounded(&picked, path.components(), value, limits, charge)?;
    }
    Ok(picked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: usize) -> Value {
        Value::Number(crate::Number::parse(&value.to_string()).expect("fixture integer"))
    }

    fn index_path(index: usize) -> Vec<PathComponent> {
        vec![PathComponent::Key("a".into()), PathComponent::Index(index)]
    }

    #[test]
    fn accumulator_rejects_depth_before_copy_or_charge() {
        let mut accumulator = PathAccumulator::new();
        let mut charges = 0;
        let mut charge = || {
            charges += 1;
            Ok(())
        };
        let components = index_path(0);
        assert_eq!(
            accumulator.push(
                &components,
                VmLimits {
                    path_stack: 1,
                    ..VmLimits::default()
                },
                &mut charge,
            ),
            Err(VmError::Resource {
                resource: "path-stack"
            })
        );
        assert_eq!(charges, 0);
        assert!(accumulator.into_paths().is_empty());
    }

    #[test]
    fn accumulator_mid_copy_failure_does_not_retain_or_charge_path_budget() {
        let mut accumulator = PathAccumulator::new();
        let components = index_path(0);
        let limits = VmLimits {
            output_bytes: 3,
            ..VmLimits::default()
        };
        let mut calls = 0;
        let mut failing_charge = || {
            calls += 1;
            if calls == 2 {
                Err(VmError::Resource { resource: "steps" })
            } else {
                Ok(())
            }
        };
        assert_eq!(
            accumulator.push(&components, limits, &mut failing_charge),
            Err(VmError::Resource { resource: "steps" })
        );
        assert!(accumulator.paths.is_empty());
        assert_eq!(accumulator.retained_lower_bound, 0);

        let mut successful_charge = || Ok(());
        accumulator
            .push(&components, limits, &mut successful_charge)
            .unwrap();
    }

    #[test]
    fn accumulator_charges_and_preserves_callback_order() {
        let mut accumulator = PathAccumulator::new();
        let mut charges = 0;
        let mut charge = || {
            charges += 1;
            Ok(())
        };
        let first = index_path(1);
        let second = index_path(0);
        accumulator
            .push(&first, VmLimits::default(), &mut charge)
            .unwrap();
        accumulator
            .push(&second, VmLimits::default(), &mut charge)
            .unwrap();
        assert_eq!(charges, 6);
        let paths = accumulator.into_paths();
        assert_eq!(paths[0].components(), first.as_slice());
        assert_eq!(paths[1].components(), second.as_slice());
    }

    #[test]
    fn delete_paths_dedupes_and_removes_higher_indices_first() {
        let root = Value::object(indexmap::IndexMap::from([(
            "a".into(),
            Value::array([number(0), number(1), number(2)]),
        )]));
        let mut paths = PathAccumulator::new();
        let mut charge = || Ok(());
        for index in [0, 2, 2, 1] {
            paths
                .push(&index_path(index), VmLimits::default(), &mut charge)
                .unwrap();
        }
        assert_eq!(
            delete_paths_bounded(&root, paths, VmLimits::default(), &mut charge)
                .unwrap()
                .to_string(),
            r#"{"a":[]}"#
        );
    }

    #[test]
    fn delete_sort_work_propagates_charge_failure_without_panicking() {
        let root = Value::object(indexmap::IndexMap::new());
        for fail_after in [1, 2, 5, 10] {
            let mut paths = PathAccumulator::new();
            let mut admission_charge = || Ok(());
            for index in (0..32).rev().chain((0..32).rev()) {
                paths
                    .push(
                        &[PathComponent::Key("a".into()), PathComponent::Index(index)],
                        VmLimits::default(),
                        &mut admission_charge,
                    )
                    .unwrap();
            }
            let mut charges = 0;
            let mut charge = || {
                charges += 1;
                if charges == fail_after {
                    Err(VmError::Resource { resource: "steps" })
                } else {
                    Ok(())
                }
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                delete_paths_bounded(&root, paths, VmLimits::default(), &mut charge)
            }));
            assert!(result.is_ok(), "sort panicked at charge {fail_after}");
            assert_eq!(
                result.unwrap(),
                Err(VmError::Resource { resource: "steps" })
            );
        }
    }

    #[test]
    fn pick_preserves_callback_order_and_missing_values_as_null() {
        let input = Value::object(indexmap::IndexMap::from([(
            "a".into(),
            Value::array([number(10), number(20), number(30)]),
        )]));
        let mut charge = || Ok(());
        let mut paths = PathAccumulator::new();
        for path in [index_path(1), index_path(9)] {
            paths.push(&path, VmLimits::default(), &mut charge).unwrap();
        }
        assert_eq!(
            pick_paths_bounded(&input, paths, VmLimits::default(), &mut charge)
                .unwrap()
                .to_string(),
            r#"{"a":[null,20,null,null,null,null,null,null,null,null]}"#
        );
    }

    #[test]
    fn retained_path_materialization_is_byte_bounded() {
        let mut accumulator = PathAccumulator::new();
        let mut charge = || Ok(());
        let limits = VmLimits {
            output_bytes: 2,
            ..VmLimits::default()
        };
        let path = [PathComponent::Index(0)];
        accumulator.push(&path, limits, &mut charge).unwrap();
        assert_eq!(
            accumulator.push(&path, limits, &mut charge),
            Err(VmError::Resource {
                resource: "output-bytes"
            })
        );
    }

    #[test]
    fn shallow_callback_results_are_not_fork_stack_limited() {
        let mut accumulator = PathAccumulator::new();
        let mut charge = || Ok(());
        for index in 0..=4096 {
            let path = [PathComponent::Index(index)];
            accumulator
                .push(&path, VmLimits::default(), &mut charge)
                .unwrap();
        }
        assert_eq!(accumulator.into_paths().len(), 4097);
    }

    #[test]
    fn pick_missing_key_is_null_and_retains_callback_order() {
        let input = Value::object(indexmap::IndexMap::from([(
            "a".into(),
            Value::object(indexmap::IndexMap::from([("b".into(), number(7))])),
        )]));
        let mut charge = || Ok(());
        let mut paths = PathAccumulator::new();
        let first = [PathComponent::Key("missing".into())];
        let second = [
            PathComponent::Key("a".into()),
            PathComponent::Key("b".into()),
        ];
        paths
            .push(&first, VmLimits::default(), &mut charge)
            .unwrap();
        paths
            .push(&second, VmLimits::default(), &mut charge)
            .unwrap();
        assert_eq!(
            pick_paths_bounded(&input, paths, VmLimits::default(), &mut charge)
                .unwrap()
                .to_string(),
            r#"{"missing":null,"a":{"b":7}}"#
        );
    }

    #[test]
    fn pick_preserves_distinct_object_key_callback_order() {
        let input = Value::object(indexmap::IndexMap::from([
            ("z".into(), number(10)),
            ("a".into(), number(20)),
        ]));
        let mut charge = || Ok(());
        let mut paths = PathAccumulator::new();
        for path in [
            [PathComponent::Key("z".into())],
            [PathComponent::Key("a".into())],
        ] {
            paths.push(&path, VmLimits::default(), &mut charge).unwrap();
        }
        assert_eq!(
            pick_paths_bounded(&input, paths, VmLimits::default(), &mut charge)
                .unwrap()
                .to_string(),
            r#"{"z":10,"a":20}"#
        );
    }
}
