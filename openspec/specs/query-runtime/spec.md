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
transcode, event-stream, subtree, hybrid streaming-blocking, document,
whole-input, and/or blocking-document. The resulting metadata MUST be available
before input consumption and included in the compiled program. Final plan
selection MAY combine this metadata with decoder and output-writer capabilities,
but it MUST NOT weaken the query requirements recorded by analysis.

#### Scenario: Semantic identity
- **WHEN** the resolved program returns each input value unchanged exactly once
- **THEN** analysis records semantic identity so output-aware planning may consider transcode

#### Scenario: Simple event consumer
- **WHEN** a program consumes jq-style stream path/value events without document operators
- **THEN** analysis permits an event execution plan

#### Scenario: Sort over a streamable collection
- **WHEN** a program constructs an array from an independently streamable projection and applies an order-sensitive `sort` suffix
- **THEN** analysis permits a hybrid streaming-blocking plan whose retained state is the projected collection rather than the complete source document

#### Scenario: Sort
- **WHEN** a program applies `sort` to a value whose production requires the complete input document
- **THEN** analysis marks it blocking-document and materializes the document as applicable

#### Scenario: Slurp
- **WHEN** CLI slurp mode is combined with a query
- **THEN** the execution plan is classified whole-input

### Requirement: Mode-safe hybrid execution
A compiled hybrid plan SHALL represent its streaming producer, collection boundary, and blocking suffix as validated typed components. Execution MUST evaluate producer results in jq encounter order, pass only completed owned values across the boundary, and invoke the suffix exactly once with the same collected value document execution would construct.

#### Scenario: Mixed projected values
- **WHEN** a hybrid producer emits nulls, booleans, numbers, strings, arrays, and objects
- **THEN** the blocking suffix receives the same ordered array of values as document execution

#### Scenario: Producer error after completed items
- **WHEN** item evaluation fails after earlier items have entered the blocking collection
- **THEN** execution returns the jq-compatible error without publishing a result from the blocking suffix

#### Scenario: Stable equal-value sort
- **WHEN** equal-comparing values enter parallel sort preparation in different batches
- **THEN** their encounter order is preserved wherever the accepted jq baseline makes that order observable

### Requirement: Validation-only selected JSON discard
When a static automatic projection proves that a JSON subtree cannot contribute a result, the decoder MAY discard that subtree without constructing structural events or runtime values. It MUST still consume the entire subtree and preserve JSON syntax, nesting-depth, token-length, numeric-envelope, input limit, cancellation, and late-error behavior.

#### Scenario: Irrelevant geometry numbers
- **WHEN** a projected metadata query encounters numeric coordinates outside its selected path
- **THEN** the decoder validates their JSON tokens and numeric resource envelope without constructing jq numbers or projector records

#### Scenario: Invalid discarded subtree
- **WHEN** an unselected subtree contains malformed JSON or exceeds a configured decoder resource limit
- **THEN** hybrid execution returns the same failure class as the non-discarding structural decoder and publishes no blocking result

### Requirement: Lightweight identity-transcode tokens
The JSON structural decoder MAY deliver keys, strings, and numeric literals to an identity-transcode consumer without constructing owned structural events or runtime values. Numeric literals MUST use the same canonical form and resource envelope as `Number`, strings MUST use the selected TOON writer's quoting rules, and consumers that require owned events MUST retain the existing event contract.

#### Scenario: Scalar-heavy identity transcode
- **WHEN** JSON identity transcode consumes root scalars, direct object scalar members, or scalar-array elements
- **THEN** the decoder and transcode consumer publish canonical TOON without constructing an intermediate tq scalar event or value for those tokens

#### Scenario: Lightweight numeric failure
- **WHEN** a numeric token exceeds tq's coefficient, exponent, or rendered-token envelope
- **THEN** lightweight transcode returns the same input failure class as owned structural decoding and does not publish a completed unframed result

#### Scenario: Owned-event consumer compatibility
- **WHEN** an event consumer does not implement lightweight token handling
- **THEN** the decoder adapts token text into the existing key and scalar events with unchanged order and values

### Requirement: Semantics-preserving blocking rewrites
The compiler MAY remove a blocking operator only when resolved query structure and input-type proof establish that the operator cannot change the result sequence, values, ordering, or jq-visible errors. Explain output MUST identify each applied rewrite.

#### Scenario: Array sort followed by length
- **WHEN** a resolved query applies the built-in `sort` and then `length` to a value proven to be an array
- **THEN** the compiler may evaluate `length` without sorting and reports the removed blocking operation in explain output

#### Scenario: Sort input is not proven array
- **WHEN** the input to `sort | length` is not statically proven to be an array
- **THEN** the compiler retains `sort` so its jq-compatible type error remains observable

#### Scenario: Sort key can fail
- **WHEN** a sort-like operation evaluates a user expression or otherwise has jq-visible evaluation effects
- **THEN** the compiler retains that operation unless it separately proves those effects unobservable

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

### Requirement: Ordered parallel decode boundary
The runtime SHALL treat ordered output from parallel selected decoding as the original input result sequence. Concurrent decoding MUST NOT make VM evaluation concurrent or change result, error, or stable-sort ordering.

#### Scenario: Fallible downstream filter
- **WHEN** ordered decoded elements enter a fallible VM filter
- **THEN** the VM evaluates them serially in source order and reports the same first runtime error as serial execution

#### Scenario: Stable blocking sort
- **WHEN** equal sort keys originate in different decode batches
- **THEN** their relative result order matches serial execution
