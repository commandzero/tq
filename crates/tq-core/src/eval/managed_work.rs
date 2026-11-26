//! Represent suspended managed work and identify its owning continuation.

use super::managed_paths::{PathTraversalCursor, PathWork};
use super::origins::{LexicalEnvironment, OriginToken, PathAliases};
use super::{UserFrames, generator, iterate, path_builtin, resource, sql};
use crate::ast::{AssignmentOperator, BinaryOperator, UnaryOperator};
use crate::bytecode::{BindingPatternOperand, InterpolationOperand};
use crate::{Bytecode, InputCursor, Object, Path, Value, VmError, stdlib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct FilterArgument {
    pub(super) node: u32,
    pub(super) environment: Arc<LexicalEnvironment>,
    pub(super) frames: UserFrames,
}

#[derive(Clone)]
pub(super) struct UserFrame {
    pub(super) symbol: u32,
    pub(super) filters: Vec<Option<FilterArgument>>,
}
#[derive(Clone)]
pub(super) enum GeneratorContinuation {
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
        object: Arc<Object>,
        input: Value,
        origin: Option<OriginToken>,
        environment: Arc<LexicalEnvironment>,
        frames: UserFrames,
    },
    ObjectValue {
        entries: Arc<[crate::bytecode::ObjectOperand]>,
        next: usize,
        object: Arc<Object>,
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
pub(super) struct GeneratorWork {
    pub(super) node: u32,
    pub(super) input: Value,
    pub(super) origin: Option<OriginToken>,
    pub(super) environment: Arc<LexicalEnvironment>,
    pub(super) frames: UserFrames,
    pub(super) continuations: Vec<GeneratorContinuation>,
}

pub(super) fn regex_error_work(
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

pub(super) enum GeneratorTask {
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
pub(super) enum PullConsumerKind {
    First,
    Last,
    IsEmpty,
    Any,
    All,
    Nth,
    Skip,
}

pub(super) struct PullConsumerState {
    pub(super) boundary: u64,
    pub(super) kind: PullConsumerKind,
    pub(super) seen: bool,
    pub(super) latest: Option<Value>,
    pub(super) latest_origin: Option<OriginToken>,
    pub(super) decision: Option<bool>,
    pub(super) remaining: usize,
}

#[derive(Clone, Copy)]
pub(super) enum KeyedCollectionMode {
    Sort,
    Unique,
    Group,
    Min,
    Max,
}

pub(super) struct ManagedKeyedValue {
    pub(super) key: Value,
    pub(super) value: Value,
    pub(super) origin: Option<OriginToken>,
}

pub(super) struct FoldState {
    pub(super) accumulator: Value,
}

pub(super) struct AddState {
    pub(super) accumulator: Option<Value>,
}

pub(super) struct SqlInState {
    pub(super) matched: bool,
}

pub(super) struct SqlIndexState {
    pub(super) builder: Option<sql::SqlIndexBuilder>,
}

pub(super) struct SqlJoinState {
    pub(super) collector: Option<sql::SqlPairCollector>,
}
#[derive(Clone)]
pub(super) enum AssignmentTarget {
    Path(Path),
    Slice {
        path: Path,
        start: usize,
        end: usize,
    },
}

pub(super) struct AssignmentState {
    pub(super) operator: AssignmentOperator,
    pub(super) value_node: u32,
    pub(super) input: Value,
    pub(super) targets: Vec<AssignmentTarget>,
    pub(super) target_origins: Vec<Option<OriginToken>>,
    pub(super) document: Value,
    pub(super) deletions: path_builtin::PathAccumulator,
}

#[derive(Clone, Copy)]
pub(super) enum PathCollectionKind {
    Delete,
    Pick,
}

pub(super) fn task_owned_by_label(task: &GeneratorTask, label: u32) -> bool {
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

pub(super) fn task_owned_by_catch(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(task, |continuation| {
        matches!(
            continuation,
            GeneratorContinuation::Catch { boundary: candidate, .. }
                | GeneratorContinuation::PathCatch { boundary: candidate, .. }
                if *candidate == boundary
        )
    })
}

pub(super) fn task_owned_by_optional(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(task, |continuation| {
        matches!(
            continuation,
            GeneratorContinuation::OptionalBoundary { boundary: candidate }
                if *candidate == boundary
        )
    })
}

pub(super) fn task_owned_by_bind_alternative(task: &GeneratorTask, boundary: u64) -> bool {
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

pub(super) fn task_owned_by_limit(task: &GeneratorTask, boundary: u64) -> bool {
    task_has_continuation(
        task,
        |continuation| matches!(continuation, GeneratorContinuation::LimitItem { boundary: candidate, .. } if *candidate == boundary),
    )
}

pub(super) fn task_owned_by_pull_consumer(task: &GeneratorTask, boundary: u64) -> bool {
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

pub(super) fn task_owned_by_pull_consumer_work(task: &GeneratorTask, boundary: u64) -> bool {
    !matches!(task, GeneratorTask::FinishPullConsumer { .. })
        && task_owned_by_pull_consumer(task, boundary)
}

pub(super) fn task_owned_by_sql(task: &GeneratorTask, boundary: u64) -> bool {
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

pub(super) fn task_owned_by_assignment_rhs(task: &GeneratorTask, boundary: u64) -> bool {
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

pub(super) fn task_owned_by_assignment_boundary(task: &GeneratorTask, boundary: u64) -> bool {
    matches!(
        task,
        GeneratorTask::AssignmentUpdateContinue {
            boundary: candidate,
            ..
        } if *candidate == boundary
    ) || task_owned_by_assignment_rhs(task, boundary)
}

pub(super) fn task_owned_by_path_collection(task: &GeneratorTask, boundary: u64) -> bool {
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
pub(super) enum InterpolationWork {
    Expand {
        next: usize,
        pieces: Vec<Option<Arc<str>>>,
    },
    Error(VmError),
}

#[derive(Clone)]
pub(super) struct TraversalCursor {
    frames: Vec<TraversalFrame>,
}

#[derive(Clone)]
struct TraversalFrame {
    value: Value,
    emitted: bool,
    next_child: usize,
}

impl TraversalCursor {
    pub(super) fn new(value: Value) -> Self {
        Self {
            frames: vec![TraversalFrame {
                value,
                emitted: false,
                next_child: 0,
            }],
        }
    }

    pub(super) fn next(&mut self, depth_limit: usize) -> Result<Option<Value>, VmError> {
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

    pub(super) fn depth(&self) -> usize {
        self.frames.len()
    }
}

pub(super) fn recursive_child(value: &Value, index: usize) -> Option<Value> {
    match value {
        Value::Array(values) => values.get(index).cloned(),
        Value::Object(values) => values.get_index(index).map(|(_, value)| value.clone()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}
