## Why

Publishing tq 0.5.0 is blocked by the current exact manual release gate. Fresh macOS execution against pinned jq 1.8.2 has 943/952 exact matches, nine differences, and 921/921 TOON matches. Historical approvals target older jq/tq binaries and cannot approve this candidate. The user chose fresh scoped review rather than a blanket waiver or changing intentional presentation behavior to imitate jq bytes.

## What Changes

- Retain exact `compatibility strict` acceptance unchanged. Add an explicit `compatibility completion` release campaign that selects tracked, version/target-specific approvals and uses the existing exact-evidence completion validator.
- Capture fresh observations on the pinned native packaging hosts: Ubuntu 24.04 x86_64 and macOS 15 ARM64. Keep the Linux ARM64 archive validation matrix unchanged; do not claim an unavailable ARM64 jq-manual pin or Windows execution.
- Review safe-library rounding separately from six intentional presentation contracts and the supplied-test runner's internal diagnostic suffix. Preserve every raw observation, exact-match count, existing case, and protected regression. Reject stale, unknown, overly broad, non-executed, or otherwise unapproved failures.
- Investigate the Linux scale-conversion witnesses as implementation defects rather than safe-library rounding. Fix confirmed native conversion mismatches safely, without FFI, then recapture identities before authoring approvals.
- Use the reviewed-completion campaign for native release acceptance only after fresh identity-bound exception review. Full native archive, platform, packaging, registry, and consumer-install checks remain mandatory.

## Impact

Affected capability: `cross-tool-compatibility`. Existing comparator schema and validation are reused. Release scripts/workflows, scoped review metadata, release documentation, and a confirmed target-specific math correction may change. This is not exact jq parity, fresh performance calibration, or Windows compatibility acceptance.

The change stays active through PR review. Synchronize its delta and archive only after clean current-head review and post-review verification. No tag or public package/release is created until the candidate gates pass.
