# TOON event boundary

`tq-toon::Decoder` keeps TOON parsing out of the tq execution engine. It reads
through `BufRead`, keeps one bounded physical line plus active container and
schema state, and yields structural events with source spans. A consumer can
stop after any event without building the rest of the document.

The public event types cover document, object, key, array, and scalar
boundaries. An array start carries its declared count. Its end reports the
observed count. The decoder handles UTF-8, quoting, indentation, delimiter
scope, row width, count validation, and resource limits. It knows nothing about
jq filters or tq bytecode.

`DomBuilder` consumes these events when a query needs a complete value.
Streaming queries consume them directly. Other consumers can project paths,
skip subtrees, or construct bounded windows.

This repository contains the decoder prototype. After the conformance corpus
and real query consumers verify its contract, the `Decoder`, event types,
configuration, errors, and conformance fixtures can move together to
`toon-rust`. tq can then depend on that package without a consumer change.
This move is outside the MVP critical path.

Dotted-path expansion happens outside the event layer. The decoder emits each
key unchanged and records whether it was quoted. `DomBuilder` applies the
requested expansion and conflict policy, so the decoder does not need object
merge rules.
