//! Pull-based iteration over one immutable JSON container.

use crate::{PathComponent, Value, VmError};

/// Cursor over an array or insertion-ordered object without materializing its
/// children in a second collection.
///
/// The evaluator owns the source value, so cloning a returned child only
/// clones the value's immutable handle. Each successful child pull invokes the
/// caller's charge callback exactly once; reaching the end or asking to
/// iterate a scalar does not consume a charge.
#[derive(Clone, Debug)]
pub(super) struct ContainerCursor {
    source: Value,
    next: Option<usize>,
}

impl ContainerCursor {
    /// Creates a cursor positioned before the first child.
    pub(super) fn new(source: Value) -> Self {
        Self {
            source,
            next: Some(0),
        }
    }

    /// Whether this source is a container accepted by jq iteration.
    #[must_use]
    pub(super) fn is_container(&self) -> bool {
        matches!(self.source, Value::Array(_) | Value::Object(_))
    }

    /// Pulls the next child and its path component.
    ///
    /// The charge callback is called only when a child is available, immediately
    /// before that child is consumed. If charging fails, the cursor remains at
    /// the same child so the caller does not lose a value while propagating the
    /// resource or cancellation error.
    pub(super) fn next_child(
        &mut self,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<(Value, PathComponent)>, VmError> {
        let Some(next) = self.next else {
            return Ok(None);
        };
        let child = match &self.source {
            Value::Array(values) => values
                .get(next)
                .cloned()
                .map(|value| (value, PathComponent::Index(next))),
            Value::Object(values) => values
                .get_index(next)
                .map(|(key, value)| (value.clone(), PathComponent::Key(key.clone()))),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
        };
        let Some(child) = child else {
            self.next = None;
            return Ok(None);
        };

        charge()?;
        self.next = next.checked_add(1);
        Ok(Some(child))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use indexmap::IndexMap;

    use super::ContainerCursor;
    use crate::{PathComponent, Value, VmError};

    fn number(value: i64) -> Value {
        Value::from_json(serde_json::json!(value)).expect("number value")
    }

    #[test]
    fn pulls_array_children_in_order_with_index_components() {
        let mut cursor = ContainerCursor::new(Value::array([number(1), number(2)]));
        let mut charges = 0;
        let mut charge = || {
            charges += 1;
            Ok(())
        };

        assert_eq!(
            cursor.next_child(&mut charge).expect("first pull"),
            Some((number(1), PathComponent::Index(0)))
        );
        assert_eq!(
            cursor.next_child(&mut charge).expect("second pull"),
            Some((number(2), PathComponent::Index(1)))
        );
        assert_eq!(cursor.next_child(&mut charge).expect("end pull"), None);
        assert_eq!(charges, 2);
    }

    #[test]
    fn pulls_object_children_in_insertion_order_without_values_collection() {
        let source = Value::object(IndexMap::from([
            (Arc::from("first"), number(1)),
            (Arc::from("second"), number(2)),
        ]));
        let mut cursor = ContainerCursor::new(source);
        let mut charges = 0;
        let mut charge = || {
            charges += 1;
            Ok(())
        };

        assert_eq!(
            cursor.next_child(&mut charge).expect("first pull"),
            Some((number(1), PathComponent::Key(Arc::from("first")),))
        );
        assert_eq!(
            cursor.next_child(&mut charge).expect("second pull"),
            Some((number(2), PathComponent::Key(Arc::from("second")),))
        );
        assert_eq!(cursor.next_child(&mut charge).expect("end pull"), None);
        assert_eq!(charges, 2);
    }

    #[test]
    fn scalar_is_not_a_container_and_does_not_charge() {
        let mut cursor = ContainerCursor::new(number(1));
        assert!(!cursor.is_container());
        let mut charges = 0;
        let mut charge = || {
            charges += 1;
            Ok(())
        };
        assert_eq!(cursor.next_child(&mut charge).expect("scalar pull"), None);
        assert_eq!(charges, 0);
    }

    #[test]
    fn failed_charge_does_not_consume_the_child() {
        let mut cursor = ContainerCursor::new(Value::array([number(1)]));
        let mut charge = || {
            Err(VmError::Resource {
                resource: "vm-steps",
            })
        };
        assert_eq!(
            cursor.next_child(&mut charge),
            Err(VmError::Resource {
                resource: "vm-steps"
            })
        );

        let mut charges = 0;
        let mut charge = || {
            charges += 1;
            Ok(())
        };
        assert_eq!(
            cursor.next_child(&mut charge).expect("retry pull"),
            Some((number(1), PathComponent::Index(0)))
        );
        assert_eq!(charges, 1);
    }
}
