## Context

The archived TOON 4.1 migration included a manual-comparison campaign, but manual execution and token accounting are independently owned by this sibling change. Its generated results came from the original frozen C9 snapshot and binaries, not a build of this focused branch. The source snapshot is `407b35d81681b8ca4ab85968f7186eea645f250f`, SHA-256 `b0a3e076abc748319859ea9810fd75bc93132b05c5b07cc116f97ffe27ecbc62`; the original native CLI SHA-256 is `8a0ba42cf81881829b93ee950db0d4d3f808a22af574b210484be7833b238d00`, and the rebuilt comparator SHA-256 is `a581e66aa551e2c3397ae4507c6daf8b7a6def331e2ce5697664e3ed134e4cf4`. The original evidence was captured against pinned jq 1.8.2. The source manifest and observations were retained under ignored `target/toon-4-1/candidate-c9/`; they are historical evidence and are not recast as a new branch build or new campaign.

The original strict comparison retained 9 exact differences and exit status 1. It accounted for 952 unique cases, original 518 IDs, and all 303 protected exact contracts. Historical token counts and eligibility belong to those captured observations. This archive preserves that record; it does not assert that this PR ran those gates anew. No fresh comparison or performance campaign is undertaken here.

## Decisions

1. Keep only manual regeneration and complete-stdout token accounting requirements in this scoped change. The native TOON migration's independent compatibility gate remains in the main specification and is not replaced by these requirements.
2. Copy original `7.1`–`7.6` task text verbatim as historical completed acceptance, annotating the source snapshot and preserving original status rather than implying new execution.
3. Preserve C9 provenance and distinguish captured original frozen-binary evidence from any later focused-branch build. `overview.md` retains the historical measured summary; `index.md` remains navigation. Neither claims that a new branch build produced the capture.
4. Keep performance guard requirements and tasks outside this scoped archive.
5. Regeneration matches a capture by its saved native target, explicit `campaign_id`/`captured_at` and measured version/SHA-256 identities. Generation time is not capture identity. Unknown legacy targets match only unknown targets; different native targets remain distinct even with identical binary identities. It updates only that capture's measured tool fields, retains authored run/tool metadata and every other native capture, and appends genuinely distinct captures.
6. Measured tools require nonempty version strings and 64-digit hexadecimal SHA-256 identities before any page is written. Their identity status is `measured`; uncaptured optional identities remain `not-recorded`. Missing historical campaign/time fields remain null.
7. Preserve authored metadata values and Markdown body bytes. YAML presentation may normalize when provenance is updated; subsequent identical regeneration is byte-stable. Do not promise preservation of incidental YAML quoting, whitespace or comments.
8. Native target identity comes from the saved `reference_execution.target` architecture/OS pair. Derive `platform` from its captured OS component; missing targets remain null. Never probe the rendering host to fill historical identity.
9. Prepare a new Markdown directory before resolving source-collection links, but validate all documents before writing any page. Strip obsolete generated Results blocks from navigation without removing authored surrounding content or introducing empty index markers.
10. The render-only CLI requires exactly the review/catalog case-ID union used by the executing manual comparator, rejecting missing, duplicate, unexpected or malformed IDs before creating output or writing pages. This is an integrity gate, not strict acceptance: complete captures with recorded differences still render. Lower-level section rendering remains usable with synthetic test subsets.
11. Live comparison records an explicit capture-completion timestamp after actual tool execution. Historical reports without `captured_at` retain null; `generated_at` remains document-generation metadata and never fills missing capture identity.

## Risks / Trade-offs

- Historical completed checkboxes can be mistaken for a new run; the source annotation and index provenance explicitly identify them as the original captured acceptance record.
- The captured strict exit status is nonzero because exact differences remain; report rendering does not convert it to acceptance.
- Fresh comparison work remains pending whenever required and is not represented as complete here.
