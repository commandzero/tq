## Context

`tq` is a greenfield standalone Rust project intended to become the TOON counterpart to `jq` and `yq`. The repository currently contains only a minimal binary crate and several natural USGS GeoJSON snapshots. The neighboring source trees provide local reference implementations: jq 1.8.x in `../jq`, yq 4.x in `../yq`, the TOON TypeScript reference in `/Users/reno/Development/toon-format/toon`, and the Rust TOON implementation in `/Users/reno/Development/VimCommando/toon-rust`.

The current Rust TOON library is a useful conformance base, but it does not yet provide the data plane required by `tq`. TOON decoding accepts a complete `&str`, the scanner copies that string into `Vec<char>`, and decoding builds a complete `serde_json::Value`. Its JSON-to-TOON streaming path avoids a full JSON DOM for some object structures, but buffers array contents because TOON array headers require the final element count and tabular schema before the body can be emitted. Key folding also requires whole-document inspection.

jq compatibility is not equivalent to parsing field expressions. jq filters form a nondeterministic, multi-result language with backtracking, closures, path updates, errors, and blocking operators. yq implements related syntax but has YAML-specific values and behavior; its streaming evaluator processes one complete document at a time. The project therefore needs an explicit compatibility target, a result-sequence model, and honest execution classifications.

The benchmark corpus is intentionally refreshable. USGS feed endpoints naturally vary in byte size and feature count over time. Benchmark results must be tied to a recorded snapshot rather than to an assumed fixed number of bytes. Large and generated cross-format artifacts must not be committed to Git.

## Goals / Non-Goals

**Goals:**

- Deliver a useful jq-compatible MVP over a JSON-shaped data model with TOON, JSON, and YAML input and an executable support matrix.
- Write and run compatibility and performance harnesses against jq and yq before implementing language features in `tq`.
- Make correctness a prerequisite for benchmark timing and make benchmark campaigns reproducible from recorded manifests.
- Compile filters to an explicit generator-oriented bytecode runtime rather than coupling syntax directly to a recursive evaluator.
- Use Rust typestate at compilation and execution-plan boundaries so unresolved or incompatible programs cannot enter the wrong executor.
- Avoid deep value cloning through immutable structural sharing and path-copying updates.
- Provide strict, incremental TOON event decoding and canonical encoding suitable for large inputs.
- Support general document execution and an explicit jq-compatible event-stream execution mode with separately stated memory guarantees.
- Define an unambiguous framed transport for zero-or-more TOON results.
- Expose buffering, blocking, spooling, limits, and fallback behavior through diagnostics and `--explain`.
- Ship the CLI and engine as separable crates so the runtime can be embedded and tested without subprocess overhead.

**Non-Goals:**

- Complete jq 1.8 parity in the MVP.
- User-defined functions, modules/imports, the full jq standard library, advanced regex compatibility, date/platform-specific built-ins, or byte-identical jq diagnostics.
- YAML tag, anchor, style, or comment preservation.
- Source-preserving TOON edits; structured results are canonically re-encoded.
- Automatic bounded-memory execution for arbitrary jq filters.
- Hiding fundamentally blocking operations such as sort, grouping, slurp, or constructed arrays behind a streaming marketing claim.
- Committing large benchmark payloads or generated YAML/TOON corpora to the repository.
- Treating a faster but semantically incorrect result as a valid performance result.

## Decisions

### 1. Establish observable baselines before implementing `tq`

The first executable deliverables are the corpus manager, compatibility runner, and performance runner. The compatibility runner records jq and yq behavior before `tq` participates. The performance runner then records jq/yq baselines for the same versioned cases. Only after those baselines are reviewable does language implementation begin.

This reverses the common sequence of building an interpreter and retrofitting tests. It makes ambiguous syntax and CLI behavior visible early, prevents benchmark cherry-picking, and gives every MVP language feature an existing executable target.

Alternative considered: port jq tests directly and begin with the parser. Rejected because it would miss yq observations, TOON transport behavior, cross-format equivalence, CLI startup, peak memory, and realistic data workloads.

### 2. Use a Rust workspace with narrow crate boundaries

The implementation will be organized around these logical crates:

```text
tq-cli ───────────▶ tq-core
   │                 values, lexer/parser, HIR, analysis, bytecode, VM
   └──────────────▶ tq-formats
                       format selection, JSON/YAML adapters, document sources
                       │
                       ├────────▶ tq-core
                       └────────▶ tq-toon ─────────▶ tq-core
                                  TOON events, DOM builder, writer, framing/spool

tq-test-support     manifests, normalization, subprocess adapters
```

