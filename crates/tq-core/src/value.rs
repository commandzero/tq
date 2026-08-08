//! Immutable JSON-shaped runtime values with structural sharing.

use std::{cmp::Ordering, fmt, sync::Arc};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Number, NumberError};

/// Insertion-ordered immutable object storage.
pub type Object = IndexMap<Arc<str>, Value>;

/// Stable runtime type classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueKind {
    /// Null.
    Null,
    /// Boolean.
    Boolean,
    /// Number.
    Number,
    /// String.
    String,
    /// Array.
    Array,
    /// Object.
    Object,
}

/// JSON-shaped immutable runtime value.
#[derive(Clone, Debug)]
pub enum Value {
    /// Null scalar.
    Null,
    /// Boolean scalar.
    Bool(bool),
    /// Hybrid finite number.
    Number(Number),
    /// Shared immutable UTF-8 string.
    String(Arc<str>),
    /// Shared immutable ordered array.
    Array(Arc<[Value]>),
    /// Shared immutable insertion-ordered object.
    Object(Arc<Object>),
}

impl Value {
    /// Creates a shared string value.
    #[must_use]
    pub fn string(value: impl Into<Arc<str>>) -> Self {
        Self::String(value.into())
    }

    /// Creates a shared array value.
    #[must_use]
    pub fn array(values: impl Into<Vec<Self>>) -> Self {
        Self::Array(values.into().into())
    }

    /// Creates a shared insertion-ordered object.
    #[must_use]
    pub fn object(values: Object) -> Self {
        Self::Object(Arc::new(values))
    }

    /// Runtime kind.
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::Null => ValueKind::Null,
            Self::Bool(_) => ValueKind::Boolean,
            Self::Number(_) => ValueKind::Number,
            Self::String(_) => ValueKind::String,
            Self::Array(_) => ValueKind::Array,
            Self::Object(_) => ValueKind::Object,
        }
    }

    /// jq truthiness: only false and null are falsey.
    #[must_use]
    pub const fn is_truthy(&self) -> bool {
        !matches!(self, Self::Null | Self::Bool(false))
    }

    /// Observes whether two array/object/string handles share allocation.
    #[must_use]
    pub fn shares_node_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(left), Self::String(right)) => Arc::ptr_eq(left, right),
            (Self::Array(left), Self::Array(right)) => Arc::ptr_eq(left, right),
            (Self::Object(left), Self::Object(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }

    /// Converts from serde JSON without losing arbitrary-precision literals or order.
    ///
    /// # Errors
    ///
    /// Returns a numeric policy error for an inadmissible JSON number.
    pub fn from_json(value: serde_json::Value) -> Result<Self, NumberError> {
        Ok(match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(value) => Self::Number(Number::parse(&value.to_string())?),
            serde_json::Value::String(value) => Self::string(value),
            serde_json::Value::Array(values) => Self::array(
                values
                    .into_iter()
                    .map(Self::from_json)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            serde_json::Value::Object(values) => {
                let values = values
                    .into_iter()
                    .map(|(key, value)| Ok((Arc::from(key), Self::from_json(value)?)))
                    .collect::<Result<Object, NumberError>>()?;
                Self::object(values)
            }
        })
    }

    /// Converts to serde JSON, retaining exact numeric tokens and encounter order.
    ///
    /// # Errors
    ///
    /// Returns a numeric error if an internal number cannot be serialized.
    pub fn to_json(&self) -> Result<serde_json::Value, NumberError> {
        Ok(match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(*value),
            Self::Number(value) => serde_json::Value::Number(
                value
                    .to_string()
                    .parse()
                    .map_err(|_| NumberError::Invalid)?,
            ),
            Self::String(value) => serde_json::Value::String(value.to_string()),
            Self::Array(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(Self::to_json)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Self::Object(values) => serde_json::Value::Object(
                values
                    .iter()
                    .map(|(key, value)| Ok((key.to_string(), value.to_json()?)))
                    .collect::<Result<serde_json::Map<_, _>, NumberError>>()?,
            ),
        })
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .all(|(key, value)| right.get(key).is_some_and(|right| value == right))
            }
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        let kind = kind_rank(self).cmp(&kind_rank(other));
        if kind != Ordering::Equal {
            return kind;
        }
        match (self, other) {
            (Self::Bool(left), Self::Bool(right)) => left.cmp(right),
            (Self::Number(left), Self::Number(right)) => left.cmp(right),
            (Self::String(left), Self::String(right)) => left.cmp(right),
            (Self::Array(left), Self::Array(right)) => left.cmp(right),
            (Self::Object(left), Self::Object(right)) => compare_objects(left, right),
            _ => Ordering::Equal,
        }
    }
}

