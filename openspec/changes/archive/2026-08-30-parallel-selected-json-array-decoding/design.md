## Context

The hybrid blocking executor already reduces retained memory by selecting `.features[]` records and projecting `.properties.release` before collection. It also parallelizes stable sort batches and merges. Measurements show that sorting and output account for only about 0.25 seconds of a roughly 10-second run; the remaining dominant stage is one `serde_json::Deserializer` walking, validating, discarding, and projecting almost four million array elements serially.

JSON cannot be divided at arbitrary byte offsets because strings, escapes, and nested containers affect structural boundaries. The static selection proof does, however, identify an array whose elements are semantically independent until they enter the ordered VM.

## Goals / Non-Goals

**Goals:**

- Overlap serial array framing, parallel element decoding/projection, and ordered blocking execution.
- Keep queued source bytes and completed results bounded.
- Preserve serial value, path, diagnostic, limit, and cancellation behavior.
- Make eligibility and retention observable.
- Demonstrate the effect with correctness-gated 1-worker and 14-worker measurements.

**Non-Goals:**

- Parallel VM evaluation or unordered result publication.
- Multiple readers for a single file in the initial implementation.
- Parallel decoding for dynamic paths, explicit jq stream mode, JSON Lines, YAML, or TOON.
- Whole-document parallel JSON syntax parsing unrelated to a proven independent array.

## Decisions

### Frame one proven array and decode batches in parallel

A serial structural framer follows the proven static prefix to the target array. At that array it captures bounded batches of complete raw element slices while correctly tracking JSON strings, escapes, and nesting. It continues validating the enclosing document and discarding unrelated subtrees. Each owned batch is submitted to the existing Rayon pool, where the normal selected decoder performs full syntax, numeric-envelope, depth, token, and projection work.

Framing is intentionally lighter than deserialization: it finds element boundaries but does not construct values or interpret numbers. This avoids the duplicate full syntax pass and per-element allocation that `serde_json::value::RawValue` batching would impose. Reading the file into one large byte buffer was rejected because it would surrender the memory reduction already achieved.

### Use bounded ordinal batches

Each batch carries a monotonically increasing ordinal, its first array index, source-position metadata, and an owned byte buffer. Limits on in-flight batch count and byte count apply across queued, running, and completed work. When a limit is reached, the framer drains the earliest result before reading more input.

Workers return records or a structured failure. A small ordinal reorder buffer delivers only the next expected batch. Batch size uses both element-count and byte thresholds so tiny records amortize scheduling while a large record cannot force a large count-based allocation.

### Keep the VM serial

Only decoding and static projection move to workers. The main executor consumes reordered records and runs the fallible VM, collection, and stable sort in the same order as before. This preserves jq error order and avoids adding synchronization to VM state.

### Limit initial eligibility to hybrid-blocking JSON

The first integration requires a proven hybrid-blocking plan, a static array prefix, a static element-local projection, JSON input, and more than one effective worker. Blocking output is not externally committed before successful completion, making rollback and ordered failure behavior straightforward. Unsupported shapes select the established serial decoder before semantic input consumption.

### Preserve the existing decoder as semantic authority

Workers invoke the same selected-record decoder on a synthetic batch array and translate local paths back to original paths. The framer is responsible only for target location and safe boundary discovery. Differential tests compare serial and parallel paths for values, paths, malformed input, duplicate keys, resource limits, and cancellation.

### Benchmark before widening eligibility

The largest catalogued GeoJSON blocking projection is measured with identical binary, input, query, sink, and correctness digest at one and fourteen workers. The candidate remains explicitly gated if parallel overhead fails to improve wall time materially; the captured measurements guide batch sizing or a return to the serial path.

## Risks / Trade-offs

- [The serial framer remains the throughput ceiling] → Keep framing lexical and allocation-light, measure its CPU share, and consider parallel readers only if storage plus framing saturates one core after worker decoding is removed.
- [Batching changes source positions] → Carry batch start line, column, and byte offset and translate worker diagnostics; differential-test malformed input near every boundary.
- [A large early element can stall ordered delivery] → Bound byte size, apply backpressure to total retained bytes, and preserve deterministic order rather than allowing later publication.
- [Rayon oversubscription with parallel stable sort] → Use the shared pool and bounded producer scheduling; decode work finishes before the final merge-heavy phase for the initial blocking plan.
- [Lexical framing accepts syntax the decoder later rejects] → Treat framing as boundary discovery only; the selected decoder remains responsible for semantic validation.

## Migration Plan

1. Add the parallel path behind static eligibility and retain serial fallback.
2. Land differential and resource tests before enabling benchmark comparisons.
3. Record the large-file candidate measurements and tune bounded batch defaults.
4. Disable eligibility without data migration if correctness or performance gates regress.
