## Context

See `proposal.md` for the benchmark evidence and motivation. The implementation on `add-streaming-transcode` adds reader-backed JSON structural events and a shared JSON/TOON `EventConsumer` contract. The automatic query path already proves static-prefix iteration and projected paths, but capability analysis selects `blocking-document` as soon as it encounters `sort`. The current Rayon work therefore receives a fully materialized array only after tq has buffered, decoded, converted, evaluated, and retained the complete source document.

The first target is the query family represented by an array constructor over a streamable generator followed by a blocking suffix. The benchmark example is `[.features[].properties.release] | sort | length`. Plain `sort | length` is removable when the compiler proves the input is an array, so an order-sensitive variant must measure sorting performance.

## Goals / Non-Goals

**Goals:**

- Split eligible queries into a decoder-backed producer and one blocking suffix before input consumption.
- Retain only producer results and required blocking state.
- Reuse structural events directly, without allocating jq `[path, value]` records for unrelated leaves.
- Overlap single-thread decoding with eligible parallel blocking preparation through a bounded handoff.
- Preserve jq behavior, deterministic output, duplicate-key rejection, limits, cancellation, and diagnostics.
- Make plan selection, optimizer rewrites, retained state, and worker activity visible in explain and execution reports.

**Non-Goals:**

- Parallel JSON syntax parsing within one document.
- Bounded total memory for operators that must retain every projected result.
- Hybrid execution for mutation, dynamic root dependencies, slurp, arbitrary `reduce` or `foreach`, YAML, or explicit jq event input in the first implementation.
- Changing jq syntax, comparison order, or the runtime value model.
- Using the TOON transcode writer or preparation spool as the query accumulator.

## Decisions

### Add a typed hybrid plan

Add `PlanKind::HybridBlocking` and a typed plan carrying two compiled components:

- `HybridProducer` contains the existing static decoder prefix, optional projection, per-item bytecode, subtree requirement, and source span.
- `HybridSuffix` contains the collection constructor contract and suffix bytecode. It also names any operator-specific preparation strategy.

`AutomaticPlan` gains a hybrid variant instead of disguising the plan as `Subtree` or `Blocking<Document>`. This keeps mode checks and explain output honest. The analyzer attempts hybrid decomposition before the unconditional blocking-document fallback. It selects the hybrid form only if the producer satisfies the existing automatic stream proof and the suffix depends solely on the completed collection.

The initial accepted shape is a streamable generator inside an array constructor, followed by a suffix that consumes that array once. This narrow rule covers the benchmark without claiming arbitrary pipeline splitting. A failed proof records one stable reason and falls back before the decoder consumes semantic input.

Alternative considered: mark every blocking query as a possible event consumer and recover at runtime. That would make fallback depend on partially consumed input and would weaken the typed planning contract.

### Feed the query from structural events

Build hybrid execution on the structural `Event` source added for streaming transcode. JSON uses `decode_json_event_stream`; TOON uses `Decoder::decode_into`. A query consumer tracks container frames, object keys, array indexes, duplicate names, and the current static-prefix relationship.

The consumer reuses the automatic projection rules rather than matching only leaf paths. It closes each direct child under the proven prefix, accounts for absent projected members, and evaluates the producer bytecode in encounter order. It materializes a subtree only when the proof says the item expression requires one. Values outside the proof are not retained.

This path does not use `TranscodeConsumer`. That consumer solves canonical TOON publication and layout preparation. The reusable pieces are the reader-backed decoder, event contract, duplicate-key behavior, limits, and cancellation checks.

The existing JSON jq-stream projector remains for explicit `--stream`. Hybrid execution consumes structural events directly so it does not allocate path arrays and wrapper values for every coordinate in a geometry-heavy document.

For a JSON path that the static proof rejects, the selected decoder uses a
validation-only visitor. It still consumes the complete subtree and enforces
JSON syntax, nesting depth, token length, and tq's numeric envelope, but it
does not canonicalize discarded numbers or construct structural events, jq
values, reference-counted keys, or projector paths. Tracked paths continue
through the ordinary value-producing visitor. This optimization is confined
to the selected automatic JSON path; explicit `--stream` and query-independent
transcoding keep their complete event contracts.

Identity JSON-to-TOON transcode uses lightweight token callbacks on the same
consumer contract. JSON keys and strings remain owned token text, and numbers
are canonicalized once into owned text without first constructing `Number`,
`Scalar`, `Event`, or `Value`. Consumers that do not implement the callbacks
keep the existing owned-event behavior through default adapters. The TOON
decoder also keeps its owned-event path because it has already interpreted its
source tokens.

`TranscodeConsumer` publishes lightweight root and direct-object scalars
immediately. Unknown-length scalar arrays store private scalar replay records
and render them directly during publication, without rebuilding tq values.
Nested containers that require TOON layout or duplicate-key preparation may
still materialize values until their preparation representation becomes fully
structural. This is an allocation reduction, not subtree discard: identity
transcode continues to consume and publish every input value.

Alternative considered: generate jq `[path, value]` records and send them through `AutomaticExecutor`. It would be simpler, but it allocates and evaluates records for millions of irrelevant geometry scalars.

### Use a bounded producer and preparation handoff

Producer evaluation remains on the decoder thread because the VM uses thread-local execution state. Once the producer completes an owned result, the handoff groups results into configured batches. The number of queued batches and their estimated bytes are finite. A full queue blocks the decoder.

