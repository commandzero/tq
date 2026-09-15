#!/bin/sh
set -eu

repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

cargo_bin=${CARGO:-cargo}
./scripts/repository-check.sh
if [ "${1:-}" = --docs ]; then exit 0; fi

"$cargo_bin" fmt --all --check
"$cargo_bin" check --workspace --all-targets --locked
"$cargo_bin" clippy --workspace --all-targets --all-features --locked -- -D warnings
"$cargo_bin" test --workspace --locked
# Main specifications are the repository contract. Associated active changes are
# selected and checked separately by openspec-check.sh at the PR boundary.
./scripts/openspec-check.sh
