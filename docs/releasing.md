---
type: Playbook
title: "Releasing tq"
description: "Prepare, validate, package, and publish a coordinated tq release."
generated: { by: codex/gpt-6, at: 2026-09-07T01:44:50Z }
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
Windows retains its existing regex, date, and ambient-capability source tests,
now before publication. Windows, Intel macOS, and musl archives have no recipe
in this pipeline.
Record measured OS/ABI minimums before promising older-host compatibility.
Ubuntu build success alone does not establish compatibility with older glibc.

To reproduce one native job, use a clean checkout of the tag:

```sh
./scripts/package-release.sh v0.4.0
```

The version above is an example, not a release claim.
The script checks tag/HEAD identity, all package versions, and the dated
changelog section. It runs preflight and platform contracts before a locked
release build. It then extracts the archive and checks the executable version,
license file, and a JSON query result.

Archives use `tq-v<version>-<rust-target-triple>.tar.gz`.
Each archive has `tq`, `LICENSE`, and `release.txt` at its root.
The metadata records tag, commit, compiler, default features, target, and host.
A `.sha256` sidecar records the digest and archive basename, without a builder
path. Existing outputs cause failure rather than replacement.

Download each workflow artifact, verify its sidecar with `shasum -a 256 -c`,
and collect the complete matrix before creating a draft GitHub release.
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
cargo install tq-cli --version '=0.4.0' --locked --root "$install_root"
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
