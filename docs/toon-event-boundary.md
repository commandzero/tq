# TOON event boundary

`tq-toon::Decoder` separates TOON syntax from the tq execution engine. It reads
through `BufRead`, retains one bounded physical line and active
container/schema state, and yields source-spanned structural events. A consumer
can stop at any event. It does not need to build the remaining document.

The public event vocabulary is intentionally small: document, object, key,
array, and scalar boundaries. Array starts retain declared counts, while array
ends report observed counts. The decoder owns syntax concerns—UTF-8, quoting,
indentation, delimiter scope, row width, count validation, and resource limits.
It does not know about jq filters or tq bytecode.

`DomBuilder` is one consumer, used where a query genuinely requires a complete
value. Streaming query paths should consume events directly. Future consumers
may project paths, skip subtrees, or construct bounded windows.

This repository contains the decoder prototype. After the conformance corpus
and real query consumers verify its contract, the `Decoder`, event types,
configuration, errors, and conformance fixtures can move together to
`toon-rust`. tq can then depend on that package without a consumer change.
This move is outside the MVP critical path.

Safe dotted-path expansion is outside the event layer: keys are emitted exactly
as decoded, together with quoted-key provenance. `DomBuilder` applies the
requested expansion/conflict policy without coupling the reusable streaming
decoder to object merge semantics.