The exact package names may be adjusted to avoid publishing conflicts, but dependency direction must remain acyclic and the CLI must not own core semantics. `tq-formats` owns format dispatch and conversion into ordered `tq-core` values; it must not contain query semantics. `toon-format` will be consumed without its default CLI/TUI features. The general TOON event decoder will be prototyped in `tq-toon`, where its contract can stabilize behind tq's conformance, compatibility, fuzz, and large-file tests. The decoder must expose a narrow, query-independent event boundary so the generally useful implementation can be upstreamed into `toon-rust` later without making that upstream work an MVP dependency. `tq-toon` will retain only tq-specific framing, planning adapters, and any thin compatibility adapter needed after upstreaming.

Alternative considered: retain one binary crate until features demand separation. Rejected because subprocess compatibility tests, library benchmarks, fuzz targets, and a reusable VM otherwise become entangled with CLI state.

### 3. Treat jq as the semantic target and yq as a compatibility peer

jq 1.8.x defines MVP filter semantics over the JSON data model. yq 4.x is expected to agree for a documented common subset, but YAML-specific values or intentional yq divergences do not redefine `tq`. Compatibility cases are classified as:

- `common`: jq, yq, and tq must produce semantically equivalent result sequences.
- `jq-target`: tq must match jq; yq output is recorded and may be divergent or unsupported.
- `cli`: tool-specific invocations are compared through an explicitly normalized contract.
- `deferred`: the case documents future parity and is not an MVP release gate.

Tool versions and build identities are part of every baseline. Updating a reference version creates a new baseline review; it never silently replaces expected behavior.

Alternative considered: implement the intersection of jq and yq. Rejected because the intersection is poorly defined and would prevent a coherent query-language identity.

### 4. Compile through typed phases and capability analysis

The public compilation lifecycle will follow this shape:

```text
Query<Parsed>
    ──resolve──▶ Query<Resolved>
    ──analyze──▶ Query<Analyzed>
    ──compile──▶ Program<Compiled>

Program<Compiled> + ExecutionMode
    ──plan──▶ Plan<Document> | Plan<Events> | planning error
```

The parser produces a source-spanned AST. Resolution binds variables and built-ins into a HIR. Analysis computes effects such as cardinality, path access, mutation, blocking, whole-document dependence, whole-input dependence, and possible failure. Compilation emits immutable bytecode and constant pools.

Typestate is limited to stable API boundaries. Runtime nesting, indentation, and VM instruction state remain explicit enums/stacks because encoding arbitrary runtime depth into generic types would add complexity without enforcing useful compile-time invariants.

Alternative considered: use typestate throughout the scanner and VM. Rejected due to generic-state explosion and poor support for dynamically nested input.

### 5. Use a bytecode VM with explicit forks and result pulling

Filters are compiled into a compact instruction set with operand, call, path, branch, fork, backtrack, error, and return operations. The VM owns explicit value, frame, and fork stacks. Evaluation is pulled as `next_result`, allowing the CLI to encode each result without collecting the entire result sequence.

Comma and iteration create resumable forks. `empty` backtracks without a value. Optional access catches only the errors defined by jq-compatible semantics. Updates evaluate paths against a stable root and rebuild changed paths without exposing Rust mutable aliases.

Alternative considered: recursively evaluate the AST into boxed iterators. Rejected because stable Rust lacks ergonomic generators, nested boxed iterators create allocation and lifetime pressure, and jq update/backtracking semantics become difficult to reason about.

### 6. Use immutable, ordered, structurally shared values

The runtime value model is JSON-shaped:

```text
Null | Bool | Number | String | Array | Object
```

Strings, arrays, objects, and their child nodes use reference-counted immutable storage. Objects preserve insertion order. Cloning a VM value handle is shallow; updates rebuild only nodes on affected paths. Materializing from event input owns decoded strings because stdin buffers cannot safely lend long-lived slices.

Numbers use a jq decimal-literal hybrid. Each finite number retains an optional normalized arbitrary-precision decimal literal together with a lazily derived IEEE-754 binary64 value. Identity, construction, encoding, and jq literal-aware comparisons preserve or use the decimal representation when jq 1.8.x does. Arithmetic follows jq's binary64 behavior and produces a value without the source literal rather than silently promising exact arbitrary-precision arithmetic. NaN and infinity are rejected as TOON, JSON, or YAML input.

The baseline phase fixes the remaining resource envelope before arithmetic implementation: maximum input digits, exponent magnitude, canonical exponent expansion/output digits, integer index range, overflow/underflow, division by zero, and non-finite arithmetic-result behavior. Values outside that documented envelope produce a numeric-range or resource diagnostic instead of silent input precision loss.

