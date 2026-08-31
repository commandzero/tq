## Context

These built-ins depend on Unicode regex behavior, operating-system time ranges,
environment state, and filesystem/process interfaces. Reproducible compatibility
requires dependency and nondeterminism policy to be explicit.

## Goals / Non-Goals

**Goals:** Match the selected jq reference where portable, classify platform
differences, bound hostile regex/input work, and make ambient access opt-in.

**Non-Goals:** A shell, arbitrary native plugin execution, network I/O, or silent
emulation of platform features that cannot be reproduced.

## Decisions

- Select one Rust regex engine only after differential Unicode/capture baselines;
  record its version and behavioral gaps in reports.
- Represent time conversion through UTC-first typed helpers and isolate local
  timezone/platform calls behind explicit built-ins.
- Admit environment and platform I/O through CLI/library capability policy, not
  implicitly from pure evaluation.

## Risks / Trade-offs

- [Regex engine differs from Oniguruma] → Publish divergence fixtures and reject unsupported constructs explicitly.
- [Time ranges differ by OS] → Test boundaries per release host and use stable range diagnostics.
- [Ambient I/O breaks reproducibility] → Require explicit policy and redact secret values from reports.
