---
type: Policy
title: "Contributor checks"
description: "Local validation, OpenSpec completion, and required PR checks."
generated: { by: codex/gpt-6, at: 2026-09-07T01:44:50Z }
---

# Contributor checks

Run `./scripts/preflight.sh` before submitting executable changes.
For documentation-only changes, use `./scripts/preflight.sh --docs`.
Both commands validate the complete documentation bundle and repository tooling.
The full command also checks Rust formatting, compilation, Clippy, tests,
compile-fail doctests, and the main OpenSpec specifications.

## Install the pinned tools

The development compiler is pinned in `rust-toolchain.toml`. Install rustup,
Node.js 22 or newer, Go, and ShellCheck, then run from the repository root:

```sh
source scripts/tool-versions.sh
cargo install okf --version "$OKF_VERSION" --locked
npm install --global "@fission-ai/openspec@$OPENSPEC_VERSION"
GOBIN="$PWD/target/tools/bin" go install "github.com/rhysd/actionlint/cmd/actionlint@v$ACTIONLINT_VERSION"
```

The Bash scripts support Bash 3.2. The dependency-free Rust repository checker
avoids a separate scripting runtime or dependency graph for governance checks.
Tool versions live in `scripts/tool-versions.sh`; update local and CI usage together.

## OpenSpec completion

Each PR body must contain exactly one association field:

```text
OpenSpec changes: none
```

Replace `none` with comma-separated change IDs whenever implementation belongs
to those changes, even when the artifacts were committed earlier.
Reviewers verify that association and any no-spec-deltas explanation.

Check out the PR head and run against its target branch:

```sh
PR_BODY='OpenSpec changes: example-change' \
  ./scripts/check-openspec.sh origin/main HEAD
```

The checker uses the merge base, examines both sides of renames, and checks
committed state. Unrelated active changes do not block the PR.
Every selected change must leave the active directory and retain its artifacts
in one date-prefixed archive. Archived tasks must be complete.

Added and modified requirements must match the main specification, including
scenario bodies, after whitespace normalization. Removed requirements must be
absent. Renames must remove the old name and supply the new name. A renamed
requirement with changed behavior must also have a MODIFIED requirement under
its new name. Unsupported delta syntax fails with a diagnostic.

A change with no specification deltas must include `no-spec-deltas.md` in its
archive. Explain why no product requirement changes. The PR reviewer must
approve that explanation. The checker does not claim to automate this judgment.

The command also runs strict main-spec validation with the pinned OpenSpec CLI.
It never synchronizes or archives on the contributor's behalf.

## GitHub enforcement

The PR workflow runs on every PR, including documentation, workflow, and body
edits. It uses read-only permissions and cancels superseded runs.
Require the `Repository preflight` check on main, with strict up-to-date checks,
and require a PR review. Strict mode forces a head update and rerun when the
base branch changes. Apply these settings after the workflow lands and its first
successful run exists; configuration files alone do not enable branch protection.

No remote setting changes when running local checks.
Maintainers must activate the required check
when adopting this workflow. The separate release-candidate workflow must pass
on every listed native host before publishing archives.