Alternative considered: use `serde_json::Value` directly. Rejected because its deep clone/update behavior, map configuration, and numeric representation do not provide the control required for jq semantics and large branching queries.

Alternative considered: store only `f64`. Rejected because identity would silently corrupt large integers and long decimals. `i64 | u64 | f64` was also rejected as the default because its 64-bit boundary does not match decimal-enabled jq. A fully exact `BigInt | BigDecimal` arithmetic tower is deferred as a possible explicit lossless mode because it would intentionally diverge from jq arithmetic and add resource-exhaustion risk.

### 7. Separate document execution from event execution

General execution builds one complete document and runs `Plan<Document>`. Multiple files are decoded and released one at a time unless `--slurp` or a whole-input operator explicitly requests aggregation.

`--stream` uses `Plan<Events>` and exposes jq-compatible path/value events from the incremental TOON or JSON decoder. It is depth-bounded apart from the current token, bounded detection replay, current path, VM stacks, and current output result. YAML input is document-at-a-time in the MVP because `yaml_serde` does not expose tq's event contract. A strict YAML override with event mode is rejected before consuming input; automatic mode may consume its bounded probe before selecting YAML and rejecting the mode, but emits no query result first. The MVP does not automatically translate ordinary arbitrary filters into event plans. Analysis rejects a document-only operation in explicit event mode before consuming input whenever the input format is already known.

Execution analysis uses these public classifications:

```text
EventStream  → depth/token bounded
Subtree      → buffers a selected subtree
Document     → buffers one complete document
WholeInput   → buffers or spills across documents
Blocking     → consumes a complete collection before producing a result
```

Alternative considered: promise automatic streaming for simple paths in the MVP. Deferred because subtle differences in missing values, updates, ordering, and multi-result behavior could create a second inconsistent evaluator.

### 8. Build TOON I/O around byte events, not `Vec<char>`

The TOON decoder reads from `BufRead`, validates UTF-8 incrementally, tracks byte offset plus line/column, and emits structural events:

```text
DocumentStart / DocumentEnd
ObjectStart / Key / ObjectEnd
ArrayStart { declared_len, fields, delimiter } / ArrayEnd
Scalar
```

Strict validation checks indentation, quoting, delimiter scope, row width, declared counts, root shape, and configured limits while consuming events. A DOM builder consumes the same events for document execution, eliminating separate parsing semantics.

The writer accepts runtime values and events and emits canonical TOON. Arrays require their count before the header, and tabular representation requires complete schema agreement. The writer therefore uses an explicit memory threshold and then a temporary spool for unknown-size arrays. Spooling is observable and can be forbidden.

Alternative considered: scan characters from an in-memory string and add streaming later. Rejected because it would make the large-file contract architectural debt from the first release.

### 9. Auto-detect TOON, YAML, and JSON with an explicit override

Structured input does not require a format flag. In automatic mode, tq probes each input source independently in this fixed faildown order:

```text
TOON ──reject──▶ YAML ──reject──▶ JSON ──reject──▶ combined input error
  │               │                │
accept          accept           accept
```

Detection is best-effort and bounded. A parser commits after it has positively recognized enough format-specific structure within the configured lookahead/replay budget. A rejection before commitment rewinds the bounded prefix and tries the next parser. After commitment, a later syntax error belongs to the selected format and does not restart another parser; this prevents unbounded replay and duplicate/rolled-back streaming output. Seekable files and non-seekable stdin follow the same observable selection order.

Because JSON is a YAML-compatible syntax, YAML will ordinarily claim JSON that reaches it. This is an intentional consequence of the requested TOON → YAML → JSON precedence. `--input-format toon|yaml|json` is a strict override: it selects exactly one parser, disables faildown, and is the way to demand JSON-specific parsing, diagnostics, numeric behavior, or performance.

The selected parser, number of rejected probes, detection bytes, and any commitment point are observable in diagnostics, execution statistics, compatibility reports, and benchmark metadata. Before reading input, `--explain` reports the configured detection order or strict override and its replay/memory implications. Raw input, null input, and explicit TOON Text Sequence input bypass structured format detection.

JSON uses an ordered, arbitrary-precision-literal-aware Serde path so conversion does not erase the hybrid number representation. YAML uses the actively maintained `yaml_serde` crate from the YAML organization rather than deprecated `serde_yaml`. Its reader-based multi-document deserializer feeds one document at a time into a `tq-formats` adapter.

