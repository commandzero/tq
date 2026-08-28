## Context

See `proposal.md` for the benchmark motivation. The current fast path is still a
document path: JSON deserializes into `serde_json::Value`, converts into the
Arc-backed runtime value, runs the VM, renders TOON into `Vec<String>`, and joins
the lines. `PreparedArray` already enforces an 8 MiB memory threshold and secure
disk transition, but the CLI writer does not use it and its replay format passes
through JSON again.

The existing planner chooses event, subtree, document, whole-input, or blocking
execution from query analysis. Transcode differs because eligibility depends on
the query, decoder behavior, output format, framing, and writer options. JSON
also accepts duplicate object names with last-value semantics, while strict TOON
rejects duplicate paths. A direct object writer cannot treat those policies as
the same.

## Goals / Non-Goals

**Goals:**

- Bound identity-conversion memory without slowing general jq programs by routing
  them through the new path.
- Keep transcode output byte-for-byte equal to forced document output for valid
  inputs and keep decoder-specific duplicate behavior.
- Give every active container one shared preparation budget and one replay source.
- Make disk cost and partial-output behavior visible beside RSS and CPU results.

**Non-Goals:**

- Stream arbitrary jq bytecode or replace event and subtree plans.
- Add direct YAML transcode in this change.
- Preserve input whitespace, JSON number spelling beyond numeric value, or TOON
  layout choices when canonical output differs.
- Support safe key folding in the first transcode plan. It falls back to document
  mode because collision checks need sibling knowledge.
- Promise that bounded memory means no temporary disk traffic.

## Decisions

### Split query proof from I/O plan selection

Query analysis will record a narrow `semantic_identity` capability when the
resolved program emits its input unchanged exactly once. The first implementation
will prove normalized `.` only. It will not guess that arbitrary equivalent
programs are identity.

After format detection and before semantic decoding, the CLI planner combines
that proof with decoder and writer capabilities. It constructs a typed transcode
plan only for JSON or TOON input, TOON output, structured identity output, and
supported writer options. Null input, slurp, explicit jq stream mode, raw or join
output, safe key folding, and non-identity programs keep their existing plans.

This keeps output details out of query analysis and prevents the VM from being
entered accidentally. Hiding a shortcut inside identity bytecode was rejected
because explain output and resource limits would describe the wrong plan.

### Transcode structural events, not jq stream values

Add a query-independent structural consumer shared by JSON and TOON adapters. It
will receive document boundaries, container boundaries, ordered keys, and exact
scalars. Decoder capability metadata will state the duplicate-key policy,
declared array information, and whether a late structural failure can follow an
emitted event.

The transcode consumer will not use jq's `[path, value]` stream representation.
That representation allocates path arrays and loses information useful to the
writer. Explicit `--stream` remains a separate user-visible data model.

### Replace line collection with a sink-backed TOON writer

Refactor the TOON encoder around a sink that writes indentation, keys, scalar
tokens, delimiters, and line boundaries directly to `Write`. Traversal over an
existing `Value` and traversal over structural events will share scalar quoting,
number formatting, key rendering, and layout code.

The sink will track whether it has written a line so it can insert LF before the
next line without adding a document-internal trailing newline. `encode()` may
remain as a compatibility wrapper over a `String` sink, but CLI output must call
the sink-backed API. This removes the complete output copy for document plans
before transcode is enabled.

### Use one preparation arena per result

Replace independent `PreparedArray` buffers with a result-scoped preparation
arena. The arena owns the aggregate memory ledger, secure spool files, byte
limits, and high-water observations. Active object and array frames borrow from
that same ledger, so nesting cannot multiply the configured threshold.

The replay format will be a private length-framed structural format containing
keys, exact scalars, and container boundaries. It will not serialize
`tq_core::Value` to JSON or parse JSON during replay. Each pending element or
member has one replay record. Layout analysis stores only count, scalar status,
candidate field order, and the offsets needed for replay.

For arrays, the frame tracks count and scalar, tabular, or expanded eligibility.
It selects one layout at close and replays the stored events once. It never keeps
pre-rendered primitive, nested, and tabular alternatives at the same time. This
is where the design intentionally differs from streaming `toon`.

