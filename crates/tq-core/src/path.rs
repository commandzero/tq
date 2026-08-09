//! Root-anchored jq value paths and persistent updates.

use std::{fmt, sync::Arc};

use indexmap::IndexMap;
use serde::Serialize;
use thiserror::Error;

use crate::Value;

/// One jq path component.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum PathComponent {
    /// Object key.
    Key(Arc<str>),
    /// Non-negative array position.
    Index(usize),
}

/// Root-anchored immutable path.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Path(Arc<[PathComponent]>);

impl Path {
    /// Root path.
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// Creates a path from owned components.
    #[must_use]
    pub fn new(components: impl Into<Vec<PathComponent>>) -> Self {
        Self(components.into().into())
    }

    /// Ordered components.
    #[must_use]
    pub fn components(&self) -> &[PathComponent] {
        &self.0
    }

    /// Returns the value currently addressed by the path.
    #[must_use]
    pub fn get<'a>(&self, root: &'a Value) -> Option<&'a Value> {
        let mut current = root;
        for component in self.components() {
            current = match (component, current) {
                (PathComponent::Key(key), Value::Object(values)) => values.get(key)?,
                (PathComponent::Index(index), Value::Array(values)) => values.get(*index)?,
                _ => return None,
            };
        }
        Some(current)
    }

    /// Persistently replaces the addressed value, rebuilding only its ancestors.
    ///
    /// # Errors
    ///
    /// Returns a missing-key or out-of-bounds error when the path is stale.
    pub fn replace(&self, root: &Value, replacement: Value) -> Result<Value, PathError> {
        replace_at(root, self.components(), replacement)
    }
}

/// Stable path update failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PathError {
    /// A key was applied to a non-object or was absent.
    #[error("object path component {key:?} is not present")]
    MissingKey {
        /// Missing key.
        key: Arc<str>,
    },
    /// An index was applied to a non-array or was out of bounds.
    #[error("array path component {index} is out of bounds")]
    MissingIndex {
        /// Missing index.
        index: usize,
    },
}

fn replace_at(
    root: &Value,
    path: &[PathComponent],
    replacement: Value,
) -> Result<Value, PathError> {
    let Some((component, tail)) = path.split_first() else {
        return Ok(replacement);
    };
    match (component, root) {
        (PathComponent::Key(key), Value::Object(values)) => {
            let child = values
                .get(key)
                .ok_or_else(|| PathError::MissingKey { key: key.clone() })?;
            let replacement = replace_at(child, tail, replacement)?;
            let mut updated: IndexMap<Arc<str>, Value> = values.as_ref().clone();
            *updated.get_mut(key).expect("key was just observed") = replacement;
            Ok(Value::object(updated))
        }
        (PathComponent::Index(index), Value::Array(values)) => {
            let child = values
                .get(*index)
                .ok_or(PathError::MissingIndex { index: *index })?;
            let replacement = replace_at(child, tail, replacement)?;
            let mut updated = values.to_vec();
            updated[*index] = replacement;
            Ok(Value::array(updated))
        }
        (PathComponent::Key(key), _) => Err(PathError::MissingKey { key: key.clone() }),
        (PathComponent::Index(index), _) => Err(PathError::MissingIndex { index: *index }),
    }
}

impl fmt::Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for component in self.components() {
            match component {
                PathComponent::Key(key) => write!(formatter, ".{key}")?,
                PathComponent::Index(index) => write!(formatter, "[{index}]")?,
            }
        }
        if self.components().is_empty() {
            formatter.write_str(".")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use indexmap::IndexMap;

    use super::{Path, PathComponent};
    use crate::Value;

    #[test]
    fn nested_update_shares_unchanged_sibling() {
        let sibling = Value::array(vec![Value::Bool(true)]);
        let root = Value::object(IndexMap::from([
            (
                Arc::from("nested"),
                Value::object(IndexMap::from([(Arc::from("value"), Value::Null)])),
            ),
            (Arc::from("sibling"), sibling.clone()),
        ]));
        let path = Path::new(vec![
            PathComponent::Key(Arc::from("nested")),
            PathComponent::Key(Arc::from("value")),
        ]);
        let updated = path.replace(&root, Value::Bool(false)).unwrap();
        let Value::Object(values) = updated else {
            panic!("object")
        };
        assert!(values["sibling"].shares_node_with(&sibling));
        assert_eq!(path.get(&Value::Object(values)), Some(&Value::Bool(false)));
    }
}