All formats enter the same JSON-shaped runtime boundary: null, boolean, finite number, string, array/sequence, and insertion-ordered string-keyed object/mapping. YAML aliases may be resolved into values, but comments, anchors, aliases, scalar styles, directives, and tags are not retained. Non-string mapping keys, unsupported custom tags, duplicate keys, and values that cannot enter the hybrid numeric envelope fail with format-specific input diagnostics rather than being silently coerced. Merge-key and scalar-resolution behavior is fixed by yq/tq YAML compatibility baselines before implementation.

The `yaml_serde` integration receives an early fidelity spike. Its public Serde value path may lower numeric scalars to fixed integers or binary64; tq must prove that every accepted YAML number reaches the hybrid representation without silent loss. If the library cannot expose enough scalar information, tq will either reject the affected numeric envelope explicitly or contribute/use a narrow raw-scalar API while retaining `yaml_serde` as the YAML parser.

Alternative considered: require explicit input selection. Rejected in favor of best-effort command-line convenience with a deterministic precedence and strict override. Alternative considered: use deprecated `serde_yaml`. Rejected in favor of its actively maintained official fork.

### 10. Define TOON Text Sequences for multi-result transport

Structured `tq` results are transported as a TOON Text Sequence inspired by JSON text sequences. Each result is prefixed by ASCII Record Separator (`0x1e`) and terminated by LF. The framed stream is distinct from an individual canonical TOON document and is unambiguous even when values are multiline.

The CLI offers an explicitly unframed single-document output mode that fails if the filter produces zero or multiple values. Raw output follows jq-style byte/text rules and does not use TOON framing. Sequence input is enabled explicitly; ordinary input remains one TOON document per file.

Alternative considered: separate results with newline or a YAML-like `---` line. Rejected because newline is ambiguous for multiline TOON and a visible separator would invent document syntax that could be confused with data.

### 11. Keep benchmark payloads external and results self-describing

The checked-in `examples/` files are smoke/development snapshots. A benchmark campaign refreshes the natural USGS feeds and the selected natural large source, records a manifest, and generates YAML and TOON equivalents before timing begins. No input is sliced, repeated, padded, or otherwise resized to meet a nominal tier.

The initial natural large source is the Microsoft Georgia building-footprint GeoJSON archive, approximately 1.04 GiB uncompressed. The source is configurable and replaceable through a reviewed manifest without changing harness code.

JSON is the source representation. Generated YAML and TOON documents must round-trip to an equivalent ordered JSON data model before they become benchmark inputs. Native-format comparisons report logical records per second as well as physical input bytes per second; cross-format byte throughput is not presented as a standalone winner.

During development, smoke, standard, and large campaigns run locally and on demand. The large campaign is opt-in because of its time, storage, and memory cost; it does not require a CI service or designated CI runner. Each report's environment manifest identifies its benchmark host, and reports from different host or corpus identities are non-comparable by default. CI scheduling and any future dedicated release runner are deferred until the project needs shared release automation.

Alternative considered: check in fixed generated corpora. Rejected due to repository size, licensing/provenance concerns, and the explicit requirement to recollect natural data.

### 12. Make correctness a benchmark gate

Every performance case identifies a compatibility case or an independently verified result digest. A tool invocation is timed only after its output, error class, and exit status pass the correctness gate for the recorded input manifest. Timeouts, unsupported cases, OOM termination, and limit failures are recorded as outcomes rather than removed from reports.

Small cases emphasize process startup and compilation. Medium and large cases cover parse/drop, scalar extraction, projection, filtering, reduction, construction, update, blocking sort, event streaming where supported, and identity re-encoding where the tool supports the selected output. Reports include wall time, user/system CPU, peak RSS, time to first result where measurable, records per second, physical MiB per second, and output size.

The required input-format matrix is explicit:

```text
Input format   jq   yq   tq
JSON            ✓    ✓    ✓
YAML            —    ✓    ✓
TOON            —    —    ✓
```

JSON supplies the direct all-three comparison because yq and tq both accept JSON. YAML supplies a direct yq/tq comparison. TOON measures tq's native path. Benchmark adapters always pass tq's explicit `--input-format` override so a JSON measurement cannot accidentally exercise the YAML parser and format detection cannot distort parser-specific timing. A tool without native support is recorded as not applicable; conversion is never inserted into a timed command to manufacture support. Separate native-format comparisons continue to compare jq/JSON, yq/YAML, and tq/TOON over semantically equivalent snapshots. Reports distinguish same-input-format results from native-format results.