### Respect duplicate-key policy instead of copying direct object output

Strict TOON rejects duplicate paths. Its object frames may write completed
members to a direct sequence sink because a later duplicate correctly turns the
current record into a failed, incomplete record.

JSON document decoding keeps the last value for a duplicate key while retaining
the key's first insertion position. JSON transcode must therefore prepare each
object until it closes. The preparation arena records member value ranges and a
bounded key index. In-memory index chunks spill as sorted runs. At object close,
a merge chooses each key's last value range and original position, then replays
survivors in document-model order. Unique wide objects still avoid a DOM, but
they may use temporary disk.

Emitting JSON members immediately and rejecting a later duplicate was rejected.
It would make `tq -i json '.'` depend on the selected output plan. Globally
rejecting duplicate JSON names was also rejected because it would change current
jq-compatible behavior.

### Make publication depend on framing

TOON Text Sequence mode writes the RS prefix when a result becomes publishable.
A decoder with rejecting duplicate semantics may then write safe object members
directly. A normalizing decoder publishes a prepared container only after it
closes. A later failure may leave the current framed record incomplete, while
earlier records remain valid.

Unframed mode keeps the current atomic behavior. The sink targets a publication
buffer backed by the same memory ledger and secure spool policy. After input
success and exactly-one-result validation, it replays the completed bytes to the
real output. Zero results, a second result, or a late error publish nothing.

Buffering every sequence result was rejected because it removes time-to-first-byte
for duplicate-rejecting inputs. Publishing unframed bytes before cardinality is
known was rejected because current tests promise no output on cardinality failure.

### Remove transient JSON trees from retained document plans

Implement `Deserialize` for `tq_core::Value` with a visitor that builds strings,
arrays, and ordered objects directly. Implement `Serialize` by matching runtime
variants directly instead of calling `to_json()`. Keep explicit conversion
helpers for callers that request a `serde_json::Value`.

This work does not make document execution streaming, but it removes one tree on
input and one tree on serialization. It also gives the preparation arena exact
scalar serialization without hidden DOM allocation.

### Treat observability as part of the plan contract

Extend plan kinds and explain output with `transcode`. Human and JSON reports will
record the identity proof, decoder duplicate policy, commitment mode, aggregate
memory high-water bytes, object-index spill, array preparation, spool bytes
written and replayed, and fallback causes. Forced-document execution will remain
available to differential tests and benchmarks through a test or benchmark-only
override rather than a new public CLI mode.

## Risks / Trade-offs

- [JSON duplicate normalization causes disk traffic on wide objects] -> Report
  spool bytes and compare CPU and wall time with forced document execution. Do not
  present low RSS without the I/O cost.
- [Nested preparation accidentally multiplies memory] -> Put allocation behind
  one arena ledger and add adversarial nested-array and nested-object tests that
  assert the aggregate high-water mark.
- [The event writer drifts from the document writer] -> Share token rendering and
  run byte-for-byte differential fixtures, generated values, malformed inputs,
  all delimiters, indentation settings, and empty containers.
- [External key-index merging is complex] -> Build it as an isolated replay-store
  component with deterministic sorted-run tests before enabling JSON transcode.
- [Spool exhaustion turns a formerly successful document query into a resource
  error] -> Apply documented finite limits, name the exhausted limit, clean up,
  and allow users to raise the limit or force the document plan through library
  configuration.
- [Direct sequence output can leave an incomplete final record] -> Emit no success
  claim, keep diagnostics on stderr, and record commitment mode in explanation.

## Migration Plan

1. Land the sink-backed writer and direct runtime-value Serde implementations
   behind existing APIs. Keep every query on its current plan and verify output
   byte stability.
2. Replace `PreparedArray` with the shared preparation arena and connect it to
   value-based array output. Exercise memory, spool limits, cleanup, and replay.
3. Add structural transcode events, typed planning, explain fields, and forced
   differential tests. Keep automatic transcode disabled by one internal switch.
4. Enable TOON identity transcode, then JSON identity transcode after duplicate
   normalization and natural-corpus gates pass.
5. Remove the internal switch after the accepted benchmark campaign. Rollback
   remains a planner change that selects document mode; no data migration or
   persistent spool format compatibility is required.
