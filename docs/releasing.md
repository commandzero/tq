---
type: Playbook
title: "Releasing tq"
description: "Prepare, validate, package, and publish a coordinated tq release."
generated: { by: codex/gpt-6-astra, at: 2026-09-15T18:15:13Z }
---

# Releasing tq

All 4 public crates release together. The workspace manifest is the version
source. Use `v<version>` tags and preserve existing tags and published bytes.
The internal test-support crate is never published.

## Prepare a release PR

1. Choose the version from the public CLI, format, and library changes.
   Before 1.0, incompatible changes use the next minor version.
2. Update the workspace version and all versioned tq path dependencies together.
   Refresh Cargo.lock and create a dated changelog section from Unreleased.
   Update comparison links and leave a fresh Unreleased section.
   Select the canonical production source epoch in `scripts/release-build-env.sh`
   before native capture; keep it fixed through metadata-only review commits.
3. Record compatibility changes, Rust minimum changes, and upgrade actions.
   Follow the [changelog policy](changelog-policy.md).
4. Pass the [contributor checks](contributor-checks.md), including this PR's
   OpenSpec completion check. Review and merge the release PR before tagging.
5. Create the version tag on the reviewed release commit. Do not move an existing tag.

## Validate native archives before publication

Run the Release candidate checks workflow with the reviewed tag.
The workflow builds each target natively and retains verified archives as
workflow artifacts. It does not publish a GitHub release or registry package.

| Candidate target | Validation host |
|---|---|
| aarch64-apple-darwin | macos-15 |
| x86_64-unknown-linux-gnu | ubuntu-24.04 |
| aarch64-unknown-linux-gnu | ubuntu-24.04-arm |

This is the candidate validation matrix, not evidence that a given tag has
passed. Do not advertise a target until that release's run succeeds.
The reusable native contract workflow retains main's reviewed jq provisioning,
shell invocation tests, native math regressions, and explicit reviewed-completion
manual gate on Linux x86_64 and macOS ARM64. It runs against the candidate tag
before publication. Windows execution remains deferred; its source tests remain
in the repository. Windows, Intel macOS, and musl archives have no recipe here.
Record measured OS/ABI minimums before promising older-host compatibility.
Ubuntu build success alone does not establish compatibility with older glibc.

### Review the manual acceptance scope before tagging

`./scripts/campaign-run.sh compatibility strict` remains an independent exact
audit: every jq difference fails, including reviewed exceptions. The separate
`./scripts/campaign-run.sh compatibility completion` command selects
`tests/compatibility/reviews/release-v<workspace-version>/disparities-<native-target>.toon`.
Missing version/target files, unapproved differences, stale fingerprints or
executable/library identities, changed raw observations, skipped cases, crashes,
timeouts, and coverage regressions fail closed. Other targets cannot inherit
these approvals.

Approve only individually investigated safe-library limitations or separately
reviewed native presentation/internal-diagnostic exceptions. Correct safely
reproducible implementation defects instead of approving them. Preserve
underlying values, result order/count, framing, required stderr, status, all
303 protected baseline contracts, and the complete 952-case inventory.
Do not normalize captures, introduce blanket numeric/ANSI tolerances, regenerate
approvals during CI, or describe exceptions as exact jq matches.

Record fresh native source/run provenance, exact identities and observations,
causes, practical impact, regression coverage, and reconsideration conditions in
the versioned review files and [manual coverage report](tests/jq-manual/coverage.md).
The release workflow must use the same native host classes, explicit native
Cargo target, and default-feature release profile as the archive recipes.
A completion pass does not replace the complete native archive matrix,
registry dry runs, consumer installation checks, or OpenSpec/review completion.

Timestamp-bearing native dependencies use the shared release build environment.
Mimalloc embeds C compilation date/time in reachable diagnostics, so release
entry points and the native workflow export the same `SOURCE_DATE_EPOCH` before
any bootstrap or candidate compilation. This denotes the reviewed source epoch,
not the actual build/run time or a changing HEAD timestamp.
The accompanying reserved CFLAGS epoch key is required: cc forwards the epoch
but does not track it, so the key invalidates warm allocator caches when the
policy changes. Preserve the key, do not override it, and use a fresh environment
if an inherited key conflicts. Independent clean native-profile builds must
produce identical executable digests before approvals are frozen.

To reproduce one native job, use a clean checkout of the tag:

```sh
./scripts/release-package.sh v0.5.0
```

The version above is an example, not a release claim.
The script checks tag/HEAD identity, all package versions, and the dated
changelog section. It runs preflight and platform contracts before a locked
release build. It then extracts the archive and checks the executable version,
license file, and a JSON query result.

Archives use `tq-v<version>-<rust-target-triple>.tar.gz`.
Each archive has `tq`, `LICENSE`, and `release.txt` at its root.
The metadata records tag, commit, compiler, source-date epoch, default features, target, and host.
A `.sha256` sidecar records the digest and archive basename, without a builder
path. Existing outputs cause failure rather than replacement.

Before tagging, exercise the archive boundaries with a locally built candidate:

```sh
cargo build --release --locked -p tq-cli --bin tq
TQ_PACKAGE_SMOKE_BINARY="$PWD/target/release/tq" ./scripts/release-package-test.sh
```

This disposable 0.5.0 fixture packages the real executable and license, checks
the extracted version/query/checksum, and tests invalid identities and
non-clobbering publication. Compiler/preflight steps remain fixture shims;
this smoke does not replace native release checks or authorize a release.
Its temporary fixture tags do not tag the product repository.

Download each workflow artifact and verify its sidecar with `shasum -a 256 -c`.
For each manual target, also extract `tq` and compare its executable SHA-256
with that version/target's reviewed identity; a mismatch blocks publication
and requires source/profile/identity reconciliation, not a digest-only update.
Collect the complete verified matrix before creating a draft GitHub release.
Use the curated changelog section as its notes. Mark prereleases explicitly.

## Publish packages and install checks

Publish in this dependency order:

```sh
cargo publish --locked -p tq-core
cargo publish --locked -p tq-toon
cargo publish --locked -p tq-formats
cargo publish --locked -p tq-cli
```

Run each command's `--dry-run` first. Later crates can require the newly
published dependency versions to be visible in the registry before their dry
runs succeed. Do not use `--no-verify` to bypass package verification.

After each publication, confirm the registry version before proceeding.
After tq-cli publishes, test the consumer installation in a temporary root:

```sh
install_root=$(mktemp -d)
cargo install tq-cli --version '=0.5.0' --locked --root "$install_root"
"$install_root/bin/tq" --version
printf '{"answer":42}\n' | "$install_root/bin/tq" -i json -o json -c '.answer'
```

Replace the example version with the selected version. Require output `42`.
Publish the draft GitHub release only after the complete matrix and registry
installation checks pass. Update any Homebrew consumer through a reviewed tap
PR after verifying archive hashes, layout, and installation on its supported hosts.

## Recover a partial failure

If a crate already published successfully, verify its version and continue at the
first unpublished dependent crate. Never attempt to replace the registry version.
If a source correction is needed, prepare a new version and release PR.

If a build or upload fails, keep the GitHub release in draft. Retain successful
artifacts and diagnostics. Compare existing hashes before retrying an upload;
accept identical bytes or fail on a differing artifact. Do not use `--clobber`.
Coordinate future archive/tag naming changes with tap URL builders and installers
in the same release plan. Preserve historical tags and downloads.