Alternative considered: benchmark only `.` or suppress all output. Rejected because it would overfit parsing and could reward implementations that skip required evaluation or serialization.

### 13. Ship a progressive compatibility surface

The MVP includes navigation, iteration, composition, literals, construction, conditionals, comparisons, boolean/alternative operators, variables, a bounded core built-in set, errors/optional access, and path updates. User-defined functions, modules, reduce/foreach, advanced regex/date built-ins, and obscure platform-specific behavior are later capabilities.

The CLI publishes `tq compatibility` output derived from the same case manifest used by local and future automated runs. Unsupported syntax fails at compile time with a stable feature identifier and a link/name for the deferred capability.

Alternative considered: parse the full jq grammar and fail at runtime for unimplemented operations. Rejected because accepting syntax creates a misleading compatibility promise and poorer diagnostics.

## Risks / Trade-offs

- **The baseline-first phase delays visible `tq` features** → Keep the harnesses executable without `tq`, publish the jq/yq baseline report, and treat this as the first project milestone rather than preparatory work.
- **jq and yq differ in semantics despite similar syntax** → Classify every case and make jq the explicit semantic target; never silently normalize away ordering, cardinality, types, errors, or exit codes.
- **A 1 GiB campaign is expensive and may fail on some developer machines** → Separate smoke, standard, and large campaigns; make large opt-in locally, record incomplete/timeout/OOM outcomes, and require the recorded host manifest for every claim.
- **Refreshed data makes historical timings non-identical** → Attach manifests and checksums to every report and compare performance only when the corpus identity is compatible or the report explicitly labels the change.
- **TOON output arrays intrinsically require future information** → Use explicit spooling and report it; do not claim all encoding is memory-only streaming.
- **A custom bytecode VM increases initial complexity** → Keep the MVP instruction set small, validate bytecode, provide trace/disassembly tools for tests, and grow opcodes only alongside compatibility cases.
- **Structural sharing can increase pointer and allocation overhead for small values** → Benchmark both startup/small data and large branching workloads; use compact node layouts and avoid reference counting for immediate scalar values.
- **The decimal-literal hybrid may diverge from jq at edges** → Establish baseline cases before arithmetic implementation, preserve literals only where jq does, define explicit resource limits, and reject rather than silently corrupt out-of-envelope inputs.
- **Temporary spooling introduces filesystem and security concerns** → Use securely created files, restrictive permissions, configurable directories and caps, cleanup guards, and no reliance on predictable names.
- **Event and document modes could drift semantically** → Build both from one decoder event contract and require equivalence cases wherever the same logical operation is representable.
- **YAML's data model is wider than jq's JSON-shaped values** → Accept an explicit string-keyed JSON-shaped profile, test YAML scalar resolution and merge behavior against yq, and reject unsupported tags/keys instead of coercing them.
- **`yaml_serde` may erase a numeric scalar's original precision through Serde callbacks** → Run the numeric-fidelity spike before the adapter is admitted, require semantic round trips, and reject or obtain a narrow raw-scalar API rather than rounding silently.
- **Best-effort detection can select a broader grammar or commit before a late error** → Fix and publish TOON → YAML → JSON precedence, bound replay, report the selected parser, and provide a strict `--input-format` override for reproducibility.
- **The local `toon-rust` API may evolve independently** → Pin revisions in development manifests, keep adapters narrow, and upstream general event APIs with conformance tests.
- **Benchmark comparisons can be misleading across formats** → Publish same-format and native-format tables separately; report physical size, logical records, query, output digest, execution mode, and tool version together; never hide conversion inside timing or publish a composite “winner” score.

## Migration Plan

This is a greenfield change, so deployment is progressive rather than migratory:

1. Initialize the standalone repository and Rust workspace without replacing the existing example data.
2. Add corpus manifests, refresh/convert/validate commands, compatibility cases, and subprocess adapters.
3. Run and review jq/yq compatibility baselines.
4. Add the performance harness, run jq/yq baselines, and archive self-describing reports.
5. Introduce `tq-core` value, parser, typed compilation phases, bytecode validation, and VM behind tests.
6. Introduce event-based TOON I/O, DOM construction, framed output, and spooling.
7. Add the CLI progressively, enabling compatibility cases category by category.
8. Run tq against the frozen MVP compatibility matrix, then admit it to performance reports only after correctness gates pass.
9. Publish an MVP compatibility manifest and locally reproducible benchmark report; propose later parity capabilities and shared benchmark automation as separate OpenSpec changes.

Each phase is additive. If a phase regresses correctness, the new capability remains disabled or is reverted without changing previously recorded baselines or corpus manifests.
