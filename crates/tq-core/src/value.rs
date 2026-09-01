//! Immutable JSON-shaped runtime values with structural sharing.

use std::{borrow::Borrow, cmp::Ordering, fmt, hash::Hash, ops::Index, sync::Arc};

use indexmap::IndexMap;
use serde::{
    Deserialize, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
};
use smallvec::SmallVec;

use crate::{Number, NumberError};

const INLINE_OBJECT_CAPACITY: usize = 3;

/// Insertion-ordered immutable object storage.
#[derive(Clone, Debug)]
pub struct Object {
    storage: ObjectStorage,
}

#[derive(Clone, Debug)]
enum ObjectStorage {
    Small(SmallVec<[(Arc<str>, Value); INLINE_OBJECT_CAPACITY]>),
    Large(IndexMap<Arc<str>, Value>),
}

impl Object {
    /// Creates an empty object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            storage: ObjectStorage::Small(SmallVec::new()),
        }
    }

    /// Creates an empty object sized for at least `capacity` members.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= INLINE_OBJECT_CAPACITY {
            Self::new()
        } else {
            Self {
                storage: ObjectStorage::Large(IndexMap::with_capacity(capacity)),
            }
        }
    }

    /// Returns the number of members.
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.storage {
            ObjectStorage::Small(values) => values.len(),
            ObjectStorage::Large(values) => values.len(),
        }
    }

    /// Returns true when the object has no members.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the value for `key`.
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        Arc<str>: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match &self.storage {
            ObjectStorage::Small(values) => values
                .iter()
                .find(|(candidate, _)| candidate.borrow() == key)
                .map(|(_, value)| value),
            ObjectStorage::Large(values) => values.get(key),
        }
    }

    /// Returns the mutable value for `key`.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
    where
        Arc<str>: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match &mut self.storage {
            ObjectStorage::Small(values) => values
                .iter_mut()
                .find(|(candidate, _)| candidate.borrow() == key)
                .map(|(_, value)| value),
            ObjectStorage::Large(values) => values.get_mut(key),
        }
    }

    /// Returns true when `key` is present.
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Arc<str>: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Inserts a member, retaining the first encounter position on replacement.
    pub fn insert(&mut self, key: Arc<str>, value: Value) -> Option<Value> {
        if let ObjectStorage::Small(values) = &mut self.storage {
            if let Some((_, current)) = values.iter_mut().find(|(candidate, _)| candidate == &key) {
                return Some(std::mem::replace(current, value));
            }
            if values.len() < INLINE_OBJECT_CAPACITY {
                values.push((key, value));
                return None;
            }
        }

        if matches!(self.storage, ObjectStorage::Small(_)) {
            let ObjectStorage::Small(values) = std::mem::replace(
                &mut self.storage,
                ObjectStorage::Large(IndexMap::with_capacity(INLINE_OBJECT_CAPACITY + 1)),
            ) else {
                unreachable!("small object selected above")
            };
            let ObjectStorage::Large(large) = &mut self.storage else {
                unreachable!("object was promoted")
            };
            large.extend(values);
        }
        let ObjectStorage::Large(values) = &mut self.storage else {
            unreachable!("full small object was promoted")
        };
        values.insert(key, value)
    }

    /// Returns the member at its encounter-order index.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<(&Arc<str>, &Value)> {
        match &self.storage {
            ObjectStorage::Small(values) => values.get(index).map(|(key, value)| (key, value)),
            ObjectStorage::Large(values) => values.get_index(index),
        }
    }

    /// Returns the first member in encounter order.
    #[must_use]
    pub fn first(&self) -> Option<(&Arc<str>, &Value)> {
        self.get_index(0)
    }

    /// Iterates members in encounter order.
    pub fn iter(&self) -> ObjectIter<'_> {
        match &self.storage {
            ObjectStorage::Small(values) => ObjectIter {
                inner: ObjectIterInner::Small(values.iter()),
            },
            ObjectStorage::Large(values) => ObjectIter {
                inner: ObjectIterInner::Large(values.iter()),
            },
        }
    }

    /// Iterates keys in encounter order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &Arc<str>> + ExactSizeIterator {
        self.iter().map(|(key, _)| key)
    }

    /// Iterates values in encounter order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Value> + ExactSizeIterator {
        self.iter().map(|(_, value)| value)
    }
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<(Arc<str>, Value)> for Object {
    fn from_iter<T: IntoIterator<Item = (Arc<str>, Value)>>(iter: T) -> Self {
        let iterator = iter.into_iter();
        let mut object = Self::with_capacity(iterator.size_hint().0);
        for (key, value) in iterator {
            object.insert(key, value);
        }
        object
    }
}

impl From<IndexMap<Arc<str>, Value>> for Object {
    fn from(values: IndexMap<Arc<str>, Value>) -> Self {
        if values.len() <= INLINE_OBJECT_CAPACITY {
            Self {
                storage: ObjectStorage::Small(values.into_iter().collect()),
            }
        } else {
            Self {
                storage: ObjectStorage::Large(values),
            }
        }
    }
}

impl<Q> Index<&Q> for Object
where
    Arc<str>: Borrow<Q>,
    Q: Eq + Hash + ?Sized,
{
    type Output = Value;

    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).expect("object key is absent")
    }
}

/// Borrowed encounter-order object iterator.
pub struct ObjectIter<'a> {
    inner: ObjectIterInner<'a>,
}

