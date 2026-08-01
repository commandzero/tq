# TOON event boundary

`tq-toon::Decoder` is the query-independent seam between TOON syntax and tq's
execution engine. It reads through `BufRead`, retains one bounded physical line
plus active container/schema state, and yields source-spanned structural events.
Consumers can stop pulling at any event; they do not need to materialize the
remaining document.

The public event vocabulary is intentionally small: document, object, key,
array, and scalar boundaries. Array starts retain declared counts, while array
ends report observed counts. The decoder owns syntax concerns—UTF-8, quoting,
indentation, delimiter scope, row width, count validation, and resource limits.
It does not know about jq filters or tq bytecode.

`DomBuilder` is one consumer, used where a query genuinely requires a complete
value. Streaming query paths should consume events directly. Future consumers
may project paths, skip subtrees, or construct bounded windows.

The decoder is prototyped in this repository. After its contract has survived
the conformance corpus and real query consumers, the `Decoder`, event types,
configuration, errors, and conformance fixtures can move together into
`toon-rust`. tq should then depend on that package without changing consumers.
Upstreaming is deliberately outside the MVP critical path.

Safe dotted-path expansion is outside the event layer: keys are emitted exactly
as decoded, together with quoted-key provenance. `DomBuilder` applies the
requested expansion/conflict policy without coupling the reusable streaming
decoder to object merge semantics.
