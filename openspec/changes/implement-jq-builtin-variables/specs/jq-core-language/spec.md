## ADDED Requirements

### Requirement: jq built-in variables

The language SHALL provide jq-compatible `$ENV` and `$__loc__` variable references without requiring callers to declare them as external variables. `$ENV` SHALL represent the process-start environment snapshot admitted by the host, and `$__loc__` SHALL represent the source location of the reference.

#### Scenario: Environment variable reference
- **WHEN** a query evaluates `$ENV` with environment access admitted
- **THEN** it emits one object whose keys and values are strings and whose contents match the invocation's startup environment snapshot

#### Scenario: Environment variable reference is capability-gated
- **WHEN** a query references `$ENV` without admitted environment access
- **THEN** resolution succeeds as a known jq variable and evaluation returns the same policy-class failure used by the `env` builtin

#### Scenario: Top-level source location
- **WHEN** the query `$__loc__` is evaluated from a one-line top-level source
- **THEN** it emits one ordered object with `file` set to the source identity and `line` set to the one-based number `1`

#### Scenario: Multi-line source location
- **WHEN** `$__loc__` occurs on a later line of a named query source
- **THEN** its `line` value identifies the one-based source line containing that reference, independent of the input document

#### Scenario: Source location inside a definition
- **WHEN** a filter definition evaluates `$__loc__` from its body
- **THEN** the returned line identifies the reference's definition source location rather than the call site

## MODIFIED Requirements

### Requirement: Variable binding

The language SHALL support `$name` references and `EXP as $name | EXP` lexical binding with jq-compatible scope and generator behavior. CLI-provided variables MUST enter the same resolved environment. The jq special references `$ENV` and `$__loc__` SHALL resolve without external declarations. Same-named CLI external arguments MUST NOT replace either special reference; lexical `$ENV` bindings MAY shadow the built-in `$ENV`, while `$__loc__` remains a reserved special reference.

#### Scenario: Bind multiple values
- **WHEN** `.items[] as $item | $item.name` is evaluated
- **THEN** the body executes once per bound item in order

#### Scenario: Unknown variable
- **WHEN** a query references an unbound variable other than a jq special variable
- **THEN** resolution fails before execution with a source-spanned compile error

#### Scenario: Built-in variable needs no declaration
- **WHEN** a query references `$ENV` or `$__loc__` without `--arg`, `--argjson`, or `--argtoon`
- **THEN** resolution succeeds and the reference is evaluated according to its built-in contract

#### Scenario: Lexical environment shadowing
- **WHEN** `1 as $ENV | $ENV` is evaluated
- **THEN** the lexical binding supplies `1` within its scope instead of the ambient environment object

#### Scenario: External special-variable arguments do not override built-ins
- **WHEN** `--arg ENV replacement` or `--arg __loc__ replacement` is supplied and the query references the corresponding special variable
- **THEN** the built-in variable semantics remain in effect