fn kind_rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(false) => 1,
        Value::Bool(true) => 2,
        Value::Number(_) => 3,
        Value::String(_) => 4,
        Value::Array(_) => 5,
        Value::Object(_) => 6,
    }
}

fn compare_objects(left: &Object, right: &Object) -> Ordering {
    let mut left_keys = left.keys().collect::<Vec<_>>();
    let mut right_keys = right.keys().collect::<Vec<_>>();
    left_keys.sort_unstable();
    right_keys.sort_unstable();
    left_keys.cmp(&right_keys).then_with(|| {
        left_keys
            .into_iter()
            .map(|key| &left[key])
            .cmp(right_keys.into_iter().map(|key| &right[key]))
    })
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = self.to_json().map_err(|_| fmt::Error)?;
        write!(formatter, "{json}")
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_json()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::from_json(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use indexmap::IndexMap;

    use super::{Value, ValueKind};
    use crate::Number;

    #[test]
    fn clones_are_shallow_and_equality_ignores_object_encounter_order() {
        let array = Value::array(vec![Value::string("x")]);
        assert!(array.shares_node_with(&array.clone()));

        let left = Value::object(IndexMap::from([
            (Arc::from("a"), Value::Number(Number::parse("1").unwrap())),
            (Arc::from("b"), Value::Bool(true)),
        ]));
        let right = Value::object(IndexMap::from([
            (Arc::from("b"), Value::Bool(true)),
            (Arc::from("a"), Value::Number(Number::parse("1.0").unwrap())),
        ]));
        assert_eq!(left, right);
    }

    #[test]
    fn ordered_json_round_trip_retains_large_literal() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"z":9007199254740993,"a":[false,null,"x"]}"#).unwrap();
        let value = Value::from_json(json).unwrap();
        assert_eq!(
            value.to_string(),
            r#"{"z":9007199254740993,"a":[false,null,"x"]}"#
        );
    }

    #[test]
    fn every_primitive_has_stable_kind_truthiness_and_display() {
        let cases = [
            (Value::Null, ValueKind::Null, false, "null"),
            (Value::Bool(false), ValueKind::Boolean, false, "false"),
            (Value::Bool(true), ValueKind::Boolean, true, "true"),
            (
                Value::Number(Number::parse("0").unwrap()),
                ValueKind::Number,
                true,
                "0",
            ),
            (Value::string(""), ValueKind::String, true, "\"\""),
            (Value::array(Vec::new()), ValueKind::Array, true, "[]"),
            (
                Value::object(IndexMap::new()),
                ValueKind::Object,
                true,
                "{}",
            ),
        ];
        for (value, kind, truthy, display) in cases {
            assert_eq!(value.kind(), kind);
            assert_eq!(value.is_truthy(), truthy);
            assert_eq!(value.to_string(), display);
        }
    }

    #[test]
    fn jq_type_order_is_total() {
        let mut values = [
            Value::object(IndexMap::new()),
            Value::array(Vec::new()),
            Value::string("a"),
            Value::Number(Number::parse("2").unwrap()),
            Value::Bool(true),
            Value::Bool(false),
            Value::Null,
        ];
        values.sort();
        assert_eq!(values[0], Value::Null);
        assert_eq!(values[1], Value::Bool(false));
        assert_eq!(values[2], Value::Bool(true));
        assert_eq!(values[3].kind(), ValueKind::Number);
        assert_eq!(values[6].kind(), ValueKind::Object);
    }

    #[test]
    fn serde_conversion_property_holds_across_nested_ordered_values() {
        let atoms = [
            Value::Null,
            Value::Bool(false),
            Value::Bool(true),
            Value::Number(Number::parse("-12.5").unwrap()),
            Value::Number(Number::parse("9007199254740993").unwrap()),
            Value::string("hello\nworld"),
        ];
        for left in &atoms {
            for right in &atoms {
                let value = Value::object(IndexMap::from([
                    (Arc::from("left"), left.clone()),
                    (
                        Arc::from("nested"),
                        Value::array(vec![right.clone(), left.clone()]),
                    ),
                ]));
                let converted = Value::from_json(value.to_json().unwrap()).unwrap();
                assert_eq!(converted, value);
                assert_eq!(converted.to_string(), value.to_string());
            }
        }
    }
}
