//! Track lexical bindings and their paths back to an input root.

use super::{Environment, OriginEnvironment, charge_managed_step, path, resource};
use crate::bytecode::BindingPatternOperand;
use crate::{Bytecode, Path, PathComponent, Value, VmError, VmLimits, VmObservations};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Runtime identity for a value derived from an input document.
///
/// This is deliberately separate from `Value`: equal scalars from different
/// evaluation branches are not the same input, and pointer identity is not
/// available for scalar values. A child path makes repeated field/index
/// selection stable while constructed/literal values receive fresh roots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OriginToken {
    pub(super) root: u64,
    pub(super) path: Path,
}

impl OriginToken {
    pub(super) fn child_bounded(
        &self,
        component: PathComponent,
        limits: VmLimits,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Self, VmError> {
        if self.path.components().len() >= limits.path_stack {
            return Err(resource("path-stack"));
        }
        charge()?;
        let mut components = Vec::new();
        components
            .try_reserve_exact(self.path.components().len().saturating_add(1))
            .map_err(|_| resource("path-stack"))?;
        for _ in 0..self.path.components().len().saturating_add(1) {
            charge()?;
        }
        components.extend_from_slice(self.path.components());
        components.push(component);
        Ok(Self {
            root: self.root,
            path: Path::new(components),
        })
    }
}

pub(super) fn origin_at_path_bounded(
    origin: Option<&OriginToken>,
    path: &Path,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Option<OriginToken>, VmError> {
    let Some(origin) = origin else {
        return Ok(None);
    };
    let total = origin
        .path
        .components()
        .len()
        .checked_add(path.components().len())
        .ok_or_else(|| resource("path-stack"))?;
    if total > limits.path_stack {
        return Err(resource("path-stack"));
    }
    let mut components = Vec::new();
    components
        .try_reserve_exact(total)
        .map_err(|_| resource("path-stack"))?;
    for _ in 0..total {
        charge()?;
    }
    components.extend_from_slice(origin.path.components());
    components.extend_from_slice(path.components());
    Ok(Some(OriginToken {
        root: origin.root,
        path: Path::new(components),
    }))
}

pub(super) fn record_pattern_origin(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    origin: Option<OriginToken>,
    origins: &mut OriginEnvironment,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    match pattern {
        BindingPatternOperand::Variable(index) => {
            if let Some(name) = bytecode.string(*index) {
                if let Some(origin) = origin {
                    origins.insert(Arc::clone(name), origin);
                } else {
                    origins.remove(name);
                }
            }
        }
        BindingPatternOperand::Array(patterns) => {
            for (index, pattern) in patterns.iter().enumerate() {
                let child = match origin.as_ref() {
                    Some(token) => {
                        Some(token.child_bounded(PathComponent::Index(index), limits, charge)?)
                    }
                    None => None,
                };
                record_pattern_origin(bytecode, pattern, child, origins, limits, charge)?;
            }
        }
        BindingPatternOperand::Object(fields) => {
            for (key, pattern) in fields {
                let child = match (bytecode.string(*key), origin.as_ref()) {
                    (Some(key), Some(token)) => Some(token.child_bounded(
                        PathComponent::Key(Arc::clone(key)),
                        limits,
                        charge,
                    )?),
                    _ => None,
                };
                record_pattern_origin(bytecode, pattern, child, origins, limits, charge)?;
            }
        }
    }
    Ok(())
}

pub(super) fn record_pattern_aliases(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    path: &Path,
    origin: Option<&OriginToken>,
    aliases: &mut PathAliases,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    match pattern {
        BindingPatternOperand::Variable(index) => {
            if let Some(name) = bytecode.string(*index)
                && let Some(origin) = origin
            {
                aliases.insert(
                    Arc::clone(name),
                    PathAlias {
                        path: path.clone(),
                        origin: origin.clone(),
                    },
                );
            }
        }
        BindingPatternOperand::Array(patterns) => {
            for (index, pattern) in patterns.iter().enumerate() {
                let child_path =
                    path_with_component_bounded(path, PathComponent::Index(index), limits, charge)?;
                let child_origin = match origin {
                    Some(token) => {
                        Some(token.child_bounded(PathComponent::Index(index), limits, charge)?)
                    }
                    None => None,
                };
                record_pattern_aliases(
                    bytecode,
                    pattern,
                    &child_path,
                    child_origin.as_ref(),
                    aliases,
                    limits,
                    charge,
                )?;
            }
        }
        BindingPatternOperand::Object(fields) => {
            for (key, pattern) in fields {
                let Some(key) = bytecode.string(*key) else {
                    continue;
                };
                let component = PathComponent::Key(Arc::clone(key));
                let child_path =
                    path_with_component_bounded(path, component.clone(), limits, charge)?;
                let child_origin = match origin {
                    Some(token) => Some(token.child_bounded(component, limits, charge)?),
                    None => None,
                };
                record_pattern_aliases(
                    bytecode,
                    pattern,
                    &child_path,
                    child_origin.as_ref(),
                    aliases,
                    limits,
                    charge,
                )?;
            }
        }
    }
    Ok(())
}

fn path_with_component_bounded(
    path: &Path,
    component: PathComponent,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Path, VmError> {
    let total = path
        .components()
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("path-stack"))?;
    if total > limits.path_stack {
        return Err(resource("path-stack"));
    }
    let mut components = Vec::new();
    components
        .try_reserve_exact(total)
        .map_err(|_| resource("path-stack"))?;
    for _ in 0..total {
        charge()?;
    }
    components.extend_from_slice(path.components());
    components.push(component);
    Ok(Path::new(components))
}

pub(super) fn alias_path_for_input(alias: &PathAlias, input_origin: &OriginToken) -> Path {
    let input_components = input_origin.path.components();
    let alias_components = alias.path.components();
    if alias_components.starts_with(input_components) {
        Path::new(alias_components[input_components.len()..].to_vec())
    } else {
        Path::root()
    }
}

pub(super) fn relative_alias_path(
    input_origin: Option<&OriginToken>,
    value_origin: &OriginToken,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Path, VmError> {
    let Some(input_origin) = input_origin else {
        return Ok(Path::root());
    };
    let input_components = input_origin.path.components();
    let value_components = value_origin.path.components();
    if input_origin.root == value_origin.root && value_components.starts_with(input_components) {
        let relative = &value_components[input_components.len()..];
        if relative.len() > limits.path_stack {
            return Err(resource("path-stack"));
        }
        let mut components = Vec::new();
        components
            .try_reserve_exact(relative.len())
            .map_err(|_| resource("path-stack"))?;
        for component in relative {
            charge()?;
            components.push(component.clone());
        }
        Ok(Path::new(components))
    } else {
        Ok(Path::root())
    }
}

pub(super) fn fresh_origin(next_root: &mut u64) -> Result<OriginToken, VmError> {
    let root = *next_root;
    *next_root = next_root
        .checked_add(1)
        .ok_or_else(|| resource("origin-token"))?;
    Ok(OriginToken {
        root,
        path: Path::root(),
    })
}
pub(super) fn getpath_managed(
    input: &Value,
    path: &[PathComponent],
    limits: VmLimits,
    observations: &mut VmObservations,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
) -> Result<Value, VmError> {
    let mut charge = || charge_managed_step(observations, limits, cancellation, stop);
    path::getpath_bounded(input, path, limits, &mut charge)
}

#[derive(Clone, Default)]
pub(super) struct LexicalEnvironment {
    pub(super) values: Arc<Environment>,
    pub(super) origins: OriginEnvironment,
}

impl LexicalEnvironment {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn from_values(values: Arc<Environment>) -> Self {
        Self {
            values,
            origins: OriginEnvironment::new(),
        }
    }
}

impl std::ops::Deref for LexicalEnvironment {
    type Target = Environment;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl std::ops::DerefMut for LexicalEnvironment {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.values)
    }
}

#[derive(Clone)]
pub(super) struct PathAlias {
    pub(super) path: Path,
    pub(super) origin: OriginToken,
}

pub(super) type PathAliases = BTreeMap<Arc<str>, PathAlias>;
pub(super) fn scalar_result_origin(
    name: &str,
    input: &Value,
    arguments: &[Value],
    origin: Option<OriginToken>,
    next_origin: &mut u64,
) -> Result<Option<OriginToken>, VmError> {
    let trim_preserves_input = match (name, input, arguments.first()) {
        ("trim", Value::String(value), _) => {
            !value.chars().next().is_some_and(char::is_whitespace)
                && !value.chars().next_back().is_some_and(char::is_whitespace)
        }
        ("ltrim", Value::String(value), _) => {
            !value.chars().next().is_some_and(char::is_whitespace)
        }
        ("rtrim", Value::String(value), _) => {
            !value.chars().next_back().is_some_and(char::is_whitespace)
        }
        ("ltrimstr", Value::String(value), Some(Value::String(prefix))) => {
            !prefix.is_empty() && !value.starts_with(prefix.as_ref())
        }
        ("rtrimstr", Value::String(value), Some(Value::String(suffix))) => {
            !suffix.is_empty() && !value.ends_with(suffix.as_ref())
        }
        ("trimstr", Value::String(value), Some(Value::String(affix))) => {
            !affix.is_empty()
                && !value.starts_with(affix.as_ref())
                && !value.ends_with(affix.as_ref())
        }
        _ => false,
    };
    let preserves_input = matches!(
        name,
        "arrays"
            | "booleans"
            | "finites"
            | "iterables"
            | "nulls"
            | "normals"
            | "numbers"
            | "objects"
            | "scalars"
            | "strings"
            | "values"
    ) || trim_preserves_input
        || (name == "abs"
            && match input {
                Value::Number(number) => !number.is_less_than_zero(),
                Value::String(_) | Value::Array(_) | Value::Object(_) => true,
                Value::Null | Value::Bool(_) => false,
            })
        || (name == "tonumber" && matches!(input, Value::Number(_)))
        || (name == "tostring" && matches!(input, Value::String(_)));
    if preserves_input {
        Ok(origin)
    } else {
        fresh_origin(next_origin).map(Some)
    }
}
