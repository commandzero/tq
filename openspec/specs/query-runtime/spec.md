# Query Runtime Specification

## Purpose

Define tq's typed compilation lifecycle, validated bytecode, pull-based VM,
execution plans, structural sharing, diagnostics, and runtime safety contracts.

## Requirements

### Requirement: Typed compilation lifecycle
The core API SHALL represent parsed, resolved, analyzed, and compiled query phases with distinct Rust types or equivalent sealed markers. APIs MUST prevent unresolved or unanalyzed queries from entering execution without an explicit transition.

#### Scenario: Compile valid query
- **WHEN** a valid parsed query is resolved, analyzed, and compiled
- **THEN** each transition consumes or wraps the prior phase and yields a validated compiled program

#### Scenario: Unknown variable during resolution
- **WHEN** resolution encounters an unbound variable
- **THEN** it returns a compile diagnostic and no compiled-program value can be constructed through the safe API

### Requirement: Source-spanned diagnostics
Lexer, parser, resolver, analyzer, bytecode compiler, and runtime errors SHALL retain query source spans. Input-related runtime errors SHALL additionally retain input filename/document identity, selected input format, and the best available TOON/JSON/YAML source position or value path.

#### Scenario: Runtime type error
- **WHEN** an arithmetic instruction receives invalid operand types
- **THEN** the diagnostic identifies the query operator span and the input document/path context

### Requirement: Validated bytecode
Compiled programs SHALL use an immutable bytecode representation with validated instruction boundaries, constant/function references, stack effects, jumps, and fork targets. Execution MUST reject malformed bytecode produced outside trusted constructors.

#### Scenario: Compile branch
- **WHEN** a conditional is compiled
- **THEN** bytecode validation confirms all branches target valid instructions and have compatible stack invariants

#### Scenario: Corrupt bytecode
- **WHEN** a test constructs an invalid jump target through a test-only decoder
- **THEN** validation rejects the program before the VM executes it

### Requirement: Pull-based multi-result VM
The VM SHALL expose one result at a time and SHALL represent forks/backtracking explicitly. It MUST support zero, one, or many results without collecting the entire result sequence.

#### Scenario: Pull comma results
- **WHEN** a caller repeatedly requests results from `.a, .b, .c`
- **THEN** the VM returns each result in order and then signals completion

#### Scenario: Downstream stops early
- **WHEN** a caller stops requesting results after the first value
- **THEN** the VM releases remaining forks and does not evaluate unnecessary branches

### Requirement: Explicit execution stacks
The VM SHALL maintain explicit value, call-frame, path, and fork stacks with configured bounds. Core evaluation MUST avoid relying on unbounded native recursion for data traversal or filter backtracking.

#### Scenario: Deep but allowed filter
- **WHEN** execution depth is within configured bounds
- **THEN** the VM executes using managed stacks and reports their high-water marks when tracing is enabled

#### Scenario: Stack limit exceeded
- **WHEN** a managed stack would exceed its configured bound
- **THEN** execution returns a resource-limit diagnostic without process stack overflow

### Requirement: Immutable structural sharing
Runtime arrays, objects, strings, and composite nodes SHALL support shallow value-handle cloning. Updating a nested path MUST rebuild only the changed path and share unaffected subtrees.

#### Scenario: Branch input
- **WHEN** a large document enters a comma expression with two read-only branches
- **THEN** the VM does not deep-clone the entire document for each branch

#### Scenario: Nested update
- **WHEN** one deeply nested field is updated
- **THEN** siblings outside the update path remain structurally shared and semantically unchanged

### Requirement: Stable path semantics
The runtime SHALL represent jq paths as ordered key/index components anchored to a root value. Assignment instructions MUST apply emitted paths to the correct root even when the left-side filter iterates or selects multiple values.

#### Scenario: Multi-path update
- **WHEN** a left-side filter emits two distinct valid paths
- **THEN** the runtime applies updates in jq-compatible order to one resulting root

#### Scenario: Stale path
- **WHEN** an earlier update makes a later emitted path invalid under jq semantics
- **THEN** the runtime follows the accepted jq baseline behavior and does not dereference an unsafe Rust reference

### Requirement: Execution capability analysis
Analysis SHALL classify a program's input and working-set requirements as
semantic identity, event-stream, subtree, document, whole-input, and/or
blocking. The resulting metadata MUST be available before input consumption and
included in the compiled program. Final plan selection MAY combine this metadata
with decoder and output-writer capabilities, but it MUST NOT weaken the query
requirements recorded by analysis.

#### Scenario: Semantic identity
- **WHEN** the resolved program returns each input value unchanged exactly once
- **THEN** analysis records semantic identity so output-aware planning may consider transcode

#### Scenario: Simple event consumer
- **WHEN** a program consumes jq-style stream path/value events without document operators
- **THEN** analysis permits an event execution plan

#### Scenario: Sort
- **WHEN** a program contains `sort` over a generated collection
- **THEN** analysis marks it blocking and document/subtree materializing as applicable

#### Scenario: Slurp
- **WHEN** CLI slurp mode is combined with a query
- **THEN** the execution plan is classified whole-input

### Requirement: Mode-safe execution plans
A compiled program SHALL be converted into a typed transcode, document, or event
plan before execution. A transcode plan MUST carry proof of semantic identity and
compatible decoder and writer capabilities. Explicit event mode MUST reject
programs requiring document values before reading input.

#### Scenario: Construct transcode plan
- **WHEN** semantic-identity analysis and compatible I/O capabilities are present
- **THEN** the planner constructs a typed transcode plan that cannot enter the document VM by accident

#### Scenario: Event-incompatible query
- **WHEN** an ordinary document update is requested in explicit event mode
- **THEN** planning fails with a capability diagnostic and no input bytes are consumed

### Requirement: Deterministic evaluation
For a fixed compiled program, variable environment, input result sequence, resource configuration, and tool version, execution SHALL produce deterministic result order, values, diagnostics, and exit classification.

#### Scenario: Repeat evaluation
- **WHEN** a compatibility case is executed repeatedly with the same inputs
- **THEN** normalized outputs and errors are identical

### Requirement: Runtime tracing and disassembly
The core SHALL provide test/development APIs for source-annotated HIR display, capability analysis, bytecode disassembly, and bounded VM tracing. These APIs MUST be disabled or inert unless requested and MUST NOT change evaluation semantics.

#### Scenario: Explain compiled query
- **WHEN** a test requests disassembly
- **THEN** it receives stable instruction offsets, operands, stack effects, and source spans suitable for golden tests

### Requirement: Safe input and query handling
The parser, compiler, and runtime SHALL forbid unsafe Rust in MVP core crates unless a later reviewed design documents the invariant and measured need. Malformed input/query data MUST return diagnostics rather than panic.

#### Scenario: Fuzzed query
- **WHEN** arbitrary bytes are supplied to the query parser
- **THEN** it returns a result or diagnostic without memory unsafety or an uncontrolled panic

### Requirement: Differential and property validation
The runtime SHALL be exercised by unit tests, bytecode validation tests, jq differential cases, update/path property tests, fuzz targets, and result-cardinality tests before a capability is marked MVP-compatible.

#### Scenario: Enable language capability
- **WHEN** a language feature's implementation is complete
- **THEN** its jq-target compatibility cases and runtime invariant tests pass before its support-matrix status changes to supported
