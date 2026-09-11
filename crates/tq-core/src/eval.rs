//! Bounded evaluator for source-mapped expression bytecode.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    io,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
    },
    slice::ParallelSliceMut,
};

use crate::{
    BuiltinRegistry, Bytecode, InputCursor, JsonInput, JsonInputError, JsonInputOptions, Number,
    Object, Path, PathComponent, Value, VmError, VmLimits, VmObservations,
    ast::{AssignmentOperator, BinaryOperator, ParameterKind, UnaryOperator},
    bytecode::{BindingPatternOperand, InterpolationOperand, KeyOperand, Operation},
    collection, format, math, parse_jq_number, stdlib, string_compat,
    vm::EffectState,
};

mod generator;
mod iterate;
mod path;
mod path_builtin;
mod scalar;
mod sql;

pub(crate) const AMBIENT_ENVIRONMENT: &str = "__tq_ambient_environment";
pub(crate) const AMBIENT_PLATFORM: &str = "__tq_ambient_platform";
pub(crate) const INPUT_FILENAME: &str = "__tq_input_filename";
pub(crate) const INPUT_LINE_NUMBER: &str = "__tq_input_line_number";

type Environment = BTreeMap<Arc<str>, Value>;
type OriginEnvironment = BTreeMap<Arc<str>, OriginToken>;
type Outcomes = Vec<Result<Value, VmError>>;
type UserFrames = Arc<[UserFrame]>;

/// Runtime identity for a value derived from an input document.
///
/// This is deliberately separate from `Value`: equal scalars from different
/// evaluation branches are not the same input, and pointer identity is not
/// available for scalar values. A child path makes repeated field/index
/// selection stable while constructed/literal values receive fresh roots.
#[derive(Clone, Debug, Eq, PartialEq)]
struct OriginToken {
    root: u64,
    path: Path,
}

impl OriginToken {
    fn child_bounded(
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

fn origin_at_path_bounded(
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

fn record_pattern_origin(
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

fn record_pattern_aliases(
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

fn alias_path_for_input(alias: &PathAlias, input_origin: &OriginToken) -> Path {
    let input_components = input_origin.path.components();
    let alias_components = alias.path.components();
    if alias_components.starts_with(input_components) {
        Path::new(alias_components[input_components.len()..].to_vec())
    } else {
        Path::root()
    }
}

fn relative_alias_path(
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

fn fresh_origin(next_root: &mut u64) -> Result<OriginToken, VmError> {
    let root = *next_root;
    *next_root = next_root
        .checked_add(1)
        .ok_or_else(|| resource("origin-token"))?;
    Ok(OriginToken {
        root,
        path: Path::root(),
    })
}

fn charge_managed_step(
    observations: &mut VmObservations,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
) -> Result<(), VmError> {
    if stop.load(Ordering::Relaxed) || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        return Err(VmError::Interrupted);
    }
    if observations.steps >= limits.steps {
        return Err(resource("vm-steps"));
    }
    observations.steps += 1;
    Ok(())
}

fn getpath_managed(
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
struct LexicalEnvironment {
    values: Environment,
    origins: OriginEnvironment,
}

impl LexicalEnvironment {
    fn new() -> Self {
        Self::default()
    }

    fn from_values(values: Environment) -> Self {
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
        &mut self.values
    }
}

#[derive(Clone)]
struct PathAlias {
    path: Path,
    origin: OriginToken,
}

type PathAliases = BTreeMap<Arc<str>, PathAlias>;

// Below this size, thread-pool startup and scheduling cost more than the work.
const PARALLEL_SORT_THRESHOLD: usize = 16 * 1024;
const PARALLEL_REDUCTION_THRESHOLD: usize = 64 * 1024;

#[derive(Clone)]
struct FilterArgument {
    node: u32,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
}

#[derive(Clone)]
struct UserFrame {
    symbol: u32,
    filters: Vec<Option<FilterArgument>>,
}

#[allow(
    clippy::too_many_arguments,
    reason = "stream evaluation keeps execution limits, observations, cancellation, and the shared input cursor explicit"
)]
pub(crate) fn evaluate_stream(
    bytecode: &Bytecode,
    input: &Value,
    variables: &Environment,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
    input_cursor: Option<&InputCursor>,
    effects: &Arc<EffectState>,
    mut emit: impl FnMut(Result<Value, VmError>, VmObservations) -> bool,
) -> VmObservations {
    if bytecode.managed_tree_execution() {
        return evaluate_generator_stream(
            bytecode,
            input,
            variables,
            limits,
            cancellation,
            stop,
            input_cursor,
            effects,
            emit,
        );
    }
    let evaluator = Evaluator {
        bytecode,
        limits,
        observations: Cell::new(VmObservations::default()),
        cancellation,
        stop,
        input_cursor,
        effects: Arc::clone(effects),
    };
    evaluator.emit_node(bytecode.root(), input, variables, 0, &mut |result| {
        let mut observations = evaluator.observations.get();
        if result.is_ok() {
            observations.results += 1;
            evaluator.observations.set(observations);
        }
        emit(result, observations)
    });
    evaluator.observations.get()
}

#[derive(Clone)]
enum GeneratorContinuation {
    AccessField(Arc<str>),
    Iterate,
    Pipe {
        node: u32,
        environment: Arc<LexicalEnvironment>,
    },
    BinaryLeft {
        operator: BinaryOperator,
        right: u32,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
    },
    BinaryRight {
        operator: BinaryOperator,
        left: Value,
    },
    Unary(UnaryOperator),
    ArrayItem(Rc<RefCell<Vec<Value>>>),
    ArrayUpdateItem {
        values: Rc<RefCell<Vec<Value>>>,
        accepted: Rc<Cell<bool>>,
    },
    ObjectItem {
        key: Arc<str>,
        values: Rc<RefCell<Object>>,
        accepted: Rc<Cell<bool>>,
    },
    IgnoreError,
    HaltError {
        input: Value,
    },
    Select {
        input: Value,
        origin: Option<OriginToken>,
    },
    HasArgument {
        input: Value,
        container_from_result: bool,
    },
    SortKeyItem(Rc<RefCell<Vec<Value>>>),
    AlternativeItem(Rc<Cell<bool>>),
    Bind {
        pattern: BindingPatternOperand,
        body: u32,
        input: Value,
        input_origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        origin: Option<OriginToken>,
    },
    BindAlternatives {
        patterns: Arc<[BindingPatternOperand]>,
        body: u32,
        input: Value,
        environment: Arc<LexicalEnvironment>,
    },
    BindAlternativeBody {
        boundary: u64,
        patterns: Arc<[BindingPatternOperand]>,
        next: usize,
        body: u32,
        value: Value,
        input: Value,
        environment: Arc<LexicalEnvironment>,
    },
    PullConsumer {
        state: Rc<RefCell<PullConsumerState>>,
    },
    PredicateItem {
        state: Rc<RefCell<PullConsumerState>>,
        condition: Option<u32>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    PredicateCondition {
        state: Rc<RefCell<PullConsumerState>>,
    },
    PullCount {
        kind: PullConsumerKind,
        input: Value,
        origin: Option<OriginToken>,
        generator: Option<u32>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    FromStreamItem(Rc<RefCell<generator::FromStreamState>>),
    TruncateStreamItem {
        count: Value,
    },
    AddItem(Rc<RefCell<AddState>>),
    DebugItem,
    FromEntries,
    Conditional {
        branches: Arc<[(u32, u32)]>,
        next: usize,
        alternative: u32,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
    },
    AccessIndex {
        node: u32,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
    },
    SliceBase {
        start: Option<u32>,
        end: Option<u32>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    SliceStart {
        base: Value,
        end: Option<u32>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    SliceEnd {
        base: Value,
        start: Option<f64>,
        origin: Option<OriginToken>,
    },
    ApplyIndex {
        base: Value,
        origin: Option<OriginToken>,
    },
    ObjectKey {
        entries: Arc<[crate::bytecode::ObjectOperand]>,
        next: usize,
        object: Object,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    ObjectValue {
        entries: Arc<[crate::bytecode::ObjectOperand]>,
        next: usize,
        object: Object,
        key: Arc<str>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    FoldInitial {
        generator: u32,
        pattern: BindingPatternOperand,
        update: u32,
        extract: Option<u32>,
        emit_each_update: bool,
        input: Value,
        environment: Arc<LexicalEnvironment>,
    },
    FoldItem {
        state: Rc<RefCell<FoldState>>,
        pattern: BindingPatternOperand,
        update: u32,
        extract: Option<u32>,
        emit_each_update: bool,
        environment: Arc<LexicalEnvironment>,
    },
    FoldUpdate {
        state: Rc<RefCell<FoldState>>,
        extract: Option<u32>,
        emit_each_update: bool,
        environment: Arc<LexicalEnvironment>,
    },
    PathField(Arc<str>),
    PathIndex {
        node: u32,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
    },
    PathIndexValue {
        path: Path,
        target: Value,
    },
    PathIterate {
        input: Value,
    },
    PathPipe {
        node: u32,
        input: Value,
        environment: Arc<LexicalEnvironment>,
    },
    PathJoin(Path),
    PathSelect(Path),
    PathOrigin,
    PathGetPath {
        base: Path,
        input: Value,
    },
    PathCollect {
        boundary: u64,
        paths: Rc<RefCell<path_builtin::PathAccumulator>>,
    },
    FreshOrigin,
    PathFilter {
        root: Value,
        filter: u32,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    PathFilterResult(Path),
    PathConditional {
        branches: Arc<[(u32, u32)]>,
        next: usize,
        alternative: u32,
        input: Value,
        origin: Option<OriginToken>,
        prefix: Path,
        environment: Arc<LexicalEnvironment>,
    },
    PathCatch {
        boundary: u64,
        node: Option<u32>,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
    },
    PathBindAlternatives {
        boundary: u64,
        patterns: Arc<[BindingPatternOperand]>,
        body: u32,
        input: Value,
        prefix: Path,
        environment: Arc<LexicalEnvironment>,
    },
    PathBindAlternativeBody {
        boundary: u64,
        patterns: Arc<[BindingPatternOperand]>,
        next: usize,
        body: u32,
        value: Value,
        input: Value,
        prefix: Path,
        environment: Arc<LexicalEnvironment>,
    },
    PathAliases(Arc<PathAliases>),
    PathBind {
        pattern: BindingPatternOperand,
        body: u32,
        input: Value,
        input_origin: Option<OriginToken>,
        prefix: Path,
        environment: Arc<LexicalEnvironment>,
        entered_from_path: bool,
    },
    PathBindValue {
        pattern: BindingPatternOperand,
        body: u32,
        input: Value,
        input_origin: Option<OriginToken>,
        prefix: Path,
        environment: Arc<LexicalEnvironment>,
        entered_from_path: bool,
    },
    PathSliceBase {
        start: Option<u32>,
        end: Option<u32>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    PathSliceStart {
        path: Path,
        target: Value,
        end: Option<u32>,
        assignment: Option<Rc<RefCell<AssignmentState>>>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    PathSliceEnd {
        path: Path,
        target: Value,
        start: Option<f64>,
        assignment: Option<Rc<RefCell<AssignmentState>>>,
    },
    PathUserReturn {
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    PathUserArgument {
        symbol: u32,
        arguments: Arc<[u32]>,
        next: usize,
        input: Value,
        input_origin: Option<OriginToken>,
        prefix: Path,
        caller_environment: Arc<LexicalEnvironment>,
        caller_frames: UserFrames,
        filters: Vec<Option<FilterArgument>>,
        bindings: LexicalEnvironment,
        from_path: bool,
    },
    AssignmentPath(Rc<RefCell<AssignmentState>>),
    AssignmentRhs(Rc<RefCell<AssignmentState>>),
    AssignmentUpdateRhs {
        state: Rc<RefCell<AssignmentState>>,
        target: AssignmentTarget,
        old: Value,
        next: usize,
        boundary: u64,
        accepted: Rc<Cell<bool>>,
    },
    OptionalBoundary {
        boundary: u64,
    },
    Catch {
        boundary: u64,
        node: Option<u32>,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
    },
    Label(u32),
    Interpolate {
        segments: Arc<[InterpolationOperand]>,
        next: usize,
        slot: usize,
        pieces: Vec<Option<Arc<str>>>,
        input: Value,
        environment: Arc<LexicalEnvironment>,
    },
    UserArgument {
        symbol: u32,
        arguments: Arc<[u32]>,
        next: usize,
        input: Value,
        caller_environment: Arc<LexicalEnvironment>,
        caller_frames: UserFrames,
        filters: Vec<Option<FilterArgument>>,
        bindings: LexicalEnvironment,
        input_origin: Option<OriginToken>,
    },
    BuiltinArguments {
        name: Arc<str>,
        arguments: Arc<[u32]>,
        order: Arc<[usize]>,
        next: usize,
        values: Vec<Option<Value>>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    SqlInRight {
        boundary: u64,
        state: Rc<RefCell<SqlInState>>,
        source: u32,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    SqlInLeft {
        boundary: u64,
        state: Rc<RefCell<SqlInState>>,
        needle: Value,
    },
    SqlIndexRow {
        boundary: u64,
        state: Rc<RefCell<SqlIndexState>>,
        key: u32,
    },
    SqlIndexKey {
        boundary: u64,
        state: Rc<RefCell<SqlIndexState>>,
        row: Value,
    },
    SqlJoinIndex {
        boundary: u64,
        key: u32,
        stream: Option<u32>,
        join: Option<u32>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    SqlJoinRow {
        boundary: u64,
        index: Value,
        key: u32,
        join: Option<u32>,
        state: Option<Rc<RefCell<SqlJoinState>>>,
    },
    SqlJoinKey {
        boundary: u64,
        index: Value,
        row: Value,
        join: Option<u32>,
        state: Option<Rc<RefCell<SqlJoinState>>>,
    },
    SqlJoinCallback {
        boundary: u64,
    },
    RegexArguments {
        name: Arc<str>,
        arguments: Arc<[u32]>,
        order: Arc<[usize]>,
        next: usize,
        values: Vec<Option<Value>>,
        replacement: Option<u32>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    RegexReplacementItem {
        name: Arc<str>,
        state: Rc<RefCell<stdlib::RegexSubstitutionState>>,
    },
    ReturnUser {
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    Raise,
    RecurseChild {
        filter: u32,
        condition: Option<u32>,
        depth: usize,
        structural: bool,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    RecurseCondition {
        child: Value,
        filter: u32,
        condition: u32,
        depth: usize,
        structural: bool,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    LoopCondition {
        condition: u32,
        update: u32,
        input: Value,
        until: bool,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    LoopUpdate {
        condition: u32,
        update: u32,
        until: bool,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    RepeatItem,
    LimitCount {
        expression: u32,
        input: Value,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    LimitItem {
        boundary: u64,
        remaining: Rc<Cell<usize>>,
    },
}

#[derive(Clone)]
struct GeneratorWork {
    node: u32,
    input: Value,
    origin: Option<OriginToken>,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    continuations: Vec<GeneratorContinuation>,
}

fn regex_error_work(
    bytecode: &Bytecode,
    environment: &Arc<LexicalEnvironment>,
    frames: &UserFrames,
    continuations: &[GeneratorContinuation],
    error: VmError,
) -> (GeneratorWork, Option<Result<Value, VmError>>) {
    (
        GeneratorWork {
            node: bytecode.root(),
            input: Value::Null,
            origin: None,
            environment: Arc::clone(environment),
            frames: Arc::clone(frames),
            continuations: continuations.to_vec(),
        },
        Some(Err(error)),
    )
}

enum GeneratorTask {
    Eval(GeneratorWork),
    InvokeBuiltin {
        name: Arc<str>,
        arguments: Vec<Value>,
        source_arity: usize,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    InvokeRegex {
        name: Arc<str>,
        arguments: Vec<Value>,
        replacement: Option<u32>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    RegexCursor {
        cursor: Rc<RefCell<stdlib::RegexCursor>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    RegexSplitCursor {
        cursor: Rc<RefCell<stdlib::RegexCursor>>,
        input: Arc<str>,
        copied: usize,
        emitted_tail: bool,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    RegexSubstitution {
        name: Arc<str>,
        state: Rc<RefCell<stdlib::RegexSubstitutionState>>,
        replacement: u32,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    RegexReplacementFinish {
        name: Arc<str>,
        state: Rc<RefCell<stdlib::RegexSubstitutionState>>,
        replacement: u32,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishBuiltinArguments {
        name: Arc<str>,
        source_arity: usize,
        values: Rc<RefCell<Vec<Value>>>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    SqlInFinish {
        boundary: u64,
        state: Rc<RefCell<SqlInState>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    SqlIndexFinish {
        boundary: u64,
        state: Rc<RefCell<SqlIndexState>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    SqlJoinFinish {
        boundary: u64,
        state: Rc<RefCell<SqlJoinState>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    BuiltinGenerator {
        state: generator::GeneratorState,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    Iterate {
        values: Arc<[Value]>,
        next: usize,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    IterateCursor {
        cursor: iterate::ContainerCursor,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    InputValues {
        cursor: InputCursor,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishArray {
        values: Rc<RefCell<Vec<Value>>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    MapArray {
        inputs: Arc<[Value]>,
        next: usize,
        argument: u32,
        values: Rc<RefCell<Vec<Value>>>,
        first_only: bool,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    MapObject {
        entries: Arc<[(Arc<str>, Value)]>,
        next: usize,
        argument: u32,
        values: Rc<RefCell<Object>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    CollectSortKeys {
        values: Arc<[Value]>,
        next: usize,
        argument: u32,
        keyed_values: Rc<RefCell<Vec<ManagedKeyedValue>>>,
        mode: KeyedCollectionMode,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishSortKey {
        input: Value,
        origin: Option<OriginToken>,
        keys: Rc<RefCell<Vec<Value>>>,
        keyed_values: Rc<RefCell<Vec<ManagedKeyedValue>>>,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishAlternative {
        matched: Rc<Cell<bool>>,
        right: u32,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishPullConsumer {
        state: Rc<RefCell<PullConsumerState>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishAdd {
        state: Rc<RefCell<AddState>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishDebug {
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FoldGenerator {
        generator: u32,
        pattern: BindingPatternOperand,
        update: u32,
        extract: Option<u32>,
        emit_each_update: bool,
        state: Rc<RefCell<FoldState>>,
        input: Value,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishFold {
        state: Rc<RefCell<FoldState>>,
        emit_final: bool,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    PathEval(PathWork),
    PathChildren {
        value: Value,
        path: Path,
        next: usize,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    PathTraverse {
        cursor: PathTraversalCursor,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishPathCollection {
        boundary: u64,
        kind: PathCollectionKind,
        input: Value,
        paths: Rc<RefCell<path_builtin::PathAccumulator>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    PathSlice {
        path: Path,
        start: usize,
        end: usize,
        next: usize,
        assignment: Option<Rc<RefCell<AssignmentState>>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    FinishAssignmentPaths {
        state: Rc<RefCell<AssignmentState>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    AssignmentUpdate {
        state: Rc<RefCell<AssignmentState>>,
        next: usize,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    AssignmentUpdateContinue {
        state: Rc<RefCell<AssignmentState>>,
        next: usize,
        boundary: u64,
        accepted: Rc<Cell<bool>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    Traverse {
        cursor: TraversalCursor,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    RecurseExpand {
        input: Value,
        filter: Option<u32>,
        condition: Option<u32>,
        depth: usize,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    RecurseChildren {
        input: Value,
        next: usize,
        depth: usize,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    RecurseValue {
        value: Value,
        filter: Option<u32>,
        condition: Option<u32>,
        depth: usize,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    Repeat {
        expression: u32,
        input: Value,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    WalkValue {
        input: Value,
        callback: u32,
        depth: usize,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    WalkArray {
        inputs: Arc<[Value]>,
        next: usize,
        callback: u32,
        depth: usize,
        values: Rc<RefCell<Vec<Value>>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
    WalkObject {
        entries: Arc<[(Arc<str>, Value)]>,
        next: usize,
        callback: u32,
        depth: usize,
        values: Rc<RefCell<Object>>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
        continuations: Vec<GeneratorContinuation>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PullConsumerKind {
    First,
    Last,
    IsEmpty,
    Any,
    All,
    Nth,
    Skip,
}

struct PullConsumerState {
    boundary: u64,
    kind: PullConsumerKind,
    seen: bool,
    latest: Option<Value>,
    latest_origin: Option<OriginToken>,
    decision: Option<bool>,
    remaining: usize,
}

#[derive(Clone, Copy)]
enum KeyedCollectionMode {
    Sort,
    Unique,
    Group,
    Min,
    Max,
}

struct ManagedKeyedValue {
    key: Value,
    value: Value,
    origin: Option<OriginToken>,
}

struct FoldState {
    accumulator: Value,
}

struct AddState {
    accumulator: Option<Value>,
}

struct SqlInState {
    matched: bool,
}

struct SqlIndexState {
    builder: Option<sql::SqlIndexBuilder>,
}

struct SqlJoinState {
    collector: Option<sql::SqlPairCollector>,
}

#[derive(Clone)]
struct PathWork {
    node: u32,
    input: Value,
    origin: Option<OriginToken>,
    prefix: Path,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    continuations: Vec<GeneratorContinuation>,
}

#[derive(Clone)]
struct PathTraversalFrame {
    value: Value,
    path: Path,
    emitted: bool,
    next_child: usize,
}

#[derive(Clone)]
struct PathTraversalCursor {
    frames: Vec<PathTraversalFrame>,
}

impl PathTraversalCursor {
    fn new(value: Value, path: Path) -> Self {
        Self {
            frames: vec![PathTraversalFrame {
                value,
                path,
                emitted: false,
                next_child: 0,
            }],
        }
    }

    fn without_root(value: Value) -> Self {
        Self {
            frames: vec![PathTraversalFrame {
                value,
                path: Path::root(),
                emitted: true,
                next_child: 0,
            }],
        }
    }

    fn next(
        &mut self,
        depth_limit: usize,
        charge: &mut impl FnMut() -> Result<(), VmError>,
    ) -> Result<Option<Path>, VmError> {
        loop {
            charge()?;
            let Some(frame) = self.frames.last_mut() else {
                return Ok(None);
            };
            if !frame.emitted {
                frame.emitted = true;
                return Ok(Some(frame.path.clone()));
            }
            let next_child = frame.next_child;
            let child = match &frame.value {
                Value::Array(values) => values
                    .get(next_child)
                    .cloned()
                    .map(|child| (PathComponent::Index(next_child), child)),
                Value::Object(values) => values
                    .get_index(next_child)
                    .map(|(key, child)| (PathComponent::Key(Arc::clone(key)), child.clone())),
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
            };
            let Some((component, child)) = child else {
                self.frames.pop();
                continue;
            };
            frame.next_child = frame.next_child.saturating_add(1);
            let parent_path = frame.path.clone();
            let total = parent_path
                .components()
                .len()
                .checked_add(1)
                .ok_or_else(|| resource("path-stack"))?;
            if total > depth_limit {
                return Err(resource("path-stack"));
            }
            let mut components = Vec::new();
            components
                .try_reserve_exact(total)
                .map_err(|_| resource("path-stack"))?;
            for _ in 0..total {
                charge()?;
            }
            components.extend_from_slice(parent_path.components());
            components.push(component);
            self.frames
                .try_reserve(1)
                .map_err(|_| resource("path-stack"))?;
            let path = Path::new(components);
            self.frames.push(PathTraversalFrame {
                value: child,
                path,
                emitted: false,
                next_child: 0,
            });
        }
    }

    fn depth(&self) -> usize {
        self.frames.len().saturating_sub(1)
    }
}

#[derive(Clone)]
enum AssignmentTarget {
    Path(Path),
    Slice {
        path: Path,
        start: usize,
        end: usize,
    },
}

struct AssignmentState {
    operator: AssignmentOperator,
    value_node: u32,
    input: Value,
    targets: Vec<AssignmentTarget>,
    target_origins: Vec<Option<OriginToken>>,
    document: Value,
    deletions: path_builtin::PathAccumulator,
}

#[derive(Clone, Copy)]
enum PathCollectionKind {
    Delete,
    Pick,
}

fn task_owned_by_label(task: &GeneratorTask, label: u32) -> bool {
    task_has_continuation(
        task,
        |continuation| matches!(continuation, GeneratorContinuation::Label(symbol) if *symbol == label),
    )
}

fn task_has_continuation(
    task: &GeneratorTask,
    predicate: impl Fn(&GeneratorContinuation) -> bool,
) -> bool {
    task_continuations(task).iter().any(predicate)
}

fn task_owned_by_catch(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(task, |continuation| {
        matches!(
            continuation,
            GeneratorContinuation::Catch { boundary: candidate, .. }
                | GeneratorContinuation::PathCatch { boundary: candidate, .. }
                if *candidate == boundary
        )
    })
}

fn task_owned_by_optional(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(task, |continuation| {
        matches!(
            continuation,
            GeneratorContinuation::OptionalBoundary { boundary: candidate }
                if *candidate == boundary
        )
    })
}

fn task_owned_by_bind_alternative(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(task, |continuation| {
        matches!(
            continuation,
            GeneratorContinuation::BindAlternativeBody {
                boundary: candidate,
                ..
            }
            | GeneratorContinuation::PathBindAlternativeBody {
                boundary: candidate,
                ..
            } if *candidate == boundary
        )
    })
}

fn task_owned_by_limit(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(
        task,
        |continuation| matches!(continuation, GeneratorContinuation::LimitItem { boundary: candidate, .. } if *candidate == boundary),
    )
}

fn task_owned_by_pull_consumer(task: &GeneratorTask, boundary: u64) -> bool {
    match task {
        GeneratorTask::FinishPullConsumer { state, .. } => state.borrow().boundary == boundary,
        _ => task_has_continuation(task, |continuation| {
            matches!(
                continuation,
                GeneratorContinuation::PullConsumer { state }
                    | GeneratorContinuation::PredicateItem { state, .. }
                    | GeneratorContinuation::PredicateCondition { state }
                    if state.borrow().boundary == boundary
            )
        }),
    }
}

fn task_owned_by_pull_consumer_work(task: &GeneratorTask, boundary: u64) -> bool {
    !matches!(task, GeneratorTask::FinishPullConsumer { .. })
        && task_owned_by_pull_consumer(task, boundary)
}

fn task_owned_by_sql(task: &GeneratorTask, boundary: u64) -> bool {
    match task {
        GeneratorTask::SqlInFinish {
            boundary: candidate,
            ..
        }
        | GeneratorTask::SqlIndexFinish {
            boundary: candidate,
            ..
        }
        | GeneratorTask::SqlJoinFinish {
            boundary: candidate,
            ..
        } => *candidate == boundary,
        _ => task_has_continuation(task, |continuation| {
            matches!(
                continuation,
                GeneratorContinuation::SqlInRight {
                    boundary: candidate,
                    ..
                }
                | GeneratorContinuation::SqlInLeft {
                    boundary: candidate,
                    ..
                }
                | GeneratorContinuation::SqlIndexRow {
                    boundary: candidate,
                    ..
                }
                | GeneratorContinuation::SqlIndexKey {
                    boundary: candidate,
                    ..
                }
                | GeneratorContinuation::SqlJoinIndex {
                    boundary: candidate,
                    ..
                }
                | GeneratorContinuation::SqlJoinRow {
                    boundary: candidate,
                    ..
                }
                | GeneratorContinuation::SqlJoinKey {
                    boundary: candidate,
                    ..
                }
                | GeneratorContinuation::SqlJoinCallback {
                    boundary: candidate,
                }
                if *candidate == boundary
            )
        }),
    }
}

fn task_owned_by_assignment_rhs(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(task, |continuation| {
        matches!(
            continuation,
            GeneratorContinuation::AssignmentUpdateRhs {
                boundary: candidate,
                ..
            } if *candidate == boundary
        )
    })
}

fn task_owned_by_assignment_boundary(task: &GeneratorTask, boundary: u64) -> bool {
    matches!(
        task,
        GeneratorTask::AssignmentUpdateContinue {
            boundary: candidate,
            ..
        } if *candidate == boundary
    ) || task_owned_by_assignment_rhs(task, boundary)
}

fn task_owned_by_path_collection(task: &GeneratorTask, boundary: u64) -> bool {
    matches!(
        task,
        GeneratorTask::FinishPathCollection {
            boundary: candidate,
            ..
        } if *candidate == boundary
    ) || task_has_continuation(task, |continuation| {
        matches!(
            continuation,
            GeneratorContinuation::PathCollect {
                boundary: candidate,
                ..
            } if *candidate == boundary
        )
    })
}

fn task_continuations(task: &GeneratorTask) -> &[GeneratorContinuation] {
    match task {
        GeneratorTask::Eval(work) => &work.continuations,
        GeneratorTask::InvokeBuiltin { continuations, .. }
        | GeneratorTask::InvokeRegex { continuations, .. }
        | GeneratorTask::RegexCursor { continuations, .. }
        | GeneratorTask::RegexSplitCursor { continuations, .. }
        | GeneratorTask::RegexSubstitution { continuations, .. }
        | GeneratorTask::RegexReplacementFinish { continuations, .. }
        | GeneratorTask::FinishBuiltinArguments { continuations, .. }
        | GeneratorTask::SqlInFinish { continuations, .. }
        | GeneratorTask::SqlIndexFinish { continuations, .. }
        | GeneratorTask::SqlJoinFinish { continuations, .. }
        | GeneratorTask::BuiltinGenerator { continuations, .. }
        | GeneratorTask::Iterate { continuations, .. }
        | GeneratorTask::IterateCursor { continuations, .. }
        | GeneratorTask::InputValues { continuations, .. }
        | GeneratorTask::FinishArray { continuations, .. }
        | GeneratorTask::MapArray { continuations, .. }
        | GeneratorTask::MapObject { continuations, .. }
        | GeneratorTask::CollectSortKeys { continuations, .. }
        | GeneratorTask::FinishSortKey { continuations, .. }
        | GeneratorTask::FinishAlternative { continuations, .. }
        | GeneratorTask::FinishPullConsumer { continuations, .. }
        | GeneratorTask::FinishAdd { continuations, .. }
        | GeneratorTask::FinishDebug { continuations, .. }
        | GeneratorTask::FoldGenerator { continuations, .. }
        | GeneratorTask::FinishFold { continuations, .. }
        | GeneratorTask::PathEval(PathWork { continuations, .. })
        | GeneratorTask::PathChildren { continuations, .. }
        | GeneratorTask::PathTraverse { continuations, .. }
        | GeneratorTask::FinishPathCollection { continuations, .. }
        | GeneratorTask::PathSlice { continuations, .. }
        | GeneratorTask::FinishAssignmentPaths { continuations, .. }
        | GeneratorTask::AssignmentUpdate { continuations, .. }
        | GeneratorTask::AssignmentUpdateContinue { continuations, .. }
        | GeneratorTask::Traverse { continuations, .. }
        | GeneratorTask::RecurseExpand { continuations, .. }
        | GeneratorTask::RecurseChildren { continuations, .. }
        | GeneratorTask::RecurseValue { continuations, .. }
        | GeneratorTask::Repeat { continuations, .. }
        | GeneratorTask::WalkValue { continuations, .. }
        | GeneratorTask::WalkArray { continuations, .. }
        | GeneratorTask::WalkObject { continuations, .. } => continuations,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "SQL scheduling carries the caller context needed by every callback phase"
)]
#[allow(
    clippy::too_many_lines,
    reason = "SQL call scheduling keeps the right-to-left callback phases together"
)]
fn schedule_sql_call(
    name: &str,
    arguments: &[u32],
    input: Value,
    origin: Option<OriginToken>,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    continuations: Vec<GeneratorContinuation>,
    limits: VmLimits,
    next_boundary: &mut u64,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let boundary = *next_boundary;
    *next_boundary = next_boundary.saturating_add(1);
    match name {
        "IN" => {
            let Some(source) = arguments.first().copied() else {
                return Some(Err(invalid("IN argument missing")));
            };
            if arguments.len() > 2 {
                return Some(Err(invalid("IN accepts at most two arguments")));
            }
            let state = Rc::new(RefCell::new(SqlInState { matched: false }));
            pending.push(GeneratorTask::SqlInFinish {
                boundary,
                state: Rc::clone(&state),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
                continuations: continuations.clone(),
            });
            let mut next_continuations = continuations;
            if let Some(needle) = arguments.get(1).copied() {
                next_continuations.push(GeneratorContinuation::SqlInRight {
                    boundary,
                    state,
                    source,
                    input: input.clone(),
                    origin: origin.clone(),
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: needle,
                    input,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                }));
            } else {
                next_continuations.push(GeneratorContinuation::SqlInLeft {
                    boundary,
                    state,
                    needle: input.clone(),
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: source,
                    input,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                }));
            }
            None
        }
        "INDEX" => {
            if !(1..=2).contains(&arguments.len()) {
                return Some(Err(invalid("INDEX expects one or two arguments")));
            }
            let key = *arguments.last().expect("INDEX arity was checked");
            let cursor =
                (arguments.len() == 1).then(|| iterate::ContainerCursor::new(input.clone()));
            if cursor.as_ref().is_some_and(|cursor| !cursor.is_container()) {
                return Some(Err(runtime(format!("Cannot iterate over {input}"))));
            }
            let state = Rc::new(RefCell::new(SqlIndexState {
                builder: Some(sql::SqlIndexBuilder::new(limits)),
            }));
            pending.push(GeneratorTask::SqlIndexFinish {
                boundary,
                state: Rc::clone(&state),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
                continuations: continuations.clone(),
            });
            let mut next_continuations = continuations;
            next_continuations.push(GeneratorContinuation::SqlIndexRow {
                boundary,
                state,
                key,
            });
            if let Some(cursor) = cursor {
                pending.push(GeneratorTask::IterateCursor {
                    cursor,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                });
            } else {
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: arguments[0],
                    input,
                    origin,
                    environment,
                    frames,
                    continuations: next_continuations,
                }));
            }
            None
        }
        "JOIN" => {
            if !(2..=4).contains(&arguments.len()) {
                return Some(Err(invalid("JOIN expects two to four arguments")));
            }
            let key = if arguments.len() == 2 {
                arguments[1]
            } else {
                arguments[2]
            };
            let stream = arguments.get(1).copied().filter(|_| arguments.len() >= 3);
            let join = arguments.get(3).copied();
            let index_input = input.clone();
            let index_environment = Arc::clone(&environment);
            let index_frames = Arc::clone(&frames);
            let mut next_continuations = continuations;
            next_continuations.push(GeneratorContinuation::SqlJoinIndex {
                boundary,
                key,
                stream,
                join,
                input,
                origin: origin.clone(),
                environment,
                frames,
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: arguments[0],
                input: index_input,
                origin,
                environment: index_environment,
                frames: index_frames,
                continuations: next_continuations,
            }));
            None
        }
        _ => Some(Err(invalid("unsupported SQL builtin"))),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "object construction carries each branch's bounded continuation state explicitly"
)]
fn schedule_object_entry(
    bytecode: &Bytecode,
    entries: &Arc<[crate::bytecode::ObjectOperand]>,
    next: usize,
    object: Object,
    input: Value,
    origin: Option<OriginToken>,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    mut continuations: Vec<GeneratorContinuation>,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let Some(entry) = entries.get(next) else {
        return Some(Ok(Value::object(object)));
    };
    match &entry.key {
        KeyOperand::Static(index) => {
            let Some(key) = bytecode.string(*index) else {
                return Some(Err(invalid("object key string missing after validation")));
            };
            continuations.push(GeneratorContinuation::ObjectValue {
                entries: Arc::clone(entries),
                next: next.saturating_add(1),
                object,
                key: Arc::clone(key),
                input: input.clone(),
                origin: origin.clone(),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: entry.value,
                input,
                origin,
                environment,
                frames,
                continuations,
            }));
        }
        KeyOperand::Computed(node) => {
            continuations.push(GeneratorContinuation::ObjectKey {
                entries: Arc::clone(entries),
                next,
                object,
                input: input.clone(),
                origin: origin.clone(),
                environment: Arc::clone(&environment),
                frames: Arc::clone(&frames),
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: *node,
                input,
                origin,
                environment,
                frames,
                continuations,
            }));
        }
    }
    None
}

fn current_path_aliases(continuations: &[GeneratorContinuation]) -> PathAliases {
    continuations
        .iter()
        .rev()
        .find_map(|continuation| match continuation {
            GeneratorContinuation::PathAliases(aliases) => Some(aliases.as_ref().clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn path_bind_candidate(bytecode: &Bytecode, node: u32) -> bool {
    fn visit(
        bytecode: &Bytecode,
        node: u32,
        memo: &mut BTreeMap<u32, bool>,
        active: &mut BTreeSet<u32>,
    ) -> bool {
        if let Some(candidate) = memo.get(&node) {
            return *candidate;
        }
        if !active.insert(node) {
            return false;
        }
        let result = match bytecode.instructions().get(node as usize) {
            Some(instruction) => match &instruction.operation {
                Operation::Identity
                | Operation::RecursiveDescent
                | Operation::Variable(_)
                | Operation::ParameterCall { .. } => true,
                Operation::AccessField { base, .. }
                | Operation::Iterate(base)
                | Operation::Optional(base) => visit(bytecode, *base, memo, active),
                Operation::AccessIndex { base, .. } | Operation::Slice { base, .. } => {
                    visit(bytecode, *base, memo, active)
                }
                Operation::Pipe { left, right } => {
                    visit(bytecode, *left, memo, active) && visit(bytecode, *right, memo, active)
                }
                Operation::Comma { left, right } => {
                    visit(bytecode, *left, memo, active) || visit(bytecode, *right, memo, active)
                }
                Operation::Conditional {
                    branches,
                    alternative,
                } => {
                    branches
                        .iter()
                        .any(|(_, body)| visit(bytecode, *body, memo, active))
                        || visit(bytecode, *alternative, memo, active)
                }
                Operation::TryCatch { expression, catch } => {
                    visit(bytecode, *expression, memo, active)
                        || catch.is_some_and(|catch| visit(bytecode, catch, memo, active))
                }
                Operation::Bind { body, .. } | Operation::BindAlternatives { body, .. } => {
                    visit(bytecode, *body, memo, active)
                }
                Operation::UserCall { symbol, .. } => bytecode
                    .functions()
                    .get(*symbol as usize)
                    .is_some_and(|function| visit(bytecode, function.body, memo, active)),
                Operation::Call { name, .. } => bytecode
                    .string(*name)
                    .is_some_and(|name| name.as_ref() == "select"),
                _ => false,
            },
            None => false,
        };
        active.remove(&node);
        memo.insert(node, result);
        result
    }

    visit(bytecode, node, &mut BTreeMap::new(), &mut BTreeSet::new())
}

fn path_candidate_with_aliases(
    bytecode: &Bytecode,
    node: u32,
    continuations: &[GeneratorContinuation],
) -> bool {
    match bytecode
        .instructions()
        .get(node as usize)
        .map(|instruction| &instruction.operation)
    {
        Some(Operation::Variable(index)) => bytecode
            .string(*index)
            .is_some_and(|name| current_path_aliases(continuations).contains_key(name)),
        _ => path_bind_candidate(bytecode, node),
    }
}

fn clear_path_aliases(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    aliases: &mut PathAliases,
) {
    match pattern {
        BindingPatternOperand::Variable(index) => {
            if let Some(name) = bytecode.string(*index) {
                aliases.remove(name);
            }
        }
        BindingPatternOperand::Array(patterns) => {
            for pattern in patterns {
                clear_path_aliases(bytecode, pattern, aliases);
            }
        }
        BindingPatternOperand::Object(fields) => {
            for (_, pattern) in fields {
                clear_path_aliases(bytecode, pattern, aliases);
            }
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "path traversal keeps the selected root and managed continuation state explicit"
)]
fn schedule_path_eval(
    bytecode: &Bytecode,
    work: PathWork,
    pending: &mut Vec<GeneratorTask>,
    next_boundary: &mut u64,
) -> Option<Result<Value, VmError>> {
    let operation = match bytecode.instructions().get(work.node as usize) {
        Some(instruction) => instruction.operation.clone(),
        None => return Some(Err(invalid("path instruction missing"))),
    };
    match operation {
        Operation::Identity => Some(Ok(path_value(&work.prefix))),
        Operation::Variable(index) => {
            let Some(name) = bytecode.string(index) else {
                return Some(Err(invalid("string missing after validation")));
            };
            if !work.environment.contains_key(name) {
                return Some(Err(runtime(format!("variable ${name} has no value"))));
            }
            let aliases = current_path_aliases(&work.continuations);
            if let Some(alias) = aliases.get(name) {
                if work.origin.as_ref() == Some(&alias.origin) {
                    let path = work
                        .origin
                        .as_ref()
                        .map_or_else(Path::root, |origin| alias_path_for_input(alias, origin));
                    return Some(Ok(path_value(&path)));
                }
                return Some(Err(runtime(
                    "assignment left side is not a path".to_owned(),
                )));
            }
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: work.node,
                input: work.input,
                origin: work.origin.clone(),
                environment: work.environment,
                frames: work.frames,
                continuations: work.continuations,
            }));
            None
        }
        Operation::AccessField { base, key } => {
            let key = match bytecode.string(key) {
                Some(key) => Arc::clone(key),
                None => return Some(Err(invalid("path field missing after validation"))),
            };
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathField(key));
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::AccessIndex { base, index } => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathIndex {
                node: index,
                input: work.input.clone(),
                origin: work.origin.clone(),
                environment: Arc::clone(&work.environment),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Slice { base, start, end } => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathSliceBase {
                start,
                end,
                input: work.input.clone(),
                origin: work.origin.clone(),
                environment: Arc::clone(&work.environment),
                frames: Arc::clone(&work.frames),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Iterate(base) => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathIterate {
                input: work.input.clone(),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: base,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::RecursiveDescent => {
            pending.push(GeneratorTask::PathTraverse {
                cursor: PathTraversalCursor::new(work.input, work.prefix),
                environment: work.environment,
                frames: work.frames,
                continuations: work.continuations,
            });
            None
        }
        Operation::Pipe { left, right } => {
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathPipe {
                node: right,
                input: work.input.clone(),
                environment: Arc::clone(&work.environment),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: left,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Comma { left, right } => {
            let right_task = if path_candidate_with_aliases(bytecode, right, &work.continuations) {
                GeneratorTask::PathEval(PathWork {
                    node: right,
                    input: work.input.clone(),
                    origin: work.origin.clone(),
                    prefix: work.prefix.clone(),
                    environment: Arc::clone(&work.environment),
                    frames: Arc::clone(&work.frames),
                    continuations: work.continuations.clone(),
                })
            } else {
                GeneratorTask::Eval(GeneratorWork {
                    node: right,
                    input: work.input.clone(),
                    origin: work.origin.clone(),
                    environment: Arc::clone(&work.environment),
                    frames: Arc::clone(&work.frames),
                    continuations: work.continuations.clone(),
                })
            };
            let left_task = if path_candidate_with_aliases(bytecode, left, &work.continuations) {
                GeneratorTask::PathEval(PathWork {
                    node: left,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations: work.continuations,
                })
            } else {
                GeneratorTask::Eval(GeneratorWork {
                    node: left,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: work.environment,
                    frames: work.frames,
                    continuations: work.continuations,
                })
            };
            pending.push(right_task);
            pending.push(left_task);
            None
        }
        Operation::Optional(child) => {
            let boundary = *next_boundary;
            *next_boundary = next_boundary.saturating_add(1);
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::OptionalBoundary { boundary });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: child,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::Conditional {
            branches,
            alternative,
        } => {
            let Some(condition) = branches.first().map(|(condition, _)| *condition) else {
                return Some(Err(invalid("conditional has no alternative")));
            };
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathConditional {
                branches: Arc::from(branches),
                next: 0,
                alternative,
                input: work.input.clone(),
                origin: work.origin.clone(),
                prefix: work.prefix.clone(),
                environment: Arc::clone(&work.environment),
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: condition,
                input: work.input,
                origin: work.origin.clone(),
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::TryCatch { expression, catch } => {
            let boundary = *next_boundary;
            *next_boundary = next_boundary.saturating_add(1);
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathCatch {
                boundary,
                node: catch,
                origin: work.origin.clone(),
                environment: Arc::clone(&work.environment),
            });
            let task = if path_candidate_with_aliases(bytecode, expression, &continuations) {
                GeneratorTask::PathEval(PathWork {
                    node: expression,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                })
            } else {
                GeneratorTask::Eval(GeneratorWork {
                    node: expression,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                })
            };
            pending.push(task);
            None
        }
        Operation::ParameterCall {
            function,
            parameter,
        } => {
            let argument = work
                .frames
                .iter()
                .rev()
                .find(|frame| frame.symbol == function)
                .and_then(|frame| frame.filters.get(parameter as usize))
                .and_then(Clone::clone);
            let Some(argument) = argument else {
                return Some(Err(invalid(
                    "filter parameter missing from active user frame",
                )));
            };
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathUserReturn {
                environment: Arc::clone(&work.environment),
                frames: Arc::clone(&work.frames),
            });
            if path_candidate_with_aliases(bytecode, argument.node, &continuations) {
                pending.push(GeneratorTask::PathEval(PathWork {
                    node: argument.node,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: argument.environment,
                    frames: argument.frames,
                    continuations,
                }));
            } else {
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: argument.node,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: argument.environment,
                    frames: argument.frames,
                    continuations,
                }));
            }
            None
        }
        Operation::BindAlternatives {
            value,
            patterns,
            body,
        } => {
            if !path_bind_candidate(bytecode, body) {
                return Some(Err(VmError::Unsupported {
                    operation: "path bind alternatives body".into(),
                }));
            }
            let boundary = *next_boundary;
            *next_boundary = next_boundary.saturating_add(1);
            let patterns: Arc<[BindingPatternOperand]> = Arc::from(patterns);
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathBindAlternatives {
                boundary,
                patterns,
                body,
                input: work.input.clone(),
                prefix: work.prefix.clone(),
                environment: Arc::clone(&work.environment),
            });
            if path_candidate_with_aliases(bytecode, value, &continuations) {
                pending.push(GeneratorTask::PathEval(PathWork {
                    node: value,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                }));
            } else {
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: value,
                    input: work.input,
                    origin: work.origin.clone(),
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                }));
            }
            None
        }
        Operation::Bind {
            value,
            pattern,
            body,
        } => {
            if path_candidate_with_aliases(bytecode, value, &work.continuations) {
                let mut continuations = work.continuations;
                continuations.push(GeneratorContinuation::PathBind {
                    pattern,
                    body,
                    input: work.input.clone(),
                    input_origin: work.origin.clone(),
                    prefix: work.prefix.clone(),
                    environment: Arc::clone(&work.environment),
                    entered_from_path: true,
                });
                pending.push(GeneratorTask::PathEval(PathWork {
                    node: value,
                    input: work.input,
                    origin: work.origin.clone(),
                    prefix: work.prefix,
                    environment: work.environment,
                    frames: work.frames,
                    continuations,
                }));
                return None;
            }
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathBindValue {
                pattern,
                body,
                input: work.input.clone(),
                input_origin: work.origin.clone(),
                prefix: work.prefix,
                environment: Arc::clone(&work.environment),
                entered_from_path: true,
            });
            pending.push(GeneratorTask::Eval(GeneratorWork {
                node: value,
                input: work.input,
                origin: work.origin.clone(),
                environment: work.environment,
                frames: work.frames,
                continuations,
            }));
            None
        }
        Operation::UserCall { symbol, arguments } => {
            let Some(function) = bytecode.functions().get(symbol as usize) else {
                return Some(Err(invalid("user function missing after validation")));
            };
            if arguments.len() != function.parameters.len() {
                return Some(Err(invalid("user function arity changed after validation")));
            }
            let filters = arguments
                .iter()
                .zip(&function.parameters)
                .map(|(node, parameter)| {
                    (parameter.kind == ParameterKind::Filter).then(|| FilterArgument {
                        node: *node,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    })
                })
                .collect::<Vec<_>>();
            if let Some(next) = function
                .parameters
                .iter()
                .position(|parameter| parameter.kind == ParameterKind::Value)
            {
                let arguments: Arc<[u32]> = Arc::from(arguments);
                let from_path =
                    path_candidate_with_aliases(bytecode, arguments[next], &work.continuations);
                let mut continuations = work.continuations;
                continuations.push(GeneratorContinuation::PathUserArgument {
                    symbol,
                    arguments: Arc::clone(&arguments),
                    next,
                    input: work.input.clone(),
                    input_origin: work.origin.clone(),
                    prefix: work.prefix.clone(),
                    caller_environment: Arc::clone(&work.environment),
                    caller_frames: Arc::clone(&work.frames),
                    filters,
                    bindings: LexicalEnvironment::new(),
                    from_path,
                });
                if from_path {
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: arguments[next],
                        input: work.input,
                        origin: work.origin.clone(),
                        prefix: work.prefix,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                } else {
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: arguments[next],
                        input: work.input,
                        origin: work.origin.clone(),
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                }
                return None;
            }
            let mut frames = work.frames.to_vec();
            frames.push(UserFrame { symbol, filters });
            let mut continuations = work.continuations;
            continuations.push(GeneratorContinuation::PathUserReturn {
                environment: Arc::clone(&work.environment),
                frames: Arc::clone(&work.frames),
            });
            pending.push(GeneratorTask::PathEval(PathWork {
                node: function.body,
                input: work.input,
                origin: work.origin.clone(),
                prefix: work.prefix,
                environment: work.environment,
                frames: frames.into(),
                continuations,
            }));
            None
        }
        Operation::Call { name, arguments } => {
            let Some(name) = bytecode.string(name) else {
                return Some(Err(invalid("path call name missing after validation")));
            };
            match name.as_ref() {
                "getpath" => {
                    let Some(argument) = arguments.first() else {
                        return Some(Err(invalid("getpath argument missing")));
                    };
                    if arguments.len() != 1 {
                        return Some(Err(invalid("getpath arity")));
                    }
                    let selected = match getpath(&work.input, work.prefix.components()) {
                        Ok(value) => value,
                        Err(error) => return Some(Err(error)),
                    };
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathGetPath {
                        base: work.prefix,
                        input: selected.clone(),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: *argument,
                        input: selected,
                        origin: None,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                    None
                }
                "select" => {
                    let Some(argument) = arguments.first() else {
                        return Some(Err(invalid("select argument missing")));
                    };
                    let selected = match getpath(&work.input, work.prefix.components()) {
                        Ok(value) => value,
                        Err(error) => return Some(Err(error)),
                    };
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathSelect(work.prefix));
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: *argument,
                        input: selected,
                        origin: None,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                    None
                }
                "error" => {
                    let Some(argument) = arguments.first() else {
                        return Some(Err(raised(work.input)));
                    };
                    let selected = match getpath(&work.input, work.prefix.components()) {
                        Ok(value) => value,
                        Err(error) => return Some(Err(error)),
                    };
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::Raise);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: *argument,
                        input: selected,
                        origin: None,
                        environment: work.environment,
                        frames: work.frames,
                        continuations,
                    }));
                    None
                }
                _ => Some(Err(runtime(
                    "assignment left side is not a path".to_owned(),
                ))),
            }
        }
        _ => Some(Err(runtime(
            "assignment left side is not a path".to_owned(),
        ))),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "slice path scheduling carries the shared managed continuation state"
)]
fn schedule_path_slice(
    path: Path,
    target: Value,
    start: Option<f64>,
    end: Option<f64>,
    assignment: Option<Rc<RefCell<AssignmentState>>>,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    continuations: Vec<GeneratorContinuation>,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let Value::Array(values) = target else {
        return if matches!(target, Value::Null) {
            None
        } else {
            Some(Err(type_error("slice", &target)))
        };
    };
    let (start, end) = slice_bounds(values.len(), start, end);
    if start < end || assignment.is_some() {
        pending.push(GeneratorTask::PathSlice {
            path,
            start,
            end,
            next: start,
            assignment,
            environment,
            frames,
            continuations,
        });
    }
    None
}

#[allow(
    clippy::too_many_arguments,
    reason = "alternative binding scheduling carries the generator state needed for retry"
)]
fn schedule_bind_alternative(
    bytecode: &Bytecode,
    patterns: Arc<[BindingPatternOperand]>,
    mut next: usize,
    value: Value,
    input: Value,
    environment: Arc<LexicalEnvironment>,
    body: u32,
    frames: UserFrames,
    mut continuations: Vec<GeneratorContinuation>,
    boundary: u64,
    pending: &mut Vec<GeneratorTask>,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
    observations: &mut VmObservations,
    path_mode: bool,
    prefix: Path,
) -> Result<(), VmError> {
    while let Some(pattern) = patterns.get(next) {
        let mut nested = environment.as_ref().clone();
        initialize_pattern_variables(bytecode, &patterns, &mut nested)?;
        let pattern_depth = continuations.len();
        let mut charge = |depth| {
            if stop.load(Ordering::Relaxed)
                || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                return Err(VmError::Interrupted);
            }
            if depth >= limits.call_stack {
                return Err(resource("call-stack"));
            }
            if observations.steps >= limits.steps {
                return Err(resource("vm-steps"));
            }
            observations.steps += 1;
            observations.call_stack_high_water = observations.call_stack_high_water.max(depth + 1);
            Ok(())
        };
        match bind_pattern(
            bytecode,
            pattern,
            &value,
            &mut nested,
            pattern_depth,
            &mut charge,
        ) {
            Ok(()) => {
                let body_input = input.clone();
                if path_mode {
                    continuations.push(GeneratorContinuation::PathBindAlternativeBody {
                        boundary,
                        patterns,
                        next: next.saturating_add(1),
                        body,
                        value,
                        input,
                        prefix: prefix.clone(),
                        environment,
                    });
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: body,
                        input: body_input,
                        origin: None,
                        prefix,
                        environment: Arc::new(nested),
                        frames,
                        continuations,
                    }));
                } else {
                    continuations.push(GeneratorContinuation::BindAlternativeBody {
                        boundary,
                        patterns,
                        next: next.saturating_add(1),
                        body,
                        value,
                        input,
                        environment,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: body,
                        input: body_input,
                        origin: None,
                        environment: Arc::new(nested),
                        frames,
                        continuations,
                    }));
                }
                return Ok(());
            }
            Err(error) => {
                if !is_pattern_mismatch(&error) {
                    return Err(error);
                }
                next = next.saturating_add(1);
                if next >= patterns.len() {
                    return Err(error);
                }
            }
        }
    }
    Err(runtime(
        "destructuring alternative has no patterns".to_owned(),
    ))
}

enum InterpolationWork {
    Expand {
        next: usize,
        pieces: Vec<Option<Arc<str>>>,
    },
    Error(VmError),
}

#[derive(Clone)]
struct TraversalCursor {
    frames: Vec<TraversalFrame>,
}

#[derive(Clone)]
struct TraversalFrame {
    value: Value,
    emitted: bool,
    next_child: usize,
}

impl TraversalCursor {
    fn new(value: Value) -> Self {
        Self {
            frames: vec![TraversalFrame {
                value,
                emitted: false,
                next_child: 0,
            }],
        }
    }

    fn next(&mut self, depth_limit: usize) -> Result<Option<Value>, VmError> {
        loop {
            let Some(frame) = self.frames.last_mut() else {
                return Ok(None);
            };
            if !frame.emitted {
                frame.emitted = true;
                return Ok(Some(frame.value.clone()));
            }
            let child = match &frame.value {
                Value::Array(values) => values.get(frame.next_child).cloned(),
                Value::Object(values) => values
                    .get_index(frame.next_child)
                    .map(|(_, value)| value.clone()),
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
            };
            if let Some(child) = child {
                frame.next_child = frame.next_child.saturating_add(1);
                if self.frames.len() >= depth_limit {
                    return Err(resource("path-stack"));
                }
                self.frames.push(TraversalFrame {
                    value: child,
                    emitted: false,
                    next_child: 0,
                });
            } else {
                self.frames.pop();
            }
        }
    }

    fn depth(&self) -> usize {
        self.frames.len()
    }
}

fn recursive_child(value: &Value, index: usize) -> Option<Value> {
    match value {
        Value::Array(values) => values.get(index).cloned(),
        Value::Object(values) => values.get_index(index).map(|(_, value)| value.clone()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn managed_subset_gap(bytecode: &Bytecode) -> Option<crate::Span> {
    let mut pending = vec![bytecode.root()];
    let mut seen = vec![false; bytecode.instructions().len()];
    while let Some(node) = pending.pop() {
        let Some(instruction) = bytecode.instructions().get(node as usize) else {
            return bytecode
                .instructions()
                .first()
                .map(|instruction| instruction.span);
        };
        if std::mem::replace(&mut seen[node as usize], true) {
            continue;
        }
        if !managed_operation_supported(bytecode, &instruction.operation) {
            return Some(instruction.span);
        }
        let Some(children) = operation_children(bytecode, &instruction.operation) else {
            return Some(instruction.span);
        };
        pending.extend(children);
    }
    None
}

pub(crate) fn user_execution_gap(bytecode: &Bytecode) -> Option<crate::Span> {
    reachable_user_call(bytecode).then(|| managed_subset_gap(bytecode))?
}

pub(crate) fn managed_execution(bytecode: &Bytecode) -> bool {
    managed_subset_gap(bytecode).is_none()
}

fn reachable_user_call(bytecode: &Bytecode) -> bool {
    let mut pending = vec![bytecode.root()];
    let mut seen = vec![false; bytecode.instructions().len()];
    while let Some(node) = pending.pop() {
        let Some(instruction) = bytecode.instructions().get(node as usize) else {
            continue;
        };
        if std::mem::replace(&mut seen[node as usize], true) {
            continue;
        }
        if matches!(instruction.operation, Operation::UserCall { .. }) {
            return true;
        }
        if let Some(children) = operation_children(bytecode, &instruction.operation) {
            pending.extend(children);
        }
    }
    false
}

#[allow(
    clippy::too_many_lines,
    reason = "managed admission is the exhaustive capability table"
)]
fn managed_operation_supported(bytecode: &Bytecode, operation: &Operation) -> bool {
    match operation {
        Operation::Identity
        | Operation::Literal(_)
        | Operation::Variable(_)
        | Operation::Empty
        | Operation::RecursiveDescent
        | Operation::Label { .. }
        | Operation::Break(_)
        | Operation::ParameterCall { .. }
        | Operation::AccessField { .. }
        | Operation::Iterate(_)
        | Operation::Optional(_)
        | Operation::Array(_)
        | Operation::Object(_)
        | Operation::Unary { .. }
        | Operation::AccessIndex { .. }
        | Operation::Slice { .. }
        | Operation::Pipe { .. }
        | Operation::Comma { .. }
        | Operation::Binary { .. }
        | Operation::Conditional { .. }
        | Operation::Bind { .. }
        | Operation::BindAlternatives { .. }
        | Operation::Reduce { .. }
        | Operation::Foreach { .. }
        | Operation::Assignment { .. }
        | Operation::UserCall { .. }
        | Operation::TryCatch { .. }
        | Operation::Interpolation(_) => true,
        Operation::Call { name, arguments } => bytecode.string(*name).is_some_and(|name| {
            regex_builtin_shape(name, arguments.len()).is_some()
                || builtin_argument_order(name, arguments.len()).is_some()
                || (matches!(name.as_ref(), "IN" | "INDEX") && (1..=2).contains(&arguments.len()))
                || (name.as_ref() == "JOIN" && (2..=4).contains(&arguments.len()))
                || (name.as_ref() == "empty" && arguments.is_empty())
                || (name.as_ref() == "error" && arguments.len() <= 1)
                || ((matches!(
                    name.as_ref(),
                    "map"
                        | "map_values"
                        | "select"
                        | "sort_by"
                        | "unique_by"
                        | "group_by"
                        | "min_by"
                        | "max_by"
                        | "any"
                        | "all"
                        | "with_entries"
                        | "add"
                        | "has"
                        | "in"
                        | "first"
                        | "last"
                        | "isempty"
                        | "nth"
                        | "skip"
                        | "fromstream"
                        | "truncate_stream"
                        | "repeat"
                        | "walk"
                        | "range"
                ) && arguments.len() == 1)
                    || (matches!(name.as_ref(), "any" | "all") && arguments.len() <= 2)
                    || (matches!(name.as_ref(), "first" | "last") && arguments.is_empty())
                    || (matches!(name.as_ref(), "nth" | "skip")
                        && (1..=2).contains(&arguments.len()))
                    || (matches!(name.as_ref(), "while" | "until") && arguments.len() == 2))
                || (name.as_ref() == "limit" && arguments.len() == 2)
                || (name.as_ref() == "recurse" && arguments.len() <= 2)
                || (matches!(name.as_ref(), "path" | "pick" | "del") && arguments.len() == 1)
                || (name.as_ref() == "paths" && arguments.len() <= 1)
                || (arguments.is_empty()
                    && matches!(
                        name.as_ref(),
                        "arrays"
                            | "add"
                            | "booleans"
                            | "keys"
                            | "keys_unsorted"
                            | "length"
                            | "iterables"
                            | "max"
                            | "min"
                            | "modulemeta"
                            | "nulls"
                            | "numbers"
                            | "objects"
                            | "scalars"
                            | "strings"
                            | "to_entries"
                            | "from_entries"
                            | "tonumber"
                            | "values"
                            | "reverse"
                            | "sort"
                            | "tostring"
                            | "type"
                            | "unique"
                            | "utf8bytelength"
                            | "@base64"
                            | "@base64d"
                            | "@csv"
                            | "@html"
                            | "@json"
                            | "@sh"
                            | "@text"
                            | "@tsv"
                            | "@uri"
                    ))
                || (matches!(
                    name.as_ref(),
                    "debug" | "input" | "inputs" | "input_filename" | "input_line_number"
                ) && arguments.is_empty())
                || (name.as_ref() == "debug" && arguments.len() == 1)
                || (name.as_ref() == "stderr" && arguments.is_empty())
                || (name.as_ref() == "halt" && arguments.is_empty())
                || (name.as_ref() == "halt_error" && arguments.len() <= 1)
        }),
        _ => false,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the child-edge table exhaustively covers bytecode operations"
)]
fn operation_children(bytecode: &Bytecode, operation: &Operation) -> Option<Vec<u32>> {
    let mut children = Vec::new();
    match operation {
        Operation::AccessField { base, .. }
        | Operation::Iterate(base)
        | Operation::Optional(base)
        | Operation::Array(base)
        | Operation::Unary { child: base, .. } => children.push(*base),
        Operation::Label { body, .. } => children.push(*body),
        Operation::AccessIndex { base, index } => children.extend([*base, *index]),
        Operation::Slice { base, start, end } => {
            children.push(*base);
            children.extend([*start, *end].into_iter().flatten());
        }
        Operation::Pipe { left, right }
        | Operation::Comma { left, right }
        | Operation::Binary { left, right, .. } => children.extend([*left, *right]),
        Operation::Object(entries) => {
            for entry in entries {
                if let KeyOperand::Computed(key) = entry.key {
                    children.push(key);
                }
                children.push(entry.value);
            }
        }
        Operation::Conditional {
            branches,
            alternative,
        } => {
            for (condition, body) in branches {
                children.extend([*condition, *body]);
            }
            children.push(*alternative);
        }
        Operation::Bind { value, body, .. } | Operation::BindAlternatives { value, body, .. } => {
            children.extend([*value, *body]);
        }
        Operation::UserCall { symbol, arguments } => {
            children.extend(arguments.iter().copied());
            children.push(bytecode.functions().get(*symbol as usize)?.body);
        }
        Operation::Reduce {
            generator,
            initial,
            update,
            ..
        } => children.extend([*generator, *initial, *update]),
        Operation::Foreach {
            generator,
            initial,
            update,
            extract,
            ..
        } => {
            children.extend([*generator, *initial, *update]);
            if let Some(extract) = extract {
                children.push(*extract);
            }
        }
        Operation::Call { arguments, .. } => children.extend(arguments.iter().copied()),
        Operation::TryCatch { expression, catch } => {
            children.push(*expression);
            children.extend(catch.iter().copied());
        }
        Operation::Interpolation(segments) => {
            children.extend(segments.iter().filter_map(|segment| match segment {
                InterpolationOperand::Literal(_) => None,
                InterpolationOperand::Expression(expression) => Some(*expression),
            }));
        }
        Operation::Assignment { path, value, .. } => children.extend([*path, *value]),
        Operation::LoadInput
        | Operation::LoadConstant(_)
        | Operation::Duplicate
        | Operation::Pop
        | Operation::Jump(_)
        | Operation::Branch { .. }
        | Operation::Fork(_)
        | Operation::Backtrack
        | Operation::Return
        | Operation::Raise(_)
        | Operation::Catch(_)
        | Operation::EndCatch
        | Operation::Identity
        | Operation::Literal(_)
        | Operation::Variable(_)
        | Operation::Empty
        | Operation::RecursiveDescent
        | Operation::Break(_)
        | Operation::ParameterCall { .. } => {}
    }
    Some(children)
}

fn builtin_argument_order(name: &str, arity: usize) -> Option<Arc<[usize]>> {
    let supported = scalar::supports(name, arity)
        || scalar::supports_ambient(name, arity)
        || (name == "range" && (1..=3).contains(&arity))
        || (name == "combinations" && arity <= 1)
        || (name == "tostream" && arity == 0);
    if !supported {
        return None;
    }
    let order = if arity > 1 && scalar::supports(name, arity) {
        (0..arity).rev().collect::<Vec<_>>()
    } else {
        (0..arity).collect::<Vec<_>>()
    };
    Some(order.into())
}

fn regex_builtin_shape(name: &str, arity: usize) -> Option<(Arc<[usize]>, Option<usize>)> {
    match name {
        "test" | "match" | "capture" | "scan" | "split" | "splits" if arity == 1 => {
            Some((Arc::from([0]), None))
        }
        "test" | "match" | "capture" if arity == 2 => Some((Arc::from([1, 0]), None)),
        "scan" | "split" | "splits" if arity == 2 => Some((Arc::from([0, 1]), None)),
        "sub" | "gsub" if arity == 2 => Some((Arc::from([0]), Some(1))),
        "sub" | "gsub" if arity == 3 => Some((Arc::from([0, 2]), Some(1))),
        _ => None,
    }
}

fn scalar_result_origin(
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

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the explicit generator loop keeps fork and continuation state in one auditable place"
)]
fn evaluate_generator_stream(
    bytecode: &Bytecode,
    input: &Value,
    variables: &Environment,
    limits: VmLimits,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
    input_cursor: Option<&InputCursor>,
    effects: &Arc<EffectState>,
    mut emit: impl FnMut(Result<Value, VmError>, VmObservations) -> bool,
) -> VmObservations {
    let environment = Arc::new(LexicalEnvironment::from_values(variables.clone()));
    let mut next_origin = 1_u64;
    let mut pending = vec![GeneratorTask::Eval(GeneratorWork {
        node: bytecode.root(),
        input: input.clone(),
        origin: Some(OriginToken {
            root: 0,
            path: Path::root(),
        }),
        environment,
        frames: Arc::from([]),
        continuations: Vec::new(),
    })];
    let mut observations = VmObservations::default();
    let mut next_boundary = 0_u64;

    'generator: while let Some(task) = pending.pop() {
        if stop.load(Ordering::Relaxed)
            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            let _ = emit(Err(VmError::Interrupted), observations);
            break;
        }
        if observations.steps >= limits.steps {
            let _ = emit(Err(resource("vm-steps")), observations);
            break;
        }
        if limits.value_stack == 0 {
            let _ = emit(Err(resource("value-stack")), observations);
            break;
        }
        observations.steps += 1;
        let mut delivered_from_path = false;
        let (mut work, delivered) = match task {
            GeneratorTask::Eval(work) => (work, None),
            GeneratorTask::InvokeBuiltin {
                name,
                arguments,
                source_arity,
                input,
                origin,
                environment,
                frames,
                continuations,
            } => match generator::start(&name, &input, &arguments, source_arity, limits) {
                Ok(Some(state)) => {
                    pending.push(GeneratorTask::BuiltinGenerator {
                        state,
                        environment,
                        frames,
                        continuations,
                    });
                    continue;
                }
                Err(error) => (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Err(error)),
                ),
                Ok(None) => {
                    let mut charge = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if observations.steps >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        observations.steps += 1;
                        Ok(())
                    };
                    let (scalar_result, getpath_path): (
                        Result<Option<Value>, VmError>,
                        Option<Option<Path>>,
                    ) = if name.as_ref() == "getpath" {
                        match arguments.first() {
                            None => (Err(invalid("getpath argument missing")), Some(None)),
                            Some(path_value) => {
                                match scalar::bounded_getpath_resolution(
                                    &input,
                                    path_value,
                                    limits,
                                    &mut charge,
                                ) {
                                    Ok(resolution) => {
                                        (Ok(Some(resolution.value)), Some(resolution.path))
                                    }
                                    Err(error) => (Err(error), Some(None)),
                                }
                            }
                        }
                    } else {
                        let result = if scalar::supports_ambient(&name, arguments.len()) {
                            scalar::evaluate_ambient(
                                &name,
                                &input,
                                &arguments,
                                &environment,
                                limits,
                                &mut charge,
                            )
                        } else {
                            scalar::evaluate(&name, &input, &arguments, limits, &mut charge)
                        };
                        (result, None)
                    };
                    match scalar_result {
                        Ok(Some(value)) => {
                            let origin_result = match getpath_path {
                                Some(Some(path)) => origin_at_path_bounded(
                                    origin.as_ref(),
                                    &path,
                                    limits,
                                    &mut charge,
                                ),
                                Some(None) => fresh_origin(&mut next_origin).map(Some),
                                None => scalar_result_origin(
                                    &name,
                                    &input,
                                    &arguments,
                                    origin,
                                    &mut next_origin,
                                ),
                            };
                            let origin = match origin_result {
                                Ok(origin) => origin,
                                Err(error) => {
                                    return {
                                        let _ = emit(Err(error), observations);
                                        observations
                                    };
                                }
                            };
                            (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Ok(value)),
                            )
                        }
                        Ok(None) => continue,
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    }
                }
            },
            GeneratorTask::FinishBuiltinArguments {
                name,
                source_arity,
                values,
                input,
                origin,
                environment,
                frames,
                continuations,
            } => {
                pending.push(GeneratorTask::InvokeBuiltin {
                    name,
                    arguments: values.take(),
                    source_arity,
                    input,
                    origin,
                    environment,
                    frames,
                    continuations,
                });
                continue;
            }
            GeneratorTask::SqlInFinish {
                boundary: _,
                state,
                environment,
                frames,
                continuations,
            } => {
                let value = Value::Bool(state.borrow().matched);
                let (origin, result) = match fresh_origin(&mut next_origin) {
                    Ok(origin) => (Some(origin), Ok(value)),
                    Err(error) => (None, Err(error)),
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(result),
                )
            }
            GeneratorTask::SqlIndexFinish {
                boundary: _,
                state,
                environment,
                frames,
                continuations,
            } => {
                let result = state
                    .borrow_mut()
                    .builder
                    .take()
                    .map(sql::SqlIndexBuilder::finish)
                    .ok_or_else(|| invalid("SQL INDEX finished more than once"));
                let (origin, result) = match result.and_then(|value| {
                    fresh_origin(&mut next_origin).map(|origin| (Some(origin), Ok(value)))
                }) {
                    Ok((origin, result)) => (origin, result),
                    Err(error) => (None, Err(error)),
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(result),
                )
            }
            GeneratorTask::SqlJoinFinish {
                boundary: _,
                state,
                environment,
                frames,
                continuations,
            } => {
                let result = state
                    .borrow_mut()
                    .collector
                    .take()
                    .map(sql::SqlPairCollector::finish)
                    .ok_or_else(|| invalid("SQL JOIN finished more than once"));
                let (origin, result) = match result.and_then(|value| {
                    fresh_origin(&mut next_origin).map(|origin| (Some(origin), Ok(value)))
                }) {
                    Ok((origin, result)) => (origin, result),
                    Err(error) => (None, Err(error)),
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(result),
                )
            }
            GeneratorTask::InvokeRegex {
                name,
                arguments,
                replacement,
                input,
                origin: _origin,
                environment,
                frames,
                continuations,
            } => 'regex: {
                let flags_index = usize::from(matches!(name.as_ref(), "sub" | "gsub")) + 1;
                let Some(pattern_value) = arguments.first() else {
                    break 'regex regex_error_work(
                        bytecode,
                        &environment,
                        &frames,
                        &continuations,
                        invalid("regex pattern argument missing"),
                    );
                };
                let array_allowed = matches!(name.as_ref(), "test" | "match" | "capture")
                    && arguments.get(flags_index).is_none();
                let (pattern, array_flags) =
                    match regex_pattern_argument(&name, pattern_value, array_allowed) {
                        Ok(parsed) => parsed,
                        Err(error) => {
                            break 'regex regex_error_work(
                                bytecode,
                                &environment,
                                &frames,
                                &continuations,
                                error,
                            );
                        }
                    };
                let flags = match array_flags {
                    Some(flags) => Ok(flags),
                    None => match arguments.get(flags_index) {
                        None | Some(Value::Null) => Ok(Arc::from("")),
                        Some(Value::String(flags)) => Ok(Arc::clone(flags)),
                        Some(value) => Err(type_error(&name, value)),
                    },
                };
                let flags = match flags {
                    Ok(flags) => flags,
                    Err(error) => {
                        break 'regex regex_error_work(
                            bytecode,
                            &environment,
                            &frames,
                            &continuations,
                            error,
                        );
                    }
                };
                let Value::String(input) = &input else {
                    break 'regex regex_error_work(
                        bytecode,
                        &environment,
                        &frames,
                        &continuations,
                        type_error(&name, &input),
                    );
                };
                let checkpoint_steps = Cell::new(observations.steps);
                let checkpoint = || {
                    if stop.load(Ordering::Relaxed)
                        || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                    {
                        return Err(VmError::Interrupted);
                    }
                    if checkpoint_steps.get() >= limits.steps {
                        return Err(resource("vm-steps"));
                    }
                    checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                    Ok(())
                };
                match name.as_ref() {
                    "test" => {
                        let test = stdlib::regex_test(input, &pattern, &flags, limits, &checkpoint);
                        observations.steps = checkpoint_steps.get();
                        let value = match test {
                            Ok(value) => value,
                            Err(error) => {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        };
                        let origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        };
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Ok(value)),
                        )
                    }
                    "match" | "capture" | "scan" => {
                        let kind = match name.as_ref() {
                            "match" => stdlib::RegexPullKind::Match,
                            "capture" => stdlib::RegexPullKind::Capture,
                            _ => stdlib::RegexPullKind::Scan,
                        };
                        match stdlib::regex_cursor(
                            Arc::clone(input),
                            &pattern,
                            &flags,
                            kind,
                            name.as_ref() == "scan",
                            limits,
                        ) {
                            Ok(cursor) => {
                                pending.push(GeneratorTask::RegexCursor {
                                    cursor: Rc::new(RefCell::new(cursor)),
                                    environment,
                                    frames,
                                    continuations,
                                });
                                continue 'generator;
                            }
                            Err(error) => {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        }
                    }
                    "splits" => {
                        match stdlib::regex_cursor(
                            Arc::clone(input),
                            &pattern,
                            &flags,
                            stdlib::RegexPullKind::Span,
                            true,
                            limits,
                        ) {
                            Ok(cursor) => {
                                pending.push(GeneratorTask::RegexSplitCursor {
                                    cursor: Rc::new(RefCell::new(cursor)),
                                    input: Arc::clone(input),
                                    copied: 0,
                                    emitted_tail: false,
                                    environment,
                                    frames,
                                    continuations,
                                });
                                continue 'generator;
                            }
                            Err(error) => {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        }
                    }
                    "split" => {
                        let split = stdlib::regex_split(
                            input,
                            &pattern,
                            &flags,
                            false,
                            arguments.get(flags_index).is_none(),
                            limits,
                            &checkpoint,
                        );
                        observations.steps = checkpoint_steps.get();
                        match split {
                            Ok(values) => {
                                let origin = match fresh_origin(&mut next_origin) {
                                    Ok(origin) => Some(origin),
                                    Err(error) => {
                                        break 'regex regex_error_work(
                                            bytecode,
                                            &environment,
                                            &frames,
                                            &continuations,
                                            error,
                                        );
                                    }
                                };
                                (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin,
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(Ok(values
                                        .into_iter()
                                        .next()
                                        .unwrap_or_else(|| Value::array(Vec::new())))),
                                )
                            }
                            Err(error) => {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        }
                    }
                    "sub" | "gsub" => {
                        let Some(replacement) = replacement else {
                            break 'regex regex_error_work(
                                bytecode,
                                &environment,
                                &frames,
                                &continuations,
                                invalid("regex replacement argument missing"),
                            );
                        };
                        match stdlib::regex_substitution(
                            Arc::clone(input),
                            &pattern,
                            &flags,
                            name.as_ref() == "gsub",
                            limits,
                        ) {
                            Ok(state) => {
                                pending.push(GeneratorTask::RegexSubstitution {
                                    name: Arc::clone(&name),
                                    state: Rc::new(RefCell::new(state)),
                                    replacement,
                                    environment,
                                    frames,
                                    continuations,
                                });
                                continue 'generator;
                            }
                            Err(error) => {
                                break 'regex regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        }
                    }
                    _ => unreachable!("regex dispatch is exhaustive"),
                }
            }
            GeneratorTask::RegexCursor {
                cursor,
                environment,
                frames,
                continuations,
            } => {
                let checkpoint_steps = Cell::new(observations.steps);
                let checkpoint = || {
                    if stop.load(Ordering::Relaxed)
                        || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                    {
                        return Err(VmError::Interrupted);
                    }
                    if checkpoint_steps.get() >= limits.steps {
                        return Err(resource("vm-steps"));
                    }
                    checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                    Ok(())
                };
                let pull = cursor.borrow_mut().next(&checkpoint);
                observations.steps = checkpoint_steps.get();
                match pull {
                    Ok(Some(pull)) => {
                        let value = match pull {
                            stdlib::RegexPull::Match { value }
                            | stdlib::RegexPull::Capture { value }
                            | stdlib::RegexPull::Scan { value } => value,
                            stdlib::RegexPull::Replacement { .. }
                            | stdlib::RegexPull::Span { .. } => {
                                let _ = emit(
                                    Err(invalid("unexpected regex cursor result")),
                                    observations,
                                );
                                return observations;
                            }
                        };
                        let origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                let _ = emit(Err(error), observations);
                                return observations;
                            }
                        };
                        pending.push(GeneratorTask::RegexCursor {
                            cursor,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Ok(value)),
                        )
                    }
                    Ok(None) => continue,
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                }
            }
            GeneratorTask::RegexSplitCursor {
                cursor,
                input,
                copied,
                emitted_tail,
                environment,
                frames,
                continuations,
            } => {
                if emitted_tail {
                    continue;
                }
                let checkpoint_steps = Cell::new(observations.steps);
                let checkpoint = || {
                    if stop.load(Ordering::Relaxed)
                        || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                    {
                        return Err(VmError::Interrupted);
                    }
                    if checkpoint_steps.get() >= limits.steps {
                        return Err(resource("vm-steps"));
                    }
                    checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                    Ok(())
                };
                let pull = cursor.borrow_mut().next(&checkpoint);
                observations.steps = checkpoint_steps.get();
                match pull {
                    Ok(Some(stdlib::RegexPull::Span { start, end })) => 'split: {
                        let value = match stdlib::regex_split_piece(
                            &input[copied..start],
                            limits,
                            &checkpoint,
                        ) {
                            Ok(value) => value,
                            Err(error) => {
                                observations.steps = checkpoint_steps.get();
                                break 'split regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        };
                        observations.steps = checkpoint_steps.get();
                        match fresh_origin(&mut next_origin) {
                            Ok(origin) => {
                                pending.push(GeneratorTask::RegexSplitCursor {
                                    cursor,
                                    input: Arc::clone(&input),
                                    copied: end,
                                    emitted_tail: false,
                                    environment: Arc::clone(&environment),
                                    frames: Arc::clone(&frames),
                                    continuations: continuations.clone(),
                                });
                                (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin: Some(origin),
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(Ok(value)),
                                )
                            }
                            Err(error) => regex_error_work(
                                bytecode,
                                &environment,
                                &frames,
                                &continuations,
                                error,
                            ),
                        }
                    }
                    Ok(None) => 'split: {
                        let value = match stdlib::regex_split_piece(
                            &input[copied..],
                            limits,
                            &checkpoint,
                        ) {
                            Ok(value) => value,
                            Err(error) => {
                                observations.steps = checkpoint_steps.get();
                                break 'split regex_error_work(
                                    bytecode,
                                    &environment,
                                    &frames,
                                    &continuations,
                                    error,
                                );
                            }
                        };
                        observations.steps = checkpoint_steps.get();
                        match fresh_origin(&mut next_origin) {
                            Ok(origin) => {
                                pending.push(GeneratorTask::RegexSplitCursor {
                                    cursor,
                                    input: Arc::clone(&input),
                                    copied,
                                    emitted_tail: true,
                                    environment: Arc::clone(&environment),
                                    frames: Arc::clone(&frames),
                                    continuations: continuations.clone(),
                                });
                                (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin: Some(origin),
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(Ok(value)),
                                )
                            }
                            Err(error) => regex_error_work(
                                bytecode,
                                &environment,
                                &frames,
                                &continuations,
                                error,
                            ),
                        }
                    }
                    Ok(Some(_)) => regex_error_work(
                        bytecode,
                        &environment,
                        &frames,
                        &continuations,
                        invalid("unexpected regex split cursor result"),
                    ),
                    Err(error) => {
                        regex_error_work(bytecode, &environment, &frames, &continuations, error)
                    }
                }
            }
            GeneratorTask::RegexSubstitution {
                name,
                state,
                replacement,
                environment,
                frames,
                continuations,
            } => {
                let checkpoint_steps = Cell::new(observations.steps);
                let checkpoint = || {
                    if stop.load(Ordering::Relaxed)
                        || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                    {
                        return Err(VmError::Interrupted);
                    }
                    if checkpoint_steps.get() >= limits.steps {
                        return Err(resource("vm-steps"));
                    }
                    checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                    Ok(())
                };
                let next = state.borrow_mut().next(&checkpoint);
                observations.steps = checkpoint_steps.get();
                match next {
                    Ok(Some(stdlib::RegexSubstitutionEvent::Replace { context })) => {
                        pending.push(GeneratorTask::RegexReplacementFinish {
                            name: Arc::clone(&name),
                            state: Rc::clone(&state),
                            replacement,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        let mut replacement_continuations = continuations;
                        replacement_continuations.push(
                            GeneratorContinuation::RegexReplacementItem {
                                name: Arc::clone(&name),
                                state,
                            },
                        );
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: replacement,
                            input: context,
                            origin: None,
                            environment,
                            frames,
                            continuations: replacement_continuations,
                        }));
                        continue;
                    }
                    Ok(Some(stdlib::RegexSubstitutionEvent::Output(value))) => {
                        let origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                let _ = emit(Err(error), observations);
                                return observations;
                            }
                        };
                        pending.push(GeneratorTask::RegexSubstitution {
                            name,
                            state,
                            replacement,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Ok(value)),
                        )
                    }
                    Ok(None) => continue,
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                }
            }
            GeneratorTask::RegexReplacementFinish {
                name,
                state,
                replacement,
                environment,
                frames,
                continuations,
            } => {
                let checkpoint_steps = Cell::new(observations.steps);
                let checkpoint = || {
                    if stop.load(Ordering::Relaxed)
                        || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                    {
                        return Err(VmError::Interrupted);
                    }
                    if checkpoint_steps.get() >= limits.steps {
                        return Err(resource("vm-steps"));
                    }
                    checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                    Ok(())
                };
                let finish = state.borrow_mut().finish_replacement_stream(&checkpoint);
                observations.steps = checkpoint_steps.get();
                match finish {
                    Ok(()) => {
                        pending.push(GeneratorTask::RegexSubstitution {
                            name,
                            state,
                            replacement,
                            environment,
                            frames,
                            continuations,
                        });
                        continue;
                    }
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                }
            }
            GeneratorTask::BuiltinGenerator {
                mut state,
                environment,
                frames,
                continuations,
                ..
            } => {
                let mut charge = || {
                    if stop.load(Ordering::Relaxed)
                        || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                    {
                        return Err(VmError::Interrupted);
                    }
                    if observations.steps >= limits.steps {
                        return Err(resource("vm-steps"));
                    }
                    observations.steps += 1;
                    Ok(())
                };
                match state.next(&mut charge) {
                    Ok(Some(value)) => match fresh_origin(&mut next_origin) {
                        Ok(origin) => {
                            pending.push(GeneratorTask::BuiltinGenerator {
                                state,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                                continuations: continuations.clone(),
                            });
                            (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin: Some(origin),
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Ok(value)),
                            )
                        }
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    },
                    Ok(None) => continue,
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                }
            }
            GeneratorTask::Iterate {
                values,
                next,
                environment,
                frames,
                continuations,
            } => {
                let Some(value) = values.get(next).cloned() else {
                    continue;
                };
                if next + 1 < values.len() {
                    pending.push(GeneratorTask::Iterate {
                        values,
                        next: next + 1,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                }
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(value)),
                )
            }
            GeneratorTask::IterateCursor {
                mut cursor,
                origin,
                environment,
                frames,
                continuations,
            } => {
                let mut charge =
                    || charge_managed_step(&mut observations, limits, cancellation, stop);
                match cursor.next_child(&mut charge) {
                    Ok(Some((value, component))) => {
                        match origin
                            .as_ref()
                            .map(|origin| origin.child_bounded(component, limits, &mut charge))
                            .transpose()
                        {
                            Ok(child_origin) => {
                                pending.push(GeneratorTask::IterateCursor {
                                    cursor,
                                    origin,
                                    environment: Arc::clone(&environment),
                                    frames: Arc::clone(&frames),
                                    continuations: continuations.clone(),
                                });
                                (
                                    GeneratorWork {
                                        node: bytecode.root(),
                                        input: Value::Null,
                                        origin: child_origin,
                                        environment,
                                        frames,
                                        continuations,
                                    },
                                    Some(Ok(value)),
                                )
                            }
                            Err(error) => (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin: None,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(Err(error)),
                            ),
                        }
                    }
                    Ok(None) => continue,
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                }
            }
            GeneratorTask::InputValues {
                cursor,
                environment,
                frames,
                continuations,
            } => match cursor.next_value() {
                Ok(Some(value)) => match fresh_origin(&mut next_origin) {
                    Ok(origin) => {
                        pending.push(GeneratorTask::InputValues {
                            cursor,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: Some(origin),
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Ok(value)),
                        )
                    }
                    Err(error) => (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(error)),
                    ),
                },
                Ok(None) => continue,
                Err(error) => (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Err(error)),
                ),
            },
            GeneratorTask::FinishArray {
                values,
                environment,
                frames,
                continuations,
            } => {
                let (origin, result) = match fresh_origin(&mut next_origin) {
                    Ok(origin) => (Some(origin), Ok(Value::array(values.take()))),
                    Err(error) => (None, Err(error)),
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(result),
                )
            }
            GeneratorTask::MapArray {
                inputs,
                next,
                argument,
                values,
                first_only,
                environment,
                frames,
                continuations,
            } => {
                if let Some(value) = inputs.get(next) {
                    pending.push(GeneratorTask::MapArray {
                        inputs: Arc::clone(&inputs),
                        next: next + 1,
                        argument,
                        values: Rc::clone(&values),
                        first_only,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut callback_continuations = continuations;
                    if first_only {
                        callback_continuations.push(GeneratorContinuation::IgnoreError);
                        callback_continuations.push(GeneratorContinuation::ArrayUpdateItem {
                            values,
                            accepted: Rc::new(Cell::new(false)),
                        });
                    } else {
                        callback_continuations.push(GeneratorContinuation::ArrayItem(values));
                    }
                    (
                        GeneratorWork {
                            node: argument,
                            input: value.clone(),
                            origin: None,
                            environment,
                            frames,
                            continuations: callback_continuations,
                        },
                        None,
                    )
                } else {
                    let (origin, result) = match fresh_origin(&mut next_origin) {
                        Ok(origin) => (Some(origin), Ok(Value::array(values.take()))),
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
            }
            GeneratorTask::MapObject {
                entries,
                next,
                argument,
                values,
                environment,
                frames,
                continuations,
            } => {
                if let Some((key, value)) = entries.get(next) {
                    pending.push(GeneratorTask::MapObject {
                        entries: Arc::clone(&entries),
                        next: next + 1,
                        argument,
                        values: Rc::clone(&values),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut callback_continuations = continuations;
                    callback_continuations.push(GeneratorContinuation::IgnoreError);
                    callback_continuations.push(GeneratorContinuation::ObjectItem {
                        key: Arc::clone(key),
                        values,
                        accepted: Rc::new(Cell::new(false)),
                    });
                    (
                        GeneratorWork {
                            node: argument,
                            input: value.clone(),
                            origin: None,
                            environment,
                            frames,
                            continuations: callback_continuations,
                        },
                        None,
                    )
                } else {
                    let output = std::mem::take(&mut *values.borrow_mut());
                    let (origin, result) = match fresh_origin(&mut next_origin) {
                        Ok(origin) => (Some(origin), Ok(Value::object(output))),
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
            }
            GeneratorTask::CollectSortKeys {
                values,
                next,
                argument,
                keyed_values,
                mode,
                origin,
                environment,
                frames,
                continuations,
            } => {
                if let Some(value) = values.get(next) {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let callback_origin = origin
                        .as_ref()
                        .map(|origin| {
                            origin.child_bounded(PathComponent::Index(next), limits, &mut charge)
                        })
                        .transpose();
                    match callback_origin {
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                        Ok(callback_origin) => {
                            pending.push(GeneratorTask::CollectSortKeys {
                                values: Arc::clone(&values),
                                next: next + 1,
                                argument,
                                keyed_values: Rc::clone(&keyed_values),
                                mode,
                                origin: origin.clone(),
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                                continuations: continuations.clone(),
                            });
                            let keys = Rc::new(RefCell::new(Vec::new()));
                            pending.push(GeneratorTask::FinishSortKey {
                                input: value.clone(),
                                origin: callback_origin.clone(),
                                keys: Rc::clone(&keys),
                                keyed_values,
                                continuations: continuations.clone(),
                            });
                            let mut callback_continuations = continuations;
                            callback_continuations
                                .push(GeneratorContinuation::SortKeyItem(Rc::clone(&keys)));
                            (
                                GeneratorWork {
                                    node: argument,
                                    input: value.clone(),
                                    origin: callback_origin,
                                    environment,
                                    frames,
                                    continuations: callback_continuations,
                                },
                                None,
                            )
                        }
                    }
                } else {
                    let keyed_values = std::mem::take(&mut *keyed_values.borrow_mut());
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let (origin, result) = match project_managed_keyed_values(
                        mode,
                        keyed_values,
                        limits,
                        &mut charge,
                        &mut next_origin,
                    ) {
                        Ok((origin, output)) => (origin, Ok(output)),
                        Err(error) => (None, Err(error)),
                    };
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                }
            }
            GeneratorTask::FinishSortKey {
                input,
                origin,
                keys,
                keyed_values,
                continuations: _,
            } => {
                let keys = std::mem::take(&mut *keys.borrow_mut());
                keyed_values.borrow_mut().push(ManagedKeyedValue {
                    key: Value::array(keys),
                    value: input,
                    origin,
                });
                continue;
            }
            GeneratorTask::FinishAlternative {
                matched,
                right,
                input,
                origin,
                environment,
                frames,
                continuations,
            } => {
                if matched.get() {
                    continue;
                }
                (
                    GeneratorWork {
                        node: right,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    None,
                )
            }
            GeneratorTask::FinishPullConsumer {
                state,
                environment,
                frames,
                continuations,
            } => {
                let (result, origin) = {
                    let mut state = state.borrow_mut();
                    match state.kind {
                        PullConsumerKind::First | PullConsumerKind::Skip => (None, None),
                        PullConsumerKind::Last | PullConsumerKind::Nth => {
                            (state.latest.take().map(Ok), state.latest_origin.take())
                        }
                        PullConsumerKind::IsEmpty => (Some(Ok(Value::Bool(!state.seen))), None),
                        PullConsumerKind::Any | PullConsumerKind::All => (
                            Some(Ok(Value::Bool(
                                state
                                    .decision
                                    .unwrap_or(matches!(state.kind, PullConsumerKind::All)),
                            ))),
                            None,
                        ),
                    }
                };
                let Some(result) = result else {
                    continue;
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(result),
                )
            }
            GeneratorTask::FinishAdd {
                state,
                environment,
                frames,
                continuations,
            } => {
                let value = state.borrow().accumulator.clone().unwrap_or(Value::Null);
                let origin = match fresh_origin(&mut next_origin) {
                    Ok(origin) => Some(origin),
                    Err(error) => {
                        return {
                            let _ = emit(Err(error), observations);
                            observations
                        };
                    }
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(value)),
                )
            }
            GeneratorTask::FinishDebug {
                input,
                origin,
                environment,
                frames,
                continuations,
            } => (
                GeneratorWork {
                    node: bytecode.root(),
                    input: Value::Null,
                    origin,
                    environment,
                    frames,
                    continuations,
                },
                Some(Ok(input)),
            ),
            GeneratorTask::FoldGenerator {
                generator,
                pattern,
                update,
                extract,
                emit_each_update,
                state,
                input,
                environment,
                frames,
                continuations,
            } => {
                pending.push(GeneratorTask::FinishFold {
                    state: Rc::clone(&state),
                    emit_final: !emit_each_update,
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                    continuations: continuations.clone(),
                });
                let mut generator_continuations = continuations;
                generator_continuations.push(GeneratorContinuation::FoldItem {
                    state,
                    pattern,
                    update,
                    extract,
                    emit_each_update,
                    environment: Arc::clone(&environment),
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: generator,
                    input,
                    origin: None,
                    environment,
                    frames,
                    continuations: generator_continuations,
                }));
                continue;
            }
            GeneratorTask::FinishFold {
                state,
                emit_final,
                environment,
                frames,
                continuations,
            } => {
                if !emit_final {
                    continue;
                }
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(state.borrow().accumulator.clone())),
                )
            }
            GeneratorTask::PathEval(path_work) => {
                delivered_from_path = true;
                let environment = Arc::clone(&path_work.environment);
                let frames = Arc::clone(&path_work.frames);
                let origin = path_work.origin.clone();
                let continuations = path_work.continuations.clone();
                let Some(result) =
                    schedule_path_eval(bytecode, path_work, &mut pending, &mut next_boundary)
                else {
                    continue;
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(result),
                )
            }
            GeneratorTask::PathChildren {
                value,
                path,
                next,
                environment,
                frames,
                continuations,
            } => {
                delivered_from_path = true;
                let child = match &value {
                    Value::Array(values) => values
                        .get(next)
                        .cloned()
                        .map(|child| (PathComponent::Index(next), child)),
                    Value::Object(values) => values
                        .get_index(next)
                        .map(|(key, child)| (PathComponent::Key(Arc::clone(key)), child.clone())),
                    _ => None,
                };
                let Some((component, _)) = child else {
                    continue;
                };
                pending.push(GeneratorTask::PathChildren {
                    value,
                    path: path.clone(),
                    next: next.saturating_add(1),
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                    continuations: continuations.clone(),
                });
                let mut components = path.components().to_vec();
                components.push(component);
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(path_value(&Path::new(components)))),
                )
            }
            GeneratorTask::PathTraverse {
                mut cursor,
                environment,
                frames,
                continuations,
            } => {
                delivered_from_path = true;
                if cursor.depth() > limits.path_stack {
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(Err(resource("path-stack"))),
                    )
                } else {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    match cursor.next(limits.path_stack, &mut charge) {
                        Ok(Some(path)) => {
                            pending.push(GeneratorTask::PathTraverse {
                                cursor,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                                continuations: continuations.clone(),
                            });
                            let (origin, result) = match fresh_origin(&mut next_origin) {
                                Ok(origin) => (Some(origin), Ok(path_value(&path))),
                                Err(error) => (None, Err(error)),
                            };
                            (
                                GeneratorWork {
                                    node: bytecode.root(),
                                    input: Value::Null,
                                    origin,
                                    environment,
                                    frames,
                                    continuations,
                                },
                                Some(result),
                            )
                        }
                        Ok(None) => continue,
                        Err(error) => (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Err(error)),
                        ),
                    }
                }
            }
            GeneratorTask::FinishPathCollection {
                boundary: _,
                kind,
                input,
                paths,
                environment,
                frames,
                continuations,
            } => {
                let accumulator = std::mem::replace(
                    &mut *paths.borrow_mut(),
                    path_builtin::PathAccumulator::new(),
                );
                let mut charge =
                    || charge_managed_step(&mut observations, limits, cancellation, stop);
                let result = match kind {
                    PathCollectionKind::Delete => {
                        path_builtin::delete_paths_bounded(&input, accumulator, limits, &mut charge)
                    }
                    PathCollectionKind::Pick => {
                        path_builtin::pick_paths_bounded(&input, accumulator, limits, &mut charge)
                    }
                };
                let (origin, result) = match result {
                    Ok(value) => match fresh_origin(&mut next_origin) {
                        Ok(origin) => (Some(origin), Ok(value)),
                        Err(error) => (None, Err(error)),
                    },
                    Err(error) => (None, Err(error)),
                };
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(result),
                )
            }
            GeneratorTask::PathSlice {
                path,
                start,
                end,
                next,
                assignment,
                environment,
                frames,
                continuations,
            } => {
                delivered_from_path = true;
                if let Some(state) = assignment {
                    state
                        .borrow_mut()
                        .targets
                        .push(AssignmentTarget::Slice { path, start, end });
                    state.borrow_mut().target_origins.push(None);
                    continue;
                }
                if next >= end {
                    continue;
                }
                pending.push(GeneratorTask::PathSlice {
                    path: path.clone(),
                    start,
                    end,
                    next: next.saturating_add(1),
                    assignment: None,
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                    continuations: continuations.clone(),
                });
                let mut components = path.components().to_vec();
                components.push(PathComponent::Index(next));
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(path_value(&Path::new(components)))),
                )
            }
            GeneratorTask::FinishAssignmentPaths {
                state,
                environment,
                frames,
                continuations,
            } => {
                let (operator, value_node, input) = {
                    let state = state.borrow();
                    (state.operator, state.value_node, state.input.clone())
                };
                if operator == AssignmentOperator::Update {
                    pending.push(GeneratorTask::AssignmentUpdate {
                        state,
                        next: 0,
                        environment,
                        frames,
                        continuations,
                    });
                    continue;
                }
                let mut continuations = continuations;
                continuations.push(GeneratorContinuation::AssignmentRhs(state));
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: value_node,
                    input,
                    origin: None,
                    environment,
                    frames,
                    continuations,
                }));
                continue;
            }
            GeneratorTask::AssignmentUpdate {
                state,
                next,
                environment,
                frames,
                continuations,
            } => {
                if next >= state.borrow().targets.len() {
                    let document = state.borrow().document.clone();
                    let deletions = std::mem::replace(
                        &mut state.borrow_mut().deletions,
                        path_builtin::PathAccumulator::new(),
                    );
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let result = path_builtin::delete_paths_bounded(
                        &document,
                        deletions,
                        limits,
                        &mut charge,
                    );
                    let mut result = result;
                    let origin = if result.is_ok() {
                        match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                result = Err(error);
                                None
                            }
                        }
                    } else {
                        None
                    };
                    if let Ok(document) = &result {
                        state.borrow_mut().document = document.clone();
                    }
                    (
                        GeneratorWork {
                            node: bytecode.root(),
                            input: Value::Null,
                            origin,
                            environment,
                            frames,
                            continuations,
                        },
                        Some(result),
                    )
                } else {
                    let target = state
                        .borrow()
                        .targets
                        .get(next)
                        .cloned()
                        .expect("target index checked above");
                    let (value_node, root, origin) = {
                        let state_ref = state.borrow();
                        (
                            state_ref.value_node,
                            state_ref.document.clone(),
                            state_ref.target_origins.get(next).cloned().flatten(),
                        )
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let old = match assignment_target_value(&root, &target, limits, &mut charge) {
                        Ok(old) => old,
                        Err(error) => {
                            return {
                                let _ = emit(Err(error), observations);
                                observations
                            };
                        }
                    };
                    let input = old.clone();
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    let accepted = Rc::new(Cell::new(false));
                    pending.push(GeneratorTask::AssignmentUpdateContinue {
                        state: Rc::clone(&state),
                        next: next.saturating_add(1),
                        boundary,
                        accepted: Rc::clone(&accepted),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut rhs_continuations = continuations;
                    rhs_continuations.push(GeneratorContinuation::AssignmentUpdateRhs {
                        state,
                        target,
                        old,
                        next: next.saturating_add(1),
                        boundary,
                        accepted,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: value_node,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations: rhs_continuations,
                    }));
                    continue;
                }
            }
            GeneratorTask::AssignmentUpdateContinue {
                state,
                next,
                boundary: _,
                accepted,
                environment,
                frames,
                continuations,
            } => {
                if accepted.get() {
                    continue;
                }
                let Some(target) = state.borrow().targets.get(next - 1).cloned() else {
                    continue;
                };
                let mut charge =
                    || charge_managed_step(&mut observations, limits, cancellation, stop);
                if let Err(error) = retain_assignment_deletion_target(
                    &mut state.borrow_mut().deletions,
                    &target,
                    limits,
                    &mut charge,
                ) {
                    return {
                        let _ = emit(Err(error), observations);
                        observations
                    };
                }
                pending.push(GeneratorTask::AssignmentUpdate {
                    state,
                    next,
                    environment,
                    frames,
                    continuations,
                });
                continue;
            }
            GeneratorTask::Traverse {
                mut cursor,
                environment,
                frames,
                continuations,
            } => {
                if cursor.depth() > limits.path_stack {
                    let _ = emit(Err(resource("path-stack")), observations);
                    break;
                }
                observations.path_stack_high_water =
                    observations.path_stack_high_water.max(cursor.depth());
                match cursor.next(limits.path_stack) {
                    Ok(Some(value)) => {
                        if pending.len() >= limits.fork_stack {
                            let _ = emit(Err(resource("fork-stack")), observations);
                            break;
                        }
                        pending.push(GeneratorTask::Traverse {
                            cursor,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: continuations.clone(),
                        });
                        (
                            GeneratorWork {
                                node: bytecode.root(),
                                input: Value::Null,
                                origin: None,
                                environment,
                                frames,
                                continuations,
                            },
                            Some(Ok(value)),
                        )
                    }
                    Ok(None) => continue,
                    Err(error) => {
                        let _ = emit(Err(error), observations);
                        break;
                    }
                }
            }
            GeneratorTask::RecurseExpand {
                input,
                filter,
                condition,
                depth,
                environment,
                frames,
                continuations,
            } => {
                if depth > limits.path_stack {
                    let _ = emit(Err(resource("path-stack")), observations);
                    break;
                }
                observations.path_stack_high_water = observations.path_stack_high_water.max(depth);
                if let Some(filter) = filter {
                    let structural = matches!(input, Value::Array(_) | Value::Object(_));
                    let mut callback_continuations = continuations;
                    callback_continuations.push(GeneratorContinuation::RecurseChild {
                        filter,
                        condition,
                        depth,
                        structural,
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                    });
                    (
                        GeneratorWork {
                            node: filter,
                            input,
                            origin: None,
                            environment,
                            frames,
                            continuations: callback_continuations,
                        },
                        None,
                    )
                } else {
                    pending.push(GeneratorTask::RecurseChildren {
                        input,
                        next: 0,
                        depth,
                        environment,
                        frames,
                        continuations,
                    });
                    continue;
                }
            }
            GeneratorTask::RecurseChildren {
                input,
                next,
                depth,
                environment,
                frames,
                continuations,
            } => {
                let Some(child) = recursive_child(&input, next) else {
                    continue;
                };
                pending.push(GeneratorTask::RecurseChildren {
                    input,
                    next: next.saturating_add(1),
                    depth,
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                    continuations: continuations.clone(),
                });
                pending.push(GeneratorTask::RecurseValue {
                    value: child,
                    filter: None,
                    condition: None,
                    depth: depth.saturating_add(1),
                    environment,
                    frames,
                    continuations,
                });
                continue;
            }
            GeneratorTask::RecurseValue {
                value,
                filter,
                condition,
                depth,
                environment,
                frames,
                continuations,
            } => {
                if depth > limits.path_stack {
                    let _ = emit(Err(resource("path-stack")), observations);
                    break;
                }
                pending.push(GeneratorTask::RecurseExpand {
                    input: value.clone(),
                    filter,
                    condition,
                    depth,
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                    continuations: continuations.clone(),
                });
                (
                    GeneratorWork {
                        node: bytecode.root(),
                        input: Value::Null,
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    Some(Ok(value)),
                )
            }
            GeneratorTask::Repeat {
                expression,
                input,
                environment,
                frames,
                continuations,
            } => {
                // jq defines repeat(exp) as `exp, _repeat`: the recursive
                // call is a sibling of exp and therefore receives the same
                // input, rather than the values emitted by exp.  Queue the
                // next iteration before evaluating exp so all of exp's
                // results are emitted first.  The VM's step budget bounds an
                // empty or otherwise unbounded repeat without native stack
                // growth.
                pending.push(GeneratorTask::Repeat {
                    expression,
                    input: input.clone(),
                    environment: Arc::clone(&environment),
                    frames: Arc::clone(&frames),
                    continuations: continuations.clone(),
                });
                let mut item_continuations = continuations;
                item_continuations.push(GeneratorContinuation::RepeatItem);
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: expression,
                    input,
                    origin: None,
                    environment,
                    frames,
                    continuations: item_continuations,
                }));
                continue;
            }
            GeneratorTask::WalkValue {
                input,
                callback,
                depth,
                environment,
                frames,
                continuations,
            } => {
                if depth > limits.path_stack {
                    let _ = emit(Err(resource("path-stack")), observations);
                    break;
                }
                observations.path_stack_high_water = observations.path_stack_high_water.max(depth);
                match input {
                    Value::Array(values) => {
                        pending.push(GeneratorTask::WalkArray {
                            inputs: values,
                            next: 0,
                            callback,
                            depth,
                            values: Rc::new(RefCell::new(Vec::new())),
                            environment,
                            frames,
                            continuations,
                        });
                        continue;
                    }
                    Value::Object(values) => {
                        let entries = values
                            .iter()
                            .map(|(key, value)| (Arc::clone(key), value.clone()))
                            .collect::<Vec<_>>();
                        pending.push(GeneratorTask::WalkObject {
                            entries: Arc::from(entries),
                            next: 0,
                            callback,
                            depth,
                            values: Rc::new(RefCell::new(Object::new())),
                            environment,
                            frames,
                            continuations,
                        });
                        continue;
                    }
                    input => (
                        GeneratorWork {
                            node: callback,
                            input,
                            origin: None,
                            environment,
                            frames,
                            continuations,
                        },
                        None,
                    ),
                }
            }
            GeneratorTask::WalkArray {
                inputs,
                next,
                callback,
                depth,
                values,
                environment,
                frames,
                continuations,
            } => {
                if let Some(value) = inputs.get(next).cloned() {
                    pending.push(GeneratorTask::WalkArray {
                        inputs,
                        next: next + 1,
                        callback,
                        depth,
                        values: Rc::clone(&values),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut child_continuations = continuations;
                    child_continuations.push(GeneratorContinuation::ArrayItem(values));
                    pending.push(GeneratorTask::WalkValue {
                        input: value,
                        callback,
                        depth: depth.saturating_add(1),
                        environment,
                        frames,
                        continuations: child_continuations,
                    });
                    continue;
                }
                (
                    GeneratorWork {
                        node: callback,
                        input: Value::array(values.take()),
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    None,
                )
            }
            GeneratorTask::WalkObject {
                entries,
                next,
                callback,
                depth,
                values,
                environment,
                frames,
                continuations,
            } => {
                if let Some((key, value)) = entries.get(next).cloned() {
                    pending.push(GeneratorTask::WalkObject {
                        entries,
                        next: next + 1,
                        callback,
                        depth,
                        values: Rc::clone(&values),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: continuations.clone(),
                    });
                    let mut child_continuations = continuations;
                    child_continuations.push(GeneratorContinuation::ObjectItem {
                        key,
                        values,
                        accepted: Rc::new(Cell::new(false)),
                    });
                    pending.push(GeneratorTask::WalkValue {
                        input: value,
                        callback,
                        depth: depth.saturating_add(1),
                        environment,
                        frames,
                        continuations: child_continuations,
                    });
                    continue;
                }
                let rebuilt = std::mem::take(&mut *values.borrow_mut());
                (
                    GeneratorWork {
                        node: callback,
                        input: Value::object(rebuilt),
                        origin: None,
                        environment,
                        frames,
                        continuations,
                    },
                    None,
                )
            }
        };
        let mut result_origin = work.origin.clone();
        observations.value_stack_high_water = observations.value_stack_high_water.max(1);
        observations.fork_stack_high_water = observations.fork_stack_high_water.max(pending.len());
        observations.call_stack_high_water = observations
            .call_stack_high_water
            .max(work.continuations.len().saturating_add(1));
        if work.continuations.len() >= limits.call_stack {
            let _ = emit(Err(resource("call-stack")), observations);
            break;
        }
        if pending.len() >= limits.fork_stack {
            let _ = emit(Err(resource("fork-stack")), observations);
            break;
        }

        let value = if let Some(result) = delivered {
            Some(result)
        } else {
            let Some(instruction) = bytecode.instructions().get(work.node as usize) else {
                let _ = emit(
                    Err(invalid("tree instruction missing after validation")),
                    observations,
                );
                break;
            };
            match &instruction.operation {
                Operation::Identity => Some(Ok(work.input)),
                Operation::Literal(index) => {
                    result_origin = match fresh_origin(&mut next_origin) {
                        Ok(origin) => Some(origin),
                        Err(error) => {
                            return {
                                let _ = emit(Err(error), observations);
                                observations
                            };
                        }
                    };
                    Some(
                        bytecode
                            .constants()
                            .get(*index as usize)
                            .cloned()
                            .ok_or_else(|| invalid("literal missing after validation")),
                    )
                }
                Operation::Variable(index) => Some(
                    bytecode
                        .string(*index)
                        .ok_or_else(|| invalid("string missing after validation"))
                        .and_then(|name| {
                            result_origin = work.environment.origins.get(name).cloned();
                            work.environment
                                .get(name)
                                .cloned()
                                .ok_or_else(|| runtime(format!("variable ${name} has no value")))
                        }),
                ),
                Operation::Empty => None,
                Operation::RecursiveDescent => {
                    pending.push(GeneratorTask::Traverse {
                        cursor: TraversalCursor::new(work.input.clone()),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    None
                }
                Operation::Interpolation(segments) => {
                    let expression_count = segments
                        .iter()
                        .filter(|segment| matches!(segment, InterpolationOperand::Expression(_)))
                        .count();
                    if expression_count >= limits.call_stack {
                        Some(Err(resource("call-stack")))
                    } else {
                        let segments = Arc::from(segments.clone());
                        match interpolation_pieces(&segments, bytecode, limits.output_bytes) {
                            Ok(pieces) => schedule_interpolation(
                                &segments,
                                segments.len(),
                                pieces,
                                work.input.clone(),
                                Arc::clone(&work.environment),
                                Arc::clone(&work.frames),
                                work.continuations.clone(),
                                bytecode,
                                limits.output_bytes,
                                &mut pending,
                            ),
                            Err(error) => Some(Err(error)),
                        }
                    }
                }
                Operation::AccessField { base, key } => match bytecode.string(*key) {
                    Some(key) => {
                        work.continuations
                            .push(GeneratorContinuation::AccessField(Arc::clone(key)));
                        work.node = *base;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    }
                    None => Some(Err(invalid("string missing after validation"))),
                },
                Operation::AccessIndex { base, index } => {
                    work.continuations.push(GeneratorContinuation::AccessIndex {
                        node: *index,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *base;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Slice { base, start, end } => {
                    work.continuations.push(GeneratorContinuation::SliceBase {
                        start: *start,
                        end: *end,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    work.node = *base;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Iterate(base) => {
                    work.continuations.push(GeneratorContinuation::Iterate);
                    work.node = *base;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Optional(child) => {
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    work.continuations
                        .push(GeneratorContinuation::OptionalBoundary { boundary });
                    work.node = *child;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Pipe { left, right } => {
                    work.continuations.push(GeneratorContinuation::Pipe {
                        node: *right,
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *left;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Comma { left, right } => {
                    if pending.len().saturating_add(2) > limits.fork_stack {
                        Some(Err(resource("fork-stack")))
                    } else {
                        let right_work = GeneratorWork {
                            node: *right,
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        };
                        work.node = *left;
                        pending.push(GeneratorTask::Eval(right_work));
                        pending.push(GeneratorTask::Eval(work.clone()));
                        observations.fork_stack_high_water =
                            observations.fork_stack_high_water.max(pending.len());
                        None
                    }
                }
                Operation::Array(child) => {
                    let values = Rc::new(RefCell::new(Vec::new()));
                    pending.push(GeneratorTask::FinishArray {
                        values: Rc::clone(&values),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    work.continuations
                        .push(GeneratorContinuation::ArrayItem(values));
                    work.node = *child;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Object(entries) => {
                    let entries: Arc<[crate::bytecode::ObjectOperand]> = Arc::from(entries.clone());
                    if entries.is_empty() {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                return {
                                    let _ = emit(Err(error), observations);
                                    observations
                                };
                            }
                        };
                    }
                    schedule_object_entry(
                        bytecode,
                        &entries,
                        0,
                        Object::new(),
                        work.input.clone(),
                        work.origin.clone(),
                        Arc::clone(&work.environment),
                        Arc::clone(&work.frames),
                        work.continuations.clone(),
                        &mut pending,
                    )
                }
                Operation::Unary { operator, child } => {
                    work.continuations
                        .push(GeneratorContinuation::Unary(*operator));
                    work.node = *child;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Binary {
                    operator,
                    left,
                    right,
                } => {
                    if *operator == BinaryOperator::Alternative {
                        let matched = Rc::new(Cell::new(false));
                        pending.push(GeneratorTask::FinishAlternative {
                            matched: Rc::clone(&matched),
                            right: *right,
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        });
                        work.continuations
                            .push(GeneratorContinuation::AlternativeItem(matched));
                    } else {
                        work.continuations.push(GeneratorContinuation::BinaryLeft {
                            operator: *operator,
                            right: *right,
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                    }
                    work.node = *left;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Bind {
                    value,
                    pattern,
                    body,
                } => {
                    work.continuations.push(GeneratorContinuation::Bind {
                        pattern: pattern.clone(),
                        body: *body,
                        input: work.input.clone(),
                        input_origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        origin: None,
                    });
                    work.node = *value;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::BindAlternatives {
                    value,
                    patterns,
                    body,
                } => {
                    work.continuations
                        .push(GeneratorContinuation::BindAlternatives {
                            patterns: Arc::from(patterns.clone()),
                            body: *body,
                            input: work.input.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                    work.node = *value;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Reduce {
                    generator,
                    pattern,
                    initial,
                    update,
                } => {
                    work.continuations.push(GeneratorContinuation::FoldInitial {
                        generator: *generator,
                        pattern: pattern.clone(),
                        update: *update,
                        extract: None,
                        emit_each_update: false,
                        input: work.input.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *initial;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Foreach {
                    generator,
                    pattern,
                    initial,
                    update,
                    extract,
                } => {
                    work.continuations.push(GeneratorContinuation::FoldInitial {
                        generator: *generator,
                        pattern: pattern.clone(),
                        update: *update,
                        extract: *extract,
                        emit_each_update: true,
                        input: work.input.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *initial;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Assignment {
                    operator,
                    path,
                    value,
                } => {
                    let state = Rc::new(RefCell::new(AssignmentState {
                        operator: *operator,
                        value_node: *value,
                        input: work.input.clone(),
                        targets: Vec::new(),
                        target_origins: Vec::new(),
                        document: work.input.clone(),
                        deletions: path_builtin::PathAccumulator::new(),
                    }));
                    pending.push(GeneratorTask::FinishAssignmentPaths {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    let mut path_continuations = work.continuations.clone();
                    path_continuations.push(GeneratorContinuation::AssignmentPath(state));
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: *path,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        prefix: Path::root(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: path_continuations,
                    }));
                    None
                }
                Operation::Conditional {
                    branches,
                    alternative,
                } => {
                    if let Some((condition, _)) = branches.first() {
                        work.continuations.push(GeneratorContinuation::Conditional {
                            branches: Arc::from(branches.clone()),
                            next: 0,
                            alternative: *alternative,
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                        });
                        work.node = *condition;
                    } else {
                        work.node = *alternative;
                    }
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::UserCall { symbol, arguments } => schedule_user_call(
                    *symbol,
                    Arc::from(arguments.clone()),
                    0,
                    work.input.clone(),
                    work.origin.clone(),
                    Arc::clone(&work.environment),
                    Arc::clone(&work.frames),
                    vec![None; arguments.len()],
                    work.environment.as_ref().clone(),
                    work.continuations.clone(),
                    bytecode,
                    limits.call_stack,
                    &mut pending,
                ),
                Operation::ParameterCall {
                    function,
                    parameter,
                } => {
                    let argument = work
                        .frames
                        .iter()
                        .rev()
                        .find(|frame| frame.symbol == *function)
                        .and_then(|frame| frame.filters.get(*parameter as usize))
                        .and_then(Clone::clone);
                    match argument {
                        Some(argument) => {
                            work.continuations.push(GeneratorContinuation::ReturnUser {
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                            });
                            work.node = argument.node;
                            work.environment = argument.environment;
                            work.frames = argument.frames;
                            pending.push(GeneratorTask::Eval(work.clone()));
                            None
                        }
                        None => Some(Err(invalid(
                            "filter parameter missing from active user frame",
                        ))),
                    }
                }
                Operation::TryCatch { expression, catch } => {
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    work.continuations.push(GeneratorContinuation::Catch {
                        boundary,
                        node: *catch,
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                    });
                    work.node = *expression;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Label { symbol, body } => {
                    work.continuations
                        .push(GeneratorContinuation::Label(*symbol));
                    work.node = *body;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Break(symbol) => Some(Err(VmError::Break { label: *symbol })),
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .and_then(|name| regex_builtin_shape(name, arguments.len()))
                        .is_some() =>
                {
                    let Some(name) = bytecode.string(*name).cloned() else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    let Some((order, replacement_slot)) =
                        regex_builtin_shape(&name, arguments.len())
                    else {
                        return {
                            let _ = emit(Err(invalid("unsupported regex call")), observations);
                            observations
                        };
                    };
                    if arguments.len() > limits.fork_stack {
                        return {
                            let _ = emit(Err(resource("fork-stack")), observations);
                            observations
                        };
                    }
                    let mut values = Vec::new();
                    if values.try_reserve(arguments.len()).is_err() {
                        return {
                            let _ = emit(Err(resource("fork-stack")), observations);
                            observations
                        };
                    }
                    values.resize(arguments.len(), None);
                    let first = order[0];
                    let mut continuations = std::mem::take(&mut work.continuations);
                    continuations.push(GeneratorContinuation::RegexArguments {
                        name,
                        arguments: Arc::from(arguments.clone()),
                        order,
                        next: 1,
                        values,
                        replacement: replacement_slot.and_then(|slot| arguments.get(slot).copied()),
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: arguments[first],
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations,
                    }));
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "IN" | "INDEX" | "JOIN")) =>
                {
                    let Some(name) = bytecode.string(*name) else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    schedule_sql_call(
                        name,
                        arguments,
                        work.input.clone(),
                        work.origin.clone(),
                        Arc::clone(&work.environment),
                        Arc::clone(&work.frames),
                        std::mem::take(&mut work.continuations),
                        limits,
                        &mut next_boundary,
                        &mut pending,
                    )
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "any" | "all")) =>
                {
                    let kind = if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "any")
                    {
                        PullConsumerKind::Any
                    } else {
                        PullConsumerKind::All
                    };
                    let (generator, condition) = match arguments.as_slice() {
                        [] => (None, None),
                        [condition] => (None, Some(*condition)),
                        [generator, condition] => (Some(*generator), Some(*condition)),
                        _ => {
                            let _ = emit(Err(invalid("predicate arity")), observations);
                            return observations;
                        }
                    };
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    let state = Rc::new(RefCell::new(PullConsumerState {
                        boundary,
                        kind,
                        seen: false,
                        latest: None,
                        latest_origin: None,
                        decision: None,
                        remaining: 0,
                    }));
                    pending.push(GeneratorTask::FinishPullConsumer {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    let mut predicate_continuations = work.continuations.clone();
                    predicate_continuations.push(GeneratorContinuation::PredicateItem {
                        state,
                        condition,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    if let Some(generator) = generator {
                        work.node = generator;
                        work.continuations = predicate_continuations;
                        pending.push(GeneratorTask::Eval(work.clone()));
                    } else {
                        let values = match &work.input {
                            Value::Array(values) => Arc::clone(values),
                            Value::Object(values) => {
                                values.values().cloned().collect::<Vec<_>>().into()
                            }
                            value => {
                                let _ = emit(Err(type_error("iterate", value)), observations);
                                return observations;
                            }
                        };
                        pending.push(GeneratorTask::Iterate {
                            values,
                            next: 0,
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: predicate_continuations,
                        });
                    }
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(name.as_ref(), "path" | "paths" | "pick" | "del")
                    }) =>
                {
                    let Some(name) = bytecode.string(*name).cloned() else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    match name.as_ref() {
                        "path" => {
                            let Some(argument) = arguments.first() else {
                                return {
                                    let _ =
                                        emit(Err(invalid("path argument missing")), observations);
                                    observations
                                };
                            };
                            if arguments.len() != 1 {
                                return {
                                    let _ = emit(Err(invalid("path arity")), observations);
                                    observations
                                };
                            }
                            let mut continuations = work.continuations.clone();
                            continuations.push(GeneratorContinuation::FreshOrigin);
                            pending.push(GeneratorTask::PathEval(PathWork {
                                node: *argument,
                                input: work.input.clone(),
                                origin: work.origin.clone(),
                                prefix: Path::root(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations,
                            }));
                            None
                        }
                        "paths" => {
                            if arguments.len() > 1 {
                                return {
                                    let _ = emit(Err(invalid("paths arity")), observations);
                                    observations
                                };
                            }
                            let mut continuations = work.continuations.clone();
                            if let Some(filter) = arguments.first().copied() {
                                continuations.push(GeneratorContinuation::PathFilter {
                                    root: work.input.clone(),
                                    filter,
                                    origin: work.origin.clone(),
                                    environment: Arc::clone(&work.environment),
                                    frames: work.frames.clone(),
                                });
                            }
                            pending.push(GeneratorTask::PathTraverse {
                                cursor: PathTraversalCursor::without_root(work.input.clone()),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations,
                            });
                            None
                        }
                        "pick" | "del" => {
                            let Some(argument) = arguments.first() else {
                                return {
                                    let _ =
                                        emit(Err(invalid("path callback missing")), observations);
                                    observations
                                };
                            };
                            if arguments.len() != 1 {
                                return {
                                    let _ = emit(Err(invalid("path callback arity")), observations);
                                    observations
                                };
                            }
                            let kind = if name.as_ref() == "pick" {
                                PathCollectionKind::Pick
                            } else {
                                PathCollectionKind::Delete
                            };
                            let boundary = next_boundary;
                            next_boundary = next_boundary.saturating_add(1);
                            let paths = Rc::new(RefCell::new(path_builtin::PathAccumulator::new()));
                            let continuations = work.continuations.clone();
                            pending.push(GeneratorTask::FinishPathCollection {
                                boundary,
                                kind,
                                input: work.input.clone(),
                                paths: Rc::clone(&paths),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: continuations.clone(),
                            });
                            let mut callback_continuations = continuations;
                            callback_continuations
                                .push(GeneratorContinuation::PathCollect { boundary, paths });
                            pending.push(GeneratorTask::PathEval(PathWork {
                                node: *argument,
                                input: work.input.clone(),
                                origin: work.origin.clone(),
                                prefix: Path::root(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: callback_continuations,
                            }));
                            None
                        }
                        _ => unreachable!("path builtin guard is exhaustive"),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .and_then(|name| builtin_argument_order(name, arguments.len()))
                        .is_some() =>
                {
                    let Some(name) = bytecode.string(*name).cloned() else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    let Some(order) = builtin_argument_order(&name, arguments.len()) else {
                        return {
                            let _ = emit(Err(invalid("unsupported builtin call")), observations);
                            observations
                        };
                    };
                    let source_arity = arguments.len();
                    let input = work.input.clone();
                    let origin = work.origin.clone();
                    let environment = Arc::clone(&work.environment);
                    let frames = Arc::clone(&work.frames);
                    if arguments.is_empty() {
                        pending.push(GeneratorTask::InvokeBuiltin {
                            name,
                            arguments: Vec::new(),
                            source_arity,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: std::mem::take(&mut work.continuations),
                        });
                        None
                    } else if name.as_ref() == "combinations" && arguments.len() == 1 {
                        let values = Rc::new(RefCell::new(Vec::new()));
                        pending.push(GeneratorTask::FinishBuiltinArguments {
                            name,
                            source_arity,
                            values: Rc::clone(&values),
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: work.continuations.clone(),
                        });
                        let mut continuations = std::mem::take(&mut work.continuations);
                        continuations.push(GeneratorContinuation::ArrayItem(values));
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[0],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations,
                        }));
                        None
                    } else {
                        if arguments.len() > limits.fork_stack {
                            return {
                                let _ = emit(Err(resource("fork-stack")), observations);
                                observations
                            };
                        }
                        let mut values = Vec::new();
                        if values.try_reserve(arguments.len()).is_err() {
                            return {
                                let _ = emit(Err(resource("fork-stack")), observations);
                                observations
                            };
                        }
                        values.resize(arguments.len(), None);
                        let first = order[0];
                        let mut continuations = std::mem::take(&mut work.continuations);
                        continuations.push(GeneratorContinuation::BuiltinArguments {
                            name,
                            arguments: Arc::from(arguments.clone()),
                            order,
                            next: 1,
                            values,
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[first],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations,
                        }));
                        None
                    }
                }
                Operation::Call { name, arguments }
                    if arguments.is_empty()
                        && bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "empty") =>
                {
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "error") =>
                {
                    if let Some(argument) = arguments.first() {
                        work.continuations.push(GeneratorContinuation::Raise);
                        work.node = *argument;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    } else {
                        Some(Err(VmError::Raised {
                            message: error_message(&work.input),
                            value: work.input.clone(),
                        }))
                    }
                }
                Operation::Call { name, .. }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "stderr") =>
                {
                    match format::text(&work.input, limits.output_bytes).and_then(|text| {
                        append_effect(effects, text.as_bytes(), limits.output_bytes).map(|()| text)
                    }) {
                        Ok(_) => Some(Ok(work.input)),
                        Err(error) => Some(Err(error)),
                    }
                }
                Operation::Call { name, .. }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "input" | "inputs")) =>
                'input: {
                    let Some(cursor) = input_cursor.cloned() else {
                        break 'input if bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "input")
                        {
                            Some(Err(runtime("break".to_owned())))
                        } else {
                            None
                        };
                    };
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "input")
                    {
                        match cursor.next_value() {
                            Ok(Some(value)) => match fresh_origin(&mut next_origin) {
                                Ok(origin) => {
                                    work.origin = Some(origin);
                                    Some(Ok(value))
                                }
                                Err(error) => Some(Err(error)),
                            },
                            Ok(None) => Some(Err(runtime("break".to_owned()))),
                            Err(error) => Some(Err(error)),
                        }
                    } else {
                        pending.push(GeneratorTask::InputValues {
                            cursor,
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: std::mem::take(&mut work.continuations),
                        });
                        None
                    }
                }
                Operation::Call { name, .. }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(name.as_ref(), "input_filename" | "input_line_number")
                    }) =>
                {
                    let Some(name) = bytecode.string(*name) else {
                        return {
                            let _ = emit(
                                Err(invalid("string missing after validation")),
                                observations,
                            );
                            observations
                        };
                    };
                    let value = if !ambient_platform(&work.environment) {
                        Err(capability_denied(format!(
                            "{name} requires platform access permitted by capability policy"
                        )))
                    } else if name.as_ref() == "input_filename" {
                        input_cursor
                            .and_then(InputCursor::current_context)
                            .map_or_else(
                                || {
                                    ambient_value(
                                        &work.environment,
                                        INPUT_FILENAME,
                                        "input_filename",
                                    )
                                },
                                |context| Ok(Value::string(context.identity)),
                            )
                    } else {
                        input_cursor
                            .and_then(InputCursor::current_context)
                            .map_or_else(
                                || {
                                    ambient_value(
                                        &work.environment,
                                        INPUT_LINE_NUMBER,
                                        "input_line_number",
                                    )
                                },
                                |context| {
                                    Ok(Value::Number(
                                        Number::parse(&context.line_number.to_string()).expect(
                                            "a source line number is an admitted exact integer",
                                        ),
                                    ))
                                },
                            )
                    };
                    match value {
                        Ok(value) => match fresh_origin(&mut next_origin) {
                            Ok(origin) => {
                                result_origin = Some(origin);
                                Some(Ok(value))
                            }
                            Err(error) => Some(Err(error)),
                        },
                        Err(error) => Some(Err(error)),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "debug") =>
                {
                    if let Some(argument) = arguments.first() {
                        pending.push(GeneratorTask::FinishDebug {
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        });
                        work.continuations.push(GeneratorContinuation::DebugItem);
                        work.node = *argument;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    } else {
                        match debug_effect(&work.input, limits.output_bytes)
                            .and_then(|bytes| append_effect(effects, &bytes, limits.output_bytes))
                        {
                            Ok(()) => Some(Ok(work.input)),
                            Err(error) => Some(Err(error)),
                        }
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "add")
                        && arguments.len() == 1 =>
                {
                    let state = Rc::new(RefCell::new(AddState { accumulator: None }));
                    pending.push(GeneratorTask::FinishAdd {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    work.continuations
                        .push(GeneratorContinuation::AddItem(state));
                    work.node = arguments[0];
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Call { name, .. }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "halt") =>
                {
                    Some(Err(VmError::Halt {
                        status: 0,
                        stderr: Arc::from([]),
                    }))
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "halt_error") =>
                {
                    if let Some(argument) = arguments.first() {
                        work.continuations.push(GeneratorContinuation::HaltError {
                            input: work.input.clone(),
                        });
                        work.node = *argument;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    } else {
                        match halt_error_text(&work.input, limits.output_bytes) {
                            Ok(stderr) => Some(Err(VmError::Halt {
                                status: 5,
                                stderr: Arc::from(stderr.as_bytes()),
                            })),
                            Err(error) => Some(Err(error)),
                        }
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(name.as_ref(), "first" | "last" | "isempty")
                    }) && arguments.len() == 1 =>
                {
                    let name = bytecode.string(*name).map_or("first", AsRef::as_ref);
                    let kind = match name {
                        "first" => PullConsumerKind::First,
                        "last" => PullConsumerKind::Last,
                        _ => PullConsumerKind::IsEmpty,
                    };
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    let state = Rc::new(RefCell::new(PullConsumerState {
                        boundary,
                        kind,
                        seen: false,
                        latest: None,
                        latest_origin: None,
                        decision: None,
                        remaining: 0,
                    }));
                    pending.push(GeneratorTask::FinishPullConsumer {
                        state: Rc::clone(&state),
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    work.continuations
                        .push(GeneratorContinuation::PullConsumer { state });
                    work.node = arguments[0];
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "first" | "last"))
                        && arguments.is_empty() =>
                {
                    let index = if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "first")
                    {
                        0_i64
                    } else {
                        -1_i64
                    };
                    let index_value = Value::Number(
                        Number::parse(&index.to_string())
                            .expect("literal pull index is a valid number"),
                    );
                    let mut result = access_index(&work.input, &index_value);
                    if result.is_ok() {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        result_origin = match index_origin(
                            &work.input,
                            #[allow(clippy::cast_precision_loss)]
                            {
                                index as f64
                            },
                            work.origin.as_ref(),
                            limits,
                            &mut charge,
                        ) {
                            Ok(origin) => origin,
                            Err(error) => {
                                result = Err(error);
                                None
                            }
                        };
                    }
                    Some(result)
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "nth" | "skip")) =>
                {
                    let kind = if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "nth")
                    {
                        PullConsumerKind::Nth
                    } else {
                        PullConsumerKind::Skip
                    };
                    let Some(count) = arguments.first().copied() else {
                        return {
                            let _ = emit(Err(invalid("pull count missing")), observations);
                            observations
                        };
                    };
                    let generator = arguments.get(1).copied();
                    if kind == PullConsumerKind::Skip && generator.is_none() {
                        return {
                            let _ = emit(Err(invalid("skip arity")), observations);
                            observations
                        };
                    }
                    if arguments.len() > 2 {
                        return {
                            let _ = emit(Err(invalid("pull arity")), observations);
                            observations
                        };
                    }
                    work.continuations.push(GeneratorContinuation::PullCount {
                        kind,
                        input: work.input.clone(),
                        origin: work.origin.clone(),
                        generator,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                    });
                    work.node = count;
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "fromstream")
                        && arguments.len() == 1 =>
                {
                    let state = Rc::new(RefCell::new(generator::FromStreamState::new()));
                    work.continuations
                        .push(GeneratorContinuation::FromStreamItem(state));
                    work.node = arguments[0];
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "truncate_stream")
                        && arguments.len() == 1 =>
                {
                    work.continuations
                        .push(GeneratorContinuation::TruncateStreamItem {
                            count: work.input.clone(),
                        });
                    work.node = arguments[0];
                    pending.push(GeneratorTask::Eval(work.clone()));
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "recurse") =>
                {
                    let filter = arguments.first().copied();
                    let condition = arguments.get(1).copied();
                    pending.push(GeneratorTask::RecurseExpand {
                        input: work.input.clone(),
                        filter,
                        condition,
                        depth: 1,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    Some(Ok(work.input))
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "while" | "until")) =>
                {
                    match (arguments.first(), arguments.get(1)) {
                        (Some(condition), Some(update)) => {
                            work.continuations
                                .push(GeneratorContinuation::LoopCondition {
                                    condition: *condition,
                                    update: *update,
                                    input: work.input.clone(),
                                    until: bytecode
                                        .string(*name)
                                        .is_some_and(|name| name.as_ref() == "until"),
                                    environment: Arc::clone(&work.environment),
                                    frames: Arc::clone(&work.frames),
                                });
                            work.node = *condition;
                            pending.push(GeneratorTask::Eval(work.clone()));
                            None
                        }
                        _ => Some(Err(invalid("loop arguments missing"))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "repeat") =>
                {
                    match arguments.first() {
                        Some(expression) => {
                            pending.push(GeneratorTask::Repeat {
                                expression: *expression,
                                input: work.input.clone(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        None => Some(Err(invalid("repeat argument missing"))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "limit") =>
                {
                    match (arguments.first(), arguments.get(1)) {
                        (Some(count), Some(expression)) => {
                            work.continuations.push(GeneratorContinuation::LimitCount {
                                expression: *expression,
                                input: work.input.clone(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                            });
                            work.node = *count;
                            pending.push(GeneratorTask::Eval(work.clone()));
                            None
                        }
                        _ => Some(Err(invalid("limit arguments missing"))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "walk") =>
                {
                    if let Some(callback) = arguments.first() {
                        pending.push(GeneratorTask::WalkValue {
                            input: work.input.clone(),
                            callback: *callback,
                            depth: 1,
                            environment: Arc::clone(&work.environment),
                            frames: Arc::clone(&work.frames),
                            continuations: work.continuations.clone(),
                        });
                        None
                    } else {
                        Some(Err(invalid("walk argument missing")))
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "map") =>
                {
                    match (arguments.first(), &work.input) {
                        (None, _) => Some(Err(invalid("map argument missing"))),
                        (
                            Some(_),
                            value @ (Value::Null
                            | Value::Bool(_)
                            | Value::Number(_)
                            | Value::String(_)),
                        ) => Some(Err(type_error("map", value))),
                        (Some(argument), Value::Array(values)) => {
                            pending.push(GeneratorTask::MapArray {
                                inputs: Arc::clone(values),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Vec::new())),
                                first_only: false,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(argument), Value::Object(values)) => {
                            let inputs = values.values().cloned().collect::<Vec<_>>();
                            pending.push(GeneratorTask::MapArray {
                                inputs: Arc::from(inputs),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Vec::new())),
                                first_only: false,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "with_entries") =>
                {
                    let Some(argument) = arguments.first() else {
                        let _ = emit(Err(invalid("with_entries argument missing")), observations);
                        return observations;
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let entries = match scalar::bounded_to_entries(&work.input, limits, &mut charge)
                    {
                        Ok(Value::Array(entries)) => entries,
                        Ok(_) => unreachable!("to_entries always returns an array"),
                        Err(error) => {
                            let _ = emit(Err(error), observations);
                            return observations;
                        }
                    };
                    let mut continuations = work.continuations.clone();
                    continuations.push(GeneratorContinuation::FromEntries);
                    pending.push(GeneratorTask::MapArray {
                        inputs: entries,
                        next: 0,
                        argument: *argument,
                        values: Rc::new(RefCell::new(Vec::new())),
                        first_only: false,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations,
                    });
                    None
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "map_values") =>
                {
                    match (arguments.first(), &work.input) {
                        (None, _) => Some(Err(invalid("map_values argument missing"))),
                        (Some(argument), Value::Object(values)) => {
                            let entries = values
                                .iter()
                                .map(|(key, value)| (Arc::clone(key), value.clone()))
                                .collect::<Vec<_>>();
                            pending.push(GeneratorTask::MapObject {
                                entries: Arc::from(entries),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Object::new())),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(argument), Value::Array(values)) => {
                            pending.push(GeneratorTask::MapArray {
                                inputs: Arc::clone(values),
                                next: 0,
                                argument: *argument,
                                values: Rc::new(RefCell::new(Vec::new())),
                                first_only: true,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(_), value) => Some(Err(type_error("map_values", value))),
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| name.as_ref() == "select") =>
                {
                    if let Some(argument) = arguments.first() {
                        work.continuations.push(GeneratorContinuation::Select {
                            input: work.input.clone(),
                            origin: work.origin.clone(),
                        });
                        work.node = *argument;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    } else {
                        Some(Err(invalid("select argument missing")))
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode
                        .string(*name)
                        .is_some_and(|name| matches!(name.as_ref(), "has" | "in")) =>
                {
                    if let Some(argument) = arguments.first() {
                        let container_from_result = bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "in");
                        work.continuations.push(GeneratorContinuation::HasArgument {
                            input: work.input.clone(),
                            container_from_result,
                        });
                        work.node = *argument;
                        pending.push(GeneratorTask::Eval(work.clone()));
                        None
                    } else {
                        Some(Err(invalid("has/in argument missing")))
                    }
                }
                Operation::Call { name, arguments }
                    if bytecode.string(*name).is_some_and(|name| {
                        matches!(
                            name.as_ref(),
                            "sort_by" | "unique_by" | "group_by" | "min_by" | "max_by"
                        )
                    }) =>
                {
                    let name = bytecode.string(*name).map_or("sort_by", AsRef::as_ref);
                    let mode = match name {
                        "sort_by" => KeyedCollectionMode::Sort,
                        "unique_by" => KeyedCollectionMode::Unique,
                        "group_by" => KeyedCollectionMode::Group,
                        "min_by" => KeyedCollectionMode::Min,
                        "max_by" => KeyedCollectionMode::Max,
                        _ => unreachable!("collection mode guard is exhaustive"),
                    };
                    match (arguments.first(), &work.input) {
                        (None, _) => Some(Err(invalid("sort key argument missing"))),
                        (Some(argument), Value::Array(values)) => {
                            pending.push(GeneratorTask::CollectSortKeys {
                                values: Arc::clone(values),
                                next: 0,
                                argument: *argument,
                                keyed_values: Rc::new(RefCell::new(Vec::new())),
                                mode,
                                origin: work.origin.clone(),
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            None
                        }
                        (Some(_), value) => Some(Err(type_error(name, value))),
                    }
                }
                Operation::Call { name, arguments }
                    if arguments.is_empty()
                        && bytecode
                            .string(*name)
                            .is_some_and(|name| name.as_ref() == "modulemeta") =>
                {
                    let result = module_metadata(bytecode, &work.input, cancellation, stop);
                    if result.is_ok() {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                return {
                                    let _ = emit(Err(error), observations);
                                    observations
                                };
                            }
                        };
                    }
                    Some(result)
                }
                Operation::Call { name, arguments } if arguments.is_empty() => {
                    let result = bytecode
                        .string(*name)
                        .ok_or_else(|| invalid("string missing after validation"))
                        .and_then(|name| generator_builtin(name, &work.input, limits.output_bytes))
                        .transpose();
                    if matches!(result, Some(Ok(_))) {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                return {
                                    let _ = emit(Err(error), observations);
                                    observations
                                };
                            }
                        };
                    }
                    result
                }
                operation => Some(Err(VmError::Unsupported {
                    operation: format!("{operation:?}").into(),
                })),
            }
        };

        let Some(mut result) = value else {
            continue;
        };
        loop {
            if let Err(mut error) = result {
                let mut handled = false;
                while let Some(continuation) = work.continuations.pop() {
                    match continuation {
                        GeneratorContinuation::OptionalBoundary { boundary }
                            if is_optional_suppressible(&error) =>
                        {
                            pending.retain(|task| !task_owned_by_optional(task, boundary));
                            handled = true;
                            break;
                        }
                        GeneratorContinuation::IgnoreError if is_optional_suppressible(&error) => {
                            handled = true;
                            break;
                        }
                        GeneratorContinuation::Catch {
                            boundary,
                            node,
                            origin,
                            environment,
                        } if is_catchable_error(&error) => {
                            pending.retain(|task| !task_owned_by_catch(task, boundary));
                            if let Some(node) = node {
                                pending.push(GeneratorTask::Eval(GeneratorWork {
                                    node,
                                    input: catch_value(&error),
                                    origin,
                                    environment,
                                    frames: work.frames,
                                    continuations: work.continuations,
                                }));
                            }
                            handled = true;
                            break;
                        }
                        GeneratorContinuation::PathCatch {
                            boundary,
                            node,
                            origin,
                            environment,
                        } if is_catchable_error(&error) => {
                            pending.retain(|task| !task_owned_by_catch(task, boundary));
                            if let Some(node) = node {
                                let input = catch_value(&error);
                                if path_candidate_with_aliases(bytecode, node, &work.continuations)
                                {
                                    pending.push(GeneratorTask::PathEval(PathWork {
                                        node,
                                        input,
                                        origin,
                                        prefix: Path::root(),
                                        environment,
                                        frames: work.frames,
                                        continuations: work.continuations,
                                    }));
                                } else {
                                    pending.push(GeneratorTask::Eval(GeneratorWork {
                                        node,
                                        input,
                                        origin,
                                        environment,
                                        frames: work.frames,
                                        continuations: work.continuations,
                                    }));
                                }
                            }
                            handled = true;
                            break;
                        }
                        GeneratorContinuation::BindAlternativeBody {
                            boundary,
                            patterns,
                            next,
                            body,
                            value,
                            input,
                            environment,
                        } => {
                            pending.retain(|task| !task_owned_by_bind_alternative(task, boundary));
                            if !is_retryable_alternative_error(&error) {
                                continue;
                            }
                            if next >= patterns.len() {
                                continue;
                            }
                            match schedule_bind_alternative(
                                bytecode,
                                patterns,
                                next,
                                value,
                                input,
                                environment,
                                body,
                                work.frames.clone(),
                                work.continuations.clone(),
                                boundary,
                                &mut pending,
                                limits,
                                cancellation,
                                stop,
                                &mut observations,
                                false,
                                Path::root(),
                            ) {
                                Ok(()) => {
                                    handled = true;
                                    break;
                                }
                                Err(next_error) => error = next_error,
                            }
                        }
                        GeneratorContinuation::PathBindAlternativeBody {
                            boundary,
                            patterns,
                            next,
                            body,
                            value,
                            input,
                            prefix,
                            environment,
                        } => {
                            pending.retain(|task| !task_owned_by_bind_alternative(task, boundary));
                            if !is_retryable_alternative_error(&error) {
                                continue;
                            }
                            if next >= patterns.len() {
                                continue;
                            }
                            match schedule_bind_alternative(
                                bytecode,
                                patterns,
                                next,
                                value,
                                input,
                                environment,
                                body,
                                work.frames.clone(),
                                work.continuations.clone(),
                                boundary,
                                &mut pending,
                                limits,
                                cancellation,
                                stop,
                                &mut observations,
                                true,
                                prefix,
                            ) {
                                Ok(()) => {
                                    handled = true;
                                    break;
                                }
                                Err(next_error) => error = next_error,
                            }
                        }
                        GeneratorContinuation::AssignmentUpdateRhs { boundary, .. } => {
                            pending
                                .retain(|task| !task_owned_by_assignment_boundary(task, boundary));
                        }
                        GeneratorContinuation::PullConsumer { state } => {
                            pending.retain(|task| {
                                !task_owned_by_pull_consumer(task, state.borrow().boundary)
                            });
                        }
                        GeneratorContinuation::PathCollect { boundary, .. } => {
                            pending.retain(|task| !task_owned_by_path_collection(task, boundary));
                        }
                        GeneratorContinuation::SqlInRight { boundary, .. }
                        | GeneratorContinuation::SqlInLeft { boundary, .. }
                        | GeneratorContinuation::SqlIndexRow { boundary, .. }
                        | GeneratorContinuation::SqlIndexKey { boundary, .. }
                        | GeneratorContinuation::SqlJoinIndex { boundary, .. }
                        | GeneratorContinuation::SqlJoinRow { boundary, .. }
                        | GeneratorContinuation::SqlJoinKey { boundary, .. }
                        | GeneratorContinuation::SqlJoinCallback { boundary } => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                        }
                        GeneratorContinuation::Label(symbol) => {
                            if matches!(error, VmError::Break { label } if label == symbol) {
                                pending.retain(|task| !task_owned_by_label(task, symbol));
                                handled = true;
                                break;
                            }
                        }
                        GeneratorContinuation::ReturnUser {
                            environment,
                            frames,
                        } => {
                            work.environment = environment;
                            work.frames = frames;
                        }
                        GeneratorContinuation::AccessField(_)
                        | GeneratorContinuation::Iterate
                        | GeneratorContinuation::Pipe { .. }
                        | GeneratorContinuation::BinaryLeft { .. }
                        | GeneratorContinuation::BinaryRight { .. }
                        | GeneratorContinuation::Unary(_)
                        | GeneratorContinuation::ArrayItem(_)
                        | GeneratorContinuation::ArrayUpdateItem { .. }
                        | GeneratorContinuation::ObjectItem { .. }
                        | GeneratorContinuation::Select { .. }
                        | GeneratorContinuation::HasArgument { .. }
                        | GeneratorContinuation::SortKeyItem(_)
                        | GeneratorContinuation::AlternativeItem(_)
                        | GeneratorContinuation::PredicateItem { .. }
                        | GeneratorContinuation::PredicateCondition { .. }
                        | GeneratorContinuation::PullCount { .. }
                        | GeneratorContinuation::FromStreamItem(_)
                        | GeneratorContinuation::TruncateStreamItem { .. }
                        | GeneratorContinuation::AddItem(_)
                        | GeneratorContinuation::DebugItem
                        | GeneratorContinuation::FromEntries
                        | GeneratorContinuation::Bind { .. }
                        | GeneratorContinuation::BindAlternatives { .. }
                        | GeneratorContinuation::Conditional { .. }
                        | GeneratorContinuation::AccessIndex { .. }
                        | GeneratorContinuation::SliceBase { .. }
                        | GeneratorContinuation::SliceStart { .. }
                        | GeneratorContinuation::SliceEnd { .. }
                        | GeneratorContinuation::ApplyIndex { .. }
                        | GeneratorContinuation::ObjectKey { .. }
                        | GeneratorContinuation::ObjectValue { .. }
                        | GeneratorContinuation::FoldInitial { .. }
                        | GeneratorContinuation::FoldItem { .. }
                        | GeneratorContinuation::FoldUpdate { .. }
                        | GeneratorContinuation::PathField(_)
                        | GeneratorContinuation::PathIndex { .. }
                        | GeneratorContinuation::PathIndexValue { .. }
                        | GeneratorContinuation::PathIterate { .. }
                        | GeneratorContinuation::PathPipe { .. }
                        | GeneratorContinuation::PathJoin(_)
                        | GeneratorContinuation::PathSelect(_)
                        | GeneratorContinuation::PathOrigin
                        | GeneratorContinuation::PathGetPath { .. }
                        | GeneratorContinuation::FreshOrigin
                        | GeneratorContinuation::PathFilter { .. }
                        | GeneratorContinuation::PathFilterResult(_)
                        | GeneratorContinuation::PathConditional { .. }
                        | GeneratorContinuation::PathCatch { .. }
                        | GeneratorContinuation::PathBindAlternatives { .. }
                        | GeneratorContinuation::PathAliases(_)
                        | GeneratorContinuation::PathBind { .. }
                        | GeneratorContinuation::PathBindValue { .. }
                        | GeneratorContinuation::PathSliceBase { .. }
                        | GeneratorContinuation::PathSliceStart { .. }
                        | GeneratorContinuation::PathSliceEnd { .. }
                        | GeneratorContinuation::PathUserReturn { .. }
                        | GeneratorContinuation::PathUserArgument { .. }
                        | GeneratorContinuation::AssignmentPath(_)
                        | GeneratorContinuation::AssignmentRhs(_)
                        | GeneratorContinuation::Interpolate { .. }
                        | GeneratorContinuation::HaltError { .. }
                        | GeneratorContinuation::UserArgument { .. }
                        | GeneratorContinuation::RecurseChild { .. }
                        | GeneratorContinuation::RecurseCondition { .. }
                        | GeneratorContinuation::LoopCondition { .. }
                        | GeneratorContinuation::LoopUpdate { .. }
                        | GeneratorContinuation::RepeatItem
                        | GeneratorContinuation::LimitCount { .. }
                        | GeneratorContinuation::LimitItem { .. }
                        | GeneratorContinuation::BuiltinArguments { .. }
                        | GeneratorContinuation::RegexArguments { .. }
                        | GeneratorContinuation::RegexReplacementItem { .. }
                        | GeneratorContinuation::Raise
                        | GeneratorContinuation::OptionalBoundary { .. }
                        | GeneratorContinuation::IgnoreError
                        | GeneratorContinuation::Catch { .. } => {}
                    }
                }
                if !handled {
                    let _ = emit(Err(error), observations);
                    return observations;
                }
                break;
            }
            let Some(continuation) = work.continuations.pop() else {
                observations.results += 1;
                if !emit(result, observations) {
                    return observations;
                }
                break;
            };
            let Ok(value) = result.as_ref() else {
                unreachable!("error handled before continuation dispatch")
            };
            let value = value.clone();
            match continuation {
                GeneratorContinuation::IgnoreError => {
                    // map_values drops an item when its filter errors, just as
                    // it drops an item when the filter emits empty. The next
                    // mapping task is already queued.
                    result = Ok(value);
                }
                GeneratorContinuation::AccessField(key) => {
                    result = access_field(&value, &key);
                    if result.is_ok()
                        && let Some(origin) = result_origin.take()
                    {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        match origin.child_bounded(
                            PathComponent::Key(Arc::clone(&key)),
                            limits,
                            &mut charge,
                        ) {
                            Ok(child) => result_origin = Some(child),
                            Err(error) => result = Err(error),
                        }
                    }
                }
                GeneratorContinuation::ApplyIndex { base, origin } => {
                    let component = path_component_for_target(&base, &value);
                    result = access_index(&base, &value);
                    if result.is_ok() {
                        result_origin = match (origin, component) {
                            (Some(origin), Ok(component)) => {
                                let mut charge = || {
                                    charge_managed_step(
                                        &mut observations,
                                        limits,
                                        cancellation,
                                        stop,
                                    )
                                };
                                match origin.child_bounded(component, limits, &mut charge) {
                                    Ok(child) => Some(child),
                                    Err(error) => {
                                        result = Err(error);
                                        None
                                    }
                                }
                            }
                            (origin, _) => origin,
                        };
                    }
                }
                GeneratorContinuation::AccessIndex {
                    node,
                    input,
                    origin,
                    environment,
                } => {
                    work.continuations.push(GeneratorContinuation::ApplyIndex {
                        base: value.clone(),
                        origin: result_origin.clone(),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SliceBase {
                    start,
                    end,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    if let Some(start) = start {
                        work.continuations.push(GeneratorContinuation::SliceStart {
                            base: value,
                            end,
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: start,
                            input,
                            origin: origin.clone(),
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    if let Some(end) = end {
                        work.continuations.push(GeneratorContinuation::SliceEnd {
                            base: value,
                            start: None,
                            origin: origin.clone(),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    result = slice(&value, None, None);
                }
                GeneratorContinuation::SliceStart {
                    base,
                    end,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let start = match slice_bound_value(&value) {
                        Ok(start) => start,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(end) = end {
                        work.continuations.push(GeneratorContinuation::SliceEnd {
                            base,
                            start,
                            origin: origin.clone(),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    result = slice(&base, start, None);
                }
                GeneratorContinuation::SliceEnd {
                    base,
                    start,
                    origin,
                } => {
                    result_origin = origin;
                    result = match slice_bound_value(&value) {
                        Ok(end) => slice(&base, start, end),
                        Err(error) => Err(error),
                    };
                }
                GeneratorContinuation::ObjectKey {
                    entries,
                    next,
                    object,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let key = match computed_key(value) {
                        Ok(key) => key,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let Some(entry) = entries.get(next) else {
                        result = Err(invalid("object value missing after validation"));
                        continue;
                    };
                    let value_node = entry.value;
                    work.continuations.push(GeneratorContinuation::ObjectValue {
                        entries,
                        next: next.saturating_add(1),
                        object,
                        key,
                        input: input.clone(),
                        origin: origin.clone(),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: value_node,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations: work.continuations.clone(),
                    }));
                    break;
                }
                GeneratorContinuation::ObjectValue {
                    entries,
                    next,
                    mut object,
                    key,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    object.insert(key, value);
                    if let Some(next_result) = schedule_object_entry(
                        bytecode,
                        &entries,
                        next,
                        object,
                        input,
                        origin,
                        environment,
                        frames,
                        work.continuations.clone(),
                        &mut pending,
                    ) {
                        // A completed object is a normal generator result;
                        // continue through outer continuations so user-call
                        // frames and pull consumers are restored correctly.
                        if next_result.is_ok() {
                            result_origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    result = Err(error);
                                    continue;
                                }
                            };
                        }
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::FoldInitial {
                    generator,
                    pattern,
                    update,
                    extract,
                    emit_each_update,
                    input,
                    environment,
                } => {
                    let state = Rc::new(RefCell::new(FoldState { accumulator: value }));
                    pending.push(GeneratorTask::FoldGenerator {
                        generator,
                        pattern,
                        update,
                        extract,
                        emit_each_update,
                        state,
                        input,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::FoldItem {
                    state,
                    pattern,
                    update,
                    extract,
                    emit_each_update,
                    environment,
                } => {
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
                    let mut charge = |depth| {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if depth >= limits.call_stack {
                            return Err(resource("call-stack"));
                        }
                        if observations.steps >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        observations.steps += 1;
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    if let Err(error) = bind_pattern(
                        bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let accumulator = state.borrow().accumulator.clone();
                    state.borrow_mut().accumulator = Value::Null;
                    work.continuations.push(GeneratorContinuation::FoldUpdate {
                        state,
                        extract,
                        emit_each_update,
                        environment: Arc::new(nested.clone()),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: update,
                        input: accumulator,
                        origin: None,
                        environment: Arc::new(nested),
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::FoldUpdate {
                    state,
                    extract,
                    emit_each_update,
                    environment,
                } => {
                    state.borrow_mut().accumulator = value.clone();
                    if let Some(extract) = extract {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: extract,
                            input: value,
                            origin: None,
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    if emit_each_update {
                        result = Ok(value);
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::PathField(key) => {
                    let Value::Array(path) = value else {
                        result = Err(runtime("assignment left side is not a path".to_owned()));
                        continue;
                    };
                    let mut components = match jq_path(&Value::Array(path)) {
                        Ok(components) => components,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    components.push(PathComponent::Key(key));
                    if components.len() > limits.path_stack {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    observations.path_stack_high_water =
                        observations.path_stack_high_water.max(components.len());
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathIndex {
                    node,
                    input,
                    origin,
                    environment,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let target = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(target) => target,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    work.continuations
                        .push(GeneratorContinuation::PathIndexValue { path, target });
                    work.continuations.push(GeneratorContinuation::PathOrigin);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::PathIndexValue { path, target } => {
                    let component = match path_component_for_target(&target, &value) {
                        Ok(component) => component,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut components = path.components().to_vec();
                    components.push(component);
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathIterate { input } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    match selected {
                        Value::Array(_) | Value::Object(_) => {
                            pending.push(GeneratorTask::PathChildren {
                                value: selected,
                                path,
                                next: 0,
                                environment: Arc::clone(&work.environment),
                                frames: Arc::clone(&work.frames),
                                continuations: work.continuations.clone(),
                            });
                            break;
                        }
                        selected => {
                            result = Err(type_error("update iteration", &selected));
                        }
                    }
                }
                GeneratorContinuation::PathPipe {
                    node,
                    input,
                    environment,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    work.continuations
                        .push(GeneratorContinuation::PathJoin(path.clone()));
                    let mut origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let selected_origin = match origin_at_path_bounded(
                        work.origin.as_ref(),
                        &path,
                        limits,
                        &mut origin_charge,
                    ) {
                        Ok(origin) => origin,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node,
                        input: selected,
                        origin: selected_origin,
                        prefix: Path::root(),
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::PathJoin(prefix) => {
                    let relative = match jq_path(&value) {
                        Ok(path) => path,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut components = prefix.components().to_vec();
                    components.extend(relative);
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathSelect(path)
                | GeneratorContinuation::PathFilterResult(path) => {
                    if value.is_truthy() {
                        delivered_from_path = true;
                        result = Ok(path_value(&path));
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::PathOrigin => {
                    delivered_from_path = true;
                    result = Ok(value);
                }
                GeneratorContinuation::PathGetPath { base, input } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let resolution =
                        scalar::bounded_getpath_resolution(&input, &value, limits, &mut charge);
                    let resolution = match resolution {
                        Ok(resolution) => resolution,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let Some(relative) = resolution.path else {
                        result = Err(runtime("assignment left side is not a path".to_owned()));
                        continue;
                    };
                    let Some(total) = base
                        .components()
                        .len()
                        .checked_add(relative.components().len())
                    else {
                        result = Err(resource("path-stack"));
                        continue;
                    };
                    if total > limits.path_stack {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    if let Err(error) = charge() {
                        result = Err(error);
                        continue;
                    }
                    let mut components = Vec::new();
                    if components.try_reserve_exact(total).is_err() {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    components.extend_from_slice(base.components());
                    components.extend_from_slice(relative.components());
                    delivered_from_path = true;
                    result = Ok(path_value(&Path::new(components)));
                }
                GeneratorContinuation::PathCollect { paths, .. } => {
                    let path = match jq_path(&value) {
                        Ok(path) => path,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    if let Err(error) = paths.borrow_mut().push(&path, limits, &mut charge) {
                        result = Err(error);
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::FreshOrigin => match fresh_origin(&mut next_origin) {
                    Ok(origin) => result_origin = Some(origin),
                    Err(error) => result = Err(error),
                },
                GeneratorContinuation::PathFilter {
                    root,
                    filter,
                    origin,
                    environment,
                    frames,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &root,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let selected_origin = match origin_at_path_bounded(
                        origin.as_ref(),
                        &path,
                        limits,
                        &mut origin_charge,
                    ) {
                        Ok(origin) => origin,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    work.continuations
                        .push(GeneratorContinuation::PathFilterResult(path));
                    work.continuations.push(GeneratorContinuation::FreshOrigin);
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: filter,
                        input: selected,
                        origin: selected_origin,
                        environment,
                        frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::PathConditional {
                    branches,
                    next,
                    alternative,
                    input,
                    origin,
                    prefix,
                    environment,
                } => {
                    let node = if value.is_truthy() {
                        branches[next].1
                    } else if let Some((condition, _)) = branches.get(next + 1) {
                        work.continuations
                            .push(GeneratorContinuation::PathConditional {
                                branches: Arc::clone(&branches),
                                next: next + 1,
                                alternative,
                                input: input.clone(),
                                origin: origin.clone(),
                                prefix: prefix.clone(),
                                environment: Arc::clone(&environment),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: *condition,
                            input,
                            origin: origin.clone(),
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                        break;
                    } else {
                        alternative
                    };
                    if path_candidate_with_aliases(bytecode, node, &work.continuations) {
                        pending.push(GeneratorTask::PathEval(PathWork {
                            node,
                            input,
                            origin: origin.clone(),
                            prefix,
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node,
                            input,
                            origin,
                            environment,
                            frames: work.frames,
                            continuations: work.continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::PathBind {
                    pattern,
                    body,
                    input,
                    input_origin,
                    prefix,
                    environment,
                    entered_from_path,
                } => {
                    if !delivered_from_path {
                        work.continuations
                            .push(GeneratorContinuation::PathBindValue {
                                pattern,
                                body,
                                input,
                                input_origin,
                                prefix,
                                environment,
                                entered_from_path,
                            });
                        result = Ok(value);
                        continue;
                    }
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let selected = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(selected) => selected,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
                    let mut charge = |depth| {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if depth >= limits.call_stack {
                            return Err(resource("call-stack"));
                        }
                        if observations.steps >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        observations.steps += 1;
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    if let Err(error) = bind_pattern(
                        bytecode,
                        &pattern,
                        &selected,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let selected_origin = match origin_at_path_bounded(
                        work.origin.as_ref(),
                        &path,
                        limits,
                        &mut origin_charge,
                    ) {
                        Ok(origin) => origin,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut pattern_origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    if let Err(error) = record_pattern_origin(
                        bytecode,
                        &pattern,
                        selected_origin.clone(),
                        &mut nested.origins,
                        limits,
                        &mut pattern_origin_charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut aliases = current_path_aliases(&work.continuations);
                    clear_path_aliases(bytecode, &pattern, &mut aliases);
                    if let Err(error) = record_pattern_aliases(
                        bytecode,
                        &pattern,
                        &path,
                        selected_origin.as_ref(),
                        &mut aliases,
                        limits,
                        &mut pattern_origin_charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                    if entered_from_path
                        && path_candidate_with_aliases(bytecode, body, &continuations)
                    {
                        pending.push(GeneratorTask::PathEval(PathWork {
                            node: body,
                            input,
                            origin: input_origin.clone(),
                            prefix,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: body,
                            input,
                            origin: input_origin,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::PathBindAlternatives {
                    boundary,
                    patterns,
                    body,
                    input,
                    prefix,
                    environment,
                } => {
                    match schedule_bind_alternative(
                        bytecode,
                        patterns,
                        0,
                        value,
                        input,
                        environment,
                        body,
                        work.frames.clone(),
                        work.continuations.clone(),
                        boundary,
                        &mut pending,
                        limits,
                        cancellation,
                        stop,
                        &mut observations,
                        true,
                        prefix,
                    ) {
                        Ok(()) => break,
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::PathBindValue {
                    pattern,
                    body,
                    input,
                    input_origin,
                    prefix,
                    environment,
                    entered_from_path,
                } => {
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
                    let mut charge = |depth| {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if depth >= limits.call_stack {
                            return Err(resource("call-stack"));
                        }
                        if observations.steps >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        observations.steps += 1;
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    if let Err(error) = bind_pattern(
                        bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut pattern_origin_charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    if let Err(error) = record_pattern_origin(
                        bytecode,
                        &pattern,
                        result_origin.clone(),
                        &mut nested.origins,
                        limits,
                        &mut pattern_origin_charge,
                    ) {
                        result = Err(error);
                        continue;
                    }
                    let mut aliases = current_path_aliases(&work.continuations);
                    clear_path_aliases(bytecode, &pattern, &mut aliases);
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                    if entered_from_path
                        && path_candidate_with_aliases(bytecode, body, &continuations)
                    {
                        pending.push(GeneratorTask::PathEval(PathWork {
                            node: body,
                            input,
                            origin: input_origin,
                            prefix,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: body,
                            input,
                            origin: input_origin,
                            environment: Arc::new(nested),
                            frames: work.frames,
                            continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::PathSliceBase {
                    start,
                    end,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let target = match getpath_managed(
                        &input,
                        path.components(),
                        limits,
                        &mut observations,
                        cancellation,
                        stop,
                    ) {
                        Ok(target) => target,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let assignment = work.continuations.last().and_then(|continuation| {
                        if let GeneratorContinuation::AssignmentPath(state) = continuation {
                            Some(Rc::clone(state))
                        } else {
                            None
                        }
                    });
                    let mut path_continuations = work.continuations.clone();
                    if assignment.is_some() {
                        path_continuations.pop();
                    }
                    if let Some(start) = start {
                        let mut slice_continuations = path_continuations;
                        slice_continuations.push(GeneratorContinuation::PathSliceStart {
                            path,
                            target,
                            end,
                            assignment,
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: start,
                            input,
                            origin: origin.clone(),
                            environment,
                            frames,
                            continuations: slice_continuations,
                        }));
                        break;
                    }
                    if let Some(end) = end {
                        let mut slice_continuations = path_continuations;
                        slice_continuations.push(GeneratorContinuation::PathSliceEnd {
                            path,
                            target,
                            start: None,
                            assignment,
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: slice_continuations,
                        }));
                        break;
                    }
                    if let Some(next_result) = schedule_path_slice(
                        path,
                        target,
                        None,
                        None,
                        assignment,
                        environment,
                        frames,
                        path_continuations,
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::PathSliceStart {
                    path,
                    target,
                    end,
                    assignment,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let start = match slice_bound_value(&value) {
                        Ok(start) => start,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(end) = end {
                        let mut slice_continuations = work.continuations;
                        slice_continuations.push(GeneratorContinuation::PathSliceEnd {
                            path,
                            target,
                            start,
                            assignment,
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: end,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: slice_continuations,
                        }));
                        break;
                    }
                    if let Some(next_result) = schedule_path_slice(
                        path,
                        target,
                        start,
                        None,
                        assignment,
                        environment,
                        frames,
                        work.continuations.clone(),
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::PathSliceEnd {
                    path,
                    target,
                    start,
                    assignment,
                } => {
                    let end = match slice_bound_value(&value) {
                        Ok(end) => end,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(next_result) = schedule_path_slice(
                        path,
                        target,
                        start,
                        end,
                        assignment,
                        Arc::clone(&work.environment),
                        Arc::clone(&work.frames),
                        work.continuations.clone(),
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::PathUserReturn {
                    environment,
                    frames,
                }
                | GeneratorContinuation::ReturnUser {
                    environment,
                    frames,
                } => {
                    work.environment = environment;
                    work.frames = frames;
                    result = Ok(value);
                }
                GeneratorContinuation::AssignmentPath(state) => {
                    if !delivered_from_path {
                        result = Err(runtime("assignment left side is not a path".to_owned()));
                        continue;
                    }
                    let path = match jq_path(&value) {
                        Ok(path) => Path::new(path),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if path.components().len() > limits.path_stack {
                        result = Err(resource("path-stack"));
                        continue;
                    }
                    state
                        .borrow_mut()
                        .targets
                        .push(AssignmentTarget::Path(path));
                    state
                        .borrow_mut()
                        .target_origins
                        .push(result_origin.clone());
                    break;
                }
                GeneratorContinuation::AssignmentRhs(state) => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let output =
                        apply_plain_assignment(&state.borrow(), &value, limits, &mut charge);
                    result = output;
                }
                GeneratorContinuation::AssignmentUpdateRhs {
                    state,
                    target,
                    old,
                    next,
                    boundary,
                    accepted,
                } => {
                    if accepted.replace(true) {
                        break;
                    }
                    pending.retain(|task| !task_owned_by_assignment_rhs(task, boundary));
                    let replacement = match update_value(state.borrow().operator, &old, &value) {
                        Ok(replacement) => replacement,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let updated = match apply_assignment_target(
                        &state.borrow().document,
                        &target,
                        replacement,
                        limits,
                        &mut charge,
                    ) {
                        Ok(updated) => updated,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    state.borrow_mut().document = updated;
                    pending.push(GeneratorTask::AssignmentUpdate {
                        state,
                        next,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::Pipe { node, environment } => {
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input: value,
                        origin: result_origin.clone(),
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::BinaryLeft {
                    operator,
                    right,
                    input,
                    origin,
                    environment,
                } => {
                    if (operator == BinaryOperator::And && !value.is_truthy())
                        || (operator == BinaryOperator::Or && value.is_truthy())
                    {
                        result_origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        result = Ok(Value::Bool(operator == BinaryOperator::Or));
                        continue;
                    }
                    work.continuations.push(GeneratorContinuation::BinaryRight {
                        operator,
                        left: value,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: right,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::BinaryRight { operator, left } => {
                    let computed = if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                        Ok(Value::Bool(value.is_truthy()))
                    } else {
                        binary_value(operator, &left, &value)
                    };
                    match fresh_origin(&mut next_origin) {
                        Ok(origin) => {
                            result_origin = Some(origin);
                            result = computed;
                        }
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::Unary(operator) => match fresh_origin(&mut next_origin) {
                    Ok(origin) => {
                        result_origin = Some(origin);
                        result = unary(operator, &value);
                    }
                    Err(error) => result = Err(error),
                },
                GeneratorContinuation::ArrayItem(values) => {
                    values.borrow_mut().push(value);
                    break;
                }
                GeneratorContinuation::ArrayUpdateItem { values, accepted } => {
                    if !accepted.replace(true) {
                        values.borrow_mut().push(value);
                    }
                    break;
                }
                GeneratorContinuation::ObjectItem {
                    key,
                    values,
                    accepted,
                } => {
                    if !accepted.replace(true) {
                        values.borrow_mut().insert(key, value);
                    }
                    break;
                }
                GeneratorContinuation::Select { input, origin } => {
                    if value.is_truthy() {
                        result_origin = origin;
                        result = Ok(input);
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::HasArgument {
                    input,
                    container_from_result,
                } => {
                    let computed = if container_from_result {
                        has(&value, &input)
                    } else {
                        has(&input, &value)
                    };
                    match fresh_origin(&mut next_origin) {
                        Ok(origin) => {
                            result_origin = Some(origin);
                            result = computed;
                        }
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::SortKeyItem(keys) => {
                    keys.borrow_mut().push(value);
                    break;
                }
                GeneratorContinuation::AlternativeItem(matched) => {
                    if value.is_truthy() {
                        matched.set(true);
                        result = Ok(value);
                    } else {
                        break;
                    }
                }
                GeneratorContinuation::Bind {
                    pattern,
                    body,
                    input,
                    input_origin,
                    environment,
                    origin,
                } => {
                    let mut nested = environment.as_ref().clone();
                    let pattern_depth = work.continuations.len();
                    let mut charge = |depth| {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if depth >= limits.call_stack {
                            return Err(resource("call-stack"));
                        }
                        if observations.steps >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        observations.steps += 1;
                        observations.call_stack_high_water =
                            observations.call_stack_high_water.max(depth + 1);
                        Ok(())
                    };
                    match bind_pattern(
                        bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        pattern_depth,
                        &mut charge,
                    ) {
                        Ok(()) => {
                            let mut pattern_origin_charge = || {
                                charge_managed_step(&mut observations, limits, cancellation, stop)
                            };
                            if let Err(error) = record_pattern_origin(
                                bytecode,
                                &pattern,
                                result_origin.clone(),
                                &mut nested.origins,
                                limits,
                                &mut pattern_origin_charge,
                            ) {
                                result = Err(error);
                                continue;
                            }
                            let mut aliases = current_path_aliases(&work.continuations);
                            clear_path_aliases(bytecode, &pattern, &mut aliases);
                            if let Some(origin) = result_origin.clone().or(origin) {
                                let alias_path = match relative_alias_path(
                                    input_origin.as_ref(),
                                    &origin,
                                    limits,
                                    &mut pattern_origin_charge,
                                ) {
                                    Ok(path) => path,
                                    Err(error) => {
                                        result = Err(error);
                                        continue;
                                    }
                                };
                                if let Err(error) = record_pattern_aliases(
                                    bytecode,
                                    &pattern,
                                    &alias_path,
                                    Some(&origin),
                                    &mut aliases,
                                    limits,
                                    &mut pattern_origin_charge,
                                ) {
                                    result = Err(error);
                                    continue;
                                }
                            }
                            let mut continuations = work.continuations;
                            continuations
                                .push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                            pending.push(GeneratorTask::Eval(GeneratorWork {
                                node: body,
                                input,
                                origin: input_origin,
                                environment: Arc::new(nested),
                                frames: work.frames,
                                continuations,
                            }));
                            break;
                        }
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::BindAlternatives {
                    patterns,
                    body,
                    input,
                    environment,
                } => {
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    match schedule_bind_alternative(
                        bytecode,
                        patterns,
                        0,
                        value,
                        input,
                        environment,
                        body,
                        work.frames.clone(),
                        work.continuations.clone(),
                        boundary,
                        &mut pending,
                        limits,
                        cancellation,
                        stop,
                        &mut observations,
                        false,
                        Path::root(),
                    ) {
                        Ok(()) => break,
                        Err(error) => result = Err(error),
                    }
                }
                GeneratorContinuation::Conditional {
                    branches,
                    next,
                    alternative,
                    input,
                    origin,
                    environment,
                } => {
                    let node = if value.is_truthy() {
                        branches[next].1
                    } else if next + 1 < branches.len() {
                        let condition = branches[next + 1].0;
                        work.continuations.push(GeneratorContinuation::Conditional {
                            branches,
                            next: next + 1,
                            alternative,
                            input: input.clone(),
                            origin: origin.clone(),
                            environment: Arc::clone(&environment),
                        });
                        condition
                    } else {
                        alternative
                    };
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node,
                        input,
                        origin,
                        environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::Iterate => {
                    let cursor = iterate::ContainerCursor::new(value.clone());
                    if !cursor.is_container() {
                        result = Err(type_error("iterate", &value));
                        continue;
                    }
                    pending.push(GeneratorTask::IterateCursor {
                        cursor,
                        origin: result_origin,
                        environment: Arc::clone(&work.environment),
                        frames: Arc::clone(&work.frames),
                        continuations: work.continuations.clone(),
                    });
                    observations.fork_stack_high_water =
                        observations.fork_stack_high_water.max(pending.len());
                    break;
                }
                GeneratorContinuation::PullConsumer { state } => {
                    let mut state = state.borrow_mut();
                    state.seen = true;
                    let boundary = state.boundary;
                    match state.kind {
                        PullConsumerKind::First => {
                            result = Ok(value);
                            drop(state);
                            pending.retain(|task| !task_owned_by_pull_consumer(task, boundary));
                        }
                        PullConsumerKind::Last => {
                            state.latest = Some(value);
                            state.latest_origin = result_origin;
                            break;
                        }
                        PullConsumerKind::IsEmpty => {
                            result = Ok(Value::Bool(false));
                            drop(state);
                            pending.retain(|task| !task_owned_by_pull_consumer(task, boundary));
                        }
                        PullConsumerKind::Nth => {
                            if state.remaining > 0 {
                                state.remaining = state.remaining.saturating_sub(1);
                                break;
                            }
                            state.latest = Some(value);
                            state.latest_origin = result_origin;
                            drop(state);
                            pending
                                .retain(|task| !task_owned_by_pull_consumer_work(task, boundary));
                            break;
                        }
                        PullConsumerKind::Skip => {
                            if state.remaining > 0 {
                                state.remaining = state.remaining.saturating_sub(1);
                                break;
                            }
                            result = Ok(value);
                        }
                        PullConsumerKind::Any | PullConsumerKind::All => {
                            drop(state);
                            result = Err(invalid("predicate consumer continuation mismatch"));
                        }
                    }
                }
                GeneratorContinuation::PredicateItem {
                    state,
                    condition,
                    environment,
                    frames,
                } => {
                    if state.borrow().decision.is_some() {
                        break;
                    }
                    if let Some(condition) = condition {
                        work.continuations
                            .push(GeneratorContinuation::PredicateCondition { state });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: condition,
                            input: value,
                            origin: result_origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    let mut state = state.borrow_mut();
                    state.seen = true;
                    let matches = value.is_truthy();
                    let decisive = match state.kind {
                        PullConsumerKind::Any => matches,
                        PullConsumerKind::All => !matches,
                        _ => false,
                    };
                    if decisive {
                        state.decision = Some(matches);
                        let boundary = state.boundary;
                        drop(state);
                        pending.retain(|task| !task_owned_by_pull_consumer_work(task, boundary));
                    }
                    break;
                }
                GeneratorContinuation::PredicateCondition { state } => {
                    if state.borrow().decision.is_some() {
                        break;
                    }
                    let mut state = state.borrow_mut();
                    state.seen = true;
                    let matches = value.is_truthy();
                    let decisive = match state.kind {
                        PullConsumerKind::Any => matches,
                        PullConsumerKind::All => !matches,
                        _ => false,
                    };
                    if decisive {
                        state.decision = Some(matches);
                        let boundary = state.boundary;
                        drop(state);
                        pending.retain(|task| !task_owned_by_pull_consumer_work(task, boundary));
                    }
                    break;
                }
                GeneratorContinuation::PullCount {
                    kind,
                    input,
                    origin,
                    generator,
                    environment,
                    frames,
                } => {
                    if kind == PullConsumerKind::Nth && generator.is_none() {
                        result = nth_index(&input, &value);
                        if result.is_ok() {
                            result_origin = match &value {
                                Value::Number(number) => {
                                    let mut charge = || {
                                        charge_managed_step(
                                            &mut observations,
                                            limits,
                                            cancellation,
                                            stop,
                                        )
                                    };
                                    match index_origin(
                                        &input,
                                        number.as_f64().trunc(),
                                        origin.as_ref(),
                                        limits,
                                        &mut charge,
                                    ) {
                                        Ok(origin) => origin,
                                        Err(error) => {
                                            result = Err(error);
                                            None
                                        }
                                    }
                                }
                                _ => None,
                            };
                        }
                        continue;
                    }
                    let count = match limit_count(&value) {
                        Ok(count) => count,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    let Some(generator) = generator else {
                        result = Err(invalid("skip arity"));
                        continue;
                    };
                    let boundary = next_boundary;
                    next_boundary = next_boundary.saturating_add(1);
                    let state = Rc::new(RefCell::new(PullConsumerState {
                        boundary,
                        kind,
                        seen: false,
                        latest: None,
                        latest_origin: None,
                        decision: None,
                        remaining: count,
                    }));
                    if kind == PullConsumerKind::Nth {
                        pending.push(GeneratorTask::FinishPullConsumer {
                            state: Rc::clone(&state),
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                            continuations: work.continuations.clone(),
                        });
                    }
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::PullConsumer { state });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: generator,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations,
                    }));
                    break;
                }
                GeneratorContinuation::FromStreamItem(state) => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    match state.borrow_mut().feed(&value, limits, &mut charge) {
                        Ok(Some(value)) => {
                            result = Ok(value);
                            result_origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    result = Err(error);
                                    None
                                }
                            };
                        }
                        Ok(None) => break,
                        Err(error) => {
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::TruncateStreamItem { count } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    match generator::truncate_stream_event(&value, &count, limits, &mut charge) {
                        Ok(Some(value)) => {
                            result = Ok(value);
                            result_origin = match fresh_origin(&mut next_origin) {
                                Ok(origin) => Some(origin),
                                Err(error) => {
                                    result = Err(error);
                                    None
                                }
                            };
                        }
                        Ok(None) => break,
                        Err(error) => {
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::AddItem(state) => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let mut state = state.borrow_mut();
                    state.accumulator = Some(match state.accumulator.take() {
                        None => value,
                        Some(accumulator) => match scalar::bounded_binary_add(
                            &accumulator,
                            &value,
                            limits,
                            &mut charge,
                        ) {
                            Ok(value) => value,
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        },
                    });
                    break;
                }
                GeneratorContinuation::DebugItem => {
                    let bytes = match debug_effect(&value, limits.output_bytes) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    match append_effect(effects, &bytes, limits.output_bytes) {
                        Ok(()) => {
                            work.continuations.push(GeneratorContinuation::DebugItem);
                            break;
                        }
                        Err(error) => {
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::FromEntries => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    result = match &value {
                        Value::Array(entries) => scalar::bounded_from_entries(
                            entries,
                            "with_entries",
                            limits,
                            &mut charge,
                        ),
                        value => Err(type_error("with_entries", value)),
                    };
                }
                GeneratorContinuation::HaltError { input } => {
                    result = match halt_error_text(&input, limits.output_bytes) {
                        Ok(stderr) => match halt_status(&value) {
                            Ok(status) => Err(VmError::Halt {
                                status,
                                stderr: Arc::from(stderr.as_bytes()),
                            }),
                            Err(error) => Err(error),
                        },
                        Err(error) => Err(error),
                    };
                }
                GeneratorContinuation::BindAlternativeBody { .. }
                | GeneratorContinuation::PathBindAlternativeBody { .. }
                | GeneratorContinuation::OptionalBoundary { .. }
                | GeneratorContinuation::Catch { .. }
                | GeneratorContinuation::PathCatch { .. }
                | GeneratorContinuation::Label(_)
                | GeneratorContinuation::PathAliases(_)
                | GeneratorContinuation::RepeatItem => {
                    result = Ok(value);
                }
                GeneratorContinuation::Interpolate {
                    segments,
                    next,
                    slot,
                    mut pieces,
                    input,
                    environment,
                } => {
                    let piece = interpolation_remaining(&pieces, limits.output_bytes)
                        .and_then(|remaining| format::text(&value, remaining));
                    let piece = match piece {
                        Ok(piece) => piece,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    pieces[slot] = Some(piece);
                    if let Some(next_result) = schedule_interpolation(
                        &segments,
                        next,
                        pieces,
                        input,
                        environment,
                        Arc::clone(&work.frames),
                        work.continuations.clone(),
                        bytecode,
                        limits.output_bytes,
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::RegexReplacementItem { name, state } => {
                    let text = match value {
                        Value::String(text) => text,
                        Value::Null => Arc::from(""),
                        value => {
                            result = Err(type_error(&name, &value));
                            continue;
                        }
                    };
                    let checkpoint_steps = Cell::new(observations.steps);
                    let checkpoint = || {
                        if stop.load(Ordering::Relaxed)
                            || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
                        {
                            return Err(VmError::Interrupted);
                        }
                        if checkpoint_steps.get() >= limits.steps {
                            return Err(resource("vm-steps"));
                        }
                        checkpoint_steps.set(checkpoint_steps.get().saturating_add(1));
                        Ok(())
                    };
                    let pushed = state.borrow_mut().push_replacement(text, &checkpoint);
                    observations.steps = checkpoint_steps.get();
                    if let Err(error) = pushed {
                        result = Err(error);
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::RegexArguments {
                    name,
                    arguments,
                    order,
                    next,
                    mut values,
                    replacement,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let Some(slot) = next.checked_sub(1).and_then(|index| order.get(index)) else {
                        result = Err(invalid("regex argument continuation is inconsistent"));
                        continue;
                    };
                    let Some(slot_value) = values.get_mut(*slot) else {
                        result = Err(invalid("regex argument slot is missing"));
                        continue;
                    };
                    *slot_value = Some(value);
                    if next < order.len() {
                        let argument = order[next];
                        work.continuations
                            .push(GeneratorContinuation::RegexArguments {
                                name,
                                arguments: Arc::clone(&arguments),
                                order: Arc::clone(&order),
                                next: next.saturating_add(1),
                                values,
                                replacement,
                                input: input.clone(),
                                origin: origin.clone(),
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[argument],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    let arguments = values
                        .into_iter()
                        .map(|value| value.unwrap_or(Value::Null))
                        .collect::<Vec<_>>();
                    pending.push(GeneratorTask::InvokeRegex {
                        name,
                        arguments,
                        replacement,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::PathUserArgument {
                    symbol,
                    arguments,
                    next,
                    input,
                    input_origin,
                    prefix,
                    caller_environment,
                    caller_frames,
                    filters,
                    mut bindings,
                    from_path,
                } => {
                    let Some(function) = bytecode.functions().get(symbol as usize) else {
                        result = Err(invalid("user function missing after validation"));
                        continue;
                    };
                    let Some(parameter) = function.parameters.get(next) else {
                        result = Err(invalid("user function parameter missing after validation"));
                        continue;
                    };
                    let Some(name) = parameter
                        .runtime_name
                        .and_then(|name| bytecode.string(name))
                    else {
                        result = Err(invalid("value parameter name missing after validation"));
                        continue;
                    };
                    let _bound_origin = if from_path {
                        let path = match jq_path(&value) {
                            Ok(path) => Path::new(path),
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        let mut origin_charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let selected_origin = match origin_at_path_bounded(
                            work.origin.as_ref(),
                            &path,
                            limits,
                            &mut origin_charge,
                        ) {
                            Ok(origin) => origin,
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        match getpath_managed(
                            &input,
                            path.components(),
                            limits,
                            &mut observations,
                            cancellation,
                            stop,
                        ) {
                            Ok(value) => {
                                let mut aliases = current_path_aliases(&work.continuations);
                                aliases.remove(name);
                                if let Some(origin) = selected_origin.clone() {
                                    aliases.insert(
                                        Arc::clone(name),
                                        PathAlias {
                                            path: path.clone(),
                                            origin,
                                        },
                                    );
                                }
                                let mut continuations = work.continuations.clone();
                                continuations
                                    .push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                                work.continuations = continuations;
                                bindings.origins.remove(name);
                                if let Some(origin) = selected_origin.clone() {
                                    bindings.origins.insert(Arc::clone(name), origin);
                                }
                                bindings.insert(Arc::clone(name), value);
                                selected_origin
                            }
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        }
                    } else {
                        let mut aliases = current_path_aliases(&work.continuations);
                        aliases.remove(name);
                        let mut continuations = work.continuations.clone();
                        continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                        work.continuations = continuations;
                        let origin = result_origin.clone();
                        bindings.origins.remove(name);
                        if let Some(origin) = origin.clone() {
                            bindings.origins.insert(Arc::clone(name), origin);
                        }
                        bindings.insert(Arc::clone(name), value);
                        origin
                    };
                    let mut next_value = next.saturating_add(1);
                    while function
                        .parameters
                        .get(next_value)
                        .is_some_and(|parameter| parameter.kind == ParameterKind::Filter)
                    {
                        next_value = next_value.saturating_add(1);
                    }
                    if function
                        .parameters
                        .get(next_value)
                        .is_some_and(|parameter| parameter.kind == ParameterKind::Value)
                    {
                        let mut next_continuations = work.continuations;
                        let from_path = path_candidate_with_aliases(
                            bytecode,
                            arguments[next_value],
                            &next_continuations,
                        );
                        next_continuations.push(GeneratorContinuation::PathUserArgument {
                            symbol,
                            arguments: Arc::clone(&arguments),
                            next: next_value,
                            input: input.clone(),
                            input_origin: input_origin.clone(),
                            prefix: prefix.clone(),
                            caller_environment: Arc::clone(&caller_environment),
                            caller_frames: Arc::clone(&caller_frames),
                            filters,
                            bindings,
                            from_path,
                        });
                        if from_path {
                            pending.push(GeneratorTask::PathEval(PathWork {
                                node: arguments[next_value],
                                input,
                                origin: input_origin.clone(),
                                prefix,
                                environment: caller_environment,
                                frames: caller_frames,
                                continuations: next_continuations,
                            }));
                        } else {
                            pending.push(GeneratorTask::Eval(GeneratorWork {
                                node: arguments[next_value],
                                input,
                                origin: input_origin.clone(),
                                environment: caller_environment,
                                frames: caller_frames,
                                continuations: next_continuations,
                            }));
                        }
                        break;
                    }
                    let mut frames = caller_frames.to_vec();
                    frames.push(UserFrame { symbol, filters });
                    let mut next_continuations = work.continuations;
                    next_continuations.push(GeneratorContinuation::PathUserReturn {
                        environment: Arc::clone(&caller_environment),
                        frames: Arc::clone(&caller_frames),
                    });
                    pending.push(GeneratorTask::PathEval(PathWork {
                        node: function.body,
                        input,
                        origin: input_origin,
                        prefix,
                        environment: Arc::new(bindings),
                        frames: frames.into(),
                        continuations: next_continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlInRight {
                    boundary,
                    state,
                    source,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::SqlInLeft {
                        boundary,
                        state,
                        needle: value,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: source,
                        input,
                        origin,
                        environment,
                        frames,
                        continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlInLeft {
                    boundary,
                    state,
                    needle,
                } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let equal = match sql::equal_bounded(&needle, &value, limits, &mut charge) {
                        Ok(equal) => equal,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                            continue;
                        }
                    };
                    if equal {
                        state.borrow_mut().matched = true;
                        pending.retain(|task| !task_owned_by_sql(task, boundary));
                        match fresh_origin(&mut next_origin) {
                            Ok(origin) => {
                                result_origin = Some(origin);
                                result = Ok(Value::Bool(true));
                            }
                            Err(error) => result = Err(error),
                        }
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::SqlIndexRow {
                    boundary,
                    state,
                    key,
                } => {
                    work.continuations.push(GeneratorContinuation::SqlIndexKey {
                        boundary,
                        state,
                        row: value.clone(),
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: key,
                        input: value,
                        origin: result_origin.clone(),
                        environment: work.environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlIndexKey {
                    boundary,
                    state,
                    row,
                    ..
                } => {
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let inserted = state
                        .borrow_mut()
                        .builder
                        .as_mut()
                        .ok_or_else(|| invalid("SQL INDEX state already finished"))
                        .and_then(|builder| builder.insert(&value, row, &mut charge));
                    match inserted {
                        Ok(()) => break,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                        }
                    }
                }
                GeneratorContinuation::SqlJoinIndex {
                    boundary,
                    key,
                    stream,
                    join,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    if let Some(stream) = stream {
                        let mut continuations = work.continuations;
                        continuations.push(GeneratorContinuation::SqlJoinRow {
                            boundary,
                            index: value,
                            key,
                            join,
                            state: None,
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: stream,
                            input,
                            origin,
                            environment,
                            frames,
                            continuations,
                        }));
                        break;
                    }

                    let cursor = iterate::ContainerCursor::new(input.clone());
                    if !cursor.is_container() {
                        result = Err(runtime(format!("Cannot iterate over {input}")));
                        continue;
                    }
                    let state = Rc::new(RefCell::new(SqlJoinState {
                        collector: Some(sql::SqlPairCollector::new(limits)),
                    }));
                    pending.push(GeneratorTask::SqlJoinFinish {
                        boundary,
                        state: Rc::clone(&state),
                        environment: Arc::clone(&environment),
                        frames: Arc::clone(&frames),
                        continuations: work.continuations.clone(),
                    });
                    let mut continuations = work.continuations;
                    continuations.push(GeneratorContinuation::SqlJoinRow {
                        boundary,
                        index: value,
                        key,
                        join,
                        state: Some(state),
                    });
                    pending.push(GeneratorTask::IterateCursor {
                        cursor,
                        origin,
                        environment,
                        frames,
                        continuations,
                    });
                    break;
                }
                GeneratorContinuation::SqlJoinRow {
                    boundary,
                    index,
                    key,
                    join,
                    state,
                } => {
                    work.continuations.push(GeneratorContinuation::SqlJoinKey {
                        boundary,
                        index,
                        row: value.clone(),
                        join,
                        state,
                    });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: key,
                        input: value,
                        origin: result_origin.clone(),
                        environment: work.environment,
                        frames: work.frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::SqlJoinKey {
                    boundary,
                    index,
                    row,
                    join,
                    state,
                } => {
                    let matched = match sql::typed_lookup(&index, &value) {
                        Ok(matched) => matched,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(state) = state {
                        let mut charge =
                            || charge_managed_step(&mut observations, limits, cancellation, stop);
                        let pushed = state
                            .borrow_mut()
                            .collector
                            .as_mut()
                            .ok_or_else(|| invalid("SQL JOIN state already finished"))
                            .and_then(|collector| collector.push(row, matched, &mut charge));
                        match pushed {
                            Ok(()) => break,
                            Err(error) => {
                                pending.retain(|task| !task_owned_by_sql(task, boundary));
                                result = Err(error);
                                continue;
                            }
                        }
                    }
                    let mut charge =
                        || charge_managed_step(&mut observations, limits, cancellation, stop);
                    let pair = match sql::pair_bounded(row, matched, limits, &mut charge) {
                        Ok(pair) => pair,
                        Err(error) => {
                            pending.retain(|task| !task_owned_by_sql(task, boundary));
                            result = Err(error);
                            continue;
                        }
                    };
                    if let Some(join) = join {
                        let origin = match fresh_origin(&mut next_origin) {
                            Ok(origin) => Some(origin),
                            Err(error) => {
                                result = Err(error);
                                continue;
                            }
                        };
                        let mut continuations = work.continuations;
                        continuations.push(GeneratorContinuation::SqlJoinCallback { boundary });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: join,
                            input: pair,
                            origin,
                            environment: work.environment,
                            frames: work.frames,
                            continuations,
                        }));
                        break;
                    }
                    result_origin = match fresh_origin(&mut next_origin) {
                        Ok(origin) => Some(origin),
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    result = Ok(pair);
                }
                GeneratorContinuation::SqlJoinCallback { .. } => {}
                GeneratorContinuation::BuiltinArguments {
                    name,
                    arguments,
                    order,
                    next,
                    mut values,
                    input,
                    origin,
                    environment,
                    frames,
                } => {
                    let Some(slot) = next.checked_sub(1).and_then(|index| order.get(index)) else {
                        result = Err(invalid("builtin argument continuation is inconsistent"));
                        continue;
                    };
                    let Some(slot_value) = values.get_mut(*slot) else {
                        result = Err(invalid("builtin argument slot is missing"));
                        continue;
                    };
                    *slot_value = Some(value);
                    if next < order.len() {
                        let argument = order[next];
                        work.continuations
                            .push(GeneratorContinuation::BuiltinArguments {
                                name,
                                arguments: Arc::clone(&arguments),
                                order: Arc::clone(&order),
                                next: next.saturating_add(1),
                                values,
                                input: input.clone(),
                                origin: origin.clone(),
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: arguments[argument],
                            input,
                            origin,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                        break;
                    }
                    let Some(arguments) = values.into_iter().collect::<Option<Vec<_>>>() else {
                        result = Err(invalid("builtin argument result is missing"));
                        continue;
                    };
                    pending.push(GeneratorTask::InvokeBuiltin {
                        name,
                        arguments,
                        source_arity: order.len(),
                        input,
                        origin,
                        environment,
                        frames,
                        continuations: work.continuations,
                    });
                    break;
                }
                GeneratorContinuation::UserArgument {
                    symbol,
                    arguments,
                    next,
                    input,
                    caller_environment,
                    caller_frames,
                    filters,
                    mut bindings,
                    input_origin,
                } => {
                    let Some(function) = bytecode.functions().get(symbol as usize) else {
                        result = Err(invalid("user function missing after validation"));
                        continue;
                    };
                    let Some(parameter) = function.parameters.get(next) else {
                        result = Err(invalid("user function parameter missing after validation"));
                        continue;
                    };
                    let Some(name) = parameter
                        .runtime_name
                        .and_then(|name| bytecode.string(name))
                    else {
                        result = Err(invalid("value parameter name missing after validation"));
                        continue;
                    };
                    bindings.insert(Arc::clone(name), value);
                    bindings.origins.remove(name);
                    if let Some(origin) = result_origin.clone() {
                        bindings.origins.insert(Arc::clone(name), origin);
                    }
                    let mut aliases = current_path_aliases(&work.continuations);
                    aliases.remove(name);
                    if let Some(origin) = result_origin.clone()
                        && origin.path.components().is_empty()
                    {
                        aliases.insert(
                            Arc::clone(name),
                            PathAlias {
                                path: Path::root(),
                                origin,
                            },
                        );
                    }
                    let mut continuations = work.continuations.clone();
                    continuations.push(GeneratorContinuation::PathAliases(Arc::new(aliases)));
                    if let Some(next_result) = schedule_user_call(
                        symbol,
                        arguments,
                        next + 1,
                        input,
                        input_origin,
                        caller_environment,
                        caller_frames,
                        filters,
                        bindings,
                        continuations,
                        bytecode,
                        limits.call_stack,
                        &mut pending,
                    ) {
                        result = next_result;
                        continue;
                    }
                    break;
                }
                GeneratorContinuation::RecurseChild {
                    filter,
                    condition,
                    depth,
                    structural,
                    environment,
                    frames,
                } => {
                    if let Some(condition) = condition {
                        work.continuations
                            .push(GeneratorContinuation::RecurseCondition {
                                child: value.clone(),
                                filter,
                                condition,
                                depth,
                                structural,
                                environment: Arc::clone(&environment),
                                frames: Arc::clone(&frames),
                            });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: condition,
                            input: value,
                            origin: None,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                    } else {
                        pending.push(GeneratorTask::RecurseValue {
                            value,
                            filter: Some(filter),
                            condition: None,
                            // A scalar filter chain retains one generator
                            // task, while a filter applied to a container
                            // consumes one structural path component.
                            depth: depth.saturating_add(usize::from(structural)),
                            environment,
                            frames,
                            continuations: work.continuations,
                        });
                    }
                    break;
                }
                GeneratorContinuation::RecurseCondition {
                    child,
                    filter,
                    condition,
                    depth,
                    structural,
                    environment,
                    frames,
                } => {
                    if value.is_truthy() {
                        pending.push(GeneratorTask::RecurseValue {
                            value: child,
                            filter: Some(filter),
                            condition: Some(condition),
                            depth: depth.saturating_add(usize::from(structural)),
                            environment,
                            frames,
                            continuations: work.continuations,
                        });
                    }
                    break;
                }
                GeneratorContinuation::LoopCondition {
                    condition,
                    update,
                    input,
                    until,
                    environment,
                    frames,
                } => {
                    let condition_satisfied = value.is_truthy();
                    if until && condition_satisfied {
                        result = Ok(input);
                    } else if !until && !condition_satisfied {
                        break;
                    } else {
                        let mut update_continuations = work.continuations.clone();
                        update_continuations.push(GeneratorContinuation::LoopUpdate {
                            condition,
                            update,
                            until,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: update,
                            input: input.clone(),
                            origin: None,
                            environment,
                            frames,
                            continuations: update_continuations,
                        }));
                        if until {
                            break;
                        }
                        result = Ok(input);
                    }
                }
                GeneratorContinuation::LoopUpdate {
                    condition,
                    update,
                    until,
                    environment,
                    frames,
                } => {
                    work.continuations
                        .push(GeneratorContinuation::LoopCondition {
                            condition,
                            update,
                            input: value.clone(),
                            until,
                            environment: Arc::clone(&environment),
                            frames: Arc::clone(&frames),
                        });
                    pending.push(GeneratorTask::Eval(GeneratorWork {
                        node: condition,
                        input: value,
                        origin: None,
                        environment,
                        frames,
                        continuations: work.continuations,
                    }));
                    break;
                }
                GeneratorContinuation::LimitCount {
                    expression,
                    input,
                    environment,
                    frames,
                } => {
                    let count = match limit_count(&value) {
                        Ok(count) => count,
                        Err(error) => {
                            result = Err(error);
                            continue;
                        }
                    };
                    if count > 0 {
                        let boundary = next_boundary;
                        next_boundary = next_boundary.saturating_add(1);
                        work.continuations.push(GeneratorContinuation::LimitItem {
                            boundary,
                            remaining: Rc::new(Cell::new(count)),
                        });
                        pending.push(GeneratorTask::Eval(GeneratorWork {
                            node: expression,
                            input,
                            origin: None,
                            environment,
                            frames,
                            continuations: work.continuations,
                        }));
                    }
                    break;
                }
                GeneratorContinuation::LimitItem {
                    boundary,
                    remaining,
                } => {
                    let next = remaining.get().saturating_sub(1);
                    remaining.set(next);
                    if next == 0 {
                        pending.retain(|task| !task_owned_by_limit(task, boundary));
                    }
                    result = Ok(value);
                }
                GeneratorContinuation::Raise => {
                    result = Err(VmError::Raised {
                        message: error_message(&value),
                        value,
                    });
                }
            }
        }
    }
    observations
}

#[allow(
    clippy::too_many_arguments,
    reason = "user-call scheduling keeps every captured VM resource explicit"
)]
fn schedule_user_call(
    symbol: u32,
    arguments: Arc<[u32]>,
    mut next: usize,
    input: Value,
    input_origin: Option<OriginToken>,
    caller_environment: Arc<LexicalEnvironment>,
    caller_frames: UserFrames,
    mut filters: Vec<Option<FilterArgument>>,
    bindings: LexicalEnvironment,
    mut continuations: Vec<GeneratorContinuation>,
    bytecode: &Bytecode,
    call_limit: usize,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    let Some(function) = bytecode.functions().get(symbol as usize) else {
        return Some(Err(invalid("user function missing after validation")));
    };
    if arguments.len() != function.parameters.len() {
        return Some(Err(invalid("user function arity changed after validation")));
    }
    while let Some(parameter) = function.parameters.get(next) {
        match parameter.kind {
            ParameterKind::Filter => {
                filters[next] = Some(FilterArgument {
                    node: arguments[next],
                    environment: Arc::clone(&caller_environment),
                    frames: Arc::clone(&caller_frames),
                });
                next += 1;
            }
            ParameterKind::Value => {
                continuations.push(GeneratorContinuation::UserArgument {
                    symbol,
                    arguments,
                    next,
                    input: input.clone(),
                    input_origin: input_origin.clone(),
                    caller_environment: Arc::clone(&caller_environment),
                    caller_frames: Arc::clone(&caller_frames),
                    filters,
                    bindings,
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node: continuations
                        .last()
                        .and_then(|continuation| match continuation {
                            GeneratorContinuation::UserArgument {
                                arguments, next, ..
                            } => arguments.get(*next).copied(),
                            _ => None,
                        })
                        .expect("just-pushed user argument is structurally complete"),
                    input,
                    origin: input_origin.clone(),
                    environment: caller_environment,
                    frames: caller_frames,
                    continuations,
                }));
                return None;
            }
        }
    }
    if caller_frames.len() >= call_limit {
        return Some(Err(resource("call-stack")));
    }
    continuations.push(GeneratorContinuation::ReturnUser {
        environment: caller_environment,
        frames: Arc::clone(&caller_frames),
    });
    let mut frames = caller_frames.to_vec();
    frames.push(UserFrame { symbol, filters });
    pending.push(GeneratorTask::Eval(GeneratorWork {
        node: function.body,
        input,
        origin: input_origin,
        environment: Arc::new(bindings),
        frames: frames.into(),
        continuations,
    }));
    None
}

#[allow(
    clippy::too_many_arguments,
    reason = "interpolation continuation state is explicit and independently bounded"
)]
fn schedule_interpolation(
    segments: &Arc<[InterpolationOperand]>,
    mut next: usize,
    mut pieces: Vec<Option<Arc<str>>>,
    input: Value,
    environment: Arc<LexicalEnvironment>,
    frames: UserFrames,
    mut continuations: Vec<GeneratorContinuation>,
    bytecode: &Bytecode,
    output_limit: usize,
    pending: &mut Vec<GeneratorTask>,
) -> Option<Result<Value, VmError>> {
    while next > 0 {
        next -= 1;
        match &segments[next] {
            InterpolationOperand::Literal(index) => {
                let Some(value) = bytecode.string(*index) else {
                    return Some(Err(invalid("string missing after validation")));
                };
                pieces[next] = Some(Arc::clone(value));
            }
            InterpolationOperand::Expression(node) => {
                let node = *node;
                continuations.push(GeneratorContinuation::Interpolate {
                    segments: Arc::clone(segments),
                    next,
                    slot: next,
                    pieces,
                    input: input.clone(),
                    environment: Arc::clone(&environment),
                });
                pending.push(GeneratorTask::Eval(GeneratorWork {
                    node,
                    input,
                    origin: None,
                    environment,
                    frames,
                    continuations,
                }));
                return None;
            }
        }
    }
    let capacity = match interpolation_capacity(&pieces, output_limit) {
        Ok(capacity) => capacity,
        Err(error) => return Some(Err(error)),
    };
    let mut output = String::with_capacity(capacity);
    for piece in pieces {
        let Some(piece) = piece else {
            return Some(Err(invalid(
                "interpolation segment missing after evaluation",
            )));
        };
        output.push_str(&piece);
    }
    Some(Ok(Value::string(output)))
}

fn interpolation_pieces(
    segments: &[InterpolationOperand],
    bytecode: &Bytecode,
    output_limit: usize,
) -> Result<Vec<Option<Arc<str>>>, VmError> {
    let mut pieces = vec![None; segments.len()];
    let mut literal_bytes = 0_usize;
    for (slot, segment) in segments.iter().enumerate() {
        let InterpolationOperand::Literal(index) = segment else {
            continue;
        };
        let value = bytecode
            .string(*index)
            .ok_or_else(|| invalid("string missing after validation"))?;
        literal_bytes = literal_bytes
            .checked_add(value.len())
            .filter(|bytes| *bytes <= output_limit)
            .ok_or_else(|| resource("output-bytes"))?;
        pieces[slot] = Some(Arc::clone(value));
    }
    Ok(pieces)
}

fn interpolation_capacity(
    pieces: &[Option<Arc<str>>],
    output_limit: usize,
) -> Result<usize, VmError> {
    let capacity = pieces.iter().flatten().try_fold(0_usize, |total, piece| {
        total
            .checked_add(piece.len())
            .ok_or_else(|| resource("output-bytes"))
    })?;
    if capacity > output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(capacity)
}

fn interpolation_remaining(
    pieces: &[Option<Arc<str>>],
    output_limit: usize,
) -> Result<usize, VmError> {
    interpolation_capacity(pieces, output_limit).map(|used| output_limit - used)
}

fn generator_builtin(
    name: &str,
    input: &Value,
    output_limit: usize,
) -> Result<Option<Value>, VmError> {
    let selected = match name {
        "builtins" => return Ok(Some(builtin_signatures())),
        "not" => return Ok(Some(Value::Bool(!input.is_truthy()))),
        "add" => {
            return fold_values(input, None, binary_add)
                .into_iter()
                .next()
                .transpose();
        }
        "arrays" => matches!(input, Value::Array(_)),
        "booleans" => matches!(input, Value::Bool(_)),
        "finites" => matches!(input, Value::Number(number) if !number.as_f64().is_infinite()),
        "iterables" => matches!(input, Value::Array(_) | Value::Object(_)),
        "nulls" => matches!(input, Value::Null),
        "normals" => matches!(input, Value::Number(number) if number.as_f64().is_normal()),
        "numbers" => matches!(input, Value::Number(_)),
        "objects" => matches!(input, Value::Object(_)),
        "scalars" => !matches!(input, Value::Array(_) | Value::Object(_)),
        "strings" => matches!(input, Value::String(_)),
        "values" => !matches!(input, Value::Null),
        "keys" | "keys_unsorted" => return keys(input, name == "keys").map(Some),
        "to_entries" => return to_entries(input).map(Some),
        "from_entries" => match input {
            Value::Array(entries) => return from_entries(entries, "from_entries").map(Some),
            value => return Err(type_error("from_entries", value)),
        },
        "length" => return length(input).map(Some),
        "max" => return extrema(input, true).into_iter().next().transpose(),
        "min" => return extrema(input, false).into_iter().next().transpose(),
        "reverse" => return reverse(input).into_iter().next().transpose(),
        "sort" => return sort_values(input).into_iter().next().transpose(),
        "tonumber" => match input {
            Value::Number(_) => return Ok(Some(input.clone())),
            Value::String(value) => {
                return parse_jq_number(value)
                    .map(Value::Number)
                    .map(Some)
                    .map_err(|error| runtime(error.to_string()));
            }
            value => return Err(type_error("tonumber", value)),
        },
        "tostring" => {
            return format::text(input, output_limit)
                .map(Value::String)
                .map(Some);
        }
        "@urid" => return string_compat::urid(input, output_limit).map(Some),
        name if name.starts_with('@') => return format::apply(name, input, output_limit).map(Some),
        "type" => return Ok(Some(Value::string(type_name(input)))),
        "unique" => return unique_values(input).into_iter().next().transpose(),
        "utf8bytelength" => match input {
            Value::String(value) => return number_usize(value.len()).map(Some),
            value => return Err(type_error("utf8bytelength", value)),
        },
        "trim" => return string_compat::trim(input).map(Some),
        "ltrim" => return string_compat::ltrim(input).map(Some),
        "rtrim" => return string_compat::rtrim(input).map(Some),
        "toboolean" => return string_compat::toboolean(input).map(Some),
        "ascii_upcase" => return string_compat::ascii_upcase(input).map(Some),
        "ascii_downcase" => return string_compat::ascii_downcase(input).map(Some),
        _ => return Err(invalid("generator built-in left admitted subset")),
    };
    Ok(selected.then(|| input.clone()))
}

fn append_effect(effects: &Arc<EffectState>, bytes: &[u8], limit: usize) -> Result<(), VmError> {
    effects.append(bytes, limit)
}

fn debug_effect(value: &Value, output_limit: usize) -> Result<Vec<u8>, VmError> {
    const PREFIX: &str = "[\"DEBUG:\",";
    const SUFFIX: &str = "]\n";
    let overhead = PREFIX.len().saturating_add(SUFFIX.len());
    if output_limit < overhead {
        return Err(resource("output-bytes"));
    }
    let Value::String(encoded) = format::apply("@json", value, output_limit - overhead)? else {
        return Err(invalid("JSON debug encoding did not produce a string"));
    };
    let total = overhead.saturating_add(encoded.len());
    let mut output = String::new();
    output
        .try_reserve_exact(total)
        .map_err(|_| resource("output-bytes"))?;
    output.push_str(PREFIX);
    output.push_str(&encoded);
    output.push_str(SUFFIX);
    Ok(output.into_bytes())
}

fn halt_error_text(value: &Value, output_limit: usize) -> Result<Arc<str>, VmError> {
    if matches!(value, Value::Null) {
        return Ok(Arc::from(""));
    }
    let text = format::text(value, output_limit)?;
    if matches!(value, Value::String(_)) {
        return Ok(text);
    }
    if text.len() >= output_limit {
        return Err(resource("output-bytes"));
    }
    Ok(Arc::from(format!("{text}\n")))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "jq truncates numeric halt status modulo 256"
)]
fn halt_status(value: &Value) -> Result<u8, VmError> {
    let Value::Number(number) = value else {
        return Err(type_error("halt_error", value));
    };
    let value = number.as_f64();
    if value.is_nan() || value <= 0.0 {
        return Ok(0);
    }
    if value.is_infinite() {
        return Ok(u8::MAX);
    }
    let status = value.trunc() % 256.0;
    Ok(status as u8)
}

fn builtin_signatures() -> Value {
    let signatures = BuiltinRegistry
        .all()
        .iter()
        .flat_map(|builtin| {
            let name = builtin.name;
            (builtin.minimum_arity..=builtin.maximum_arity)
                .map(move |arity| Value::string(format!("{name}/{arity}")))
        })
        .collect::<Vec<_>>();
    Value::array(signatures)
}

fn module_metadata(
    bytecode: &Bytecode,
    input: &Value,
    cancellation: Option<&AtomicBool>,
    stop: &AtomicBool,
) -> Result<Value, VmError> {
    if stop.load(Ordering::Relaxed) || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        return Err(VmError::Interrupted);
    }
    let Value::String(requested) = input else {
        return Err(type_error("modulemeta", input));
    };
    if let Some(module) = bytecode
        .modules()
        .iter()
        .find(|module| module.name == requested.as_ref())
    {
        return Ok(module.metadata.clone());
    }
    let metadata = bytecode
        .load_dynamic_module_metadata(requested)
        .map(|module| module.metadata)
        .map_err(|error| runtime(error.to_string()))?;
    if stop.load(Ordering::Relaxed) || cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed))
    {
        return Err(VmError::Interrupted);
    }
    Ok(metadata)
}

struct Evaluator<'a> {
    bytecode: &'a Bytecode,
    limits: VmLimits,
    observations: Cell<VmObservations>,
    cancellation: Option<&'a AtomicBool>,
    stop: &'a AtomicBool,
    input_cursor: Option<&'a InputCursor>,
    effects: Arc<EffectState>,
}

enum WalkFrame {
    Scalar {
        value: Value,
        depth: usize,
    },
    Array {
        values: Arc<[Value]>,
        next: usize,
        children: Vec<Outcomes>,
        depth: usize,
    },
    Object {
        entries: Vec<(Arc<str>, Value)>,
        next: usize,
        children: Vec<Outcomes>,
        depth: usize,
    },
}

impl WalkFrame {
    fn new(value: Value, depth: usize) -> Self {
        match value {
            Value::Array(values) => Self::Array {
                values,
                next: 0,
                children: Vec::new(),
                depth,
            },
            Value::Object(values) => Self::Object {
                entries: values
                    .iter()
                    .map(|(key, value)| (Arc::clone(key), value.clone()))
                    .collect(),
                next: 0,
                children: Vec::new(),
                depth,
            },
            value => Self::Scalar { value, depth },
        }
    }

    fn next_value(&mut self) -> Option<Value> {
        match self {
            Self::Scalar { .. } => None,
            Self::Array { values, next, .. } => {
                let value = values.get(*next).cloned();
                *next = next.saturating_add(1);
                value
            }
            Self::Object { entries, next, .. } => {
                let value = entries.get(*next).map(|(_, value)| value.clone());
                *next = next.saturating_add(1);
                value
            }
        }
    }

    fn push_children(&mut self, children: Outcomes) {
        match self {
            Self::Scalar { .. } => {}
            Self::Array {
                values,
                next,
                children: collected,
                ..
            } => {
                let failed = children.first().is_some_and(Result::is_err);
                collected.push(children);
                if failed {
                    *next = values.len();
                }
            }
            Self::Object {
                entries,
                next,
                children: collected,
                ..
            } => {
                let failed = children.first().is_some_and(Result::is_err);
                collected.push(children);
                if failed {
                    *next = entries.len();
                }
            }
        }
    }

    fn finish(self, fork_limit: usize) -> Vec<Result<Value, VmError>> {
        match self {
            Self::Scalar { value, .. } => vec![Ok(value)],
            Self::Array { children, .. } => rebuild_walk_array(children),
            Self::Object {
                entries, children, ..
            } => rebuild_walk_object(entries, children, fork_limit),
        }
    }

    fn depth(&self) -> usize {
        match self {
            Self::Scalar { depth, .. } | Self::Array { depth, .. } | Self::Object { depth, .. } => {
                *depth
            }
        }
    }
}

fn rebuild_walk_array(children: Vec<Outcomes>) -> Outcomes {
    let mut values = Vec::new();
    for child in children {
        for result in child {
            match result {
                Ok(value) => values.push(value),
                Err(error) => return one_error(error),
            }
        }
    }
    vec![Ok(Value::array(values))]
}

fn rebuild_walk_object(
    entries: Vec<(Arc<str>, Value)>,
    children: Vec<Outcomes>,
    fork_limit: usize,
) -> Outcomes {
    let mut object = Object::new();
    for ((key, _), child) in entries.into_iter().zip(children) {
        match child.first() {
            Some(Ok(value)) => {
                if object.len() >= fork_limit {
                    return one_error(resource("fork-stack"));
                }
                object.insert(key, value.clone());
            }
            Some(Err(error)) => return one_error(error.clone()),
            None => {}
        }
    }
    vec![Ok(Value::object(object))]
}

impl Evaluator<'_> {
    fn cancelled(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
            || self
                .cancellation
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    fn enter(&self, depth: usize) -> Result<(), VmError> {
        if self.cancelled() {
            return Err(VmError::Interrupted);
        }
        if depth >= self.limits.call_stack {
            return Err(resource("call-stack"));
        }
        let mut observations = self.observations.get();
        if observations.steps >= self.limits.steps {
            return Err(resource("vm-steps"));
        }
        observations.steps += 1;
        observations.call_stack_high_water = observations.call_stack_high_water.max(depth + 1);
        self.observations.set(observations);
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "streaming operation dispatch mirrors the exhaustive evaluator"
    )]
    fn emit_node(
        &self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(instruction) = self.bytecode.instructions().get(node as usize) else {
            return emit(Err(invalid("tree instruction missing after validation")));
        };
        let operation = instruction.operation.clone();
        match operation {
            Operation::RecursiveDescent => self.emit_recursive(input, depth, emit),
            Operation::Interpolation(segments) => {
                self.emit_interpolation(&Arc::from(segments), input, environment, depth, emit)
            }
            Operation::AccessField { base, key } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                let key = match self.string(key) {
                    Ok(key) => Arc::clone(key),
                    Err(error) => return emit(Err(error)),
                };
                self.emit_node(base, input, environment, depth + 1, &mut |result| {
                    emit(result.and_then(|value| access_field(&value, &key)))
                })
            }
            Operation::AccessIndex { base, index } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(base, input, environment, depth + 1, &mut |base| {
                    let Ok(base) = base else {
                        return emit(base);
                    };
                    self.emit_node(index, input, environment, depth + 1, &mut |index| {
                        emit(index.and_then(|index| access_index(&base, &index)))
                    })
                })
            }
            Operation::Iterate(base) => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(
                    base,
                    input,
                    environment,
                    depth + 1,
                    &mut |base| match base {
                        Ok(Value::Array(values)) => {
                            for value in values.iter().cloned() {
                                if !emit(Ok(value)) {
                                    return false;
                                }
                            }
                            true
                        }
                        Ok(Value::Object(values)) => {
                            for value in values.values().cloned() {
                                if !emit(Ok(value)) {
                                    return false;
                                }
                            }
                            true
                        }
                        Ok(value) => emit(Err(type_error("iterate", &value))),
                        Err(error) => emit(Err(error)),
                    },
                )
            }
            Operation::Optional(child) => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(
                    child,
                    input,
                    environment,
                    depth + 1,
                    &mut |result| match result {
                        Err(error) if is_optional_suppressible(&error) => true,
                        result => emit(result),
                    },
                )
            }
            Operation::Pipe { left, right } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(left, input, environment, depth + 1, &mut |left| {
                    let Ok(left) = left else {
                        return emit(left);
                    };
                    self.emit_node(right, &left, environment, depth + 1, emit)
                })
            }
            Operation::Comma { left, right } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                let mut failed = false;
                let keep_going =
                    self.emit_node(left, input, environment, depth + 1, &mut |result| {
                        failed |= result.is_err();
                        emit(result)
                    });
                keep_going && !failed && self.emit_node(right, input, environment, depth + 1, emit)
            }
            Operation::Unary { operator, child } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(child, input, environment, depth + 1, &mut |result| {
                    emit(result.and_then(|value| unary(operator, &value)))
                })
            }
            Operation::Binary {
                operator,
                left,
                right,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                if operator == BinaryOperator::Alternative {
                    let mut accepted = false;
                    let keep_going =
                        self.emit_node(left, input, environment, depth + 1, &mut |result| {
                            match result {
                                Ok(value) if value.is_truthy() => {
                                    accepted = true;
                                    emit(Ok(value))
                                }
                                Ok(_) | Err(_) => true,
                            }
                        });
                    return keep_going
                        && (accepted
                            || self.emit_node(right, input, environment, depth + 1, emit));
                }
                self.emit_node(left, input, environment, depth + 1, &mut |left| {
                    let Ok(left) = left else {
                        return emit(left);
                    };
                    if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                        let left_truthy = left.is_truthy();
                        if (operator == BinaryOperator::And && !left_truthy)
                            || (operator == BinaryOperator::Or && left_truthy)
                        {
                            return emit(Ok(Value::Bool(operator == BinaryOperator::Or)));
                        }
                        return self.emit_node(
                            right,
                            input,
                            environment,
                            depth + 1,
                            &mut |right| emit(right.map(|value| Value::Bool(value.is_truthy()))),
                        );
                    }
                    self.emit_node(
                        right,
                        input,
                        environment,
                        depth + 1,
                        &mut |right| match right {
                            Ok(right) => emit(binary_value(operator, &left, &right)),
                            Err(error) => emit(Err(error)),
                        },
                    )
                })
            }
            Operation::Conditional {
                branches,
                alternative,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_conditional(
                    &branches,
                    0,
                    alternative,
                    input,
                    environment,
                    depth + 1,
                    emit,
                )
            }
            Operation::Bind {
                value,
                pattern,
                body,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(value, input, environment, depth + 1, &mut |value| {
                    let Ok(value) = value else {
                        return emit(value);
                    };
                    let mut nested = environment.clone();
                    let mut charge = |pattern_depth| self.enter(pattern_depth);
                    if let Err(error) = bind_pattern(
                        self.bytecode,
                        &pattern,
                        &value,
                        &mut nested,
                        depth.saturating_add(1),
                        &mut charge,
                    ) {
                        return emit(Err(error));
                    }
                    self.emit_node(body, input, &nested, depth + 1, emit)
                })
            }
            Operation::BindAlternatives {
                value,
                patterns,
                body,
            } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(value, input, environment, depth + 1, &mut |value| {
                    let Ok(value) = value else {
                        return emit(value);
                    };
                    self.emit_bind_alternatives(
                        &patterns,
                        &value,
                        input,
                        body,
                        environment,
                        depth,
                        emit,
                    )
                })
            }
            Operation::Reduce {
                generator,
                pattern,
                initial,
                update,
            } => self.emit_fold(
                generator,
                &pattern,
                initial,
                update,
                None,
                false,
                input,
                environment,
                depth,
                emit,
            ),
            Operation::Foreach {
                generator,
                pattern,
                initial,
                update,
                extract,
            } => self.emit_fold(
                generator,
                &pattern,
                initial,
                update,
                extract,
                true,
                input,
                environment,
                depth,
                emit,
            ),
            Operation::TryCatch { expression, catch } => {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(
                    expression,
                    input,
                    environment,
                    depth + 1,
                    &mut |result| match result {
                        Ok(value) => emit(Ok(value)),
                        Err(error) if is_catchable_error(&error) => catch.is_none_or(|catch| {
                            self.emit_node(
                                catch,
                                &catch_value(&error),
                                environment,
                                depth + 1,
                                emit,
                            )
                        }),
                        Err(error) => emit(Err(error)),
                    },
                )
            }
            Operation::Label { symbol, body } => {
                self.emit_node(body, input, environment, depth + 1, &mut |result| {
                    if matches!(result, Err(VmError::Break { label }) if label == symbol) {
                        false
                    } else {
                        emit(result)
                    }
                });
                true
            }
            Operation::Break(symbol) => emit(Err(VmError::Break { label: symbol })),
            Operation::Call { name, arguments } => {
                let name = match self.string(name) {
                    Ok(name) => Arc::clone(name),
                    Err(error) => return emit(Err(error)),
                };
                match name.as_ref() {
                    "IN" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_sql_in(&arguments, input, environment, depth + 1, emit)
                    }
                    "combinations" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_combinations(&arguments, input, environment, depth + 1, emit)
                    }
                    "JOIN" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_sql_join(&arguments, input, environment, depth + 1, emit)
                    }
                    "empty" => self
                        .enter(depth)
                        .map_or_else(|error| emit(Err(error)), |()| true),
                    "first" | "last" | "nth" | "skip" | "isempty" => self.emit_pull_consumer(
                        name.as_ref(),
                        &arguments,
                        input,
                        environment,
                        depth + 1,
                        emit,
                    ),
                    "select" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(argument) = arguments.first() else {
                            return emit(Err(invalid("select argument missing")));
                        };
                        self.emit_node(*argument, input, environment, depth + 1, &mut |selected| {
                            match selected {
                                Ok(value) if value.is_truthy() => emit(Ok(input.clone())),
                                Ok(_) => true,
                                Err(error) => emit(Err(error)),
                            }
                        })
                    }
                    "range" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        self.emit_range(&arguments, input, environment, depth + 1, emit)
                    }
                    "error" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(argument) = arguments.first() else {
                            return emit(Err(raised(input.clone())));
                        };
                        let mut completed = false;
                        self.emit_node(*argument, input, environment, depth + 1, &mut |result| {
                            if completed {
                                return false;
                            }
                            completed = true;
                            emit(result.map_or_else(Err, |value| Err(raised(value))));
                            false
                        })
                    }
                    "input" | "inputs" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(cursor) = self.input_cursor else {
                            return if name.as_ref() == "input" {
                                emit(Err(runtime("break".to_owned())))
                            } else {
                                true
                            };
                        };
                        if name.as_ref() == "input" {
                            return match cursor.next_value() {
                                Ok(Some(value)) => emit(Ok(value)),
                                Ok(None) => emit(Err(runtime("break".to_owned()))),
                                Err(error) => emit(Err(error)),
                            };
                        }
                        loop {
                            match cursor.next_value() {
                                Ok(Some(value)) => {
                                    if !emit(Ok(value)) {
                                        return true;
                                    }
                                }
                                Ok(None) => return true,
                                Err(error) => return emit(Err(error)),
                            }
                        }
                    }
                    "paths" => self.emit_paths(&arguments, input, environment, depth + 1, emit),
                    "fromstream" => {
                        self.emit_fromstream(&arguments, input, environment, depth + 1, emit)
                    }
                    "tostream" => self.emit_tostream(input, depth + 1, emit),
                    "truncate_stream" => {
                        self.emit_truncate_stream(&arguments, input, environment, depth + 1, emit)
                    }
                    "limit" => {
                        if let Err(error) = self.enter(depth) {
                            return emit(Err(error));
                        }
                        let Some(count_node) = arguments.first() else {
                            return emit(Err(invalid("limit count missing")));
                        };
                        let Some(expression) = arguments.get(1) else {
                            return emit(Err(invalid("limit expression missing")));
                        };
                        let mut keep_going = true;
                        self.emit_node(*count_node, input, environment, depth + 1, &mut |count| {
                            let count = match count.and_then(|value| limit_count(&value)) {
                                Ok(count) => count,
                                Err(error) => {
                                    keep_going = emit(Err(error));
                                    return keep_going;
                                }
                            };
                            let mut emitted = 0usize;
                            if count > 0 {
                                self.emit_node(
                                    *expression,
                                    input,
                                    environment,
                                    depth + 1,
                                    &mut |result| {
                                        let success = result.is_ok();
                                        keep_going = emit(result);
                                        if success {
                                            emitted += 1;
                                        }
                                        keep_going && emitted < count
                                    },
                                );
                            }
                            keep_going
                        });
                        true
                    }
                    _ => self
                        .node(node, input, environment, depth)
                        .into_iter()
                        .all(emit),
                }
            }
            _ => self
                .node(node, input, environment, depth)
                .into_iter()
                .all(emit),
        }
    }

    fn emit_recursive(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut cursor = TraversalCursor::new(input.clone());
        loop {
            if cursor.depth() > self.limits.path_stack {
                return emit(Err(resource("path-stack")));
            }
            let mut observations = self.observations.get();
            observations.path_stack_high_water =
                observations.path_stack_high_water.max(cursor.depth());
            self.observations.set(observations);
            let value = match cursor.next(self.limits.path_stack) {
                Ok(Some(value)) => value,
                Ok(None) => return true,
                Err(error) => return emit(Err(error)),
            };
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            if !emit(Ok(value)) {
                return false;
            }
        }
    }

    fn emit_descendant_paths(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut pending = Vec::new();
        push_children(input, &[], &mut pending);
        while let Some((value, components)) = pending.pop() {
            if components.len() > self.limits.path_stack {
                return emit(Err(resource("path-stack")));
            }
            if let Err(error) = self.enter(depth.saturating_add(components.len())) {
                return emit(Err(error));
            }
            if !emit(Ok(path_value(&Path::new(components.clone())))) {
                return true;
            }
            push_children(&value, &components, &mut pending);
        }
        true
    }

    fn emit_paths(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(filter) = arguments.first() else {
            return self.emit_descendant_paths(input, depth, emit);
        };
        let candidates = match self.descendant_path_values(input, depth) {
            Ok(candidates) => candidates,
            Err(error) => return emit(Err(error)),
        };
        for (path, value) in candidates.into_iter().skip(1) {
            let mut continue_paths = true;
            self.emit_node(
                *filter,
                &value,
                environment,
                depth + 1,
                &mut |result| match result {
                    Ok(value) if value.is_truthy() => {
                        continue_paths = emit(Ok(path_value(&path)));
                        continue_paths
                    }
                    Ok(_) => true,
                    Err(error) => {
                        let _ = emit(Err(error));
                        continue_paths = false;
                        false
                    }
                },
            );
            if !continue_paths {
                return false;
            }
        }
        true
    }

    fn emit_tostream(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        enum Frame {
            Visit(Value, Vec<PathComponent>, bool),
            Close(Vec<PathComponent>),
        }

        let mut pending = vec![Frame::Visit(input.clone(), Vec::new(), true)];
        while let Some(frame) = pending.pop() {
            match frame {
                Frame::Close(path) => {
                    if let Err(error) = self.enter(depth.saturating_add(path.len())) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(Value::array(vec![path_value(&Path::new(path))]))) {
                        return true;
                    }
                }
                Frame::Visit(value, path, root) => {
                    if path.len() > self.limits.path_stack {
                        return emit(Err(resource("path-stack")));
                    }
                    if let Err(error) = self.enter(depth.saturating_add(path.len())) {
                        return emit(Err(error));
                    }
                    match &value {
                        Value::Array(values) if !values.is_empty() => {
                            if !root && (path.len() != 1 || root_child_is_last(input, &path)) {
                                pending.push(Frame::Close(path.clone()));
                            }
                            for (index, child) in values.iter().enumerate().rev() {
                                let mut child_path = path.clone();
                                child_path.push(PathComponent::Index(index));
                                pending.push(Frame::Visit(child.clone(), child_path, false));
                            }
                        }
                        Value::Object(values) if !values.is_empty() => {
                            if !root && (path.len() != 1 || root_child_is_last(input, &path)) {
                                pending.push(Frame::Close(path.clone()));
                            }
                            for (key, child) in values.iter().rev() {
                                let mut child_path = path.clone();
                                child_path.push(PathComponent::Key(Arc::clone(key)));
                                pending.push(Frame::Visit(child.clone(), child_path, false));
                            }
                        }
                        _ => {
                            let record =
                                Value::array(vec![path_value(&Path::new(path.clone())), value]);
                            if !emit(Ok(record)) {
                                return false;
                            }
                            if !root
                                && (path.len() != 1 || root_child_is_last(input, &path))
                                && !emit(Ok(Value::array(vec![path_value(&Path::new(path))])))
                            {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn emit_interpolation(
        &self,
        segments: &Arc<[InterpolationOperand]>,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let expression_count = segments
            .iter()
            .filter(|segment| matches!(segment, InterpolationOperand::Expression(_)))
            .count();
        if expression_count >= self.limits.call_stack {
            return emit(Err(resource("call-stack")));
        }
        let pieces = match interpolation_pieces(segments, self.bytecode, self.limits.output_bytes) {
            Ok(pieces) => pieces,
            Err(error) => return emit(Err(error)),
        };

        let mut pending = vec![InterpolationWork::Expand {
            next: segments.len(),
            pieces,
        }];
        while let Some(work) = pending.pop() {
            let (mut next, pieces) = match work {
                InterpolationWork::Expand { next, pieces } => (next, pieces),
                InterpolationWork::Error(error) => return emit(Err(error)),
            };

            let mut expanded = false;
            while next > 0 {
                next -= 1;
                let InterpolationOperand::Expression(node) = &segments[next] else {
                    continue;
                };
                let slot = next;
                let available = self.limits.fork_stack.saturating_sub(pending.len());
                let mut outcomes = Vec::new();
                let mut overflow = false;
                self.emit_node(*node, input, environment, depth + 1, &mut |result| {
                    if outcomes.len() >= available {
                        overflow = true;
                        return false;
                    }
                    let failed = result.is_err();
                    outcomes.push(result);
                    !failed
                });
                if overflow {
                    return emit(Err(resource("fork-stack")));
                }
                for outcome in outcomes.into_iter().rev() {
                    match outcome {
                        Ok(value) => {
                            let mut nested = pieces.clone();
                            let piece = interpolation_remaining(&nested, self.limits.output_bytes)
                                .and_then(|remaining| format::text(&value, remaining));
                            match piece {
                                Ok(piece) => {
                                    nested[slot] = Some(piece);
                                    pending.push(InterpolationWork::Expand {
                                        next,
                                        pieces: nested,
                                    });
                                }
                                Err(error) => pending.push(InterpolationWork::Error(error)),
                            }
                        }
                        Err(error) => pending.push(InterpolationWork::Error(error)),
                    }
                }
                let mut observations = self.observations.get();
                observations.fork_stack_high_water =
                    observations.fork_stack_high_water.max(pending.len());
                self.observations.set(observations);
                expanded = true;
                break;
            }
            if expanded {
                continue;
            }

            let capacity = match interpolation_capacity(&pieces, self.limits.output_bytes) {
                Ok(capacity) => capacity,
                Err(error) => return emit(Err(error)),
            };
            let mut output = String::with_capacity(capacity);
            for piece in pieces {
                let Some(piece) = piece else {
                    return emit(Err(invalid(
                        "interpolation segment missing after evaluation",
                    )));
                };
                output.push_str(&piece);
            }
            if !emit(Ok(Value::string(output))) {
                return false;
            }
        }
        true
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "fold bytecode carries four bodies plus the shared evaluator context"
    )]
    fn emit_fold(
        &self,
        generator: u32,
        pattern: &BindingPatternOperand,
        initial: u32,
        update: u32,
        extract: Option<u32>,
        emit_each_update: bool,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Err(error) = self.enter_fold_frame(depth) {
            return emit(Err(error));
        }
        self.emit_node(
            initial,
            input,
            environment,
            depth + 1,
            &mut |initial_result| {
                let mut accumulator = match initial_result {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = emit(Err(error));
                        return false;
                    }
                };
                let mut fold_failed = false;
                let keep_going =
                    self.emit_node(generator, input, environment, depth + 1, &mut |generated| {
                        let item = match generated {
                            Ok(value) => value,
                            Err(error) => {
                                fold_failed = true;
                                let _ = emit(Err(error));
                                return false;
                            }
                        };
                        if let Err(error) = self.enter(depth + 1) {
                            fold_failed = true;
                            let _ = emit(Err(error));
                            return false;
                        }
                        let mut nested = environment.clone();
                        let mut charge = |pattern_depth| self.enter(pattern_depth);
                        if let Err(error) = bind_pattern(
                            self.bytecode,
                            pattern,
                            &item,
                            &mut nested,
                            depth.saturating_add(1),
                            &mut charge,
                        ) {
                            fold_failed = true;
                            let _ = emit(Err(error));
                            return false;
                        }
                        let mut next_accumulator = Value::Null;
                        let mut update_failed = false;
                        let update_kept_going = self.emit_node(
                            update,
                            &accumulator,
                            &nested,
                            depth + 1,
                            &mut |updated| {
                                let updated = match updated {
                                    Ok(value) => value,
                                    Err(error) => {
                                        update_failed = true;
                                        let _ = emit(Err(error));
                                        return false;
                                    }
                                };
                                next_accumulator = updated.clone();
                                let Some(extract) = extract else {
                                    return if emit_each_update {
                                        emit(Ok(updated))
                                    } else {
                                        true
                                    };
                                };
                                let mut extract_failed = false;
                                let extracted = self.emit_node(
                                    extract,
                                    &updated,
                                    &nested,
                                    depth + 1,
                                    &mut |result| {
                                        extract_failed |= result.is_err();
                                        let accepted = emit(result);
                                        accepted && !extract_failed
                                    },
                                );
                                if extract_failed {
                                    update_failed = true;
                                }
                                extracted && !extract_failed
                            },
                        );
                        accumulator = next_accumulator;
                        if update_failed {
                            fold_failed = true;
                        }
                        update_kept_going && !update_failed
                    });
                if !keep_going || fold_failed {
                    return false;
                }
                extract.is_some() || emit_each_update || emit(Ok(accumulator))
            },
        )
    }

    fn enter_fold_frame(&self, depth: usize) -> Result<(), VmError> {
        self.enter(depth)?;
        let frames = depth.saturating_add(1);
        if frames > self.limits.value_stack {
            return Err(resource("value-stack"));
        }
        let mut observations = self.observations.get();
        observations.value_stack_high_water = observations.value_stack_high_water.max(frames);
        self.observations.set(observations);
        Ok(())
    }

    fn emit_range(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut numbers = Vec::new();
        for argument in arguments {
            match first_value(self.node(*argument, input, environment, depth)) {
                Ok(Value::Number(number)) => numbers.push(number.as_f64()),
                Ok(value) => return emit(Err(type_error("range", &value))),
                Err(error) => return emit(Err(error)),
            }
        }
        let (mut current, end, step) = match numbers.as_slice() {
            [end] => (0.0, *end, 1.0),
            [start, end] => (*start, *end, 1.0),
            [start, end, step] => (*start, *end, *step),
            _ => return emit(Err(invalid("range arity"))),
        };
        if step == 0.0 {
            return emit(Err(runtime("range step cannot be zero".to_owned())));
        }
        while (step > 0.0 && current < end) || (step < 0.0 && current > end) {
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            let Some(value) = number_value(current).pop() else {
                return emit(Err(invalid("range produced no numeric value")));
            };
            if !emit(value) {
                return false;
            }
            let next = current + step;
            if next.to_bits() == current.to_bits() {
                return emit(Err(resource("vm-steps")));
            }
            current = next;
        }
        true
    }

    fn emit_current_values(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        match input {
            Value::Array(values) => {
                for value in values.iter().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            Value::Object(values) => {
                for value in values.values().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            value => emit(Err(type_error("index", value))),
        }
    }

    fn emit_pull_consumer(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Err(error) = self.enter(depth) {
            return emit(Err(error));
        }
        match name {
            "first" if arguments.is_empty() => Self::emit_direct_index(0, input, emit),
            "first" => self.emit_first(arguments, input, environment, depth, emit),
            "last" if arguments.is_empty() => Self::emit_direct_index(-1, input, emit),
            "last" => self.emit_last(arguments, input, environment, depth, emit),
            "nth" | "skip" => {
                self.emit_nth_or_skip(name, arguments, input, environment, depth, emit)
            }
            "isempty" => self.emit_isempty(arguments, input, environment, depth, emit),
            _ => emit(Err(invalid("unknown pull consumer"))),
        }
    }

    fn emit_direct_index(
        index: i64,
        input: &Value,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let index = Value::Number(
            Number::parse(&index.to_string()).expect("literal consumer index is valid"),
        );
        emit(access_index(input, &index))
    }

    fn emit_pull_generator(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let generator_argument = match name {
            "nth" | "skip" => arguments.len() == 2,
            _ => arguments.len() == 1,
        };
        if generator_argument && let Some(argument) = arguments.last().copied() {
            return self.emit_node(argument, input, environment, depth + 1, emit);
        }
        self.emit_current_values(input, depth + 1, emit)
    }

    fn emit_first(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if arguments.len() > 1 {
            return emit(Err(invalid("first arity")));
        }
        let mut accepted = true;
        self.emit_pull_generator(
            "first",
            arguments,
            input,
            environment,
            depth,
            &mut |result| {
                accepted = emit(result);
                false
            },
        );
        accepted
    }

    fn emit_last(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if arguments.len() > 1 {
            return emit(Err(invalid("last arity")));
        }
        let mut latest = None;
        let mut failure = None;
        self.emit_pull_generator(
            "last",
            arguments,
            input,
            environment,
            depth,
            &mut |result| {
                match result {
                    Ok(value) => latest = Some(value),
                    Err(error) => {
                        failure = Some(error);
                        return false;
                    }
                }
                true
            },
        );
        if let Some(error) = failure {
            emit(Err(error))
        } else if let Some(value) = latest {
            emit(Ok(value))
        } else {
            true
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "nth and skip retain the count expression and generator evaluation context"
    )]
    fn emit_nth_or_skip(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(count_node) = arguments.first().copied() else {
            return emit(Err(invalid(if name == "nth" {
                "nth count missing"
            } else {
                "skip count missing"
            })));
        };
        if name == "nth" && arguments.len() == 1 {
            let mut accepted = true;
            self.emit_node(count_node, input, environment, depth, &mut |count| {
                accepted = emit(count.and_then(|count| nth_index(input, &count)));
                accepted
            });
            return accepted;
        }
        if name == "nth" && arguments.len() > 2 {
            return emit(Err(invalid("nth arity")));
        }
        if name == "skip" && arguments.len() != 2 {
            return emit(Err(invalid("skip arity")));
        }
        let mut accepted = true;
        self.emit_node(count_node, input, environment, depth, &mut |count| {
            let count = match count {
                Ok(count) => match limit_count(&count) {
                    Ok(count) => count,
                    Err(error) => {
                        accepted = emit(Err(error));
                        return accepted;
                    }
                },
                Err(error) => {
                    accepted = emit(Err(error));
                    return accepted;
                }
            };
            let mut remaining = count;
            self.emit_pull_generator(name, arguments, input, environment, depth, &mut |result| {
                match result {
                    Ok(value) if name == "nth" && remaining == 0 => {
                        accepted = emit(Ok(value));
                        false
                    }
                    Ok(_) if name == "nth" => {
                        remaining = remaining.saturating_sub(1);
                        true
                    }
                    Ok(value) if remaining == 0 => {
                        accepted = emit(Ok(value));
                        accepted
                    }
                    Ok(_) => {
                        remaining = remaining.saturating_sub(1);
                        true
                    }
                    Err(error) => {
                        accepted = emit(Err(error));
                        false
                    }
                }
            });
            accepted
        });
        accepted
    }

    fn emit_isempty(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if arguments.len() != 1 {
            return emit(Err(invalid("isempty arity")));
        }
        let mut accepted = true;
        let mut found = false;
        self.emit_pull_generator(
            "isempty",
            arguments,
            input,
            environment,
            depth,
            &mut |result| {
                found = true;
                accepted = match result {
                    Ok(_) => emit(Ok(Value::Bool(false))),
                    Err(error) => emit(Err(error)),
                };
                false
            },
        );
        if found {
            accepted
        } else {
            emit(Ok(Value::Bool(true)))
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "alternative binding keeps retry and resource state explicit"
    )]
    fn emit_bind_alternatives(
        &self,
        patterns: &[BindingPatternOperand],
        value: &Value,
        input: &Value,
        body: u32,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let mut last_error = None;
        for pattern in patterns {
            let mut nested = environment.clone();
            if let Err(error) = initialize_pattern_variables(self.bytecode, patterns, &mut nested) {
                last_error = Some(error);
                continue;
            }
            let mut charge = |pattern_depth| self.enter(pattern_depth);
            if let Err(error) = bind_pattern(
                self.bytecode,
                pattern,
                value,
                &mut nested,
                depth.saturating_add(1),
                &mut charge,
            ) {
                if !is_pattern_mismatch(&error) {
                    return emit(Err(error));
                }
                last_error = Some(error);
            } else {
                let mut body_failed = None;
                let keep_going =
                    self.emit_node(
                        body,
                        input,
                        &nested,
                        depth + 1,
                        &mut |result| match result {
                            Ok(value) => emit(Ok(value)),
                            Err(error) => {
                                body_failed = Some(error);
                                true
                            }
                        },
                    );
                if let Some(error) = body_failed {
                    if !is_retryable_alternative_error(&error) {
                        return emit(Err(error));
                    }
                    last_error = Some(error);
                    continue;
                }
                return keep_going;
            }
        }
        emit(Err(last_error.unwrap_or_else(|| {
            runtime("destructuring alternative has no patterns".to_owned())
        })))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the bytecode operation dispatch is intentionally exhaustive"
    )]
    fn node(&self, node: u32, input: &Value, environment: &Environment, depth: usize) -> Outcomes {
        if let Err(error) = self.enter(depth) {
            return one_error(error);
        }
        let Some(instruction) = self.bytecode.instructions().get(node as usize) else {
            return one_error(invalid("tree instruction missing after validation"));
        };
        match &instruction.operation {
            Operation::Identity => vec![Ok(input.clone())],
            Operation::Literal(index) => self
                .bytecode
                .constants()
                .get(*index as usize)
                .cloned()
                .map_or_else(
                    || one_error(invalid("literal missing after validation")),
                    |value| vec![Ok(value)],
                ),
            Operation::Variable(index) => match self.string(*index) {
                Ok(name) => environment.get(name).cloned().map_or_else(
                    || one_error(runtime(format!("variable ${name} has no value"))),
                    |value| vec![Ok(value)],
                ),
                Err(error) => one_error(error),
            },
            Operation::Empty => Vec::new(),
            Operation::RecursiveDescent => {
                let mut output = Vec::new();
                self.emit_recursive(input, depth, &mut |result| {
                    output.push(result);
                    true
                });
                output
            }
            Operation::Interpolation(segments) => {
                let mut output = Vec::new();
                self.emit_interpolation(
                    &Arc::from(segments.clone()),
                    input,
                    environment,
                    depth,
                    &mut |result| {
                        output.push(result);
                        true
                    },
                );
                output
            }
            Operation::AccessField { base, key } => {
                let key = match self.string(*key) {
                    Ok(key) => Arc::clone(key),
                    Err(error) => return one_error(error),
                };
                let bases = self.node(*base, input, environment, depth + 1);
                map_outcomes(bases, |value| access_field(value, &key))
            }
            Operation::AccessIndex { base, index } => {
                let bases = self.node(*base, input, environment, depth + 1);
                let mut output = Vec::new();
                for base in bases {
                    match base {
                        Ok(base) => {
                            for index in self.node(*index, input, environment, depth + 1) {
                                output.push(index.and_then(|index| access_index(&base, &index)));
                            }
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Slice { base, start, end } => {
                let bases = self.node(*base, input, environment, depth + 1);
                let mut output = Vec::new();
                for base in bases {
                    let Ok(base) = base else {
                        output.push(base);
                        continue;
                    };
                    let starts = self.bound(*start, input, environment, depth + 1);
                    let ends = self.bound(*end, input, environment, depth + 1);
                    for start in &starts {
                        for end in &ends {
                            output.push(match (start, end) {
                                (Ok(start), Ok(end)) => slice(&base, *start, *end),
                                (Err(error), _) | (_, Err(error)) => Err(error.clone()),
                            });
                        }
                    }
                }
                output
            }
            Operation::Iterate(base) => {
                let bases = self.node(*base, input, environment, depth + 1);
                let mut output = Vec::new();
                for base in bases {
                    match base {
                        Ok(Value::Array(values)) => output.extend(values.iter().cloned().map(Ok)),
                        Ok(Value::Object(values)) => {
                            output.extend(values.values().cloned().map(Ok));
                        }
                        Ok(value) => output.push(Err(type_error("iterate", &value))),
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Optional(child) => self
                .node(*child, input, environment, depth + 1)
                .into_iter()
                .flatten()
                .map(Ok)
                .collect(),
            Operation::Pipe { left, right } => {
                let mut output = Vec::new();
                for value in self.node(*left, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            output.extend(self.node(*right, &value, environment, depth + 1));
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Comma { left, right } => {
                let mut output = self.node(*left, input, environment, depth + 1);
                if !ends_in_error(&output) {
                    output.extend(self.node(*right, input, environment, depth + 1));
                }
                output
            }
            Operation::Array(child) => {
                let results = self.node(*child, input, environment, depth + 1);
                match collect_values(results) {
                    Ok(values) => vec![Ok(Value::array(values))],
                    Err(error) => one_error(error),
                }
            }
            Operation::Object(entries) => {
                let mut candidates = vec![Object::new()];
                for entry in entries {
                    let keys = match &entry.key {
                        KeyOperand::Static(index) => match self.string(*index) {
                            Ok(key) => vec![Ok(Arc::clone(key))],
                            Err(error) => return one_error(error),
                        },
                        KeyOperand::Computed(node) => self
                            .node(*node, input, environment, depth + 1)
                            .into_iter()
                            .map(|value| value.and_then(computed_key))
                            .collect(),
                    };
                    let values = self.node(entry.value, input, environment, depth + 1);
                    let mut next = Vec::new();
                    for candidate in &candidates {
                        for key in &keys {
                            for value in &values {
                                match (key, value) {
                                    (Ok(key), Ok(value)) => {
                                        let mut object = candidate.clone();
                                        object.insert(Arc::clone(key), value.clone());
                                        next.push(object);
                                    }
                                    (Err(error), _) | (_, Err(error)) => {
                                        return one_error(error.clone());
                                    }
                                }
                            }
                        }
                    }
                    candidates = next;
                }
                candidates.into_iter().map(Value::object).map(Ok).collect()
            }
            Operation::Unary { operator, child } => {
                let values = self.node(*child, input, environment, depth + 1);
                map_outcomes(values, |value| unary(*operator, value))
            }
            Operation::Binary {
                operator,
                left,
                right,
            } => self.binary(*operator, *left, *right, input, environment, depth + 1),
            Operation::Conditional {
                branches,
                alternative,
            } => {
                for (condition, body) in branches {
                    let conditions = self.node(*condition, input, environment, depth + 1);
                    if let Some(error) = first_error(&conditions) {
                        return one_error(error);
                    }
                    if conditions
                        .iter()
                        .any(|value| value.as_ref().is_ok_and(Value::is_truthy))
                    {
                        return self.node(*body, input, environment, depth + 1);
                    }
                }
                self.node(*alternative, input, environment, depth + 1)
            }
            Operation::Bind {
                value,
                pattern,
                body,
            } => {
                let mut output = Vec::new();
                for value in self.node(*value, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            let mut nested = environment.clone();
                            let mut charge = |pattern_depth| self.enter(pattern_depth);
                            match bind_pattern(
                                self.bytecode,
                                pattern,
                                &value,
                                &mut nested,
                                depth.saturating_add(1),
                                &mut charge,
                            ) {
                                Ok(()) => {
                                    output.extend(self.node(*body, input, &nested, depth + 1));
                                }
                                Err(error) => output.push(Err(error)),
                            }
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::BindAlternatives {
                value,
                patterns,
                body,
            } => {
                let mut output = Vec::new();
                for value in self.node(*value, input, environment, depth + 1) {
                    match value {
                        Ok(value) => {
                            self.emit_bind_alternatives(
                                patterns,
                                &value,
                                input,
                                *body,
                                environment,
                                depth,
                                &mut |result| {
                                    output.push(result);
                                    true
                                },
                            );
                        }
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            Operation::Reduce {
                generator,
                pattern,
                initial,
                update,
            } => {
                let mut output = Vec::new();
                self.emit_fold(
                    *generator,
                    pattern,
                    *initial,
                    *update,
                    None,
                    false,
                    input,
                    environment,
                    depth,
                    &mut |result| {
                        output.push(result);
                        true
                    },
                );
                output
            }
            Operation::Foreach {
                generator,
                pattern,
                initial,
                update,
                extract,
            } => {
                let mut output = Vec::new();
                self.emit_fold(
                    *generator,
                    pattern,
                    *initial,
                    *update,
                    *extract,
                    true,
                    input,
                    environment,
                    depth,
                    &mut |result| {
                        output.push(result);
                        true
                    },
                );
                output
            }
            Operation::Call { name, arguments } => match self.string(*name).cloned() {
                Ok(name) => self.call(&name, arguments, input, environment, depth + 1),
                Err(error) => one_error(error),
            },
            Operation::TryCatch { expression, catch } => {
                let values = self.node(*expression, input, environment, depth + 1);
                let mut output = Vec::new();
                for value in values {
                    match value {
                        Ok(value) => output.push(Ok(value)),
                        Err(error) => {
                            if let Some(catch) = catch {
                                output.extend(self.node(
                                    *catch,
                                    &catch_value(&error),
                                    environment,
                                    depth + 1,
                                ));
                            }
                        }
                    }
                }
                output
            }
            Operation::Label { symbol, body } => {
                let mut output = Vec::new();
                for result in self.node(*body, input, environment, depth + 1) {
                    if matches!(result, Err(VmError::Break { label }) if label == *symbol) {
                        break;
                    }
                    output.push(result);
                }
                output
            }
            Operation::Break(symbol) => one_error(VmError::Break { label: *symbol }),
            Operation::Assignment {
                operator,
                path,
                value,
            } => self.assignment(*operator, *path, *value, input, environment, depth + 1),
            operation => one_error(VmError::Unsupported {
                operation: format!("{operation:?}").into(),
            }),
        }
    }

    fn string(&self, index: u32) -> Result<&Arc<str>, VmError> {
        self.bytecode
            .string(index)
            .ok_or_else(|| invalid("string missing after validation"))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the streaming branch callback carries the evaluator context explicitly"
    )]
    fn emit_conditional(
        &self,
        branches: &[(u32, u32)],
        index: usize,
        alternative: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some((condition, body)) = branches.get(index).copied() else {
            return self.emit_node(alternative, input, environment, depth, emit);
        };
        self.emit_node(
            condition,
            input,
            environment,
            depth,
            &mut |result| match result {
                Ok(value) if value.is_truthy() => {
                    self.emit_node(body, input, environment, depth, emit)
                }
                Ok(_) => self.emit_conditional(
                    branches,
                    index.saturating_add(1),
                    alternative,
                    input,
                    environment,
                    depth,
                    emit,
                ),
                Err(error) => emit(Err(error)),
            },
        )
    }

    fn bound(
        &self,
        node: Option<u32>,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Vec<Result<Option<f64>, VmError>> {
        match node {
            None => vec![Ok(None)],
            Some(node) => self
                .node(node, input, environment, depth)
                .into_iter()
                .map(|value| value.and_then(|value| slice_bound_value(&value)))
                .collect(),
        }
    }

    fn binary(
        &self,
        operator: BinaryOperator,
        left: u32,
        right: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let left_values = self.node(left, input, environment, depth);
        if operator == BinaryOperator::Alternative {
            let accepted = left_values
                .iter()
                .filter_map(|value| value.as_ref().ok())
                .filter(|value| value.is_truthy())
                .cloned()
                .map(Ok)
                .collect::<Vec<_>>();
            if !accepted.is_empty() {
                return accepted;
            }
            return self.node(right, input, environment, depth);
        }
        if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
            let mut output = Vec::new();
            for left in left_values {
                let Ok(left) = left else {
                    output.push(left);
                    continue;
                };
                let left_truthy = left.is_truthy();
                if (operator == BinaryOperator::And && !left_truthy)
                    || (operator == BinaryOperator::Or && left_truthy)
                {
                    output.push(Ok(Value::Bool(operator == BinaryOperator::Or)));
                } else {
                    output.extend(
                        self.node(right, input, environment, depth)
                            .into_iter()
                            .map(|value| value.map(|value| Value::Bool(value.is_truthy()))),
                    );
                }
            }
            return output;
        }
        let right_values = self.node(right, input, environment, depth);
        let mut output = Vec::new();
        for left in &left_values {
            for right in &right_values {
                output.push(match (left, right) {
                    (Ok(left), Ok(right)) => binary_value(operator, left, right),
                    (Err(error), _) | (_, Err(error)) => Err(error.clone()),
                });
            }
        }
        output
    }

    fn walk(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(callback) = arguments.first() else {
            return one_error(invalid("walk argument missing"));
        };
        let root_depth = depth.saturating_add(1);
        if root_depth > self.limits.path_stack {
            return one_error(resource("path-stack"));
        }
        let mut observations = self.observations.get();
        observations.path_stack_high_water = observations.path_stack_high_water.max(root_depth);
        self.observations.set(observations);
        let mut frames = vec![WalkFrame::new(input.clone(), root_depth)];
        let mut completed = None;

        loop {
            if let Some(outcomes) = completed.take() {
                if let Some(parent) = frames.last_mut() {
                    parent.push_children(outcomes);
                    continue;
                }
                return outcomes;
            }

            let Some(frame) = frames.last_mut() else {
                return Vec::new();
            };
            let Some(value) = frame.next_value() else {
                let frame = frames
                    .pop()
                    .expect("walk frame exists after last frame lookup");
                let callback_depth = frame.depth();
                let mut outcomes = Vec::new();
                for rebuilt in frame.finish(self.limits.fork_stack) {
                    match rebuilt {
                        Ok(value) => outcomes.extend(self.node(
                            *callback,
                            &value,
                            environment,
                            callback_depth,
                        )),
                        Err(error) => outcomes.push(Err(error)),
                    }
                    if outcomes.len() >= self.limits.fork_stack {
                        outcomes = one_error(resource("fork-stack"));
                        break;
                    }
                }
                completed = Some(outcomes);
                continue;
            };

            let child_depth = frame.depth().saturating_add(1);
            if let Err(error) = self.enter(child_depth) {
                completed = Some(one_error(error));
                continue;
            }
            if child_depth > self.limits.path_stack {
                completed = Some(one_error(resource("path-stack")));
                continue;
            }
            let mut observations = self.observations.get();
            observations.path_stack_high_water =
                observations.path_stack_high_water.max(child_depth);
            self.observations.set(observations);
            frames.push(WalkFrame::new(value, child_depth));
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the versioned built-in registry is executed in one exhaustive dispatch"
    )]
    fn call(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        match name {
            "first" | "last" | "nth" | "skip" | "isempty" => {
                let mut output = Vec::new();
                self.emit_pull_consumer(
                    name,
                    arguments,
                    input,
                    environment,
                    depth,
                    &mut |result| {
                        output.push(result);
                        true
                    },
                );
                output
            }
            "empty" => Vec::new(),
            "modulemeta" => vec![module_metadata(
                self.bytecode,
                input,
                self.cancellation,
                self.stop,
            )],
            "type" => vec![Ok(Value::string(type_name(input)))],
            "length" => vec![length(input)],
            "abs" => vec![absolute_value(input)],
            "utf8bytelength" => match input {
                Value::String(value) => vec![number_usize(value.len())],
                value => one_error(type_error("utf8bytelength", value)),
            },
            "keys" | "keys_unsorted" => vec![keys(input, name == "keys")],
            "builtins" => vec![Ok(builtin_signatures())],
            "not" => vec![Ok(Value::Bool(!input.is_truthy()))],
            "has" => self.argument_values(arguments, input, environment, depth, 0, |key| {
                has(input, key)
            }),
            "in" => self.argument_values(arguments, input, environment, depth, 0, |container| {
                has(container, input)
            }),
            "IN" => self.sql_in(arguments, input, environment, depth),
            "INDEX" => self.sql_index(arguments, input, environment, depth),
            "JOIN" => self.sql_join(arguments, input, environment, depth),
            "contains" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::contains(input, needle, &|| self.enter(depth))
            }),
            "inside" => {
                self.argument_values(arguments, input, environment, depth, 0, |container| {
                    collection::inside(input, container, &|| self.enter(depth))
                })
            }
            "combinations" => self.combinations(arguments, input, environment, depth),
            "transpose" => self.transpose(input, depth),
            "bsearch" => self.bsearch(arguments, input, environment, depth),
            "indices" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::indices(input, needle, &|| self.enter(depth))
            }),
            "index" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::index(input, needle, false, &|| self.enter(depth))
            }),
            "rindex" => self.argument_values(arguments, input, environment, depth, 0, |needle| {
                collection::index(input, needle, true, &|| self.enter(depth))
            }),
            "arrays" => selector(input, matches!(input, Value::Array(_))),
            "booleans" => selector(input, matches!(input, Value::Bool(_))),
            "finites" => selector(
                input,
                matches!(input, Value::Number(number) if !number.as_f64().is_infinite()),
            ),
            "iterables" => selector(input, matches!(input, Value::Array(_) | Value::Object(_))),
            "nulls" => selector(input, matches!(input, Value::Null)),
            "normals" => selector(
                input,
                matches!(input, Value::Number(number) if number.as_f64().is_normal()),
            ),
            "numbers" => selector(input, matches!(input, Value::Number(_))),
            "objects" => selector(input, matches!(input, Value::Object(_))),
            "scalars" => selector(input, !matches!(input, Value::Array(_) | Value::Object(_))),
            "strings" => selector(input, matches!(input, Value::String(_))),
            "values" => selector(input, !matches!(input, Value::Null)),
            "select" => {
                let Some(argument) = arguments.first() else {
                    return one_error(invalid("select argument missing"));
                };
                let mut output = Vec::new();
                for selected in self.node(*argument, input, environment, depth) {
                    match selected {
                        Ok(value) if value.is_truthy() => output.push(Ok(input.clone())),
                        Ok(_) => {}
                        Err(error) => output.push(Err(error)),
                    }
                }
                output
            }
            "map" => self.map(arguments, input, environment, depth, false),
            "map_values" => self.map(arguments, input, environment, depth, true),
            "to_entries" => vec![to_entries(input)],
            "from_entries" => match input {
                Value::Array(entries) => vec![from_entries(entries, "from_entries")],
                value => one_error(type_error("from_entries", value)),
            },
            "with_entries" => self.with_entries(arguments, input, environment, depth),
            "tonumber" => match input {
                Value::Number(_) => vec![Ok(input.clone())],
                Value::String(value) => parse_jq_number(value)
                    .map(Value::Number)
                    .map(Ok)
                    .map_or_else(
                        |error| one_error(runtime(error.to_string())),
                        |value| vec![value],
                    ),
                value => one_error(type_error("tonumber", value)),
            },
            "tostring" => vec![format::text(input, self.limits.output_bytes).map(Value::String)],
            "@urid" => vec![string_compat::urid(input, self.limits.output_bytes)],
            name if name.starts_with('@') => {
                vec![format::apply(name, input, self.limits.output_bytes)]
            }
            "tojson" => {
                vec![format::bounded_json(input, self.limits.output_bytes).map(Value::string)]
            }
            "fromjson" => {
                let mut checkpoint = || {
                    if self.cancelled() {
                        Err(VmError::Interrupted)
                    } else {
                        Ok(())
                    }
                };
                vec![fromjson(input, self.limits, &mut checkpoint)]
            }
            "range" => self.range(arguments, input, environment, depth),
            "add" => self.add(arguments, input, environment, depth),
            "min" => extrema(input, false),
            "max" => extrema(input, true),
            "sort" => sort_values(input),
            "sort_by" => self.sort_by(arguments, input, environment, depth, false),
            "group_by" => self.group_by(arguments, input, environment, depth),
            "min_by" => self.keyed_extrema(arguments, input, environment, depth, false),
            "max_by" => self.keyed_extrema(arguments, input, environment, depth, true),
            "unique" => unique_values(input),
            "unique_by" => self.sort_by(arguments, input, environment, depth, true),
            "reverse" => reverse(input),
            "flatten" => self.flatten(arguments, input, environment, depth),
            "walk" => self.walk(arguments, input, environment, depth),
            "limit" => self.limit(arguments, input, environment, depth),
            "any" => self.any_all(arguments, input, environment, depth, true),
            "all" => self.any_all(arguments, input, environment, depth, false),
            "trim" => vec![string_compat::trim(input)],
            "ltrim" => vec![string_compat::ltrim(input)],
            "rtrim" => vec![string_compat::rtrim(input)],
            "ltrimstr" => self.argument_values(arguments, input, environment, depth, 0, |prefix| {
                string_compat::ltrimstr(input, prefix)
            }),
            "rtrimstr" => self.argument_values(arguments, input, environment, depth, 0, |suffix| {
                string_compat::rtrimstr(input, suffix)
            }),
            "startswith" => {
                self.argument_values(arguments, input, environment, depth, 0, |prefix| {
                    string_compat::startswith(input, prefix)
                })
            }
            "endswith" => self.argument_values(arguments, input, environment, depth, 0, |suffix| {
                string_compat::endswith(input, suffix)
            }),
            "trimstr" => self.argument_values(arguments, input, environment, depth, 0, |affix| {
                string_compat::trimstr(input, affix)
            }),
            "join" => self.argument_values(arguments, input, environment, depth, 0, |separator| {
                string_compat::join(input, separator, self.limits.output_bytes)
            }),
            "toboolean" => vec![string_compat::toboolean(input)],
            "ascii_upcase" => vec![string_compat::ascii_upcase(input)],
            "ascii_downcase" => vec![string_compat::ascii_downcase(input)],
            "explode" => vec![explode(input)],
            "implode" => vec![implode(input)],
            "nan" => vec![Ok(runtime_number(f64::NAN))],
            "infinite" => vec![Ok(runtime_number(f64::INFINITY))],
            // tq's hybrid Number retains decimal literal provenance and
            // therefore provides the decimal-number capability advertised by
            // jq's have_decnum/0 and have_literal_numbers/0 predicates.
            "have_decnum" | "have_literal_numbers" => vec![Ok(Value::Bool(true))],
            "isnan" | "isinfinite" | "isfinite" | "isnormal" => {
                vec![numeric_predicate(name, input)]
            }
            "acos" | "acosh" | "asin" | "asinh" | "atan" | "atan2" | "atanh" | "cbrt" | "ceil"
            | "copysign" | "cos" | "cosh" | "drem" | "erf" | "erfc" | "exp" | "exp10" | "exp2"
            | "expm1" | "fabs" | "fdim" | "fma" | "fmax" | "fmin" | "fmod" | "floor" | "frexp"
            | "gamma" | "hypot" | "j0" | "j1" | "jn" | "ldexp" | "lgamma" | "log" | "log10"
            | "log1p" | "log2" | "logb" | "modf" | "nearbyint" | "nextafter" | "nexttoward"
            | "pow" | "remainder" | "rint" | "round" | "scalb" | "scalbln" | "significand"
            | "sin" | "sinh" | "sqrt" | "tan" | "tanh" | "tgamma" | "trunc" | "y0" | "y1"
            | "yn" => self.math_call(name, arguments, input, environment, depth),
            "paths" if arguments.is_empty() => match self.descendant_paths(input, depth + 1) {
                Ok(paths) => paths
                    .into_iter()
                    .map(|path| Ok(path_value(&path)))
                    .collect(),
                Err(error) => one_error(error),
            },
            "paths" => self.paths_filter(arguments, input, environment, depth),
            "path" => self.path_outcomes(arguments, input, environment, depth),
            "pick" => self.pick(arguments, input, environment, depth),
            "del" => self.delete(arguments, input, environment, depth),
            "delpaths" => self.delete_paths(arguments, input, environment, depth),
            "getpath" => {
                let path_limit = self.limits.path_stack;
                self.argument_values(arguments, input, environment, depth, 0, |path| {
                    let path = jq_path(path)?;
                    if path.len() > path_limit {
                        return Err(resource("path-stack"));
                    }
                    getpath(input, &path)
                })
            }
            "setpath" => self.setpath(arguments, input, environment, depth),
            "fromstream" => self.fromstream(arguments, input, environment, depth),
            "tostream" => match tostream(input, self.limits.path_stack) {
                Ok(records) => records.into_iter().map(Ok).collect(),
                Err(error) => one_error(error),
            },
            "truncate_stream" => self.truncate_stream(arguments, input, environment, depth),
            "stderr" => match format::text(input, self.limits.output_bytes) {
                Ok(text) => append_effect(&self.effects, text.as_bytes(), self.limits.output_bytes)
                    .map_or_else(one_error, |()| vec![Ok(input.clone())]),
                Err(error) => one_error(error),
            },
            "debug" => {
                let values = arguments.first().map_or_else(
                    || vec![Ok(input.clone())],
                    |argument| self.node(*argument, input, environment, depth + 1),
                );
                let mut output = Vec::new();
                for value in values {
                    let value = match value {
                        Ok(value) => value,
                        Err(error) => return one_error(error),
                    };
                    if let Err(error) =
                        debug_effect(&value, self.limits.output_bytes).and_then(|bytes| {
                            append_effect(&self.effects, &bytes, self.limits.output_bytes)
                        })
                    {
                        return one_error(error);
                    }
                }
                output.push(Ok(input.clone()));
                output
            }
            "halt" => vec![Err(VmError::Halt {
                status: 0,
                stderr: Arc::from([]),
            })],
            "halt_error" => {
                let stderr = match halt_error_text(input, self.limits.output_bytes) {
                    Ok(stderr) => stderr,
                    Err(error) => return one_error(error),
                };
                let status_values = arguments.first().map_or_else(
                    || {
                        vec![Ok(Value::Number(
                            Number::parse("5").expect("halt_error default status"),
                        ))]
                    },
                    |argument| self.node(*argument, input, environment, depth + 1),
                );
                status_values
                    .into_iter()
                    .map(|value| {
                        value.and_then(|value| {
                            Err(VmError::Halt {
                                status: halt_status(&value)?,
                                stderr: Arc::from(stderr.as_bytes()),
                            })
                        })
                    })
                    .collect()
            }
            "error" => {
                let Some(argument) = arguments.first() else {
                    return vec![Err(raised(input.clone()))];
                };
                let Some(result) = self
                    .node(*argument, input, environment, depth)
                    .into_iter()
                    .next()
                else {
                    return Vec::new();
                };
                vec![result.map_or_else(Err, |value| Err(raised(value)))]
            }
            "test" | "match" | "capture" | "scan" | "split" | "splits" | "sub" | "gsub" => {
                self.regex_call(name, arguments, input, environment, depth)
            }
            "fromdate" | "fromdateiso8601" => vec![stdlib::fromdate_iso8601(input)],
            "todate" | "todateiso8601" => {
                vec![stdlib::todate_iso8601(input, self.limits.output_bytes)]
            }
            "gmtime" => vec![stdlib::gmtime(input)],
            "localtime" => vec![stdlib::localtime(input, ambient_platform(environment))],
            "mktime" => vec![stdlib::mktime(input)],
            "strptime" => {
                self.argument_values(
                    arguments,
                    input,
                    environment,
                    depth,
                    0,
                    |format| match format {
                        Value::String(format) => stdlib::strptime(input, format),
                        value => Err(type_error("strptime", value)),
                    },
                )
            }
            "strftime" => {
                self.argument_values(
                    arguments,
                    input,
                    environment,
                    depth,
                    0,
                    |format| match format {
                        Value::String(format) => {
                            stdlib::strftime(input, format, self.limits.output_bytes)
                        }
                        value => Err(type_error("strftime", value)),
                    },
                )
            }
            "strflocaltime" => self.argument_values(
                arguments,
                input,
                environment,
                depth,
                0,
                |format| match format {
                    Value::String(format) => stdlib::strflocaltime(
                        input,
                        format,
                        ambient_platform(environment),
                        self.limits.output_bytes,
                    ),
                    value => Err(type_error("strflocaltime", value)),
                },
            ),
            "now" => vec![stdlib::now(ambient_platform(environment))],
            "env" => vec![ambient_environment(environment)],
            "input_filename" => {
                if ambient_platform(environment) {
                    vec![
                        self.input_cursor
                            .and_then(InputCursor::current_context)
                            .map_or_else(
                                || ambient_value(environment, INPUT_FILENAME, "input_filename"),
                                |context| Ok(Value::string(context.identity)),
                            ),
                    ]
                } else {
                    one_error(capability_denied(
                        "input_filename requires platform access permitted by capability policy"
                            .to_owned(),
                    ))
                }
            }
            "input_line_number" => {
                if ambient_platform(environment) {
                    vec![
                        self.input_cursor
                            .and_then(InputCursor::current_context)
                            .map_or_else(
                                || {
                                    ambient_value(
                                        environment,
                                        INPUT_LINE_NUMBER,
                                        "input_line_number",
                                    )
                                },
                                |context| {
                                    Ok(Value::Number(
                                        Number::parse(&context.line_number.to_string()).expect(
                                            "a source line number is an admitted exact integer",
                                        ),
                                    ))
                                },
                            ),
                    ]
                } else {
                    one_error(capability_denied(
                        "input_line_number requires platform access permitted by capability policy"
                            .to_owned(),
                    ))
                }
            }
            "input" | "inputs" => {
                let Some(cursor) = self.input_cursor else {
                    return if name == "input" {
                        one_error(runtime("break".to_owned()))
                    } else {
                        Vec::new()
                    };
                };
                if name == "input" {
                    return match cursor.next_value() {
                        Ok(Some(value)) => vec![Ok(value)],
                        Ok(None) => one_error(runtime("break".to_owned())),
                        Err(error) => one_error(error),
                    };
                }
                let mut values = Vec::new();
                loop {
                    match cursor.next_value() {
                        Ok(Some(value)) => values.push(Ok(value)),
                        Ok(None) => break,
                        Err(error) => return one_error(error),
                    }
                }
                values
            }
            _ => one_error(VmError::Unsupported {
                operation: format!("builtin {name}").into(),
            }),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "regex dispatch keeps bounded engine and jq overload handling together"
    )]
    fn regex_call(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Value::String(input) = input else {
            return one_error(type_error(name, input));
        };
        let Some(pattern_node) = arguments.first() else {
            return one_error(invalid("regex pattern argument missing"));
        };
        let pattern_values = self.node(
            *pattern_node,
            &Value::String(Arc::clone(input)),
            environment,
            depth,
        );
        let flags_index = usize::from(matches!(name, "sub" | "gsub")) + 1;
        let mut output = Vec::new();
        for pattern in &pattern_values {
            let array_allowed = matches!(name, "test" | "match" | "capture")
                && arguments.get(flags_index).is_none();
            let (pattern, array_flags) = match pattern
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|value| regex_pattern_argument(name, value, array_allowed))
            {
                Ok(parsed) => parsed,
                Err(error) => {
                    output.push(Err(error));
                    continue;
                }
            };
            let flag_values = if let Some(flags) = array_flags {
                vec![Ok(Value::String(flags))]
            } else if let Some(flags) = arguments.get(flags_index) {
                self.node(
                    *flags,
                    &Value::String(Arc::clone(input)),
                    environment,
                    depth,
                )
            } else {
                vec![Ok(Value::string(""))]
            };
            for flags in &flag_values {
                let flags = match flags {
                    Ok(Value::String(flags)) => Arc::clone(flags),
                    Ok(Value::Null) => Arc::from(""),
                    Ok(value) => {
                        output.push(Err(type_error(name, value)));
                        continue;
                    }
                    Err(error) => {
                        output.push(Err(error.clone()));
                        continue;
                    }
                };
                let checkpoint = || self.enter(depth);
                let result = match name {
                    "test" => stdlib::regex_test(input, &pattern, &flags, self.limits, &checkpoint)
                        .map(|value| vec![value]),
                    "match" => {
                        stdlib::regex_matches(input, &pattern, &flags, self.limits, &checkpoint)
                    }
                    "capture" => {
                        stdlib::regex_capture(input, &pattern, &flags, self.limits, &checkpoint)
                    }
                    "scan" => stdlib::regex_scan(input, &pattern, &flags, self.limits, &checkpoint),
                    "split" => stdlib::regex_split(
                        input,
                        &pattern,
                        &flags,
                        false,
                        arguments.get(flags_index).is_none(),
                        self.limits,
                        &checkpoint,
                    ),
                    "splits" => stdlib::regex_split(
                        input,
                        &pattern,
                        &flags,
                        true,
                        false,
                        self.limits,
                        &checkpoint,
                    ),
                    "sub" | "gsub" => {
                        let Some(replacement_node) = arguments.get(1) else {
                            return one_error(invalid("regex replacement argument missing"));
                        };
                        stdlib::regex_substitute(
                            input,
                            &pattern,
                            &flags,
                            name == "gsub",
                            self.limits,
                            &checkpoint,
                            |context| {
                                self.regex_replacement_values(
                                    name,
                                    *replacement_node,
                                    context,
                                    environment,
                                    depth,
                                )
                            },
                        )
                    }
                    _ => unreachable!("regex dispatch is exhaustive"),
                };
                match result {
                    Ok(values) => output.extend(values.into_iter().map(Ok)),
                    Err(error) => output.push(Err(error)),
                }
            }
        }
        output
    }

    fn regex_replacement_values(
        &self,
        name: &str,
        node: u32,
        context: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<Arc<str>>, VmError> {
        let mut values = Vec::new();
        for value in self.node(node, context, environment, depth) {
            if values.len() >= self.limits.regex_replacement_limit {
                return Err(resource("regex-replacement-count"));
            }
            values.push(match value? {
                Value::String(value) => value,
                Value::Null => Arc::from(""),
                value => return Err(type_error(name, &value)),
            });
        }
        Ok(values)
    }

    fn argument_values(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        index: usize,
        apply: impl Fn(&Value) -> Result<Value, VmError>,
    ) -> Outcomes {
        let Some(argument) = arguments.get(index) else {
            return one_error(invalid("builtin argument missing"));
        };
        self.node(*argument, input, environment, depth)
            .into_iter()
            .map(|value| value.and_then(|value| apply(&value)))
            .collect()
    }

    fn sql_in(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_sql_in(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_sql_in(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let (Some(source), maybe_needle) = (arguments.first(), arguments.get(1)) else {
            return emit(Err(invalid("IN argument missing")));
        };
        if let Some(needle) = maybe_needle {
            let mut matched = false;
            let mut search_error = None;
            self.emit_node(*needle, input, environment, depth, &mut |needle| {
                if matched || search_error.is_some() {
                    return false;
                }
                let needle = match needle {
                    Ok(needle) => needle,
                    Err(error) => {
                        search_error = Some(error);
                        return false;
                    }
                };
                self.emit_node(
                    *source,
                    input,
                    environment,
                    depth,
                    &mut |source| match source {
                        Ok(source) => {
                            let mut charge = || self.enter(depth);
                            match sql::equal_bounded(&source, &needle, self.limits, &mut charge) {
                                Ok(true) => {
                                    matched = true;
                                    false
                                }
                                Ok(false) => true,
                                Err(error) => {
                                    search_error = Some(error);
                                    false
                                }
                            }
                        }
                        Err(error) => {
                            search_error = Some(error);
                            false
                        }
                    },
                );
                !matched && search_error.is_none()
            });
            if let Some(error) = search_error {
                return emit(Err(error));
            }
            return emit(Ok(Value::Bool(matched)));
        }

        let mut matched = false;
        let mut source_error = None;
        self.emit_node(
            *source,
            input,
            environment,
            depth,
            &mut |source| match source {
                Ok(source) => {
                    let mut charge = || self.enter(depth);
                    match sql::equal_bounded(input, &source, self.limits, &mut charge) {
                        Ok(true) => {
                            matched = true;
                            false
                        }
                        Ok(false) => true,
                        Err(error) => {
                            source_error = Some(error);
                            false
                        }
                    }
                }
                Err(error) => {
                    source_error = Some(error);
                    false
                }
            },
        );
        if let Some(error) = source_error {
            return emit(Err(error));
        }
        emit(Ok(Value::Bool(matched)))
    }

    fn sql_index(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(key_node) = arguments.last().copied() else {
            return one_error(invalid("INDEX argument missing"));
        };
        let rows = if arguments.len() == 1 {
            match self.sql_iterable_rows(input, depth) {
                Ok(rows) => rows.into_iter().map(Ok).collect(),
                Err(error) => return one_error(error),
            }
        } else {
            self.node(arguments[0], input, environment, depth)
        };
        let mut index = Object::new();
        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(error) => return one_error(error),
            };
            if let Err(error) = self.enter(depth) {
                return one_error(error);
            }
            for key in self.node(key_node, &row, environment, depth) {
                let key = match key {
                    Ok(key) => key,
                    Err(error) => return one_error(error),
                };
                let key = match format::text(&key, self.limits.output_bytes) {
                    Ok(key) => key,
                    Err(error) => return one_error(error),
                };
                index.insert(key, row.clone());
            }
        }
        vec![Ok(Value::object(index))]
    }

    fn sql_join(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_sql_join(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_sql_join(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(index_node) = arguments.first().copied() else {
            return emit(Err(invalid("JOIN index argument missing")));
        };
        let Some(key_node) = arguments
            .get(if arguments.len() == 2 { 1 } else { 2 })
            .copied()
        else {
            return emit(Err(invalid("JOIN key argument missing")));
        };
        self.emit_node(index_node, input, environment, depth, &mut |index| {
            let index = match index {
                Ok(index) => index,
                Err(error) => return emit(Err(error)),
            };
            if arguments.len() == 2 {
                let rows = match self.sql_iterable_rows(input, depth) {
                    Ok(rows) => rows,
                    Err(error) => return emit(Err(error)),
                };
                let mut grouped = Vec::new();
                for row in rows {
                    let mut key_error = None;
                    let mut keys = Vec::new();
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    self.emit_node(key_node, &row, environment, depth, &mut |key| match key {
                        Ok(key) => {
                            keys.push(key);
                            true
                        }
                        Err(error) => {
                            key_error = Some(error);
                            false
                        }
                    });
                    if let Some(error) = key_error {
                        return emit(Err(error));
                    }
                    for key in keys {
                        let match_value = match access_index(&index, &key) {
                            Ok(value) => value,
                            Err(error) => return emit(Err(error)),
                        };
                        grouped.push(Value::array(vec![row.clone(), match_value]));
                    }
                }
                return emit(Ok(Value::array(grouped)));
            }

            let Some(stream_node) = arguments.get(1).copied() else {
                return emit(Err(invalid("JOIN stream argument missing")));
            };
            self.emit_node(stream_node, input, environment, depth, &mut |row| {
                let row = match row {
                    Ok(row) => row,
                    Err(error) => return emit(Err(error)),
                };
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                self.emit_node(key_node, &row, environment, depth, &mut |key| {
                    let key = match key {
                        Ok(key) => key,
                        Err(error) => return emit(Err(error)),
                    };
                    let match_value = match access_index(&index, &key) {
                        Ok(value) => value,
                        Err(error) => return emit(Err(error)),
                    };
                    let pair = Value::array(vec![row.clone(), match_value]);
                    if let Some(join_node) = arguments.get(3).copied() {
                        self.emit_node(join_node, &pair, environment, depth, emit)
                    } else {
                        emit(Ok(pair))
                    }
                })
            })
        })
    }

    fn sql_iterable_rows(&self, input: &Value, depth: usize) -> Result<Vec<Value>, VmError> {
        let values = match input {
            Value::Array(values) => values.iter().cloned().collect::<Vec<_>>(),
            Value::Object(values) => values.values().cloned().collect::<Vec<_>>(),
            value => return Err(runtime(format!("Cannot iterate over {value}"))),
        };
        for _ in &values {
            self.enter(depth)?;
        }
        Ok(values)
    }

    fn combinations(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_combinations(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_combinations(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Some(argument) = arguments.first() {
            let Value::Array(base_values) = input else {
                return emit(Err(type_error("combinations", input)));
            };
            let mut dimensions = Vec::new();
            let mut argument_error = None;
            self.emit_node(*argument, input, environment, depth, &mut |value| {
                if let Err(error) = self.enter(depth) {
                    argument_error = Some(error);
                    return false;
                }
                let value = match value {
                    Ok(value) => value,
                    Err(error) => {
                        argument_error = Some(error);
                        return false;
                    }
                };
                let count = match value {
                    Value::Number(number) if number.as_f64().is_finite() => {
                        let count = number.as_f64().ceil();
                        if count <= 0.0 {
                            0
                        } else {
                            match count.to_string().parse::<usize>() {
                                Ok(count) if count <= self.limits.fork_stack => count,
                                _ => {
                                    argument_error = Some(resource("fork-stack"));
                                    return false;
                                }
                            }
                        }
                    }
                    Value::Number(_) => {
                        argument_error = Some(resource("fork-stack"));
                        return false;
                    }
                    value => {
                        argument_error = Some(type_error("combinations", &value));
                        return false;
                    }
                };
                if dimensions.len().saturating_add(count) > self.limits.fork_stack {
                    argument_error = Some(resource("fork-stack"));
                    return false;
                }
                for _ in 0..count {
                    if let Err(error) = self.enter(depth) {
                        argument_error = Some(error);
                        return false;
                    }
                    dimensions.push(base_values.as_ref());
                }
                true
            });
            if let Some(error) = argument_error {
                return emit(Err(error));
            }
            let mut current = Vec::new();
            if current.try_reserve(dimensions.len()).is_err() {
                return emit(Err(resource("fork-stack")));
            }
            self.emit_combination_product(&dimensions, &mut current, depth, emit)
        } else {
            let Value::Array(values) = input else {
                return emit(Err(type_error("combinations", input)));
            };
            if values.len() > self.limits.fork_stack {
                return emit(Err(resource("fork-stack")));
            }
            let mut dimensions = Vec::new();
            if dimensions.try_reserve(values.len()).is_err() {
                return emit(Err(resource("fork-stack")));
            }
            for value in values.iter() {
                if let Err(error) = self.enter(depth) {
                    return emit(Err(error));
                }
                let Value::Array(dimension) = value else {
                    return emit(Err(type_error("combinations", value)));
                };
                dimensions.push(dimension.as_ref());
            }
            let mut current = Vec::new();
            if current.try_reserve(dimensions.len()).is_err() {
                return emit(Err(resource("fork-stack")));
            }
            self.emit_combination_product(&dimensions, &mut current, depth, emit)
        }
    }

    fn emit_combination_product(
        &self,
        dimensions: &[&[Value]],
        current: &mut Vec<Value>,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if dimensions.is_empty() {
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            return emit(Ok(Value::array(Vec::new())));
        }

        // Iterate an odometer rather than recursing once per dimension. The
        // dimension count is user-controlled, so native call-stack depth must
        // not become an unmetered VM resource.
        if dimensions.iter().any(|dimension| dimension.is_empty()) {
            return true;
        }

        let mut indices: Vec<usize> = Vec::new();
        if indices.try_reserve(dimensions.len()).is_err() {
            return emit(Err(resource("fork-stack")));
        }
        indices.resize(dimensions.len(), 0);
        current.clear();
        for dimension in dimensions {
            current.push(dimension[0].clone());
        }

        loop {
            if let Err(error) = self.enter(depth) {
                return emit(Err(error));
            }
            if !emit(Ok(Value::array(current.clone()))) {
                return false;
            }

            let mut position = dimensions.len();
            loop {
                if position == 0 {
                    return true;
                }
                position -= 1;
                indices[position] = indices[position].saturating_add(1);
                if indices[position] < dimensions[position].len() {
                    current[position] = dimensions[position][indices[position]].clone();
                    break;
                }
                indices[position] = 0;
                current[position] = dimensions[position][0].clone();
            }
        }
    }

    fn transpose(&self, input: &Value, depth: usize) -> Outcomes {
        let mut charge = || self.enter(depth);
        scalar::transpose_value(input, self.limits, &mut charge)
            .map_or_else(one_error, |value| vec![Ok(value)])
    }

    fn bsearch(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        self.argument_values(arguments, input, environment, depth, 0, |needle| {
            let mut charge = || self.enter(depth);
            scalar::bsearch_value(input, needle, self.limits, &mut charge)
        })
    }

    fn math_call(
        &self,
        name: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        if arguments.is_empty() {
            let value = match math_argument(name, input) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
            return self
                .math_result(name, &[value], depth)
                .map_or_else(one_error, |value| vec![Ok(value)]);
        }

        let mut argument_values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            argument_values.push(
                self.node(*argument, input, environment, depth)
                    .into_iter()
                    .map(|value| value.and_then(|value| math_argument(name, &value)))
                    .collect::<Vec<_>>(),
            );
        }

        // jq evaluates explicit arguments left-to-right, but emits the
        // Cartesian product with the rightmost argument as the outer loop.
        // Retain errors from a later argument when an earlier argument is
        // empty (`pow(empty; error("later"))`), while an earlier error is
        // suppressed if a later argument is empty.
        let mut combinations = argument_values
            .pop()
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.map(|value| vec![value]))
            .collect::<Vec<_>>();
        for values in argument_values.into_iter().rev() {
            if values.is_empty() {
                combinations.retain(Result::is_err);
                continue;
            }
            let suffixes = combinations;
            let mut next = Vec::new();
            for suffix in suffixes {
                for value in &values {
                    let combination = match (value, &suffix) {
                        (Ok(value), Ok(suffix)) => {
                            let mut combination = Vec::with_capacity(suffix.len() + 1);
                            combination.push(*value);
                            combination.extend(suffix.iter().copied());
                            Ok(combination)
                        }
                        (_, Err(error)) | (Err(error), Ok(_)) => Err(error.clone()),
                    };
                    next.push(combination);
                    if next.len() > self.limits.fork_stack {
                        return one_error(resource("fork-stack"));
                    }
                }
            }
            combinations = next;
        }

        combinations
            .into_iter()
            .map(|values| values.and_then(|values| self.math_result(name, &values, depth)))
            .collect()
    }

    fn math_result(&self, name: &str, arguments: &[f64], depth: usize) -> Result<Value, VmError> {
        if matches!(name, "jn" | "yn") {
            self.charge_bessel_work(arguments, depth)?;
        }
        math::evaluate(name, arguments)
            .map(math_result_value)
            .map_err(|error| math_error(name, &error))
    }

    fn charge_bessel_work(&self, arguments: &[f64], depth: usize) -> Result<(), VmError> {
        let Some(order) = arguments.first().copied() else {
            return Ok(());
        };
        if !order.is_finite() || order.abs() > 1_024.0 {
            return Ok(());
        }
        let mut remaining = order.abs().trunc();
        while remaining >= 1.0 {
            self.enter(depth)?;
            remaining -= 1.0;
        }
        Ok(())
    }

    fn map(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        values_only: bool,
    ) -> Outcomes {
        let Some(argument) = arguments.first() else {
            return one_error(invalid("map argument missing"));
        };
        match input {
            Value::Array(values) => {
                let mut mapped = Vec::new();
                for value in values.iter() {
                    if values_only {
                        match self
                            .node(*argument, value, environment, depth)
                            .into_iter()
                            .next()
                        {
                            Some(Ok(value)) => mapped.push(value),
                            Some(Err(error)) => return one_error(error),
                            None => {}
                        }
                    } else {
                        match collect_values(self.node(*argument, value, environment, depth)) {
                            Ok(values) => mapped.extend(values),
                            Err(error) => return one_error(error),
                        }
                    }
                }
                vec![Ok(Value::array(mapped))]
            }
            Value::Object(object) if values_only => {
                let mut mapped = Object::new();
                for (key, value) in object.iter() {
                    let values = self.node(*argument, value, environment, depth);
                    match values.into_iter().next() {
                        Some(Ok(value)) => {
                            mapped.insert(Arc::clone(key), value);
                        }
                        Some(Err(error)) => return one_error(error),
                        None => {}
                    }
                }
                vec![Ok(Value::object(mapped))]
            }
            Value::Object(object) => {
                let mut mapped = Vec::new();
                for value in object.values() {
                    match collect_values(self.node(*argument, value, environment, depth)) {
                        Ok(values) => mapped.extend(values),
                        Err(error) => return one_error(error),
                    }
                }
                vec![Ok(Value::array(mapped))]
            }
            value => one_error(type_error(
                if values_only { "map_values" } else { "map" },
                value,
            )),
        }
    }

    fn add(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(generator) = arguments.first() else {
            return fold_values(input, None, binary_add);
        };
        let mut result = None;
        for value in self.node(*generator, input, environment, depth) {
            let value = match value {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
            result = Some(match result {
                None => value,
                Some(result) => match binary_add(&result, &value) {
                    Ok(value) => value,
                    Err(error) => return one_error(error),
                },
            });
        }
        vec![Ok(result.unwrap_or(Value::Null))]
    }

    fn flatten(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        if arguments.is_empty() {
            return flatten(input, None).map_or_else(one_error, |value| vec![Ok(value)]);
        }
        let mut output = Vec::new();
        for requested in self.node(arguments[0], input, environment, depth) {
            let requested = match requested {
                Ok(Value::Number(number)) => match number.exact_index() {
                    Some(depth) => Some(depth),
                    None if number.as_f64().is_sign_negative() => {
                        output.push(Err(runtime(
                            "flatten depth must not be negative".to_owned(),
                        )));
                        break;
                    }
                    None => None,
                },
                Ok(value) => {
                    output.push(Err(type_error("flatten", &value)));
                    break;
                }
                Err(error) => {
                    output.push(Err(error));
                    break;
                }
            };
            match flatten(input, requested) {
                Ok(value) => output.push(Ok(value)),
                Err(error) => {
                    output.push(Err(error));
                    break;
                }
            }
        }
        output
    }

    fn range(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut numbers = Vec::new();
        for argument in arguments {
            match first_value(self.node(*argument, input, environment, depth)) {
                Ok(Value::Number(number)) => numbers.push(number.as_f64()),
                Ok(value) => return one_error(type_error("range", &value)),
                Err(error) => return one_error(error),
            }
        }
        let (mut current, end, step) = match numbers.as_slice() {
            [end] => (0.0, *end, 1.0),
            [start, end] => (*start, *end, 1.0),
            [start, end, step] => (*start, *end, *step),
            _ => return one_error(invalid("range arity")),
        };
        if step == 0.0 {
            return one_error(runtime("range step cannot be zero".to_owned()));
        }
        let mut output = Vec::new();
        while (step > 0.0 && current < end) || (step < 0.0 && current > end) {
            output.extend(number_value(current));
            current += step;
            if output.len() >= self.limits.fork_stack {
                return one_error(resource("fork-stack"));
            }
        }
        output
    }

    fn sort_by(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        unique: bool,
    ) -> Outcomes {
        let Value::Array(values) = input else {
            return one_error(type_error("sort_by", input));
        };
        let Some(argument) = arguments.first() else {
            return one_error(invalid("sort_by argument missing"));
        };
        let mut keyed = Vec::new();
        for value in values.iter() {
            match collect_values(self.node(*argument, value, environment, depth)) {
                Ok(key_values) => keyed.push((Value::array(key_values), value.clone())),
                Err(error) => return one_error(error),
            }
        }
        sort_by_cached_key(&mut keyed);
        if unique {
            keyed.dedup_by(|left, right| collection::jq_equal(&left.0, &right.0));
        }
        vec![Ok(Value::array(
            keyed
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>(),
        ))]
    }

    fn with_entries(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(argument) = arguments.first() else {
            return one_error(invalid("with_entries argument missing"));
        };
        let entries = match to_entries(input) {
            Ok(Value::Array(entries)) => entries,
            Ok(_) => unreachable!("to_entries always returns an array"),
            Err(error) => return one_error(error),
        };
        let mut mapped = Vec::new();
        for entry in entries.iter() {
            match collect_values(self.node(*argument, entry, environment, depth)) {
                Ok(values) => mapped.extend(values),
                Err(error) => return one_error(error),
            }
        }
        vec![from_entries(&mapped, "with_entries")]
    }

    fn keyed_values(
        &self,
        operation: &str,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<(Value, Value)>, VmError> {
        let Value::Array(values) = input else {
            return Err(type_error(operation, input));
        };
        let Some(argument) = arguments.first() else {
            return Err(invalid("keyed builtin argument missing"));
        };
        let mut keyed = Vec::with_capacity(values.len());
        for value in values.iter() {
            let keys = collect_values(self.node(*argument, value, environment, depth))?;
            keyed.push((Value::array(keys), value.clone()));
        }
        Ok(keyed)
    }

    fn group_by(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut keyed = match self.keyed_values("group_by", arguments, input, environment, depth) {
            Ok(keyed) => keyed,
            Err(error) => return one_error(error),
        };
        sort_by_cached_key(&mut keyed);
        let mut groups: Vec<Vec<Value>> = Vec::new();
        let mut previous: Option<Value> = None;
        for (key, value) in keyed {
            if previous
                .as_ref()
                .is_none_or(|previous| !collection::jq_equal(previous, &key))
            {
                groups.push(Vec::new());
                previous = Some(key);
            }
            groups.last_mut().expect("group was created").push(value);
        }
        vec![Ok(Value::array(
            groups.into_iter().map(Value::array).collect::<Vec<_>>(),
        ))]
    }

    fn keyed_extrema(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        maximum: bool,
    ) -> Outcomes {
        let operation = if maximum { "max_by" } else { "min_by" };
        let keyed = match self.keyed_values(operation, arguments, input, environment, depth) {
            Ok(keyed) => keyed,
            Err(error) => return one_error(error),
        };
        let selected = if maximum {
            keyed
                .into_iter()
                .max_by(|left, right| jq_compare(&left.0, &right.0))
        } else {
            keyed
                .into_iter()
                .min_by(|left, right| jq_compare(&left.0, &right.0))
        };
        vec![Ok(selected.map_or(Value::Null, |(_, value)| value))]
    }

    fn limit(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(count_node) = arguments.first() else {
            return one_error(invalid("limit count missing"));
        };
        let Some(expression) = arguments.get(1) else {
            return one_error(invalid("limit expression missing"));
        };
        let mut output = Vec::new();
        for count in self.node(*count_node, input, environment, depth) {
            let count = match count.and_then(|value| limit_count(&value)) {
                Ok(count) => count,
                Err(error) => {
                    output.push(Err(error));
                    break;
                }
            };
            if count == 0 {
                continue;
            }
            let mut emitted = 0usize;
            self.emit_node(*expression, input, environment, depth, &mut |result| {
                let success = result.is_ok();
                output.push(result);
                if success {
                    emitted += 1;
                }
                success && emitted < count
            });
        }
        output
    }

    fn any_all(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        any: bool,
    ) -> Outcomes {
        let (generator, condition) = match arguments {
            [] => (None, None),
            [condition] => (None, Some(*condition)),
            [generator, condition] => (Some(*generator), Some(*condition)),
            _ => return one_error(invalid("predicate arity")),
        };
        let mut decision = None;
        let mut failure = None;
        let mut test = |generated: Result<Value, VmError>| {
            let generated = match generated {
                Ok(value) => value,
                Err(error) => {
                    failure = Some(error);
                    return false;
                }
            };
            if let Some(condition) = condition {
                self.emit_node(condition, &generated, environment, depth, &mut |tested| {
                    let tested = match tested {
                        Ok(value) => value.is_truthy(),
                        Err(error) => {
                            failure = Some(error);
                            return false;
                        }
                    };
                    if any == tested {
                        decision = Some(any);
                        false
                    } else {
                        true
                    }
                });
                decision.is_none() && failure.is_none()
            } else if any == generated.is_truthy() {
                decision = Some(any);
                false
            } else {
                true
            }
        };
        if let Some(generator) = generator {
            self.emit_node(generator, input, environment, depth, &mut test);
        } else {
            self.emit_iterable_values(input, depth, &mut test);
        }
        if let Some(error) = failure {
            one_error(error)
        } else {
            vec![Ok(Value::Bool(decision.unwrap_or(!any)))]
        }
    }

    fn emit_iterable_values(
        &self,
        input: &Value,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        if let Err(error) = self.enter(depth) {
            return emit(Err(error));
        }
        match input {
            Value::Array(values) => {
                for value in values.iter().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            Value::Object(values) => {
                for value in values.values().cloned() {
                    if let Err(error) = self.enter(depth) {
                        return emit(Err(error));
                    }
                    if !emit(Ok(value)) {
                        return false;
                    }
                }
                true
            }
            value => emit(Err(type_error("iterate", value))),
        }
    }

    fn setpath(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(path_node) = arguments.first() else {
            return one_error(invalid("setpath path missing"));
        };
        let Some(value_node) = arguments.get(1) else {
            return one_error(invalid("setpath value missing"));
        };
        let paths = self.node(*path_node, input, environment, depth);
        let values = self.node(*value_node, input, environment, depth);
        let mut output = Vec::new();
        for value in &values {
            match value {
                Ok(value) => {
                    for path in &paths {
                        let path = match path {
                            Ok(path) => match jq_path(path) {
                                Ok(path) => path,
                                Err(error) => return one_error(error),
                            },
                            Err(error) => return one_error(error.clone()),
                        };
                        if path.len() > self.limits.path_stack {
                            return one_error(resource("path-stack"));
                        }
                        let mut charge = || self.enter(depth);
                        output.push(path::replace_or_create_bounded(
                            input,
                            &path,
                            value.clone(),
                            self.limits,
                            &mut charge,
                        ));
                    }
                }
                Err(error) => return one_error(error.clone()),
            }
        }
        output
    }

    fn paths_filter(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(filter) = arguments.first() else {
            return one_error(invalid("paths filter missing"));
        };
        let candidates = match self.descendant_path_values(input, depth + 1) {
            Ok(candidates) => candidates,
            Err(error) => return one_error(error),
        };
        let mut output = Vec::new();
        for (path, value) in candidates.into_iter().skip(1) {
            let values = self.node(*filter, &value, environment, depth + 1);
            for value in values {
                match value {
                    Ok(value) if value.is_truthy() => output.push(Ok(path_value(&path))),
                    Ok(_) => {}
                    Err(error) => {
                        output.push(Err(error));
                        return output;
                    }
                }
            }
        }
        output
    }

    fn pick(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(path_node) = arguments.first() else {
            return one_error(invalid("pick path expression missing"));
        };
        let paths = match self.paths(*path_node, input, environment, depth) {
            Ok(paths) => paths,
            Err(error) => return one_error(error),
        };
        let mut picked = Value::Null;
        for path in paths {
            let value = path.get(input).cloned().unwrap_or(Value::Null);
            let mut charge = || self.enter(depth);
            picked = match path::replace_or_create_bounded(
                &picked,
                path.components(),
                value,
                self.limits,
                &mut charge,
            ) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
        }
        vec![Ok(picked)]
    }

    fn delete(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(path_node) = arguments.first() else {
            return one_error(invalid("del path expression missing"));
        };
        let mut paths = match self.paths(*path_node, input, environment, depth) {
            Ok(paths) => paths,
            Err(error) => return one_error(error),
        };
        paths.sort_by(compare_paths_for_deletion);
        paths.dedup_by(|left, right| left == right);
        let mut deleted = input.clone();
        for path in paths {
            let mut charge = || self.enter(depth);
            deleted = match path::delete_path_bounded(
                &deleted,
                path.components(),
                self.limits,
                &mut charge,
            ) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
        }
        vec![Ok(deleted)]
    }

    fn delete_paths(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(paths_node) = arguments.first() else {
            return one_error(invalid("delpaths paths missing"));
        };
        let paths_values = self.node(*paths_node, input, environment, depth);
        let mut output = Vec::new();
        for paths_value in paths_values {
            let paths_value = match paths_value {
                Ok(value) => value,
                Err(error) => {
                    output.push(Err(error));
                    return output;
                }
            };
            let Value::Array(path_values) = paths_value else {
                return one_error(type_error("delpaths", &paths_value));
            };
            let mut paths = Vec::new();
            for path_value in path_values.iter() {
                match jq_path(path_value) {
                    Ok(path) if path.len() <= self.limits.path_stack => paths.push(Path::new(path)),
                    Ok(_) => return one_error(resource("path-stack")),
                    Err(error) => return one_error(error),
                }
            }
            paths.sort_by(compare_paths_for_deletion);
            paths.dedup_by(|left, right| left == right);
            let mut deleted = input.clone();
            for path in paths {
                let mut charge = || self.enter(depth);
                deleted = match path::delete_path_bounded(
                    &deleted,
                    path.components(),
                    self.limits,
                    &mut charge,
                ) {
                    Ok(value) => value,
                    Err(error) => return one_error(error),
                };
            }
            output.push(Ok(deleted));
        }
        output
    }

    fn fromstream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_fromstream(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_fromstream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let Some(stream) = arguments.first() else {
            return emit(Err(invalid("fromstream stream argument missing")));
        };
        let mut root = None;
        let stream_completed = self.emit_node(*stream, input, environment, depth, &mut |event| {
            let event = match event {
                Ok(event) => event,
                Err(error) => {
                    return emit(Err(error));
                }
            };
            let (path, value) = match stream_event(&event) {
                Ok(event) => event,
                Err(error) => {
                    return emit(Err(error));
                }
            };
            if path.len() > self.limits.path_stack {
                return emit(Err(resource("path-stack")));
            }
            if let Err(error) = self.enter(depth.saturating_add(path.len())) {
                return emit(Err(error));
            }
            match value {
                Some(value) if path.is_empty() => emit(Ok(value)),
                Some(value) => {
                    let base = root.take().unwrap_or(Value::Null);
                    let mut charge = || self.enter(depth.saturating_add(path.len()));
                    match path::replace_or_create_bounded(
                        &base,
                        &path,
                        value,
                        self.limits,
                        &mut charge,
                    ) {
                        Ok(rebuilt) => {
                            root = Some(rebuilt);
                            true
                        }
                        Err(error) => emit(Err(error)),
                    }
                }
                None if path.len() == 1 => emit_stream_root(&mut root, emit),
                None => true,
            }
        });
        if !stream_completed {
            return false;
        }
        true
    }

    fn truncate_stream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let mut output = Vec::new();
        self.emit_truncate_stream(arguments, input, environment, depth, &mut |result| {
            output.push(result);
            true
        });
        output
    }

    fn emit_truncate_stream(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
        emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
    ) -> bool {
        let count = match limit_count(input) {
            Ok(count) => count,
            Err(error) => return emit(Err(error)),
        };
        for event_node in arguments {
            let keep_going = self.emit_node(*event_node, input, environment, depth, &mut |event| {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => return emit(Err(error)),
                };
                let (mut path, value) = match stream_event(&event) {
                    Ok(event) => event,
                    Err(error) => return emit(Err(error)),
                };
                if path.len() <= count {
                    return true;
                }
                path.drain(..count);
                if path.len() > self.limits.path_stack {
                    return emit(Err(resource("path-stack")));
                }
                let mut record = vec![path_value(&Path::new(path))];
                if let Some(value) = value {
                    record.push(value);
                }
                emit(Ok(Value::array(record)))
            });
            if !keep_going {
                return false;
            }
        }
        true
    }

    fn assignment(
        &self,
        operator: AssignmentOperator,
        path_node: u32,
        value_node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let paths = match self.paths(path_node, input, environment, depth) {
            Ok(paths) => paths,
            Err(error) => return one_error(error),
        };
        if operator == AssignmentOperator::Update {
            let mut document = input.clone();
            let mut deletions = path_builtin::PathAccumulator::new();
            for path in paths {
                let old = path.get(&document).cloned().unwrap_or(Value::Null);
                let mut values = self.node(value_node, &old, environment, depth);
                match values.drain(..).next() {
                    Some(Ok(replacement)) => {
                        let mut charge = || self.enter(depth);
                        document = match path::replace_or_create_bounded(
                            &document,
                            path.components(),
                            replacement,
                            self.limits,
                            &mut charge,
                        ) {
                            Ok(value) => value,
                            Err(error) => return one_error(error),
                        };
                    }
                    Some(Err(error)) => return one_error(error),
                    None => {
                        let mut charge = || self.enter(depth);
                        if let Err(error) =
                            deletions.push(path.components(), self.limits, &mut charge)
                        {
                            return one_error(error);
                        }
                    }
                }
            }
            let mut charge = || self.enter(depth);
            document = match path_builtin::delete_paths_bounded(
                &document,
                deletions,
                self.limits,
                &mut charge,
            ) {
                Ok(value) => value,
                Err(error) => return one_error(error),
            };
            return vec![Ok(document)];
        }

        let values = self.node(value_node, input, environment, depth);
        let mut documents = Vec::new();
        for replacement in values {
            let replacement = match replacement {
                Ok(replacement) => replacement,
                Err(error) => {
                    documents.push(Err(error));
                    break;
                }
            };
            let mut document = input.clone();
            for path in &paths {
                let old = path.get(input).cloned().unwrap_or(Value::Null);
                let replacement = match update_value(operator, &old, &replacement) {
                    Ok(value) => value,
                    Err(error) => {
                        documents.push(Err(error));
                        return documents;
                    }
                };
                let mut charge = || self.enter(depth);
                document = match path::replace_or_create_bounded(
                    &document,
                    path.components(),
                    replacement,
                    self.limits,
                    &mut charge,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        documents.push(Err(error));
                        return documents;
                    }
                };
            }
            documents.push(Ok(document));
        }
        documents
    }

    fn descendant_paths(&self, input: &Value, depth: usize) -> Result<Vec<Path>, VmError> {
        Ok(self
            .descendant_path_values(input, depth)?
            .into_iter()
            .skip(1)
            .map(|(path, _)| path)
            .collect())
    }

    fn descendant_path_values(
        &self,
        input: &Value,
        depth: usize,
    ) -> Result<Vec<(Path, Value)>, VmError> {
        let mut output = Vec::new();
        let mut pending = vec![(input.clone(), Vec::new())];
        while let Some((value, components)) = pending.pop() {
            if components.len() > self.limits.path_stack {
                return Err(resource("path-stack"));
            }
            self.enter(depth.saturating_add(components.len()))?;
            let mut observations = self.observations.get();
            observations.path_stack_high_water =
                observations.path_stack_high_water.max(components.len());
            self.observations.set(observations);
            output.push((Path::new(components.clone()), value.clone()));
            push_children(&value, &components, &mut pending);
        }
        Ok(output)
    }

    fn paths(
        &self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Result<Vec<Path>, VmError> {
        self.path_values(node, input, environment, depth)
            .into_iter()
            .map(|result| {
                result.and_then(|(path, _)| {
                    let length = path.components().len();
                    if length > self.limits.path_stack {
                        Err(resource("path-stack"))
                    } else {
                        let mut observations = self.observations.get();
                        observations.path_stack_high_water =
                            observations.path_stack_high_water.max(length);
                        self.observations.set(observations);
                        Ok(path)
                    }
                })
            })
            .collect()
    }

    fn path_outcomes(
        &self,
        arguments: &[u32],
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Outcomes {
        let Some(node) = arguments.first() else {
            return one_error(invalid("path argument missing"));
        };
        self.path_values(*node, input, environment, depth)
            .into_iter()
            .map(|result| result.map(|(path, _)| path_value(&path)))
            .collect()
    }

    #[allow(
        clippy::too_many_lines,
        reason = "path traversal keeps assignment operations exhaustive in one bounded interpreter"
    )]
    fn path_values(
        &self,
        node: u32,
        input: &Value,
        environment: &Environment,
        depth: usize,
    ) -> Vec<Result<(Path, Value), VmError>> {
        if let Err(error) = self.enter(depth) {
            return vec![Err(error)];
        }
        let operation = match self.bytecode.instructions().get(node as usize) {
            Some(instruction) => instruction.operation.clone(),
            None => return vec![Err(invalid("path instruction missing"))],
        };
        match operation {
            Operation::Identity => vec![Ok((Path::root(), input.clone()))],
            Operation::AccessField { base, key } => {
                let key = match self.string(key) {
                    Ok(key) => Arc::clone(key),
                    Err(error) => return vec![Err(error)],
                };
                let mut output = Vec::new();
                for result in self.path_values(base, input, environment, depth + 1) {
                    let (path, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    let selected = match access_field(&value, &key) {
                        Ok(selected) => selected,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    let mut components = path.components().to_vec();
                    components.push(PathComponent::Key(Arc::clone(&key)));
                    output.push(Ok((Path::new(components), selected)));
                }
                output
            }
            Operation::AccessIndex { base, index } => {
                let mut output = Vec::new();
                for result in self.path_values(base, input, environment, depth + 1) {
                    let (path, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    for index in self.node(index, input, environment, depth + 1) {
                        let index = match index {
                            Ok(index) => index,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let component = match path_component_for_target(&value, &index) {
                            Ok(component) => component,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let selected = match access_index(&value, &index) {
                            Ok(selected) => selected,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let mut components = path.components().to_vec();
                        components.push(component);
                        output.push(Ok((Path::new(components), selected)));
                    }
                }
                output
            }
            Operation::Iterate(base) => {
                let mut output = Vec::new();
                for result in self.path_values(base, input, environment, depth + 1) {
                    let (path, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    match value {
                        Value::Array(values) => {
                            for (index, selected) in values.iter().enumerate() {
                                let mut components = path.components().to_vec();
                                components.push(PathComponent::Index(index));
                                output.push(Ok((Path::new(components), selected.clone())));
                            }
                        }
                        Value::Object(values) => {
                            for (key, selected) in values.iter() {
                                let mut components = path.components().to_vec();
                                components.push(PathComponent::Key(Arc::clone(key)));
                                output.push(Ok((Path::new(components), selected.clone())));
                            }
                        }
                        value => {
                            output.push(Err(type_error("update iteration", &value)));
                            return output;
                        }
                    }
                }
                output
            }
            Operation::RecursiveDescent => match self.descendant_path_values(input, depth + 1) {
                Ok(values) => values.into_iter().map(Ok).collect(),
                Err(error) => vec![Err(error)],
            },
            Operation::Pipe { left, right } => {
                let mut output = Vec::new();
                for result in self.path_values(left, input, environment, depth + 1) {
                    let (prefix, value) = match result {
                        Ok(value) => value,
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    };
                    for result in self.path_values(right, &value, environment, depth + 1) {
                        let (relative, selected) = match result {
                            Ok(value) => value,
                            Err(error) => {
                                output.push(Err(error));
                                return output;
                            }
                        };
                        let mut components = prefix.components().to_vec();
                        components.extend(relative.components().iter().cloned());
                        output.push(Ok((Path::new(components), selected)));
                    }
                }
                output
            }
            Operation::Comma { left, right } => {
                let mut output = self.path_values(left, input, environment, depth + 1);
                if output.iter().any(Result::is_err) {
                    return output;
                }
                output.extend(self.path_values(right, input, environment, depth + 1));
                output
            }
            Operation::Optional(child) => {
                let values = self.path_values(child, input, environment, depth + 1);
                if values
                    .last()
                    .is_some_and(|result| result.as_ref().is_err_and(is_optional_suppressible))
                {
                    values
                        .into_iter()
                        .filter_map(|result| result.ok().map(Ok))
                        .collect::<Vec<_>>()
                } else {
                    values
                }
            }
            Operation::Call { name, arguments } => {
                let is_select = match self.string(name) {
                    Ok(name) => name.as_ref() == "select",
                    Err(error) => return vec![Err(error)],
                };
                if !is_select {
                    if self.string(name).is_ok_and(|name| name.as_ref() == "error") {
                        let Some(argument) = arguments.first() else {
                            return vec![Err(raised(input.clone()))];
                        };
                        return self
                            .node(*argument, input, environment, depth + 1)
                            .into_iter()
                            .map(|result| result.map_or_else(Err, |value| Err(raised(value))))
                            .collect();
                    }
                    return vec![Err(runtime(
                        "assignment left side is not a path".to_owned(),
                    ))];
                }
                let Some(argument) = arguments.first() else {
                    return vec![Err(invalid("select argument missing"))];
                };
                let mut output = Vec::new();
                for result in self.node(*argument, input, environment, depth + 1) {
                    match result {
                        Ok(value) if value.is_truthy() => {
                            output.push(Ok((Path::root(), input.clone())));
                        }
                        Ok(_) => {}
                        Err(error) => {
                            output.push(Err(error));
                            return output;
                        }
                    }
                }
                output
            }
            _ => vec![Err(runtime(
                "assignment left side is not a path".to_owned(),
            ))],
        }
    }
}

fn limit_count(value: &Value) -> Result<usize, VmError> {
    let Value::Number(number) = value else {
        return Err(type_error("limit", value));
    };
    let count = number.exact_index().or_else(|| {
        let value = number.as_f64();
        (value.is_finite() && value >= 0.0)
            .then(|| Number::from_runtime_f64(value.trunc()))
            .and_then(|number| number.exact_index())
    });
    match count {
        Some(count) if count >= 0 => Ok(usize::try_from(count).unwrap_or(usize::MAX)),
        _ => Err(runtime(
            "limit count must be a non-negative number".to_owned(),
        )),
    }
}

fn catch_value(error: &VmError) -> Value {
    match error {
        VmError::Raised { value, .. } => value.clone(),
        VmError::Runtime { message } | VmError::CapabilityDenied { message } => {
            Value::string(Arc::clone(message))
        }
        VmError::Break { .. } => {
            let mut sentinel = Object::new();
            sentinel.insert(
                Arc::from("__jq"),
                Value::Number(Number::parse("0").expect("zero is a valid number")),
            );
            Value::object(sentinel)
        }
        _ => Value::string(error.to_string()),
    }
}

fn error_message(value: &Value) -> Arc<str> {
    match value {
        Value::String(message) => Arc::clone(message),
        value => value.to_string().into(),
    }
}

fn raised(value: Value) -> VmError {
    VmError::Raised {
        message: error_message(&value),
        value,
    }
}

fn is_catchable_error(error: &VmError) -> bool {
    matches!(
        error,
        VmError::Input { .. }
            | VmError::Runtime { .. }
            | VmError::CapabilityDenied { .. }
            | VmError::Raised { .. }
            | VmError::NumericRange { .. }
            | VmError::Break { .. }
    )
}

fn is_optional_suppressible(error: &VmError) -> bool {
    matches!(
        error,
        VmError::Input { .. }
            | VmError::Runtime { .. }
            | VmError::CapabilityDenied { .. }
            | VmError::Raised { .. }
            | VmError::NumericRange { .. }
            | VmError::Break { .. }
    )
}

fn map_outcomes(values: Outcomes, apply: impl Fn(&Value) -> Result<Value, VmError>) -> Outcomes {
    values
        .into_iter()
        .map(|value| value.and_then(|value| apply(&value)))
        .collect()
}

fn collect_values(values: Outcomes) -> Result<Vec<Value>, VmError> {
    values.into_iter().collect()
}

fn first_value(values: Outcomes) -> Result<Value, VmError> {
    values
        .into_iter()
        .next()
        .unwrap_or_else(|| Err(runtime("filter produced no value".to_owned())))
}

fn first_error(values: &Outcomes) -> Option<VmError> {
    values
        .iter()
        .find_map(|value| value.as_ref().err().cloned())
}

fn ends_in_error(values: &Outcomes) -> bool {
    values.last().is_some_and(Result::is_err)
}

fn one_error(error: VmError) -> Outcomes {
    vec![Err(error)]
}

fn runtime(message: String) -> VmError {
    VmError::Runtime {
        message: message.into(),
    }
}

fn capability_denied(message: String) -> VmError {
    VmError::CapabilityDenied {
        message: message.into(),
    }
}

fn invalid(message: &'static str) -> VmError {
    VmError::InvalidProgram { message }
}

fn resource(resource: &'static str) -> VmError {
    VmError::Resource { resource }
}

fn ambient_platform(environment: &Environment) -> bool {
    matches!(environment.get(AMBIENT_PLATFORM), Some(Value::Bool(true)))
}

fn ambient_environment(environment: &Environment) -> Result<Value, VmError> {
    match environment.get(AMBIENT_ENVIRONMENT) {
        Some(Value::Object(values)) => Ok(Value::Object(Arc::clone(values))),
        _ => Err(capability_denied(
            "env requires environment access permitted by capability policy".to_owned(),
        )),
    }
}

fn ambient_value(environment: &Environment, key: &str, operation: &str) -> Result<Value, VmError> {
    if !ambient_platform(environment) {
        return Err(capability_denied(format!(
            "{operation} requires platform access permitted by capability policy"
        )));
    }
    environment.get(key).cloned().ok_or_else(|| {
        runtime(format!(
            "{operation} metadata is unavailable for this input mode"
        ))
    })
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_error(operation: &str, value: &Value) -> VmError {
    runtime(format!(
        "{operation} cannot be applied to {}",
        type_name(value)
    ))
}

fn is_pattern_mismatch(error: &VmError) -> bool {
    matches!(error, VmError::Runtime { message } if message.contains("destructuring cannot be applied"))
}

fn is_retryable_alternative_error(error: &VmError) -> bool {
    matches!(
        error,
        VmError::Runtime { .. }
            | VmError::CapabilityDenied { .. }
            | VmError::Raised { .. }
            | VmError::Break { .. }
    )
}

fn bind_pattern(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    value: &Value,
    environment: &mut Environment,
    depth: usize,
    charge: &mut dyn FnMut(usize) -> Result<(), VmError>,
) -> Result<(), VmError> {
    charge(depth)?;
    match pattern {
        BindingPatternOperand::Variable(name) => {
            let name = bytecode
                .string(*name)
                .ok_or_else(|| invalid("pattern variable missing after validation"))?;
            environment.insert(Arc::clone(name), value.clone());
            Ok(())
        }
        BindingPatternOperand::Array(patterns) => {
            match value {
                Value::Array(values) => {
                    for (index, pattern) in patterns.iter().enumerate() {
                        let value = values.get(index).cloned().unwrap_or(Value::Null);
                        bind_pattern(
                            bytecode,
                            pattern,
                            &value,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                Value::Null => {
                    for pattern in patterns {
                        bind_pattern(
                            bytecode,
                            pattern,
                            &Value::Null,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                value => return Err(type_error("array destructuring", value)),
            }
            Ok(())
        }
        BindingPatternOperand::Object(entries) => {
            match value {
                Value::Object(values) => {
                    for (key, pattern) in entries {
                        let key = bytecode
                            .string(*key)
                            .ok_or_else(|| invalid("pattern key missing after validation"))?;
                        let value = values.get(key).cloned().unwrap_or(Value::Null);
                        bind_pattern(
                            bytecode,
                            pattern,
                            &value,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                Value::Null => {
                    for (_, pattern) in entries {
                        bind_pattern(
                            bytecode,
                            pattern,
                            &Value::Null,
                            environment,
                            depth.saturating_add(1),
                            charge,
                        )?;
                    }
                }
                value => return Err(type_error("object destructuring", value)),
            }
            Ok(())
        }
    }
}

fn initialize_pattern_variables(
    bytecode: &Bytecode,
    patterns: &[BindingPatternOperand],
    environment: &mut Environment,
) -> Result<(), VmError> {
    for pattern in patterns {
        initialize_pattern_variables_one(bytecode, pattern, environment)?;
    }
    Ok(())
}

fn initialize_pattern_variables_one(
    bytecode: &Bytecode,
    pattern: &BindingPatternOperand,
    environment: &mut Environment,
) -> Result<(), VmError> {
    match pattern {
        BindingPatternOperand::Variable(name) => {
            let name = bytecode
                .string(*name)
                .ok_or_else(|| invalid("pattern variable missing after validation"))?;
            environment.entry(Arc::clone(name)).or_insert(Value::Null);
        }
        BindingPatternOperand::Array(patterns) => {
            for pattern in patterns {
                initialize_pattern_variables_one(bytecode, pattern, environment)?;
            }
        }
        BindingPatternOperand::Object(entries) => {
            for (_, pattern) in entries {
                initialize_pattern_variables_one(bytecode, pattern, environment)?;
            }
        }
    }
    Ok(())
}

fn access_field(value: &Value, key: &Arc<str>) -> Result<Value, VmError> {
    match value {
        Value::Object(object) => Ok(object.get(key).cloned().unwrap_or(Value::Null)),
        Value::Null => Ok(Value::Null),
        value => Err(type_error("field access", value)),
    }
}

fn access_index(value: &Value, index: &Value) -> Result<Value, VmError> {
    match (value, index) {
        (Value::Object(object), Value::String(key)) => {
            Ok(object.get(key).cloned().unwrap_or(Value::Null))
        }
        (Value::Array(values), Value::Number(number)) => {
            let index = number
                .exact_index()
                .ok_or_else(|| runtime("array index must be an exact integer".to_owned()))?;
            let index = normalize_index(index, values.len());
            Ok(index
                .and_then(|index| values.get(index).cloned())
                .unwrap_or(Value::Null))
        }
        (Value::Null, Value::String(_) | Value::Number(_)) => Ok(Value::Null),
        (_, Value::String(_) | Value::Number(_)) => Err(type_error("index", value)),
        _ => Err(runtime(
            "index must be a string or exact integer".to_owned(),
        )),
    }
}

fn nth_index(value: &Value, index: &Value) -> Result<Value, VmError> {
    match index {
        Value::Number(number) => {
            let index = Value::Number(Number::from_runtime_f64(number.as_f64().trunc()));
            access_index(value, &index)
        }
        index => access_index(value, index),
    }
}

fn index_origin(
    value: &Value,
    index: f64,
    origin: Option<&OriginToken>,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Option<OriginToken>, VmError> {
    let Value::Array(values) = value else {
        return Ok(None);
    };
    if !index.is_finite() {
        return Ok(None);
    }
    let index = index.trunc();
    #[allow(clippy::cast_possible_truncation)]
    let index = index as i128;
    let Some(index) = i64::try_from(index).ok() else {
        return Ok(None);
    };
    let Some(index) = normalize_index(index, values.len()) else {
        return Ok(None);
    };
    origin
        .map(|origin| origin.child_bounded(PathComponent::Index(index), limits, charge))
        .transpose()
}

fn normalize_index(index: i64, length: usize) -> Option<usize> {
    if index >= 0 {
        usize::try_from(index).ok().filter(|index| *index < length)
    } else {
        let signed_length = i64::try_from(length).ok()?;
        usize::try_from(signed_length + index)
            .ok()
            .filter(|index| *index < length)
    }
}

fn slice_bound_value(value: &Value) -> Result<Option<f64>, VmError> {
    match value {
        Value::Null => Ok(None),
        Value::Number(number) => Ok(Some(number.as_f64())),
        value => Err(type_error("slice", value)),
    }
}

fn slice(value: &Value, start: Option<f64>, end: Option<f64>) -> Result<Value, VmError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Array(values) => {
            let (start, end) = slice_bounds(values.len(), start, end);
            Ok(Value::array(values[start..end].to_vec()))
        }
        Value::String(value) => {
            let characters = value.chars().collect::<Vec<_>>();
            let (start, end) = slice_bounds(characters.len(), start, end);
            Ok(Value::string(
                characters[start..end].iter().collect::<String>(),
            ))
        }
        value => Err(type_error("slice", value)),
    }
}

#[allow(clippy::cast_precision_loss)]
pub(super) fn slice_bounds(length: usize, start: Option<f64>, end: Option<f64>) -> (usize, usize) {
    let normalize = |value: f64, is_end: bool| {
        let rounded = if value.is_nan() {
            if is_end {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        } else if value.is_infinite() {
            value
        } else if is_end {
            value.ceil()
        } else {
            value.floor()
        };
        if rounded < 0.0 || (is_end && value < 0.0 && rounded == 0.0) {
            let magnitude = -rounded;
            if magnitude >= length as f64 {
                0
            } else {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    length.saturating_sub(magnitude as usize)
                }
            }
        } else if rounded >= length as f64 || rounded.is_infinite() {
            length
        } else {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                rounded as usize
            }
        }
    };
    let start = start.map_or(0, |value| normalize(value, false));
    let end = end.map_or(length, |value| normalize(value, true));
    (start.min(end), end)
}

fn computed_key(value: Value) -> Result<Arc<str>, VmError> {
    match value {
        Value::String(value) => Ok(value),
        value => Err(type_error("computed object key", &value)),
    }
}

fn unary(operator: UnaryOperator, value: &Value) -> Result<Value, VmError> {
    match operator {
        UnaryOperator::Not => Ok(Value::Bool(!value.is_truthy())),
        UnaryOperator::Negate => match value {
            Value::Number(value) => Ok(Value::Number(value.negate())),
            value => Err(type_error("negation", value)),
        },
    }
}

fn binary_value(operator: BinaryOperator, left: &Value, right: &Value) -> Result<Value, VmError> {
    match operator {
        BinaryOperator::Equal => Ok(Value::Bool(collection::jq_equal(left, right))),
        BinaryOperator::NotEqual => Ok(Value::Bool(!collection::jq_equal(left, right))),
        BinaryOperator::Less => Ok(Value::Bool(jq_compare(left, right).is_lt())),
        BinaryOperator::LessEqual => Ok(Value::Bool(!jq_compare(left, right).is_gt())),
        BinaryOperator::Greater => Ok(Value::Bool(jq_compare(left, right).is_gt())),
        BinaryOperator::GreaterEqual => Ok(Value::Bool(!jq_compare(left, right).is_lt())),
        BinaryOperator::Add => binary_add(left, right),
        BinaryOperator::Subtract => binary_subtract(left, right),
        BinaryOperator::Multiply => binary_multiply(left, right),
        BinaryOperator::Divide => match (left, right) {
            (Value::String(_), Value::String(_)) => string_compat::divide(left, right),
            _ => numeric(left, right, Number::divide, "divide"),
        },
        BinaryOperator::Remainder => match (left, right) {
            (Value::Number(left), Value::Number(right)) if right.as_f64() != 0.0 => {
                match (
                    remainder_operand(left.as_f64()),
                    remainder_operand(right.as_f64()),
                ) {
                    (Some(left), Some(right)) if right != 0 => {
                        Ok(runtime_number(integer_remainder(left, right)))
                    }
                    (Some(_), Some(0)) => Err(runtime("cannot divide by zero".to_owned())),
                    _ => Ok(runtime_number(f64::NAN)),
                }
            }
            (Value::Number(_), Value::Number(_)) => {
                Err(runtime("cannot divide by zero".to_owned()))
            }
            _ => Err(runtime("remainder requires numbers".to_owned())),
        },
        BinaryOperator::Alternative | BinaryOperator::Or | BinaryOperator::And => {
            Err(invalid("short-circuit operator reached scalar dispatch"))
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn remainder_operand(value: f64) -> Option<i64> {
    if value.is_nan() {
        None
    } else if value >= 9_223_372_036_854_775_808.0 {
        Some(i64::MAX)
    } else if value <= -9_223_372_036_854_775_808.0 {
        Some(i64::MIN)
    } else {
        Some(value.trunc() as i64)
    }
}

#[allow(clippy::cast_precision_loss)]
fn integer_remainder(left: i64, right: i64) -> f64 {
    let remainder = if left == i64::MIN && right == -1 {
        0
    } else {
        left % right
    };
    remainder as f64
}

fn jq_compare(left: &Value, right: &Value) -> std::cmp::Ordering {
    let kind = jq_kind_rank(left).cmp(&jq_kind_rank(right));
    if kind != std::cmp::Ordering::Equal {
        return kind;
    }
    match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
        (Value::Number(left), Value::Number(right)) => jq_number_compare(left, right),
        (Value::String(left), Value::String(right)) => left.cmp(right),
        (Value::Array(left), Value::Array(right)) => left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| jq_compare(left, right))
            .find(|ordering| *ordering != std::cmp::Ordering::Equal)
            .unwrap_or_else(|| left.len().cmp(&right.len())),
        (Value::Object(left), Value::Object(right)) => jq_object_compare(left, right),
        _ => std::cmp::Ordering::Equal,
    }
}

fn jq_number_compare(left: &Number, right: &Number) -> std::cmp::Ordering {
    let left_value = left.as_f64();
    let right_value = right.as_f64();
    match (left_value.is_nan(), right_value.is_nan()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) if left_value.is_finite() && right_value.is_finite() => left.cmp(right),
        (false, false) => left_value.total_cmp(&right_value),
    }
}

fn jq_kind_rank(value: &Value) -> u8 {
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

fn jq_object_compare(left: &Object, right: &Object) -> std::cmp::Ordering {
    let mut left_keys = left.keys().collect::<Vec<_>>();
    let mut right_keys = right.keys().collect::<Vec<_>>();
    left_keys.sort_unstable();
    right_keys.sort_unstable();
    let key_order = left_keys
        .iter()
        .zip(right_keys.iter())
        .map(|(left, right)| left.cmp(right))
        .find(|ordering| *ordering != std::cmp::Ordering::Equal)
        .unwrap_or(std::cmp::Ordering::Equal);
    if key_order != std::cmp::Ordering::Equal {
        return key_order;
    }
    let length_order = left_keys.len().cmp(&right_keys.len());
    if length_order != std::cmp::Ordering::Equal {
        return length_order;
    }
    left_keys
        .iter()
        .zip(right_keys.iter())
        .map(|(left_key, right_key)| {
            jq_compare(
                left.get(left_key.as_ref()).expect("sorted key exists"),
                right.get(right_key.as_ref()).expect("sorted key exists"),
            )
        })
        .find(|ordering| *ordering != std::cmp::Ordering::Equal)
        .unwrap_or(std::cmp::Ordering::Equal)
}

fn binary_add(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Null, value) | (value, Value::Null) => Ok(value.clone()),
        (Value::Number(left), Value::Number(right)) => left
            .add(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::String(left), Value::String(right)) => Ok(Value::string(format!("{left}{right}"))),
        (Value::Array(left), Value::Array(right)) => Ok(Value::array(
            left.iter().chain(right.iter()).cloned().collect::<Vec<_>>(),
        )),
        (Value::Object(left), Value::Object(right)) => {
            let mut object = left.as_ref().clone();
            for (key, value) in right.iter() {
                object.insert(Arc::clone(key), value.clone());
            }
            Ok(Value::object(object))
        }
        _ => Err(runtime(format!(
            "cannot add {} and {}",
            type_name(left),
            type_name(right)
        ))),
    }
}

fn binary_subtract(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .subtract(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::Array(left), Value::Array(right)) => Ok(Value::array(
            left.iter()
                .filter(|value| {
                    !right
                        .iter()
                        .any(|candidate| collection::jq_equal(candidate, value))
                })
                .cloned()
                .collect::<Vec<_>>(),
        )),
        _ => Err(runtime("subtraction requires numbers or arrays".to_owned())),
    }
}

fn binary_multiply(left: &Value, right: &Value) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .multiply(right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        (Value::Object(left), Value::Object(right)) => {
            Ok(Value::object(merge_objects(left, right)))
        }
        _ => Err(runtime(format!(
            "cannot multiply {} and {}",
            type_name(left),
            type_name(right)
        ))),
    }
}

fn merge_objects(left: &Object, right: &Object) -> Object {
    let mut merged = left.clone();
    merge_into_object(&mut merged, right);
    merged
}

fn merge_into_object(merged: &mut Object, right: &Object) {
    for (key, right_value) in right {
        if let Some(left_value) = merged.get_mut(key) {
            match (left_value, right_value) {
                (Value::Object(left), Value::Object(right)) => {
                    merge_into_object(Arc::make_mut(left), right);
                }
                (left, right) => *left = right.clone(),
            }
        } else {
            merged.insert(Arc::clone(key), right_value.clone());
        }
    }
}

fn numeric(
    left: &Value,
    right: &Value,
    apply: fn(&Number, &Number) -> Result<Number, crate::NumberError>,
    name: &str,
) -> Result<Value, VmError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => apply(left, right)
            .map(Value::Number)
            .map_err(|error| runtime(error.to_string())),
        _ => Err(runtime(format!("{name} requires numbers"))),
    }
}

fn number_value(number: f64) -> Outcomes {
    Number::from_f64(number)
        .map(Value::Number)
        .map(Ok)
        .map_or_else(
            |error| one_error(runtime(error.to_string())),
            |value| vec![value],
        )
}

fn number_usize(number: usize) -> Result<Value, VmError> {
    Number::parse(&number.to_string())
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn length(value: &Value) -> Result<Value, VmError> {
    let length = match value {
        Value::Null => 0.0,
        Value::Number(number) => number.as_f64().abs(),
        Value::String(value) => return number_usize(value.chars().count()),
        Value::Array(value) => return number_usize(value.len()),
        Value::Object(value) => return number_usize(value.len()),
        Value::Bool(_) => return Err(type_error("length", value)),
    };
    Number::from_f64(length)
        .map(Value::Number)
        .map_err(|error| runtime(error.to_string()))
}

fn absolute_value(value: &Value) -> Result<Value, VmError> {
    match value {
        Value::Number(number) if number.is_less_than_zero() => Ok(Value::Number(number.negate())),
        Value::Number(_) | Value::String(_) | Value::Array(_) | Value::Object(_) => {
            Ok(value.clone())
        }
        Value::Null | Value::Bool(_) => Err(type_error("negation", value)),
    }
}

fn keys(value: &Value, sorted: bool) -> Result<Value, VmError> {
    let mut keys = match value {
        Value::Array(values) => (0..values.len())
            .map(|index| Number::parse(&index.to_string()).map(Value::Number))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| runtime(error.to_string()))?,
        Value::Object(values) => values
            .keys()
            .map(|key| Value::String(Arc::clone(key)))
            .collect(),
        value => return Err(type_error("keys", value)),
    };
    if sorted {
        stable_sort_values(&mut keys);
    }
    Ok(Value::array(keys))
}

fn has(container: &Value, key: &Value) -> Result<Value, VmError> {
    let found = match (container, key) {
        (Value::Object(values), Value::String(key)) => values.contains_key(key),
        (Value::Array(values), Value::Number(index)) => index
            .exact_index()
            .and_then(|index| normalize_index(index, values.len()))
            .is_some(),
        (Value::Object(_), _) => {
            return Err(runtime("object membership requires string key".to_owned()));
        }
        (Value::Array(_), _) => {
            return Err(runtime(
                "array membership requires integer index".to_owned(),
            ));
        }
        (value, _) => return Err(type_error("has", value)),
    };
    Ok(Value::Bool(found))
}

fn selector(input: &Value, selected: bool) -> Outcomes {
    if selected {
        vec![Ok(input.clone())]
    } else {
        Vec::new()
    }
}

fn to_entries(input: &Value) -> Result<Value, VmError> {
    let entries = match input {
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                Ok(Value::object(Object::from_iter([
                    (Arc::from("key"), number_usize(index)?),
                    (Arc::from("value"), value.clone()),
                ])))
            })
            .collect::<Result<Vec<_>, VmError>>()?,
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| {
                Value::object(Object::from_iter([
                    (Arc::from("key"), Value::String(Arc::clone(key))),
                    (Arc::from("value"), value.clone()),
                ]))
            })
            .collect(),
        value => return Err(type_error("to_entries", value)),
    };
    Ok(Value::array(entries))
}

fn from_entries(entries: &[Value], operation: &str) -> Result<Value, VmError> {
    let mut object = Object::new();
    for entry in entries {
        let Value::Object(entry) = entry else {
            return Err(type_error(operation, entry));
        };
        let key = ["key", "Key", "name", "Name"]
            .into_iter()
            .filter_map(|name| entry.get(name))
            .find(|value| value.is_truthy())
            .ok_or_else(|| runtime("entry is missing a key".to_owned()))?;
        let value = entry
            .get("value")
            .or_else(|| entry.get("Value"))
            .cloned()
            .unwrap_or(Value::Null);
        let key = match key {
            Value::String(key) => Arc::clone(key),
            value => return Err(type_error(operation, value)),
        };
        object.insert(key, value);
    }
    Ok(Value::object(object))
}

fn explode(input: &Value) -> Result<Value, VmError> {
    let Value::String(input) = input else {
        return Err(type_error("explode", input));
    };
    input
        .chars()
        .map(|character| number_usize(character as usize))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::array)
}

fn implode(input: &Value) -> Result<Value, VmError> {
    let Value::Array(input) = input else {
        return Err(type_error("implode", input));
    };
    let mut output = String::with_capacity(input.len());
    for value in input.iter() {
        let Value::Number(number) = value else {
            return Err(type_error("implode", value));
        };
        let codepoint = number
            .exact_index()
            .and_then(|value| u32::try_from(value).ok())
            .and_then(char::from_u32)
            .unwrap_or(char::REPLACEMENT_CHARACTER);
        output.push(codepoint);
    }
    Ok(Value::string(output))
}

fn math_argument(operation: &str, value: &Value) -> Result<f64, VmError> {
    match value {
        Value::Number(number) => Ok(number.as_f64()),
        value => Err(type_error(operation, value)),
    }
}

fn runtime_number(value: f64) -> Value {
    Value::Number(Number::from_runtime_f64(value))
}

fn math_result_value(result: math::MathResult) -> Value {
    match result {
        math::MathResult::Scalar(value) => runtime_number(value),
        math::MathResult::Pair(values) => {
            Value::array(values.into_iter().map(runtime_number).collect::<Vec<_>>())
        }
    }
}

fn regex_pattern_argument(
    name: &str,
    value: &Value,
    array_allowed: bool,
) -> Result<(Arc<str>, Option<Arc<str>>), VmError> {
    match value {
        Value::String(pattern) => Ok((Arc::clone(pattern), None)),
        Value::Array(values) if array_allowed => {
            let Some(Value::String(pattern)) = values.first() else {
                return Err(type_error(name, value));
            };
            let flags = match values.get(1) {
                None | Some(Value::Null) => Arc::from(""),
                Some(Value::String(flags)) => Arc::clone(flags),
                Some(value) => return Err(type_error(name, value)),
            };
            Ok((Arc::clone(pattern), Some(flags)))
        }
        _ => Err(type_error(name, value)),
    }
}

fn math_error(name: &str, error: &math::MathError) -> VmError {
    let message = error.to_string();
    match error {
        math::MathError::UnknownFunction => VmError::Unsupported {
            operation: format!("math builtin {name}").into(),
        },
        math::MathError::InvalidOrder { .. } => resource("math-bessel-order"),
        math::MathError::Arity { .. } => runtime(message),
    }
}

fn numeric_predicate(name: &str, input: &Value) -> Result<Value, VmError> {
    let Value::Number(number) = input else {
        return Ok(Value::Bool(false));
    };
    let value = number.as_f64();
    let result = match name {
        "isnan" => value.is_nan(),
        "isinfinite" => value.is_infinite(),
        // jq's decNumber predicate treats NaN as finite; only infinities
        // are non-finite in the public filter contract.
        "isfinite" => !value.is_infinite(),
        "isnormal" => value.is_normal(),
        _ => {
            return Err(VmError::Unsupported {
                operation: format!("numeric predicate {name}").into(),
            });
        }
    };
    Ok(Value::Bool(result))
}

fn fromjson(
    input: &Value,
    limits: VmLimits,
    checkpoint: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let Value::String(input) = input else {
        return Err(type_error("fromjson", input));
    };
    if input.len() > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    let mut reader = JsonInput::new(
        io::Cursor::new(input.as_bytes()),
        JsonInputOptions {
            maximum_depth: limits.json_depth,
            maximum_token_bytes: limits.json_token_bytes,
        },
    );
    let mut checkpoint_error = None;
    let mut io_checkpoint = || match checkpoint() {
        Ok(()) => Ok(()),
        Err(error) => {
            checkpoint_error = Some(error);
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "evaluator checkpoint failed",
            ))
        }
    };
    let parsed = reader
        .next_value(&mut io_checkpoint)
        .and_then(|value| {
            value.ok_or_else(|| JsonInputError::Syntax {
                position: reader.position(),
                message: "expected JSON value".into(),
            })
        })
        .and_then(|value| reader.check_end(&mut io_checkpoint).map(|()| value));
    if let Some(error) = checkpoint_error {
        return Err(error);
    }
    let value = parsed.map_err(|error| match error {
        JsonInputError::Limit { limit, .. } => match limit {
            crate::JsonLimit::Depth => resource("depth"),
            crate::JsonLimit::TokenBytes => resource("token-bytes"),
        },
        error => runtime(format!("invalid JSON: {error}")),
    })?;
    Ok(value)
}

fn fold_values(
    input: &Value,
    initial: Option<Value>,
    apply: fn(&Value, &Value) -> Result<Value, VmError>,
) -> Outcomes {
    let values = match input {
        Value::Array(values) => values.iter().collect::<Vec<_>>(),
        Value::Object(values) => values.values().collect::<Vec<_>>(),
        value => return one_error(type_error("add", value)),
    };
    let mut iterator = values.into_iter();
    let mut result = if let Some(initial) = initial {
        initial
    } else if let Some(first) = iterator.next() {
        first.clone()
    } else {
        return vec![Ok(Value::Null)];
    };
    for value in iterator {
        match apply(&result, value) {
            Ok(next) => result = next,
            Err(error) => return one_error(error),
        }
    }
    vec![Ok(result)]
}

fn extrema(input: &Value, maximum: bool) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error(if maximum { "max" } else { "min" }, input));
    };
    let selected = if values.len() >= PARALLEL_REDUCTION_THRESHOLD {
        if maximum {
            values
                .par_iter()
                .enumerate()
                .max_by(|left, right| {
                    jq_compare(left.1, right.1).then_with(|| left.0.cmp(&right.0))
                })
                .map(|(_, value)| value)
        } else {
            values
                .par_iter()
                .enumerate()
                .min_by(|left, right| {
                    jq_compare(left.1, right.1).then_with(|| left.0.cmp(&right.0))
                })
                .map(|(_, value)| value)
        }
    } else if maximum {
        values.iter().max()
    } else {
        values.iter().min()
    };
    vec![Ok(selected.cloned().unwrap_or(Value::Null))]
}

fn sort_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("sort", input));
    };
    let mut values = values.to_vec();
    stable_sort_values(&mut values);
    vec![Ok(Value::array(values))]
}

fn unique_values(input: &Value) -> Outcomes {
    let Value::Array(values) = input else {
        return one_error(type_error("unique", input));
    };
    let mut values = values.to_vec();
    stable_sort_values(&mut values);
    values.dedup_by(|left, right| collection::jq_equal(left, right));
    vec![Ok(Value::array(values))]
}

/// Stably orders values using the shared thresholded parallel sort policy.
pub fn stable_sort_values(values: &mut [Value]) {
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort_by(jq_compare);
    } else {
        values.sort_by(jq_compare);
    }
}

/// Number of workers in the shared Rayon execution pool.
#[must_use]
pub fn parallel_worker_count() -> usize {
    rayon::current_num_threads()
}

/// High-water observations from an overlapping stable-sort preparation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StableSortPipelineObservations {
    /// Number of independently sorted input runs.
    pub batches: usize,
    /// Maximum worker batches simultaneously in flight.
    pub in_flight_batches: usize,
    /// Maximum estimated bytes simultaneously in flight.
    pub in_flight_bytes: usize,
}

/// Bounded producer for stable sorted runs backed by the shared Rayon pool.
pub struct StableSortPipeline {
    batch: Vec<Value>,
    batch_bytes: usize,
    batch_values_limit: usize,
    batch_bytes_limit: usize,
    in_flight_batches_limit: usize,
    in_flight_bytes_limit: usize,
    in_flight_batches: usize,
    in_flight_bytes: usize,
    next_ordinal: usize,
    sender: Sender<(usize, usize, Vec<Value>, bool)>,
    receiver: Receiver<(usize, usize, Vec<Value>, bool)>,
    runs: BTreeMap<usize, Vec<Value>>,
    observations: StableSortPipelineObservations,
    cancellation: Option<Arc<AtomicBool>>,
}

impl StableSortPipeline {
    /// Creates a finite run queue. Every limit must be non-zero.
    #[must_use]
    pub fn new(
        batch_values_limit: usize,
        batch_bytes_limit: usize,
        in_flight_batches_limit: usize,
        in_flight_bytes_limit: usize,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            batch: Vec::with_capacity(batch_values_limit.max(1)),
            batch_bytes: 0,
            batch_values_limit: batch_values_limit.max(1),
            batch_bytes_limit: batch_bytes_limit.max(1),
            in_flight_batches_limit: in_flight_batches_limit.max(1),
            in_flight_bytes_limit: in_flight_bytes_limit.max(1),
            in_flight_batches: 0,
            in_flight_bytes: 0,
            next_ordinal: 0,
            sender,
            receiver,
            runs: BTreeMap::new(),
            observations: StableSortPipelineObservations::default(),
            cancellation: None,
        }
    }

    /// Adds cooperative cancellation shared with decoding and evaluation.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: Arc<AtomicBool>) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Adds one producer result, blocking only when the finite worker queue is full.
    ///
    /// # Errors
    ///
    /// Returns a stable resource name when one value exceeds the byte envelope or
    /// a worker disconnects.
    pub fn push(&mut self, value: Value, estimated_bytes: usize) -> Result<(), &'static str> {
        self.check_cancellation()?;
        if estimated_bytes > self.in_flight_bytes_limit {
            return Err("hybrid-in-flight-bytes");
        }
        if !self.batch.is_empty()
            && (self.batch.len() >= self.batch_values_limit
                || self.batch_bytes.saturating_add(estimated_bytes) > self.batch_bytes_limit)
        {
            self.dispatch()?;
        }
        self.batch.push(value);
        self.batch_bytes = self.batch_bytes.saturating_add(estimated_bytes);
        if self.batch.len() >= self.batch_values_limit || self.batch_bytes >= self.batch_bytes_limit
        {
            self.dispatch()?;
        }
        Ok(())
    }

    /// Drains every run and performs deterministic pairwise stable merges.
    ///
    /// # Errors
    ///
    /// Returns a stable resource name if a worker disconnects.
    pub fn finish(mut self) -> Result<(Vec<Value>, StableSortPipelineObservations), &'static str> {
        self.check_cancellation()?;
        self.dispatch()?;
        while self.in_flight_batches != 0 {
            self.receive_one()?;
        }
        let runs = self.runs.into_values().collect::<Vec<_>>();
        Ok((
            merge_sorted_runs(runs, self.cancellation.as_ref())?,
            self.observations,
        ))
    }

    fn dispatch(&mut self) -> Result<(), &'static str> {
        self.check_cancellation()?;
        if self.batch.is_empty() {
            return Ok(());
        }
        let bytes = self.batch_bytes;
        while self.in_flight_batches >= self.in_flight_batches_limit
            || self.in_flight_bytes.saturating_add(bytes) > self.in_flight_bytes_limit
        {
            self.receive_one()?;
        }
        let ordinal = self.next_ordinal;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        let mut values =
            std::mem::replace(&mut self.batch, Vec::with_capacity(self.batch_values_limit));
        self.batch_bytes = 0;
        let sender = self.sender.clone();
        let cancellation = self.cancellation.clone();
        rayon::spawn_fifo(move || {
            if !cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                stable_sort_values(&mut values);
            }
            let cancelled = cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed));
            let _ = sender.send((ordinal, bytes, values, cancelled));
        });
        self.in_flight_batches = self.in_flight_batches.saturating_add(1);
        self.in_flight_bytes = self.in_flight_bytes.saturating_add(bytes);
        self.observations.batches = self.observations.batches.saturating_add(1);
        self.observations.in_flight_batches = self
            .observations
            .in_flight_batches
            .max(self.in_flight_batches);
        self.observations.in_flight_bytes =
            self.observations.in_flight_bytes.max(self.in_flight_bytes);
        Ok(())
    }

    fn receive_one(&mut self) -> Result<(), &'static str> {
        self.check_cancellation()?;
        let (ordinal, bytes, values, worker_cancelled) =
            self.receiver.recv().map_err(|_| "hybrid-sort-worker")?;
        if worker_cancelled {
            return Err("interrupted");
        }
        self.check_cancellation()?;
        self.in_flight_batches = self.in_flight_batches.saturating_sub(1);
        self.in_flight_bytes = self.in_flight_bytes.saturating_sub(bytes);
        self.runs.insert(ordinal, values);
        Ok(())
    }

    fn check_cancellation(&self) -> Result<(), &'static str> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err("interrupted")
        } else {
            Ok(())
        }
    }
}

fn merge_sorted_runs(
    mut runs: Vec<Vec<Value>>,
    cancellation: Option<&Arc<AtomicBool>>,
) -> Result<Vec<Value>, &'static str> {
    while runs.len() > 1 {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("interrupted");
        }
        let mut pairs = Vec::with_capacity(runs.len().div_ceil(2));
        let mut iterator = runs.into_iter();
        while let Some(left) = iterator.next() {
            pairs.push((left, iterator.next()));
        }
        runs = if let Some(cancellation) = cancellation {
            pairs
                .into_par_iter()
                .map(|(left, right)| match right {
                    Some(right) => stable_merge_cancellable(left, right, cancellation),
                    None => Ok(left),
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            pairs
                .into_par_iter()
                .map(|(left, right)| match right {
                    Some(right) => stable_merge(left, right),
                    None => left,
                })
                .collect()
        };
    }
    Ok(runs.pop().unwrap_or_default())
}

fn stable_merge_cancellable(
    left: Vec<Value>,
    right: Vec<Value>,
    cancellation: &AtomicBool,
) -> Result<Vec<Value>, &'static str> {
    let mut merged = Vec::with_capacity(left.len().saturating_add(right.len()));
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    let mut comparisons_until_check = 0usize;
    while let (Some(left_value), Some(right_value)) = (left.peek(), right.peek()) {
        if comparisons_until_check == 0 {
            if cancellation.load(Ordering::Relaxed) {
                return Err("interrupted");
            }
            comparisons_until_check = 16 * 1024;
        }
        comparisons_until_check = comparisons_until_check.saturating_sub(1);
        if jq_compare(left_value, right_value).is_gt() {
            merged.push(right.next().expect("right value was peeked"));
        } else {
            merged.push(left.next().expect("left value was peeked"));
        }
    }
    merged.extend(left);
    merged.extend(right);
    Ok(merged)
}

fn stable_merge(left: Vec<Value>, right: Vec<Value>) -> Vec<Value> {
    let mut merged = Vec::with_capacity(left.len().saturating_add(right.len()));
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    while let (Some(left_value), Some(right_value)) = (left.peek(), right.peek()) {
        if jq_compare(left_value, right_value).is_gt() {
            merged.push(right.next().expect("right value was peeked"));
        } else {
            merged.push(left.next().expect("left value was peeked"));
        }
    }
    merged.extend(left);
    merged.extend(right);
    merged
}

fn sort_by_cached_key(values: &mut [(Value, Value)]) {
    // `sort_by` and Rayon `par_sort_by` are stable, so equal keys retain the
    // order in which jq produced them.
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort_by(|left, right| jq_compare(&left.0, &right.0));
    } else {
        values.sort_by(|left, right| jq_compare(&left.0, &right.0));
    }
}

fn sort_managed_keyed_values(values: &mut [ManagedKeyedValue]) {
    if values.len() >= PARALLEL_SORT_THRESHOLD {
        values.par_sort_by(|left, right| jq_compare(&left.key, &right.key));
    } else {
        values.sort_by(|left, right| jq_compare(&left.key, &right.key));
    }
}

fn ensure_managed_container_size(length: usize, limits: VmLimits) -> Result<(), VmError> {
    let lower_bound = length
        .checked_add(2)
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

fn ensure_managed_group_projection_size(
    groups: usize,
    members: usize,
    limits: VmLimits,
) -> Result<(), VmError> {
    let group_delimiters = groups
        .checked_mul(2)
        .ok_or_else(|| resource("output-bytes"))?;
    let lower_bound = 2_usize
        .checked_add(group_delimiters)
        .and_then(|size| size.checked_add(members))
        .ok_or_else(|| resource("output-bytes"))?;
    if lower_bound > limits.output_bytes {
        return Err(resource("output-bytes"));
    }
    Ok(())
}

fn project_managed_keyed_values(
    mode: KeyedCollectionMode,
    mut keyed_values: Vec<ManagedKeyedValue>,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
    next_origin: &mut u64,
) -> Result<(Option<OriginToken>, Value), VmError> {
    if matches!(
        mode,
        KeyedCollectionMode::Sort | KeyedCollectionMode::Unique | KeyedCollectionMode::Group
    ) {
        sort_managed_keyed_values(&mut keyed_values);
    }
    match mode {
        KeyedCollectionMode::Sort => {
            ensure_managed_container_size(keyed_values.len(), limits)?;
            let mut output = Vec::new();
            output
                .try_reserve_exact(keyed_values.len())
                .map_err(|_| resource("output-bytes"))?;
            for entry in keyed_values {
                charge()?;
                output.push(entry.value);
            }
            Ok((Some(fresh_origin(next_origin)?), Value::array(output)))
        }
        KeyedCollectionMode::Unique => {
            keyed_values.dedup_by(|left, right| collection::jq_equal(&left.key, &right.key));
            ensure_managed_container_size(keyed_values.len(), limits)?;
            let mut output = Vec::new();
            output
                .try_reserve_exact(keyed_values.len())
                .map_err(|_| resource("output-bytes"))?;
            for entry in keyed_values {
                charge()?;
                output.push(entry.value);
            }
            Ok((Some(fresh_origin(next_origin)?), Value::array(output)))
        }
        KeyedCollectionMode::Group => {
            let mut group_count = 0_usize;
            let mut previous: Option<&Value> = None;
            for entry in &keyed_values {
                if previous.is_none_or(|key| !collection::jq_equal(key, &entry.key)) {
                    group_count = group_count
                        .checked_add(1)
                        .ok_or_else(|| resource("output-bytes"))?;
                    previous = Some(&entry.key);
                }
            }
            ensure_managed_group_projection_size(group_count, keyed_values.len(), limits)?;
            let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
            groups
                .try_reserve_exact(group_count)
                .map_err(|_| resource("output-bytes"))?;
            for entry in keyed_values {
                charge()?;
                let starts_new_group = groups
                    .last()
                    .is_none_or(|(key, _)| !collection::jq_equal(key, &entry.key));
                if starts_new_group {
                    groups.push((entry.key, Vec::new()));
                }
                let (_, values) = groups.last_mut().expect("group was created");
                let value_count = values
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| resource("output-bytes"))?;
                ensure_managed_container_size(value_count, limits)?;
                values
                    .try_reserve(1)
                    .map_err(|_| resource("output-bytes"))?;
                values.push(entry.value);
            }
            ensure_managed_container_size(groups.len(), limits)?;
            let mut output = Vec::new();
            output
                .try_reserve_exact(groups.len())
                .map_err(|_| resource("output-bytes"))?;
            for (_, values) in groups {
                charge()?;
                output.push(Value::array(values));
            }
            Ok((Some(fresh_origin(next_origin)?), Value::array(output)))
        }
        KeyedCollectionMode::Min | KeyedCollectionMode::Max => {
            let maximum = matches!(mode, KeyedCollectionMode::Max);
            let selected = keyed_values.into_iter().try_fold(None, |selected, entry| {
                charge()?;
                let replace = selected.as_ref().is_none_or(|current: &ManagedKeyedValue| {
                    let ordering = jq_compare(&entry.key, &current.key);
                    if maximum {
                        ordering.is_gt() || ordering.is_eq()
                    } else {
                        ordering.is_lt()
                    }
                });
                Ok::<_, VmError>(if replace { Some(entry) } else { selected })
            })?;
            match selected {
                Some(entry) => Ok((entry.origin, entry.value)),
                None => Ok((Some(fresh_origin(next_origin)?), Value::Null)),
            }
        }
    }
}

fn reverse(input: &Value) -> Outcomes {
    match input {
        Value::Array(values) => vec![Ok(Value::array(
            values.iter().rev().cloned().collect::<Vec<_>>(),
        ))],
        Value::String(value) => vec![Ok(Value::string(value.chars().rev().collect::<String>()))],
        value => one_error(type_error("reverse", value)),
    }
}

fn flatten(input: &Value, depth: Option<i64>) -> Result<Value, VmError> {
    let Value::Array(values) = input else {
        return Err(type_error("flatten", input));
    };
    if depth.is_some_and(|depth| depth < 0) {
        return Err(runtime("flatten depth cannot be negative".to_owned()));
    }
    let mut output = Vec::new();
    let mut stack = values
        .iter()
        .rev()
        .cloned()
        .map(|value| (value, depth))
        .collect::<Vec<_>>();
    while let Some((value, depth)) = stack.pop() {
        match value {
            Value::Array(values) if depth != Some(0) => {
                let next_depth = depth.map(|depth| depth - 1);
                stack.extend(
                    values
                        .iter()
                        .rev()
                        .cloned()
                        .map(|value| (value, next_depth)),
                );
            }
            value => output.push(value),
        }
    }
    Ok(Value::array(output))
}

fn path_component(value: &Value) -> Result<PathComponent, VmError> {
    match value {
        Value::String(key) => Ok(PathComponent::Key(Arc::clone(key))),
        Value::Number(index) => index
            .exact_index()
            .and_then(|index| usize::try_from(index).ok())
            .map(PathComponent::Index)
            .ok_or_else(|| runtime("path index must be a non-negative exact integer".to_owned())),
        value => Err(type_error("path component", value)),
    }
}

fn path_component_for_target(target: &Value, value: &Value) -> Result<PathComponent, VmError> {
    match value {
        Value::Number(index) if index.as_f64() < 0.0 => {
            let Value::Array(values) = target else {
                return path_component(value);
            };
            let index = index
                .exact_index()
                .and_then(|index| normalize_index(index, values.len()))
                .ok_or_else(|| runtime("path index is outside the target array".to_owned()))?;
            Ok(PathComponent::Index(index))
        }
        _ => path_component(value),
    }
}

fn jq_path(value: &Value) -> Result<Vec<PathComponent>, VmError> {
    let Value::Array(components) = value else {
        return Err(type_error("path", value));
    };
    components.iter().map(path_component).collect()
}

fn stream_event(value: &Value) -> Result<(Vec<PathComponent>, Option<Value>), VmError> {
    let Value::Array(parts) = value else {
        return Err(runtime("stream event must be an array".to_owned()));
    };
    if !(1..=2).contains(&parts.len()) {
        return Err(runtime(
            "stream event must contain a path and optional value".to_owned(),
        ));
    }
    let path = jq_path(&parts[0])?;
    Ok((path, parts.get(1).cloned()))
}

fn emit_stream_root(
    root: &mut Option<Value>,
    emit: &mut dyn FnMut(Result<Value, VmError>) -> bool,
) -> bool {
    root.take().is_none_or(|root| emit(Ok(root)))
}

fn path_value(path: &Path) -> Value {
    Value::array(
        path.components()
            .iter()
            .map(|component| match component {
                PathComponent::Key(key) => Ok(Value::String(Arc::clone(key))),
                PathComponent::Index(index) => number_usize(*index),
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("usize path indices are representable jq numbers"),
    )
}

fn root_child_is_last(input: &Value, path: &[PathComponent]) -> bool {
    let Some(component) = path.first() else {
        return false;
    };
    match (input, component) {
        (Value::Array(values), PathComponent::Index(index)) => {
            index.saturating_add(1) == values.len()
        }
        (Value::Object(values), PathComponent::Key(key)) => {
            values.keys().last().is_some_and(|last| last == key)
        }
        _ => false,
    }
}

fn getpath(input: &Value, path: &[PathComponent]) -> Result<Value, VmError> {
    let mut current = input.clone();
    for component in path {
        current = match component {
            PathComponent::Key(key) => access_index(&current, &Value::String(Arc::clone(key)))?,
            PathComponent::Index(index) => access_index(&current, &number_usize(*index)?)?,
        };
    }
    Ok(current)
}

fn push_children(
    value: &Value,
    prefix: &[PathComponent],
    pending: &mut Vec<(Value, Vec<PathComponent>)>,
) {
    match value {
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate().rev() {
                let mut path = prefix.to_vec();
                path.push(PathComponent::Index(index));
                pending.push((child.clone(), path));
            }
        }
        Value::Object(values) => {
            for (key, child) in values.iter().rev() {
                let mut path = prefix.to_vec();
                path.push(PathComponent::Key(Arc::clone(key)));
                pending.push((child.clone(), path));
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn tostream(input: &Value, limit: usize) -> Result<Vec<Value>, VmError> {
    let mut output = Vec::new();
    tostream_at(input, Vec::new(), true, limit, &mut output)?;
    Ok(output)
}

fn tostream_at(
    value: &Value,
    path: Vec<PathComponent>,
    root: bool,
    limit: usize,
    output: &mut Vec<Value>,
) -> Result<(), VmError> {
    if path.len() > limit {
        return Err(resource("path-stack"));
    }
    let empty_container = matches!(value, Value::Array(values) if values.is_empty())
        || matches!(value, Value::Object(values) if values.is_empty());
    match value {
        Value::Array(values) if !values.is_empty() => {
            for (index, child) in values.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(PathComponent::Index(index));
                tostream_at(child, child_path, false, limit, output)?;
            }
        }
        Value::Object(values) if !values.is_empty() => {
            for (key, child) in values.iter() {
                let mut child_path = path.clone();
                child_path.push(PathComponent::Key(Arc::clone(key)));
                tostream_at(child, child_path, false, limit, output)?;
            }
        }
        _ => output.push(Value::array(vec![
            path_value(&Path::new(path.clone())),
            value.clone(),
        ])),
    }
    if !root {
        output.push(Value::array(vec![path_value(&Path::new(path))]));
    } else if root && empty_container {
        // Empty roots were already emitted as scalar-like stream records.
    }
    Ok(())
}

fn update_value(
    operator: AssignmentOperator,
    old: &Value,
    replacement: &Value,
) -> Result<Value, VmError> {
    match operator {
        AssignmentOperator::Set | AssignmentOperator::Update => Ok(replacement.clone()),
        AssignmentOperator::Add => binary_add(old, replacement),
        AssignmentOperator::Subtract => binary_subtract(old, replacement),
        AssignmentOperator::Multiply => binary_multiply(old, replacement),
        AssignmentOperator::Divide => numeric(old, replacement, Number::divide, "divide"),
        AssignmentOperator::Alternative => Ok(if old.is_truthy() {
            old.clone()
        } else {
            replacement.clone()
        }),
    }
}

fn apply_plain_assignment(
    state: &AssignmentState,
    replacement: &Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    let mut document = state.input.clone();
    for target in &state.targets {
        let old = assignment_target_value(&state.input, target, limits, charge)?;
        let replacement = update_value(state.operator, &old, replacement)?;
        document = apply_assignment_target(&document, target, replacement, limits, charge)?;
    }
    Ok(document)
}

fn assignment_target_value(
    root: &Value,
    target: &AssignmentTarget,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match target {
        AssignmentTarget::Path(path) => {
            path::getpath_bounded(root, path.components(), limits, charge)
        }
        AssignmentTarget::Slice { path, start, end } => {
            match path::getpath_bounded(root, path.components(), limits, charge)? {
                Value::Array(values) => {
                    let Some(values) = values.get(*start..*end) else {
                        return Ok(Value::Null);
                    };
                    charge()?;
                    ensure_managed_container_size(values.len(), limits)?;
                    let mut selected = Vec::new();
                    selected
                        .try_reserve_exact(values.len())
                        .map_err(|_| resource("output-bytes"))?;
                    for value in values {
                        charge()?;
                        selected.push(value.clone());
                    }
                    Ok(Value::array(selected))
                }
                _ => Ok(Value::Null),
            }
        }
    }
}

fn apply_assignment_target(
    root: &Value,
    target: &AssignmentTarget,
    replacement: Value,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<Value, VmError> {
    match target {
        AssignmentTarget::Path(path) => {
            path::replace_or_create_bounded(root, path.components(), replacement, limits, charge)
        }
        AssignmentTarget::Slice { path, start, end } => path::replace_slice_bounded(
            root,
            path.components(),
            *start,
            *end,
            replacement,
            limits,
            charge,
        ),
    }
}

fn retain_assignment_deletion_target(
    accumulator: &mut path_builtin::PathAccumulator,
    target: &AssignmentTarget,
    limits: VmLimits,
    charge: &mut impl FnMut() -> Result<(), VmError>,
) -> Result<(), VmError> {
    match target {
        AssignmentTarget::Path(path) => accumulator.push(path.components(), limits, charge),
        AssignmentTarget::Slice { path, start, end } => {
            let total = path
                .components()
                .len()
                .checked_add(1)
                .ok_or_else(|| resource("path-stack"))?;
            if total > limits.path_stack {
                return Err(resource("path-stack"));
            }
            for index in *start..*end {
                let mut components = Vec::new();
                components
                    .try_reserve_exact(total)
                    .map_err(|_| resource("path-stack"))?;
                for component in path.components() {
                    charge()?;
                    components.push(component.clone());
                }
                charge()?;
                components.push(PathComponent::Index(index));
                accumulator.push(&components, limits, charge)?;
            }
            Ok(())
        }
    }
}

fn compare_paths_for_deletion(left: &Path, right: &Path) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    for (left, right) in left.components().iter().zip(right.components()) {
        let ordering = match (left, right) {
            (PathComponent::Key(left), PathComponent::Key(right)) => left.cmp(right),
            (PathComponent::Index(left), PathComponent::Index(right)) => left.cmp(right),
            (PathComponent::Key(_), PathComponent::Index(_)) => Ordering::Less,
            (PathComponent::Index(_), PathComponent::Key(_)) => Ordering::Greater,
        };
        if ordering != Ordering::Equal {
            return ordering.reverse();
        }
    }
    right.components().len().cmp(&left.components().len())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use indexmap::IndexMap;

    use super::{
        PARALLEL_REDUCTION_THRESHOLD, PARALLEL_SORT_THRESHOLD, StableSortPipeline, Value, extrema,
        sort_by_cached_key, stable_sort_values,
    };
    use crate::{InputCursor, ResolveOptions, Vm, VmLimits, analyze, parse, resolve};

    fn run(query: &str, input: &str) -> Vec<Result<Value, String>> {
        run_with_variables(query, input, BTreeMap::new())
    }

    fn run_with_variables(
        query: &str,
        input: &str,
        variables: BTreeMap<Arc<str>, Value>,
    ) -> Vec<Result<Value, String>> {
        let query = parse(query).unwrap();
        let query = resolve(
            query,
            &ResolveOptions {
                variables: variables.keys().cloned().collect::<BTreeSet<_>>(),
                ..ResolveOptions::default()
            },
        )
        .unwrap();
        let plan = analyze(query).compile().unwrap().document_plan();
        let input: Value = serde_json::from_str(input).unwrap();
        let mut vm = Vm::new_with_variables(&plan, input, VmLimits::default(), variables);
        let mut values = Vec::new();
        loop {
            match vm.next_result() {
                Ok(Some(value)) => values.push(Ok(value)),
                Ok(None) => break,
                Err(error) => {
                    values.push(Err(error.to_string()));
                    break;
                }
            }
        }
        values
    }

    fn first_error_with_limits(query: &str, input: &str, limits: VmLimits) -> crate::VmError {
        let plan = analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let input: Value = serde_json::from_str(input).unwrap();
        Vm::new(&plan, input, limits)
            .next_result()
            .expect_err("query should fail before producing a result")
    }

    fn json(values: Vec<Result<Value, String>>) -> Vec<String> {
        values
            .into_iter()
            .map(|value| {
                value.map_or_else(|error| format!("error:{error}"), |value| value.to_string())
            })
            .collect()
    }

    #[test]
    fn lexical_labels_prune_owned_work_and_preserve_outer_work() {
        assert_eq!(
            json(run("label $out | 1, break $out, 2", "null")),
            vec!["1"]
        );
        assert_eq!(
            json(run(
                "label $x | (1, (label $x | 2, break $x, 3), 4)",
                "null"
            )),
            vec!["1", "2", "4"]
        );
        assert_eq!(
            json(run("label $a | (label $b | 1, break $a, 2), 3", "null")),
            vec!["1"]
        );
    }

    #[test]
    fn label_break_observes_try_function_and_fold_boundaries() {
        assert_eq!(
            json(run(
                "label $out | try (1, break $out, 2) catch \"caught\", 3",
                "null"
            )),
            vec!["1", r#""caught""#, "3"]
        );
        assert_eq!(
            json(run(
                "try (label $out | 1, break $out, 2) catch \"caught\"",
                "null"
            )),
            vec!["1"]
        );
        assert_eq!(
            json(run("label $x | try break $x catch .", "null")),
            vec![r#"{"__jq":0}"#]
        );
        assert_eq!(
            json(run("label $x | def f: 1, break $x, 2; f, 3", "null")),
            vec!["1"]
        );
        assert_eq!(
            json(run(
                "label $out | foreach .[] as $item (null; $item; if . == false then break $out else . end)",
                "[1,2,false,3,null]"
            )),
            vec!["1", "2"]
        );
    }

    #[test]
    fn recursive_builtins_match_jq_order_and_cardinality() {
        assert_eq!(
            json(run("[recurse(.[]?)]", r#"{"a":[1]}"#)),
            vec![r#"[{"a":[1]},[1],1]"#]
        );
        assert_eq!(
            json(run("recurse", r#"{"a":[1]}"#)),
            vec![r#"{"a":[1]}"#, "[1]", "1"]
        );
        assert_eq!(json(run("recurse(.+1; . < 4)", "2")), vec!["2", "3"]);
        assert_eq!(
            json(run("recurse(if . < 2 then .+1, .+10 else empty end)", "0")),
            vec!["0", "1", "2", "11", "10"]
        );
    }

    #[test]
    fn walk_is_post_order_and_preserves_jq_callback_cardinality() {
        assert_eq!(
            json(run(
                "walk(if type == \"number\" then . + 1 else . end)",
                r#"{"a":[1,2]}"#
            )),
            vec![r#"{"a":[2,3]}"#]
        );
        assert_eq!(json(run("walk(., .)", "[1]")), vec!["[1,1]", "[1,1]"]);
        assert_eq!(
            json(run(
                "walk(if type == \"number\" then ., .+10 else . end)",
                r#"{"a":1,"b":2}"#
            )),
            vec![r#"{"a":1,"b":2}"#]
        );
    }

    #[test]
    fn recursive_control_flow_stops_and_obeys_limits_and_cancellation() {
        assert_eq!(
            json(run("limit(1; recurse(error(\"late\")))", "null")),
            vec!["null"]
        );
        assert_eq!(
            json(run(
                "limit(1; label $out | 1, error(\"late\"), break $out)",
                "null"
            )),
            vec!["1"]
        );

        for (limits, resource_name) in [
            (
                VmLimits {
                    steps: 1,
                    ..VmLimits::default()
                },
                "vm-steps",
            ),
            (
                VmLimits {
                    call_stack: 1,
                    ..VmLimits::default()
                },
                "call-stack",
            ),
        ] {
            let error = first_error_with_limits("[label $out | 1, 2]", "null", limits);
            assert_eq!(
                error,
                crate::VmError::Resource {
                    resource: resource_name
                }
            );
        }

        for query in ["[recurse(.[]?)]", "walk(.)"] {
            let error = first_error_with_limits(
                query,
                "[[[0]]]",
                VmLimits {
                    path_stack: 2,
                    ..VmLimits::default()
                },
            );
            assert_eq!(
                error,
                crate::VmError::Resource {
                    resource: "path-stack"
                }
            );
        }

        let plan = analyze(
            resolve(
                parse("label $out | recurse(.[]?)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let cancellation = Arc::new(AtomicBool::new(true));
        let mut vm = Vm::new(
            &plan,
            serde_json::from_str("[[[0]]]").unwrap(),
            VmLimits::default(),
        )
        .with_cancellation(cancellation);
        assert_eq!(vm.next_result().unwrap_err(), crate::VmError::Interrupted);
    }

    #[test]
    fn default_recurse_pulls_wide_children_on_demand() {
        let plan = analyze(
            resolve(
                parse("limit(2; recurse)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let source = format!(
            "[{}]",
            (0..128)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let input: Value = serde_json::from_str(&source).unwrap();
        let mut vm = Vm::new(
            &plan,
            input.clone(),
            VmLimits {
                fork_stack: 8,
                ..VmLimits::default()
            },
        );

        assert_eq!(vm.next_result().unwrap(), Some(input));
        assert_eq!(
            vm.next_result().unwrap(),
            Some(serde_json::from_str("0").unwrap())
        );
        assert_eq!(vm.next_result().unwrap(), None);
    }

    #[test]
    fn regex_builtins_match_jq_unicode_captures_scans_splits_and_substitution() {
        assert_eq!(
            json(run(r#"match("a"; "g")"#, r#""éaéa""#)),
            vec![
                r#"{"offset":1,"length":1,"string":"a","captures":[]}"#,
                r#"{"offset":3,"length":1,"string":"a","captures":[]}"#,
            ]
        );
        assert_eq!(
            json(run(r#"capture("(?<x>a)(?<y>z)?")"#, r#""a""#)),
            vec![r#"{"x":"a","y":null}"#]
        );
        assert_eq!(
            json(run(r#"scan("([a-z]+)([0-9]+)")"#, r#""ab12cd34""#)),
            vec![r#"["ab","12"]"#, r#"["cd","34"]"#]
        );
        assert_eq!(
            json(run(
                r#"sub("(?<x>[a-z]+)(?<n>[0-9]+)"; "\(.n)-\(.x)")"#,
                r#""abc123""#,
            )),
            vec![r#""123-abc""#]
        );
        assert_eq!(json(run(r#"test("(?=a)")"#, r#""a""#)), vec!["true"]);
    }

    #[test]
    fn regex_pattern_input_and_compiled_program_limits_are_resources() {
        for (query, limits, expected) in [
            (
                r#"test("ab")"#,
                VmLimits {
                    regex_pattern_bytes: 1,
                    ..VmLimits::default()
                },
                "regex-pattern-bytes",
            ),
            (
                r#"test("ab")"#,
                VmLimits {
                    regex_input_bytes: 1,
                    ..VmLimits::default()
                },
                "regex-input-bytes",
            ),
            (
                r#"test("(?:[A-Za-z0-9_]{1,100}){100}")"#,
                VmLimits {
                    regex_compiled_bytes: 1,
                    ..VmLimits::default()
                },
                "regex-compiled-bytes",
            ),
        ] {
            let error = first_error_with_limits(query, r#""ab""#, limits);
            assert_eq!(error, crate::VmError::Resource { resource: expected });
        }
    }

    #[test]
    fn utc_date_builtins_round_trip_with_stable_arrays() {
        assert_eq!(
            json(run("fromdateiso8601", r#""2015-03-05T23:51:47Z""#)),
            vec!["1425599507"]
        );
        assert_eq!(
            json(run("gmtime", "1425599507")),
            vec!["[2015,2,5,23,51,47,4,63]"]
        );
        assert_eq!(
            json(run(
                r#"gmtime | strftime("%Y-%m-%dT%H:%M:%SZ")"#,
                "1425599507",
            )),
            vec![r#""2015-03-05T23:51:47Z""#]
        );
        assert!(matches!(
            first_error_with_limits("todateiso8601", "253402300800", VmLimits::default()),
            crate::VmError::NumericRange { .. }
        ));
    }

    #[test]
    fn regex_date_platform_release_host_contract_covers_utc_boundaries() {
        for timestamp in ["0000-01-01T00:00:00Z", "9999-12-30T22:00:00Z"] {
            let input = serde_json::to_string(timestamp).unwrap();
            assert_eq!(
                json(run("fromdateiso8601 | todateiso8601", &input)),
                [input]
            );
        }
    }

    #[test]
    fn ambient_builtins_require_reserved_policy_admission_without_echoing_values() {
        assert!(json(run("env", "null"))[0].contains("capability policy"));
        let mut variables = BTreeMap::new();
        variables.insert(
            Arc::from(super::AMBIENT_ENVIRONMENT),
            Value::object(IndexMap::from([(
                Arc::from("SECRET"),
                Value::string("redacted"),
            )])),
        );
        variables.insert(Arc::from(super::AMBIENT_PLATFORM), Value::Bool(true));
        variables.insert(
            Arc::from(super::INPUT_FILENAME),
            Value::string("fixture.json"),
        );
        assert_eq!(
            json(run_with_variables("env | type", "null", variables.clone())),
            vec![r#""object""#]
        );
        assert_eq!(
            json(run_with_variables("input_filename", "null", variables)),
            vec![r#""fixture.json""#]
        );
    }

    #[test]
    fn wave_one_navigation_and_generators_compose() {
        assert_eq!(json(run(".a.b", r#"{"a":{"b":2}}"#)), ["2"]);
        assert_eq!(json(run(".[-1]", "[1,2,3]")), ["3"]);
        assert_eq!(json(run(".[1:3]", "[0,1,2,3]")), ["[1,2]"]);
        assert_eq!(json(run(".[] | .x", r#"[{"x":1},{"x":2}]"#)), ["1", "2"]);
        assert_eq!(json(run(".a, .missing?", r#"{"a":1}"#)), ["1", "null"]);
        assert_eq!(run("empty", "null"), Vec::<Result<Value, String>>::new());
    }

    #[test]
    fn wave_two_construction_control_and_operators_compose() {
        assert_eq!(json(run("[.[] | . * 2]", "[1,2,3]")), ["[2,4,6]"]);
        assert_eq!(
            json(run(
                "{name, ok: (.age >= 18)}",
                r#"{"name":"Ada","age":30}"#
            )),
            [r#"{"name":"Ada","ok":true}"#]
        );
        assert_eq!(
            json(run("if . then \"yes\" else \"no\" end", "0")),
            [r#""yes""#]
        );
        assert_eq!(json(run("null // false // 7", "null")), ["7"]);
        assert_eq!(json(run(r#""a" + "b""#, "null")), [r#""ab""#]);
        assert_eq!(
            json(run(r#"{"a":1}+{"a":2,"b":3}"#, "null")),
            [r#"{"a":2,"b":3}"#]
        );
    }

    #[test]
    fn object_multiplication_recursively_merges_and_preserves_key_order() {
        assert_eq!(
            json(run(
                r#"{"a":{"b":1},"replace":{"left":true},"keep":0} * {"a":{"c":2},"replace":4,"new":3}"#,
                "null"
            )),
            [r#"{"a":{"b":1,"c":2},"replace":4,"keep":0,"new":3}"#]
        );
    }

    #[test]
    fn object_multiplication_uses_the_right_value_for_non_object_conflicts() {
        assert_eq!(
            json(run(
                r#"[{"a":1} * {"a":{"nested":true}}, {"a":{"nested":true}} * {"a":1}, 6 * 7]"#,
                "null"
            )),
            [r#"[{"a":{"nested":true}},{"a":1},42]"#]
        );
    }

    #[test]
    fn multiplication_rejects_mixed_object_and_number_operands() {
        assert_eq!(
            json(run(r#"{"a":1} * 2"#, "null")),
            ["error:runtime error: cannot multiply object and number"]
        );
    }

    #[test]
    fn multiplication_update_recursively_merges_objects() {
        assert_eq!(
            json(run(
                r#".settings *= {"display":{"theme":"dark"}}"#,
                r#"{"settings":{"display":{"density":"compact"},"cache":true}}"#
            )),
            [r#"{"settings":{"display":{"density":"compact","theme":"dark"},"cache":true}}"#]
        );
    }

    #[test]
    fn wave_three_variables_builtins_errors_and_updates_compose() {
        let variables = BTreeMap::from([(
            Arc::from("n"),
            Value::from_json(serde_json::json!(3)).unwrap(),
        )]);
        assert_eq!(json(run_with_variables("$n + 1", "null", variables)), ["4"]);
        assert_eq!(
            json(run(
                "map(.x) | sort | unique",
                r#"[{"x":2},{"x":1},{"x":2}]"#
            )),
            ["[1,2]"]
        );
        assert_eq!(json(run("type, length", r"[1,2]")), [r#""array""#, "2"]);
        assert_eq!(json(run("tostring | length", "1e4096")), ["7"]);
        assert_eq!(json(run(r#""\(.)""#, "1e4096")), [r#""1E+4096""#]);
        assert_eq!(
            json(run("try error(\"boom\") catch .", "null")),
            [r#""boom""#]
        );
        let error_after = json(run("1, error(\"later\")", "null"));
        assert_eq!(error_after[0], "1");
        assert!(error_after[1].starts_with("error:runtime error: later"));
        assert_eq!(json(run(".a.b = 2", "{}")), [r#"{"a":{"b":2}}"#]);
        assert_eq!(
            json(run("(.a, .b) += 10", r#"{"a":1,"b":2,"c":[3]}"#)),
            [r#"{"a":11,"b":12,"c":[3]}"#]
        );
    }

    #[test]
    fn collection_and_scalar_utility_builtins_match_jq() {
        assert_eq!(
            json(run("to_entries", r#"{"z":1,"a":2}"#)),
            [r#"[{"key":"z","value":1},{"key":"a","value":2}]"#]
        );
        assert_eq!(
            json(run("with_entries(.value += 1)", r#"{"a":1,"b":2}"#)),
            [r#"{"a":2,"b":3}"#]
        );
        assert_eq!(
            json(run(
                "group_by(.k)",
                r#"[{"k":2,"v":"a"},{"k":1},{"k":2,"v":"b"}]"#
            )),
            [r#"[[{"k":1}],[{"k":2,"v":"a"},{"k":2,"v":"b"}]]"#]
        );
        assert_eq!(
            json(run(
                "[min_by(.k), max_by(.k)]",
                r#"[{"k":2},{"k":1},{"k":3}]"#
            )),
            [r#"[{"k":1},{"k":3}]"#]
        );
        assert_eq!(json(run("[limit(2; range(0; 5))]", "null")), ["[0,1]"]);
        assert_eq!(
            json(run("[limit((1,2); range(0; 4))]", "null")),
            ["[0,0,1]"]
        );
        assert_eq!(json(run("[limit(empty; error(\"late\"))]", "null")), ["[]"]);
        assert_eq!(
            json(run("[any(.[]; . > 2), all(.[]; . < 4)]", "[1,2,3]")),
            ["[true,true]"]
        );
        assert_eq!(
            json(run(
                r#"[ltrimstr("pre"), ascii_downcase, explode, (explode | implode)]"#,
                r#""preABCé""#,
            )),
            [r#"["ABCé","preabcé",[112,114,101,65,66,67,233],"preABCé"]"#]
        );
        assert_eq!(json(run("[floor, ceil, fabs]", "-1.5")), ["[-2,-1,1.5]"]);
    }

    #[test]
    fn path_and_stream_builtins_match_jq_order_and_creation() {
        assert_eq!(
            json(run("[paths]", r#"{"a":[10],"z":{}}"#)),
            [r#"[["a"],["a",0],["z"]]"#]
        );
        assert_eq!(
            json(run("path(.a[0]), path(.missing)", r#"{"a":[10]}"#)),
            [r#"["a",0]"#, r#"["missing"]"#]
        );
        assert_eq!(
            json(run(
                r#"[getpath(["a",0]), getpath(["x"]), setpath(["b",1]; 7)]"#,
                r#"{"a":[10]}"#,
            )),
            [r#"[10,null,{"a":[10],"b":[null,7]}]"#]
        );
        assert_eq!(
            json(run(r#"setpath((["a"],["b"]); (1,2))"#, "{}")),
            [r#"{"a":1}"#, r#"{"b":1}"#, r#"{"a":2}"#, r#"{"b":2}"#]
        );
        assert_eq!(
            json(run("tostream", r#"{"a":[1],"z":{}}"#)),
            [
                r#"[["a",0],1]"#,
                r#"[["a",0]]"#,
                r#"[["z"],{}]"#,
                r#"[["z"]]"#,
            ]
        );
        assert_eq!(
            json(run(
                r#"truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]])"#,
                "1",
            )),
            ["[[0],\"b\"]", "[[0]]"]
        );
        assert_eq!(
            json(run(
                r#"fromstream(1|truncate_stream([[0],"a"],[[1,0],"b"],[[1,0]],[[1]]))"#,
                "null",
            )),
            [r#"["b"]"#]
        );
        assert_eq!(
            json(run("fromstream((1, [], {}, [1,[2]]) | tostream)", "null",)),
            ["1", "[]", "{}", "[1,[2]]"]
        );
        assert_eq!(
            json(run("fromstream(([1],[2]) | tostream)", "null")),
            ["[1]", "[2]"]
        );
        assert_eq!(
            json(run("fromstream(([[0],1],[[0]],[[1],2],[[1]]))", "null",)),
            ["[1]", "[null,2]"]
        );
        assert!(json(run("fromstream(([[0,0],1],[[0,0]]))", "null")).is_empty());
        let prior_then_error = json(run("fromstream((([[],1], error(\"later\"))))", "null"));
        assert_eq!(prior_then_error[0], "1");
        assert!(prior_then_error[1].contains("later"));
        assert_eq!(
            json(run(
                "first(fromstream((([1] | tostream), error(\"later\"))))",
                "null",
            )),
            ["[1]"]
        );
        assert_eq!(
            json(run(
                ". as $dot|fromstream($dot|tostream)|.==$dot",
                "[0,[1,{\"a\":1},{\"b\":2}]]",
            )),
            ["true"]
        );
        assert!(run("fromstream([[0],1])", "null").is_empty());
    }

    #[test]
    fn keyed_sort_uses_every_value_from_its_filter() {
        assert_eq!(
            json(run(
                "sort_by(.a,.b)",
                r#"[{"a":1,"b":2},{"a":1,"b":1},{"a":0,"b":9}]"#,
            )),
            [r#"[{"a":0,"b":9},{"a":1,"b":1},{"a":1,"b":2}]"#]
        );
        assert_eq!(
            json(run(
                "unique_by(.a,.b)",
                r#"[{"a":1,"b":2,"id":"first"},{"a":1,"b":1},{"a":1,"b":2,"id":"last"}]"#,
            )),
            [r#"[{"a":1,"b":1},{"a":1,"b":2,"id":"first"}]"#]
        );
        assert_eq!(json(run("sort_by(empty)", "[3,1,2]")), ["[3,1,2]"]);
        assert_eq!(json(run("sort_by(.)", "[3,1,2]")), ["[1,2,3]"]);
        assert!(
            json(run(r#"sort_by(.a, error("bad"))"#, r#"[{"a":1}]"#,))[0]
                .starts_with("error:runtime error: bad")
        );
    }

    #[test]
    fn group_by_preserves_equal_key_order() {
        assert_eq!(
            json(run(
                "group_by(.k)",
                r#"[{"k":1,"v":"first"},{"k":1,"v":"second"},{"k":1,"v":"third"}]"#
            )),
            [r#"[[{"k":1,"v":"first"},{"k":1,"v":"second"},{"k":1,"v":"third"}]]"#]
        );
    }

    #[test]
    fn json_text_conversion_matches_jq() {
        assert_eq!(
            json(run("tojson", r#"{"z":9007199254740993,"a":"é"}"#)),
            [r#""{\"z\":9007199254740993,\"a\":\"é\"}""#]
        );
        assert_eq!(
            json(run("fromjson", r#""{\"z\":9007199254740993,\"a\":\"é\"}""#)),
            [r#"{"z":9007199254740993,"a":"é"}"#]
        );
        assert_eq!(
            json(run("fromjson", r#""{\"b\":1,\"a\":2,\"b\":3}""#)),
            [r#"{"b":3,"a":2}"#]
        );
        assert_eq!(json(run("fromjson", r#""NaN""#)), ["null"]);
        assert_eq!(
            json(run("fromjson", r#""Infinity""#)),
            ["1.7976931348623157e+308"]
        );
        assert_eq!(json(run("fromjson", r#""+01.200""#)), ["1.200"]);
        assert_eq!(
            json(run(
                r#"["NaN","Infinity","sNaN","+01.200","1."] | map(tonumber)"#,
                "null",
            )),
            ["[null,1.7976931348623157e+308,null,1.200,1]"]
        );
        assert_eq!(
            json(run(
                r#"[" 1 ","[1]","null","1 2","0x10"] | map(try tonumber catch "bad")"#,
                "null",
            )),
            [r#"["bad","bad","bad","bad","bad"]"#]
        );
        assert!(json(run("fromjson", r#""not json""#))[0].starts_with("error:runtime error:"));
    }

    #[test]
    fn fromjson_applies_json_depth_and_token_limits() {
        let nested = r#""[[1]]""#;
        let error = first_error_with_limits(
            "fromjson",
            nested,
            VmLimits {
                json_depth: 1,
                ..VmLimits::default()
            },
        );
        assert_eq!(error, crate::VmError::Resource { resource: "depth" });

        let error = first_error_with_limits(
            "fromjson",
            r#""12345""#,
            VmLimits {
                json_token_bytes: 4,
                ..VmLimits::default()
            },
        );
        assert_eq!(
            error,
            crate::VmError::Resource {
                resource: "token-bytes"
            }
        );
    }

    #[test]
    fn with_entries_accepts_jq_key_aliases() {
        assert_eq!(
            json(run(
                "with_entries(if .key == \"a\" then {Key: \"x\", Value: .value} else {name: \"y\", value: .value} end)",
                r#"{"a":1,"b":2}"#,
            )),
            [r#"{"x":1,"y":2}"#]
        );
    }

    #[test]
    fn inputs_pull_from_one_shared_cursor() {
        let plan = analyze(
            resolve(
                parse("[., limit(1; inputs)]").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let cursor = InputCursor::new(vec![
            serde_json::from_str("2").unwrap(),
            serde_json::from_str("3").unwrap(),
        ]);
        let mut first = Vm::new(
            &plan,
            serde_json::from_str("1").unwrap(),
            VmLimits::default(),
        )
        .with_input_cursor(cursor.clone());
        assert_eq!(first.next_result().unwrap().unwrap().to_string(), "[1,2]");
        assert_eq!(cursor.next_value().unwrap().unwrap().to_string(), "3");
        assert!(cursor.next_value().unwrap().is_none());
    }

    #[test]
    fn parallel_collections_preserve_order_and_extrema_ties() {
        let count = PARALLEL_SORT_THRESHOLD + 1;
        let original = (0..count)
            .map(|index| Value::string(format!("{index:05}")))
            .collect::<Vec<_>>();

        let mut values = original.iter().rev().cloned().collect::<Vec<_>>();
        stable_sort_values(&mut values);
        assert_eq!(values, original);

        let mut keyed = original
            .iter()
            .cloned()
            .map(|value| (Value::Null, value))
            .collect::<Vec<_>>();
        sort_by_cached_key(&mut keyed);
        assert!(keyed.iter().map(|(_, value)| value).eq(original.iter()));

        let first_equal = Value::object(IndexMap::from([
            (Arc::from("a"), Value::Null),
            (Arc::from("b"), Value::Null),
        ]));
        let last_equal = Value::object(IndexMap::from([
            (Arc::from("b"), Value::Null),
            (Arc::from("a"), Value::Null),
        ]));
        let mut equal_values = vec![first_equal.clone(); PARALLEL_REDUCTION_THRESHOLD];
        equal_values.push(last_equal.clone());
        let equal_values = Value::array(equal_values);
        let minimum = extrema(&equal_values, false).pop().unwrap().unwrap();
        let maximum = extrema(&equal_values, true).pop().unwrap().unwrap();
        assert!(minimum.shares_node_with(&first_equal));
        assert!(maximum.shares_node_with(&last_equal));
    }

    #[test]
    fn recursive_descent_is_depth_first_and_streams_scalars_in_object_order() {
        assert_eq!(
            json(run("..", r#"{"z":1,"a":[2,{"b":3}]}"#)),
            [
                r#"{"z":1,"a":[2,{"b":3}]}"#,
                "1",
                r#"[2,{"b":3}]"#,
                "2",
                r#"{"b":3}"#,
                "3",
            ]
        );
        assert_eq!(
            json(run(".. | scalars", r#"{"z":1,"a":[2,{"b":3}]}"#)),
            ["1", "2", "3"]
        );
        assert_eq!(json(run("..", "42")), ["42"]);
    }

    #[test]
    fn user_functions_preserve_generator_arguments_scope_and_managed_recursion() {
        assert_eq!(
            json(run("def pair($x; $y): $x, $y; pair((1,2); (3,4))", "null")),
            ["1", "3", "1", "4", "2", "3", "2", "4"]
        );
        assert_eq!(
            json(run("def pair($x; $y): $x, $y; pair(1,2; 3,4)", "null")),
            ["1", "3", "1", "4", "2", "3", "2", "4"]
        );
        assert_eq!(json(run("def twice(f): f | f; twice(. + 1)", "1")), ["3"]);
        assert_eq!(
            json(run("def wrapped(f): [f, (not)]; wrapped((1,2))", "null")),
            ["[1,2,true]"]
        );
        assert_eq!(
            json(run("def fallback: empty // 9; fallback", "null")),
            ["9"]
        );
        assert_eq!(
            json(run(
                "1 as $x | def captured: $x; 2 as $x | captured",
                "null"
            )),
            ["1"]
        );
        assert_eq!(
            json(run(
                "def down($n): if $n == 0 then 0 else $n, down($n - 1) end; down(3)",
                "null"
            )),
            ["3", "2", "1", "0"]
        );

        let plan = analyze(
            resolve(
                parse("def forever: forever; forever").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut limited = Vm::new(
            &plan,
            Value::Null,
            VmLimits {
                call_stack: 8,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "call-stack"
            })
        ));
    }

    #[test]
    fn user_functions_execute_inside_callback_builtins() {
        let cases = [
            ("def f: . + 1; map(f)", "[1,2]", "[2,3]"),
            ("def f: ., . + 10; map(f)", "[1,2]", "[1,11,2,12]"),
            (
                "def f: . + 1; map_values(f)",
                r#"{"a":1,"b":2}"#,
                r#"{"a":2,"b":3}"#,
            ),
            ("def f: empty; map_values(f)", r#"{"a":1}"#, "{}"),
            ("def f: ., . + 10; map_values(f)", "[1,2]", "[1,2]"),
            (
                "def f: if . == 1 then empty else . end; map_values(f)",
                "[1,2]",
                "[2]",
            ),
            ("def f: true, true; [.[] | select(f)]", "[1,2]", "[1,1,2,2]"),
            (
                r#"def f: "a", "missing"; [has(f)]"#,
                r#"{"a":1}"#,
                "[true,false]",
            ),
            ("def f: [1,2]; 1 | in(f)", "null", "true"),
            (
                "def f: .k; sort_by(f)",
                r#"[{"k":2,"v":"a"},{"k":1,"v":"b"},{"k":2,"v":"c"}]"#,
                r#"[{"k":1,"v":"b"},{"k":2,"v":"a"},{"k":2,"v":"c"}]"#,
            ),
            (
                "def f: .k; unique_by(f)",
                r#"[{"k":2,"v":"a"},{"k":1,"v":"b"},{"k":2,"v":"c"}]"#,
                r#"[{"k":1,"v":"b"},{"k":2,"v":"a"}]"#,
            ),
            ("def f: empty; sort_by(f)", "[3,1,2]", "[3,1,2]"),
            ("def f: ., -.; sort_by(f)", "[3,1,2]", "[1,2,3]"),
            ("1 as $x | def f: . + $x; map(f)", "[1,2]", "[2,3]"),
            ("def f($x): . + $x; map(f(2))", "[1,2]", "[3,4]"),
            ("def apply(f): map(f); apply(. + 1)", "[1,2]", "[2,3]"),
        ];
        for (query, input, expected) in cases {
            assert_eq!(json(run(query, input)), [expected], "query: {query}");
        }
    }

    #[test]
    fn trusted_compilation_accepts_slice_user_function_closure() {
        let compiled = analyze(
            resolve(
                parse("def f: .; .[0:1] | f").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .expect("slice and user call share the managed execution path");
        assert!(compiled.bytecode().managed_tree_execution());
    }

    #[test]
    fn managed_admission_covers_every_ported_callback_builtin() {
        for query in [
            "def f: .; map(f)",
            "def f: .; map_values(f)",
            "def f: true; select(f)",
            r#"def f: "a"; has(f)"#,
            "def f: [1]; 1 | in(f)",
            "def f: .; sort_by(f)",
            "def f: .; unique_by(f)",
        ] {
            let compiled =
                analyze(resolve(parse(query).unwrap(), &ResolveOptions::default()).unwrap())
                    .compile();
            assert!(
                compiled.is_ok(),
                "query should use managed execution: {query}"
            );
        }
    }

    #[test]
    fn callback_user_functions_obey_errors_limits_cancellation_and_early_stop() {
        assert_eq!(
            json(run(
                r#"def f: if . == 2 then error("bad") else . end; map(f)"#,
                "[1,2,3]"
            )),
            ["error:runtime error: bad"]
        );

        assert!(matches!(
            first_error_with_limits(
                "def f: ., .; map(f)",
                "[1,2,3]",
                VmLimits {
                    fork_stack: 2,
                    ..VmLimits::default()
                }
            ),
            crate::VmError::Resource {
                resource: "fork-stack"
            }
        ));
        assert!(matches!(
            first_error_with_limits(
                "def f: . + 1; map(f)",
                "[1,2,3]",
                VmLimits {
                    steps: 2,
                    ..VmLimits::default()
                }
            ),
            crate::VmError::Resource {
                resource: "vm-steps"
            }
        ));

        let plan = analyze(
            resolve(
                parse("def down($n): if $n == 0 then 0 else $n, down($n - 1) end; map(down(3))")
                    .unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut vm = Vm::new(
            &plan,
            serde_json::from_str("[null]").unwrap(),
            VmLimits {
                call_stack: 16,
                ..VmLimits::default()
            },
        );
        assert_eq!(
            vm.next_result().unwrap(),
            Some(Value::array([3, 2, 1, 0].map(|value| {
                Value::Number(crate::Number::parse(&value.to_string()).unwrap())
            })))
        );
        assert!(vm.observations().call_stack_high_water <= 16);
        assert!(vm.observations().fork_stack_high_water <= VmLimits::default().fork_stack);

        let cancellation = Arc::new(AtomicBool::new(true));
        let mut cancelled = Vm::new(&plan, Value::array([Value::Null]), VmLimits::default())
            .with_cancellation(cancellation);
        assert!(matches!(
            cancelled.next_result(),
            Err(crate::VmError::Interrupted)
        ));

        let plan = analyze(
            resolve(
                parse("def f: true, true; .[] | select(f)").unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut early = Vm::new(
            &plan,
            Value::array([
                Value::Number(crate::Number::parse("1").unwrap()),
                Value::Number(crate::Number::parse("2").unwrap()),
            ]),
            VmLimits::default(),
        );
        assert!(early.next_result().unwrap().is_some());
        drop(early);
    }

    #[test]
    fn interpolation_matches_jq_conversion_generator_order_nesting_and_errors() {
        assert_eq!(
            json(run(
                r#""v=\(null),\(false),\(1),\("x"),\([1]),\({a:1})""#,
                "null",
            )),
            [r#""v=null,false,1,x,[1],{\"a\":1}""#]
        );
        assert_eq!(
            json(run(r#""x=\(1,2);y=\("a","b")""#, "null")),
            [
                r#""x=1;y=a""#,
                r#""x=2;y=a""#,
                r#""x=1;y=b""#,
                r#""x=2;y=b""#
            ]
        );
        assert_eq!(
            json(run(r#""\(if true then "x\(1,2)" else "z" end)""#, "null")),
            [r#""x1""#, r#""x2""#]
        );
        assert_eq!(json(run(r#""sum=\(1 + 2)""#, "null")), [r#""sum=3""#]);
        assert_eq!(
            json(run(r#""x=\((1,2) + 1);y=\(("a","b") + "!")""#, "null",)),
            [
                r#""x=2;y=a!""#,
                r#""x=3;y=a!""#,
                r#""x=2;y=b!""#,
                r#""x=3;y=b!""#,
            ]
        );
        let partial = json(run(r#""before=\(1,error("boom"),2)""#, "null"));
        assert_eq!(partial[0], r#""before=1""#);
        assert!(partial[1].starts_with("error:runtime error: boom"));
        assert_eq!(
            run(r#""\(empty)""#, "null"),
            Vec::<Result<Value, String>>::new()
        );
    }

    #[test]
    fn format_filters_match_jq_across_generator_and_fallback_paths() {
        assert_eq!(json(run("@base64", r#""hello""#)), [r#""aGVsbG8=""#]);
        assert_eq!(
            json(run("@base64 | .[0:]", r#""hello""#)),
            [r#""aGVsbG8=""#]
        );
        assert_eq!(
            json(run(
                r#"@uri "https://x.test?q=\(.q, .alt)""#,
                r#"{"q":"a b","alt":"<&"}"#,
            )),
            [
                r#""https://x.test?q=a%20b""#,
                r#""https://x.test?q=%3C%26""#,
            ]
        );
        assert!(run(r#"@uri "x=\(empty)""#, "null").is_empty());
        assert_eq!(
            json(run("@csv, @tsv, @sh", r#"["a b",1,null,true]"#)),
            [
                r#""\"a b\",1,,true""#,
                r#""a b\t1\t\ttrue""#,
                r#""'a b' 1 null true""#,
            ]
        );
        assert!(json(run("@base64d", r#""not base64""#))[0].starts_with("error:runtime error:"));

        let plan = analyze(resolve(parse("@uri").unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let mut limited = Vm::new(
            &plan,
            Value::string(" "),
            VmLimits {
                output_bytes: 2,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "output-bytes"
            })
        ));
    }

    #[test]
    fn recursive_cursor_obeys_depth_work_and_cancellation_after_partial_output() {
        let plan = analyze(resolve(parse("..").unwrap(), &ResolveOptions::default()).unwrap())
            .compile()
            .unwrap()
            .document_plan();
        let mut input = Value::Null;
        for _ in 0..128 {
            input = Value::array([input]);
        }
        let mut depth_limited = Vm::new(
            &plan,
            input.clone(),
            VmLimits {
                path_stack: 16,
                ..VmLimits::default()
            },
        );
        for _ in 0..16 {
            assert!(depth_limited.next_result().unwrap().is_some());
        }
        assert!(matches!(
            depth_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "path-stack"
            })
        ));

        let mut work_limited = Vm::new(
            &plan,
            input.clone(),
            VmLimits {
                steps: 2,
                ..VmLimits::default()
            },
        );
        assert!(work_limited.next_result().unwrap().is_some());
        assert!(matches!(
            work_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "vm-steps"
            })
        ));

        let cancellation = Arc::new(AtomicBool::new(false));
        let mut cancelled =
            Vm::new(&plan, input, VmLimits::default()).with_cancellation(Arc::clone(&cancellation));
        assert!(cancelled.next_result().unwrap().is_some());
        cancellation.store(true, Ordering::Relaxed);
        assert!(matches!(
            cancelled.next_result(),
            Err(crate::VmError::Interrupted)
        ));
    }

    #[test]
    fn interpolation_generator_explosion_is_bounded_by_managed_forks() {
        let plan = analyze(
            resolve(
                parse(r#""\(1,2,3)-\("a","b")""#).unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut limited = Vm::new(
            &plan,
            Value::Null,
            VmLimits {
                fork_stack: 1,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "fork-stack"
            })
        ));

        let fallback_plan = analyze(
            resolve(
                parse(r#""\(range(0;3) + 0)""#).unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut fallback_limited = Vm::new(
            &fallback_plan,
            Value::Null,
            VmLimits {
                fork_stack: 1,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            fallback_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "fork-stack"
            })
        ));

        let output_plan = analyze(
            resolve(
                parse(r#""prefix=\(.)""#).unwrap(),
                &ResolveOptions::default(),
            )
            .unwrap(),
        )
        .compile()
        .unwrap()
        .document_plan();
        let mut output_limited = Vm::new(
            &output_plan,
            Value::string("value"),
            VmLimits {
                output_bytes: 8,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            output_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "output-bytes"
            })
        ));

        let wide = Value::array(vec![Value::string("0123456789"); 100_000]);
        let mut serialization_limited = Vm::new(
            &output_plan,
            wide,
            VmLimits {
                output_bytes: 16,
                ..VmLimits::default()
            },
        );
        assert!(matches!(
            serialization_limited.next_result(),
            Err(crate::VmError::Resource {
                resource: "output-bytes"
            })
        ));
    }

    #[test]
    fn stable_sort_pipeline_bounds_runs_and_merges_deterministically() {
        let mut pipeline = StableSortPipeline::new(2, 256, 2, 512);
        for value in [3, 1, 2, 1, 3, 2] {
            pipeline
                .push(serde_json::from_str(&value.to_string()).unwrap(), 32)
                .unwrap();
        }
        let (values, observations) = pipeline.finish().unwrap();
        assert_eq!(
            values,
            [1, 1, 2, 2, 3, 3]
                .map(|value| serde_json::from_str(&value.to_string()).unwrap())
                .to_vec()
        );
        assert_eq!(observations.batches, 3);
        assert!(observations.in_flight_batches <= 2);
        assert!(observations.in_flight_bytes <= 512);
    }

    #[test]
    fn stable_sort_pipeline_cancels_with_workers_active() {
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut pipeline =
            StableSortPipeline::new(1, 64, 2, 128).with_cancellation(Arc::clone(&cancellation));
        pipeline
            .push(serde_json::from_str("2").unwrap(), 32)
            .unwrap();
        cancellation.store(true, Ordering::Relaxed);
        assert_eq!(pipeline.finish().unwrap_err(), "interrupted");
    }

    #[test]
    fn stable_sort_pipeline_preserves_equal_object_order_across_runs() {
        let first: Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let second: Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
        assert_eq!(first.cmp(&second), std::cmp::Ordering::Equal);
        assert_ne!(first.to_string(), second.to_string());

        let run = || {
            let mut pipeline = StableSortPipeline::new(1, 256, 2, 512);
            pipeline.push(first.clone(), 64).unwrap();
            pipeline.push(second.clone(), 64).unwrap();
            pipeline
                .finish()
                .unwrap()
                .0
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        };
        let expected = vec![first.to_string(), second.to_string()];
        assert_eq!(run(), expected);
        assert_eq!(run(), expected);
    }
}