The generic strategy appends completed batches to the collection in encounter order. Operators with a sound preparation strategy may use Rayon while decoding continues. The first strategy is stable sort-run preparation:

1. Preserve encounter order within each batch with a stable jq-value sort.
2. Assign each completed batch a monotonic batch ordinal.
3. Keep completed runs in batch order even when workers finish out of order.
4. Perform a left-biased stable parallel merge after the producer closes.
5. Run the remaining suffix bytecode against the completed array.

All semantic evaluation that may fail stays in encounter order on the producer thread. Sort comparison is total and infallible for runtime values. Worker failures are limited to cancellation or resource failures, so they cannot reorder jq errors. No suffix output is published before producer completion.

For operators without an incremental preparation proof, hybrid execution still avoids root materialization but defers parallel work until collection completes. The Rayon size thresholds remain authoritative for choosing serial or parallel work.

Alternative considered: send complete feature subtrees to Rayon and run one VM per worker. The current VM state is not thread-safe, and parallel item evaluation could reorder errors, step accounting, and observable side effects.

### Run resolved-HIR rewrites before capability analysis

Add a small optimizer pass after name resolution and before capability analysis. It may replace built-in `sort | length` with `length` only when the incoming value is proven to be an array. An array constructor supplies that proof. The pass does not rewrite `sort_by`, user functions, unknown inputs, or expressions whose evaluation can fail or emit additional results.

Analysis and explain output operate on the rewritten HIR while retaining a list of source-spanned rewrites. This prevents a removed sort from forcing a blocking plan and prevents benchmarks from claiming they measured an operator that never ran.

Alternative considered: optimize in the evaluator. Planning would still select the expensive blocking-document path, so the rewrite would arrive too late to remove materialization.

### Keep resource classification exact

The explain name is `hybrid-streaming-blocking`. Its retained working set is reported as:

```text
bounded decoder and capture state
+ bounded in-flight batches
+ projected result collection
+ blocking operator state
```

Reports include the producer proof, blocking cause, root-materialized flag, decoder depth, batch count and bytes, retained result count and estimated bytes, sort runs, Rayon worker count, cancellation outcome, and optimizer rewrites. Hybrid reports never claim the event plan's fixed-memory guarantee.

The existing input byte, nesting, token, VM stack, result, output, execution-step, and cancellation policies apply. Add finite configuration for batch values, in-flight batches, and in-flight bytes. Retained projected values remain governed by the existing result and process-level policies until a dedicated retained-value byte limit is introduced.

### Validate semantics before performance

Differential tests compare forced document execution with hybrid execution for empty collections, absent projected fields, mixed jq values, equal-comparing objects, multiple input documents, duplicate keys inside selected and discarded subtrees, malformed late input, limits, and cancellation. Plan tests prove that dynamic paths, mutation, cross-item dependencies, slurp, and unsupported formats fall back before semantic input consumption.

The large benchmark has separate jq, forced single-thread tq, and configured multi-thread tq rows. A sort case must expose sorted content in its result digest and confirm `hybrid-streaming-blocking` plus an executed sort through machine-readable explain output. Reports go to `~/Development/commandzero/tq-benchmarks` and include wall time, user and system CPU, total CPU, peak RSS, worker count, exact command lines, corpus identity, and correctness digests.

## Risks / Trade-offs

- [JSON tokenization remains single-threaded] -> Report decoder and worker phases separately. Treat parallel parsing of independently framed or indexed input as later work.
- [Structural events still visit irrelevant geometry scalars] -> Avoid path-record and root allocations now, then use profiles to decide whether the decoder needs a validation-only scalar path.
- [A discard shortcut could hide malformed or over-limit input] -> Traverse
  every rejected value with a validation-only seed and differential-test
  syntax, depth, string/key token, numeric-envelope, and late-error behavior.
- [Lightweight transcode tokens could bypass canonical output rules] ->
  canonicalize every JSON number under the same `NumberLimits`, apply the
  existing TOON string quoting rules, and byte-compare lightweight transcode
  with forced document execution.
- [Chunked stable sorting can diverge at equal values] -> Use stable in-run sorting, preserve batch ordinals, prefer the left run during equal-value merges, and add differential cases with equal-comparing values that have distinguishable representations.
- [A broad split rule could change jq error timing] -> Admit only the narrow proven collection shape and keep fallible item evaluation serial and ordered.
- [The projected collection can still be large] -> Label it as cardinality-proportional blocking state, enforce existing result limits, expose retained-byte observations, and never advertise fixed memory.
- [The streaming-transcode branch and Rayon edits currently have different bases] -> Integrate the completed transcode branch first, then reapply and test the Rayon changes before implementing the hybrid plan.

## Migration Plan

1. Bring the completed `add-streaming-transcode` implementation into the working branch and resolve the existing Rayon edits without changing behavior.
2. Land typed planning, explain output, and fallback tests while hybrid execution remains unavailable by default.
3. Add structural-event execution and differential tests, then enable the plan for JSON and strict TOON when its proof succeeds.
4. Add bounded batch preparation and stable sort runs behind the existing Rayon thread configuration.
5. Add the resolved-HIR rewrite and optimizer-aware benchmark checks.
6. Run the large benchmark campaign and store the report in `~/Development/commandzero/tq-benchmarks`.

Rollback consists of disabling hybrid selection so every affected query returns to its existing document plan. The document executor and query semantics remain intact throughout the change.
