## Acceptance boundary

Reuse `ReviewedDisparity`, `DisparityEvidence::from_report`, and `validate_completion_manual_report`. Do not add a second approval schema, tolerances, automatic approval regeneration, or a normalization layer. Stable validation already checks case fingerprints, complete observations, executable byte sizes/digests, versions, build features, and runtime-library identities while treating installation directories as provenance.

Strict execution remains strict and keeps its existing report paths. Completion selects `tests/compatibility/reviews/release-v<package-version>/disparities-<native-target>.toon`; a missing or stale file fails. The raw strict reports remain evidence, not rewritten passing results. Native manual hosts are pinned to the existing archive recipes (Ubuntu 24.04 x86_64 and macOS 15 ARM64). Linux ARM64 archive validation remains required; there is no invented ARM64/Windows manual approval.

## Scoped review findings

Fresh local macOS source execution binds tq 0.5.0 / TOON v4.4 and official jq 1.8.2. Its nine failures are two implemented safe-libm rounding witnesses, six intentional ANSI quote/span contracts, and jq's internal test-runner suffix. The six ANSI captures have identical underlying bytes after diagnostic SGR removal, matching status and empty stderr; the actual gate will retain and compare the unmodified colored bytes. The supplied test in `manual.invoking.run-tests` passes in both tools; jq additionally runs its C-library internal tests, which tq must not pretend to execute.

Review of `math.rs` identifies a separate inherited Linux x86_64 scale-conversion issue: the common helper saturates invalid C-int conversions and maps NaN to zero. The pinned x86_64 jq wrapper instead exposes the native invalid-conversion sentinel for `int`/`long`. This is not ULP rounding or an inability of safe Rust to reproduce the witnessed contract. Fresh Linux capture must confirm the mismatch before correction. A correction must distinguish C parameter widths, truncate valid fractions, range-check before safe conversion, preserve other target behavior, and cover non-finite/out-of-range boundaries. Its two case IDs must become exact matches, not approvals.

## Approval content and authoring

Each approved case needs its exact cause, practical impact, existing behavior-focused regression coverage, and reconsideration condition. Math approvals are individual input/target/identity witnesses, not function-wide accuracy bounds. Presentation and internal-diagnostic exceptions are explicitly classified in their rationales and do not become safe-library approvals or exact matches.

Author fresh files from complete native reports only after per-case review, using a temporary executable linked to the existing typed evidence API. The authoring tool refuses unknown or non-failing cases and refuses to produce a complete approval set while unapproved failures remain. The tool is temporary; it is not shipped or a new repository convention. Historical jq 1.8.1/TOON v3 approvals remain unchanged.

## Verification and lifecycle

Capture unapproved source-bound reports first. After any implementation fix, recapture both native target identities before approval authoring. Prove actual native completion acceptance, unchanged strict failure, stale/missing/unapproved evidence rejection, and protected ordered data/process/coverage contracts. Complete ordinary preflight and current-head PR review while the change remains active. Re-verify after review, synchronize the delta, archive all artifacts, and pass the committed-head completion gate before tagging or publication.

A compiler, native image/ABI, source/library/reference identity, case, or observation change invalidates the exact approval and requires review. Never refresh a digest merely to make CI green. Neither completion acceptance nor this release establishes fresh calibrated performance or native Windows acceptance.
