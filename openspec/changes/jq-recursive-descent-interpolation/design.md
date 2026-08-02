## Context

Recursive descent is a depth-first generator over arbitrary values;
interpolation embeds arbitrary generators inside a string literal. Both stress
continuation ordering and bounded execution.

## Goals / Non-Goals

**Goals:** Match jq traversal/interpolation output, source spans, and errors on
managed stacks with cancellation and depth/work governance.

**Non-Goals:** General user functions, regex interpolation transforms, or
automatic graph-cycle handling for non-tree external values.

## Decisions

- Implement descent as an explicit stack of array indices/object entry cursors.
  Recursive Rust calls were rejected for hostile depth safety.
- Parse interpolation into alternating literal and expression segments, then
  compute the ordered Cartesian generator product using VM forks.
- Charge traversal visits and interpolation combinations to VM work/results and
  retain source spans for every embedded expression.

## Risks / Trade-offs

- [Traversal and interpolation can explode results] → Enforce work/result/output limits before allocation.
- [Escaping differs at lexer boundaries] → Derive golden tests directly from jq bytes and Unicode cases.
- [Deep malformed input fails before traversal] → Keep decoder and VM depth diagnostics distinct.