enum ObjectIterInner<'a> {
    Small(std::slice::Iter<'a, (Arc<str>, Value)>),
    Large(indexmap::map::Iter<'a, Arc<str>, Value>),
}

impl<'a> Iterator for ObjectIter<'a> {
    type Item = (&'a Arc<str>, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            ObjectIterInner::Small(values) => values.next().map(|(key, value)| (key, value)),
            ObjectIterInner::Large(values) => values.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            ObjectIterInner::Small(values) => values.size_hint(),
            ObjectIterInner::Large(values) => values.size_hint(),
        }
    }
}

impl DoubleEndedIterator for ObjectIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            ObjectIterInner::Small(values) => values.next_back().map(|(key, value)| (key, value)),
            ObjectIterInner::Large(values) => values.next_back(),
        }
    }
}

impl ExactSizeIterator for ObjectIter<'_> {}

impl<'a> IntoIterator for &'a Object {
    type Item = (&'a Arc<str>, &'a Value);
    type IntoIter = ObjectIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

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
    pub fn object(values: impl Into<Object>) -> Self {
        Self::Object(Arc::new(values.into()))
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
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => value.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values.iter() {
                    sequence.serialize_element(value)?;
                }
                sequence.end()
            }
            Self::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                match &values.storage {
                    ObjectStorage::Small(values) => {
                        for (key, value) in values {
                            map.serialize_entry(key.as_ref(), value)?;
                        }
                    }
                    ObjectStorage::Large(values) => {
                        for (key, value) in values {
                            map.serialize_entry(key.as_ref(), value)?;
                        }
                    }
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

const SERDE_JSON_NUMBER_TOKEN: &str = "$serde_json::private::Number";

struct ValueVisitor;

impl ValueVisitor {
    fn number<E: de::Error>(source: &str) -> Result<Value, E> {
        Number::parse(source).map(Value::Number).map_err(E::custom)
    }
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON-shaped value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Self::number(&value.to_string())
    }

    fn visit_i128<E: de::Error>(self, value: i128) -> Result<Self::Value, E> {
        Self::number(&value.to_string())
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Self::number(&value.to_string())
    }

    fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
        Self::number(&value.to_string())
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .map_err(E::custom)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::string(value))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::string(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(Value::array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let Some(first_key) = map.next_key::<String>()? else {
            return Ok(Value::object(Object::new()));
        };
        let first_value = map.next_value::<Value>()?;
        let second_key = map.next_key::<String>()?;
        if first_key == SERDE_JSON_NUMBER_TOKEN
            && second_key.is_none()
            && let Value::String(literal) = &first_value
        {
            match Number::parse(literal) {
                Ok(number) => return Ok(Value::Number(number)),
                Err(NumberError::Invalid) => {}
                Err(error) => return Err(de::Error::custom(error)),
            }
        }

        let mut values = Object::with_capacity(map.size_hint().unwrap_or(0).saturating_add(2));
        values.insert(Arc::from(first_key), first_value);
        if let Some(key) = second_key {
            values.insert(Arc::from(key), map.next_value()?);
        }
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            values.insert(Arc::from(key), value);
        }
        Ok(Value::object(values))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use indexmap::IndexMap;

    use super::{Object, ObjectStorage, Value, ValueKind};
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
    fn small_objects_promote_without_changing_encounter_order() {
        let mut object = Object::new();
        for (key, value) in [("a", false), ("b", true), ("c", false)] {
            object.insert(Arc::from(key), Value::Bool(value));
        }
        assert!(matches!(&object.storage, ObjectStorage::Small(_)));

        object.insert(Arc::from("b"), Value::Null);
        assert_eq!(
            object.keys().map(AsRef::as_ref).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert_eq!(object["b"], Value::Null);

        object.insert(Arc::from("d"), Value::Bool(true));
        assert!(matches!(&object.storage, ObjectStorage::Large(_)));
        assert_eq!(
            object.keys().map(AsRef::as_ref).collect::<Vec<_>>(),
            ["a", "b", "c", "d"]
        );
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
    fn number_envelope_falls_back_for_non_number_objects() {
        let value: Value =
            serde_json::from_str(r#"{"$serde_json::private::Number":"not-a-number"}"#).unwrap();
        assert_eq!(
            value.to_string(),
            r#"{"$serde_json::private::Number":"not-a-number"}"#
        );

        let value: Value =
            serde_json::from_str(r#"{"$serde_json::private::Number":"123","other":true}"#).unwrap();
        assert_eq!(
            value.to_string(),
            r#"{"$serde_json::private::Number":"123","other":true}"#
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

    #[test]
    fn direct_serde_round_trip_preserves_order_and_exact_numbers() {
        let source = r#"{"z":9007199254740993,"a":[1e1000000,-0.0,"x"]}"#;
        let value: Value = serde_json::from_str(source).unwrap();
        let Value::Object(object) = &value else {
            panic!("expected object")
        };
        assert_eq!(
            object.keys().map(AsRef::as_ref).collect::<Vec<_>>(),
            ["z", "a"]
        );
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#"{"z":9007199254740993,"a":[1e+1000000,0,"x"]}"#
        );

        let oversized = "1".repeat(4097);
        assert!(serde_json::from_str::<Value>(&oversized).is_err());
    }

    #[test]
    fn direct_deserializer_keeps_first_position_and_last_duplicate_value() {
        let value: Value = serde_json::from_str(r#"{"b":1,"a":2,"b":3}"#).unwrap();
        let Value::Object(object) = value else {
            panic!("expected object")
        };
        assert_eq!(
            object.keys().map(AsRef::as_ref).collect::<Vec<_>>(),
            ["b", "a"]
        );
        assert_eq!(object["b"], Value::Number(Number::parse("3").unwrap()));
    }
}
