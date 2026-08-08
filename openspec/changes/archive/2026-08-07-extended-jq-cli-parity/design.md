## Context

The MVP parser intentionally implements a constrained jq-shaped surface plus
TOON/YAML extensions. Remaining switches overlap output, file access, module
paths, argument sources, and terminal behavior.

## Goals / Non-Goals

**Goals:** Add high-value jq 1.8.x scripting parity from executable baselines,
retain stable tq format rules, and reject ambiguous combinations before input.

**Non-Goals:** Undocumented jq internals, backward compatibility with accidental
MVP bugs, or collapsing TOON Text Sequence framing into JSON conventions.

## Decisions

- Add options only through compatibility cases capturing argv, stdin/files,
  stdout bytes, stderr, and exit class. Help text derives from the admitted registry.
- Keep common jq switches semantically aligned; define explicit precedence for
  tq format/framing extensions rather than inferring from filenames.
- Separate pure formatting switches from ambient filesystem/environment helpers
  so library callers can govern capabilities.

## Risks / Trade-offs

- [Short-option clusters conflict] → Parse against a reviewed jq argv corpus before promotion.
- [Terminal/color output destabilizes reports] → Disable ambient detection in harnesses and record explicit mode.
- [File helpers broaden access] → Apply path/resource policy and preserve source identity in diagnostics.
