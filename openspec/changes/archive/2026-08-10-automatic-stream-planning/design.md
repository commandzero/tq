## Context

The MVP has typed plan markers and event decoders, but only explicit `--stream`
uses them. Ordinary queries compile to eager document evaluation even when a
small projection could execute from decoder events.

## Goals / Non-Goals

**Goals:** Automatically select the least-retaining sound plan, preserve jq
semantics, explain all retention/fallback decisions, and prove bounded memory on
natural large files.

**Non-Goals:** Streaming unsound blocking queries, changing result order, or
making jq/yq ratios universal release gates.

## Decisions

- Extend effect analysis with required path prefixes, subtree completeness, and
  escape/blocking facts; convert only through typestate-checked plan constructors.
- Use event execution for scalar/path-local pipelines and bounded subtree capture
  when complete values are required. Speculative runtime fallback was rejected
  because it can duplicate output after commitment.
- Decide the plan before semantic decoder consumption and record syntax causes,
  retained state, limits, and any rejection in explain/report output. A bounded
  replayable format probe may precede selection.
- Run the bounded, replayable format probe before plan selection in automatic
  input mode. JSON/TOON decoder formats admit bounded plans; detected YAML
  conservatively selects the document plan.

## Risks / Trade-offs

- [Unsound analysis changes semantics] → Differential cases for every admitted shape and default to document plans.
- [Subtree capture grows unexpectedly] → Charge bytes/depth and fail before unbounded retention.
- [Plan complexity hurts startup] → Benchmark compile/startup separately and cache only immutable analysis.
