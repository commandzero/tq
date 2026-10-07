---
type: Report
title: TOON event boundary
description: Decoder event responsibilities and the boundary with query execution.
generated: { by: codex/gpt-6-astra, at: 2026-09-13T16:02:56Z }
---

# TOON event boundary

`tq-toon::Decoder` keeps TOON parsing out of the tq execution engine. It reads
through `BufRead`, keeps one bounded physical line, its pending events, and active
container/schema state, and yields structural events with source spans. A
consumer can stop after any event without building the rest of the document.

The public event types cover document, object, key, array, and scalar
boundaries. An array start carries its declared count. Its end reports the
observed count. The native TOON 4.1 decoder handles UTF-8, BOM/CRLF, full-line
comments, quoting and escapes, indentation, recursive/keyed schemas, delimiter
scope, row width, counts, sibling duplicates, and aggregate resource limits.
Semantic container depth includes implicit row/group objects even when the
consumer discards events. The decoder knows nothing about jq filters or bytecode.

`DomBuilder` consumes these events when a query needs a complete value.
Streaming queries consume them directly. Other consumers can project paths,
skip subtrees, or construct bounded windows.

The ordered DOM writer and event transcode renderer share recursive schema and
scalar rules. Arrays and objects finish shape-dependent preparation before
publishing their selected canonical layout. The event transcode path uses one
result-scoped arena for active indexes, schemas, replay buffers, and publication;
completed bodies can spill to a secure replay file. Active key metadata remains
memory-bounded and may fail admission even when replay disk space is available.

Dotted names, quoted or unquoted, are literal keys. Recursive header groups
construct nested objects explicitly; keyed headers construct ordered entry
objects. `DomBuilder` applies document-order last-write-wins when non-strict
decoding is requested. Direct event consumers that cannot retract values reject
that mode before consumption rather than approximating duplicate semantics.

The CLI's `--stream --non-strict` TOON and TOON-sequence modes stage each
complete document before projecting jq-compatible leaf and container-close
records. Duplicate replacement therefore finishes before any projected value
is emitted; this is not direct incremental event decoding.

`toon-format` 0.6.0 is a pinned conformance oracle, not the production decoder or
writer. Upstream full-document convenience streaming APIs are not used by the
bounded native event path.
