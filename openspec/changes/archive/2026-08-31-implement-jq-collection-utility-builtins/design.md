## Context

The resolver has a static built-in registry, and `eval.rs` dispatches calls by name. Pure value filters fit that model. Filter-taking builtins need evaluator access so they can preserve generator cardinality and lexical environments. Path assignment already has internal path collection and persistent creation helpers, but ordinary builtins cannot call that machinery yet.

The CLI currently decodes one document and creates one `Vm` at a time. This works for independent filters but gives `inputs` no way to pull the remaining source sequence. See the proposal and delta specs for the required behavior.

## Goals / Non-goals

Goals:

- Keep one versioned source of truth for builtin names, arities, and blocking classification.
- Reuse jq comparison, path, JSON, number, and resource-limit policies already implemented by tq.
- Preserve lazy generator behavior for `limit`, `any`, `all`, `paths`, `tostream`, and `inputs`.
- Give the evaluator controlled access to remaining decoded inputs without coupling `tq-core` to files or stdin.
- Aim for no more than 2.0 times jq's median wall time and 1.5 times jq's maximum observed peak RSS on representative, correctness-equivalent JSON workloads.

Non-goals:

- Loading jq's external `builtin.jq` at runtime.
- Adding `input/0`, optional overloads not named in issue #5, labels, or `break`.
- Changing tq's exact-number envelope or its documented JSON duplicate-key policy.
- Optimizing these filters into event, subtree, hybrid, or direct-transcode plans in the first implementation.

## Decisions

### Register the filters as tq builtins

Add the issue #5 signatures to `BuiltinRegistry` and keep their implementation in tq. Do not inject an implicit jq source prelude. Native registration preserves compile-time arity errors, planning metadata, stable diagnostics, and VM accounting. It also avoids changing name shadowing and module metadata with hidden user definitions.

Most scalar and collection mechanics should live in focused pure helpers outside the evaluator dispatch. The evaluator remains responsible for filter arguments, lexical environments, short-circuiting, and emitted-result order.

An embedded jq prelude was considered because jq defines several of these names in its standard library. It was rejected for this batch because `limit`, path capture, and `inputs` still need evaluator primitives, while a mixed hidden-prelude model adds another resolution layer without removing that work.

### Reuse one path representation and one path walker

Extend the existing `Path` and `PathComponent` support with checked conversion to and from jq path arrays. Factor depth-first traversal into a pull-driven walker shared by `paths` and `tostream`. Expose the assignment path collector to `path(expression)` and reuse `replace_or_create` for `setpath`.

This keeps path validity and creation semantics aligned with update assignment. Independent path implementations were rejected because negative indices, null ancestor creation, optional access, and object encounter order would drift.

### Add a core input-provider boundary

Give document evaluation an optional pull-based input provider. The provider returns the next decoded `Value` or a classified source error. `tq-core` owns only the small provider interface. `tq-cli` owns files, stdin, format detection, proxy rules, source metadata, and the shared cursor.

The CLI creates one provider for the ordered invocation source set. Its outer evaluation loop and every `inputs` call pull from that same cursor. This is the key invariant: once `inputs` advances the provider, the outer loop cannot see those documents again. Unit tests can supply an in-memory provider without CLI I/O.

Preloading all documents into a VM variable was rejected. It changes memory behavior, delays parse errors, breaks streaming source consumption, and duplicates values for large input sets. Letting the evaluator open files was also rejected because it would put CLI policy and I/O inside `tq-core`.

### Preserve pull-driven short circuiting

Implement `limit` as a wrapper around downstream emission so it stops its child generator after `n` successful results. Implement `any` and `all` by pulling generator results one at a time, applying the condition, and stopping on the first decisive truth value. Errors produced before the decision remain observable; work after the decision does not run.

Do not collect generator output into a temporary vector. Collection would make infinite or large generators unusable and violate the resource contract.

### Use existing value and codec policies

`group_by`, keyed extrema, and predicate filters use the evaluator's existing jq total comparison. `tojson` and `fromjson` call shared JSON value codecs configured with the same numeric, nesting, duplicate-key, and byte limits used elsewhere. Character filters operate on Unicode scalar values while `ascii_downcase` changes only `A` through `Z`. Math filters use the existing jq binary64 arithmetic boundary and reject non-numbers through the normal runtime type diagnostic.

### Start with document-plan eligibility

Mark full-array transforms as blocking. Keep path walkers, predicates, `limit`, and `inputs` pull-driven inside the document evaluator, but treat every query using these new builtins as ineligible for automatic event, subtree, hybrid, and transcode plans until each planner has an explicit proof. This trades some immediate performance for correct plan selection.

### Measure a soft jq-relative performance target

Add representative same-format JSON benchmarks for collection transforms, path traversal, JSON conversion, short-circuit generators, scalar utilities, and `inputs`. Compare release builds on the same recorded host with identical inputs, equivalent queries, output sinks, warmups, and sample counts. Use the median valid sample for wall time and the maximum observed peak RSS. The target is a wall-time ratio no greater than 2.0 and a peak-RSS ratio no greater than 1.5 relative to jq.

These are diagnostic targets, not acceptance gates. Correctness, bounded resource behavior, and tq's existing self-regression policy remain hard requirements. A target miss stays in the report and creates an optimization lead, but does not encourage workload-specific shortcuts or block the compatible implementation.

## Risks / Trade-offs

- [The shared input provider makes VM execution stateful across top-level documents] -> Keep it optional, single-owner, and pull-only. Document the cursor invariant and test nested calls, early termination, errors, and exhaustion.
- [Path capture and update assignment can diverge] -> Use the same collector, component conversion, and creation helper for both features.
- [A large batch can hide semantic gaps behind happy-path tests] -> Add one compatibility case per supported arity plus targeted ordering, multiplicity, empty, invalid-type, and short-circuit cases.
- [Native implementations may differ from jq's source-level standard library on generators] -> Compare normalized result sequences and errors against jq 1.8.x, not only final arrays.
- [JSON conversion could bypass tq's resource envelope] -> Route both filters through bounded shared codecs and add limit tests.
- [Small jq workloads can make process noise dominate ratios] -> Use the repository's warmup and sampling policy, report dispersion, and mark cases incomparable when the host or required metrics do not support a sound ratio.

## Migration plan

Land registry, evaluator, provider, and tests together so no new name resolves to an unimplemented dispatch branch. Update reviewed compatibility evidence after jq and tq pass the new cases, then record the comparative benchmark report. Rollback removes the signatures and their cases as one unit; there is no stored-data migration or new dependency.
