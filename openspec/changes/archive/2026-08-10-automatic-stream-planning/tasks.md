## 1. Planning Contract

- [x] 1.1 Add jq differential and retention goldens for every candidate query shape
- [x] 1.2 Extend HIR effects with path-prefix, subtree-completeness, escape, and blocking proofs
- [x] 1.3 Add typed automatic event/subtree/document/whole-input/blocking plan selection tests

## 2. Execution

- [x] 2.1 Lower eligible navigation, projection, and selection pipelines onto decoder events
- [x] 2.2 Implement byte/depth-bounded subtree capture and deterministic pre-input fallback
- [x] 2.3 Publish proof causes, retention, limits, and high-water observations through explain/reports
- [x] 2.4 Select decoder-event plans for bounded auto-detected JSON/TOON while retaining YAML document fallback

## 3. Release Evidence

- [x] 3.1 Add hostile-boundary, cancellation, partial-output, and plan-soundness fuzz/property tests
- [x] 3.2 Run complete compatibility plus natural-large RSS/throughput regression campaigns
- [x] 3.3 Record a manifest-pinned natural-large peak RSS measurement through the default auto-format path
