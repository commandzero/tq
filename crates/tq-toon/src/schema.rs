//! Ordered recursive field groups shared by document writing and prepared replay.

use std::{mem, sync::Arc};

use tq_core::{Object, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RowSchema {
    pub(crate) fields: Vec<FieldSchema>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldSchema {
    pub(crate) key: Arc<str>,
    pub(crate) children: Option<RowSchema>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SchemaLimits {
    pub(crate) depth: usize,
    pub(crate) fields: usize,
    pub(crate) bytes: usize,
}

impl Default for SchemaLimits {
    fn default() -> Self {
        Self {
            depth: 256,
            fields: 65_536,
            bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchemaError {
    Depth(usize),
    Fields(usize),
    Bytes(usize),
}

impl RowSchema {
    /// Ineligible shapes return `None`; resource exhaustion is never ineligibility.
    /// Keys share the source object's Arc allocation; no values are retained.
    pub(crate) fn from_object(
        object: &Object,
        limits: SchemaLimits,
    ) -> Result<Option<Self>, SchemaError> {
        let mut fields = 0;
        let mut bytes = mem::size_of::<Self>();
        if bytes > limits.bytes {
            return Err(SchemaError::Bytes(limits.bytes));
        }
        Self::build(object, 1, limits, &mut fields, &mut bytes)
    }

    fn build(
        object: &Object,
        depth: usize,
        limits: SchemaLimits,
        field_count: &mut usize,
        bytes: &mut usize,
    ) -> Result<Option<Self>, SchemaError> {
        if object.is_empty() {
            return Ok(None);
        }
        if depth > limits.depth {
            return Err(SchemaError::Depth(limits.depth));
        }
        *field_count = field_count
            .checked_add(object.len())
            .filter(|count| *count <= limits.fields)
            .ok_or(SchemaError::Fields(limits.fields))?;
        let field_bytes = object
            .len()
            .checked_mul(mem::size_of::<FieldSchema>())
            .ok_or(SchemaError::Bytes(limits.bytes))?;
        charge_bytes(bytes, field_bytes, limits.bytes)?;
        // Charge every shared key conservatively before allocating the field vector.
        for key in object.keys() {
            charge_bytes(bytes, retained_key_bytes(key), limits.bytes)?;
        }
        let mut fields = Vec::with_capacity(object.len());
        for (key, value) in object {
            let children = match value {
                Value::Object(nested) => {
                    let Some(schema) = Self::build(nested, depth + 1, limits, field_count, bytes)?
                    else {
                        return Ok(None);
                    };
                    Some(schema)
                }
                Value::Array(_) => return Ok(None),
                _ => None,
            };
            fields.push(FieldSchema {
                key: Arc::clone(key),
                children,
            });
        }
        Ok(Some(Self { fields }))
    }

    /// Compare key sets recursively, not later objects' encounter order.
    pub(crate) fn matches_value(&self, value: &Value) -> bool {
        let Value::Object(object) = value else {
            return false;
        };
        self.matches_object(object)
    }

    pub(crate) fn matches_object(&self, object: &Object) -> bool {
        object.len() == self.fields.len()
            && self.fields.iter().all(|field| {
                let Some(value) = object.get(&field.key) else {
                    return false;
                };
                match &field.children {
                    Some(children) => children.matches_value(value),
                    None => !matches!(value, Value::Array(_) | Value::Object(_)),
                }
            })
    }
}

fn charge_bytes(bytes: &mut usize, additional: usize, maximum: usize) -> Result<(), SchemaError> {
    *bytes = bytes
        .checked_add(additional)
        .filter(|bytes| *bytes <= maximum)
        .ok_or(SchemaError::Bytes(maximum))?;
    Ok(())
}

pub(crate) fn retained_key_bytes(key: &str) -> usize {
    let alignment = mem::align_of::<usize>();
    key.len()
        .saturating_add(alignment - 1)
        .saturating_div(alignment)
        .saturating_mul(alignment)
        .saturating_add(2 * mem::size_of::<usize>())
}

#[cfg(test)]
mod tests {
    use tq_core::Value;

    use super::{RowSchema, SchemaError, SchemaLimits};

    fn value(json: &str) -> Value {
        Value::from_json(serde_json::from_str(json).unwrap()).unwrap()
    }

    #[test]
    fn schemas_match_recursive_key_sets_not_later_order() {
        let Value::Object(first) = value(r#"{"id":1,"profile":{"name":"Ada","city":"Paris"}}"#)
        else {
            unreachable!()
        };
        let schema = RowSchema::from_object(&first, SchemaLimits::default())
            .unwrap()
            .unwrap();

        assert!(schema.matches_value(&value(r#"{"profile":{"city":"Rome","name":"Bob"},"id":2}"#)));
        assert!(!schema.matches_value(&value(r#"{"profile":{"city":"Rome"},"id":2}"#)));
        assert!(!schema.matches_value(&value(
            r#"{"profile":{"city":"Rome","name":"Bob","extra":true},"id":2}"#
        )));
        assert!(!schema.matches_value(&value(r#"{"profile":[],"id":2}"#)));
    }

    #[test]
    fn schemas_reject_empty_nested_objects_and_enforce_budgets() {
        let Value::Object(empty_nested) = value(r#"{"child":{}}"#) else {
            unreachable!()
        };
        assert!(
            RowSchema::from_object(&empty_nested, SchemaLimits::default())
                .unwrap()
                .is_none()
        );

        let Value::Object(object) = value(r#"{"id":1}"#) else {
            unreachable!()
        };
        assert_eq!(
            RowSchema::from_object(
                &object,
                SchemaLimits {
                    bytes: 1,
                    ..SchemaLimits::default()
                }
            ),
            Err(SchemaError::Bytes(1))
        );
        assert_eq!(
            RowSchema::from_object(
                &object,
                SchemaLimits {
                    fields: 0,
                    ..SchemaLimits::default()
                }
            ),
            Err(SchemaError::Fields(0))
        );
    }
}
