## 1. Benchmark setup and fixtures

- [x] 1.1 Finalize the 5-group inventory against public APIs on the candidate and v0.3.0; deliver a boundary table identifying owning crates, included costs, historical adapters, and any non-comparable cases.
- [x] 1.2 Select and lock a Rust 1.95-compatible Criterion development dependency and explicit benchmark targets; verify optimized target compilation and that the normal release dependency graph excludes Criterion.
- [x] 1.3 Add deterministic nested and tabular-compatible JSON/TOON fixtures at 16 and 1024 items; verify equivalent logical values, independent expected ordered stream/results, and accepted, rejected, and container-end coverage through focused correctness checks.

## 2. Event-stream measurement groups

- [x] 2.1 Add compiled event-plan execution cases for the exact catalog query, including VM setup, synchronous evaluation, effect drain, and teardown; verify smoke correctness and that fixture decoding and compilation occur outside timing.
- [x] 2.2 Add separate JSON and TOON structural event-decoding groups with bounded observable consumers; verify expected event sequences and filtered smoke runs for both sizes and shapes.
- [x] 2.3 Add JSON and TOON jq stream-record production groups; verify ordered path/value records before timing and confirm the measured loop includes record allocation and destruction.
- [x] 2.4 Add result encoding and sequence-framing cases for explicit unframed TOON and JSON output with a bounded observable writer; verify exact expected output bytes outside timing and sequence completion inside timing.
- [x] 2.5 Add in-process runner cases for both input formats using equivalent explicit output settings; verify exact ordered output and document preparation/compilation costs included by the existing runner boundary.

## 3. Reproduction and CI

- [x] 3.1 Document full, filtered, smoke, and baseline-comparison commands, stable IDs, units, timing boundaries, build profiles, and fixture policy; verify the documented commands against the new targets.
- [x] 3.2 Connect optimized benchmark compilation and bounded correctness/smoke execution to the shared preflight path used by CI; verify a local run passes and no timing threshold controls CI success.
- [x] 3.3 Prepare isolated historical and candidate benchmark runs with equivalent release optimization settings; verify adapter patches, fixture hashes, production lock identities, and benchmark boundary parity before comparing results.

## 4. Initial evidence and completion

- [x] 4.1 Record a controlled same-host v0.3.0/candidate comparison, starting with per-event execution and covering every comparable group; retain raw samples, dispersion, confidence intervals, host/build/source identities, commands, and reasons for excluded comparisons.
- [x] 4.2 Deliver an attribution summary separating measured stage changes from hypotheses and stating that native wall-time/RSS validation remains under #48; verify it makes no cross-host, additive stage-budget, or process-memory claims from Criterion results.
- [x] 4.3 Run repository preflight and strict OpenSpec validation, then verify the delivered inventory and evidence satisfy every added requirement; retain command outcomes and any unresolved limitations before synchronization and archival.
