#!/bin/sh
set -eu

repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

cargo_bin=${CARGO:-cargo}
openspec_bin=${OPENSPEC:-openspec}

"$cargo_bin" fmt --all --check
"$cargo_bin" check --workspace --all-targets
"$cargo_bin" clippy --workspace --all-targets --all-features -- -D warnings
"$cargo_bin" test --workspace
OPENSPEC_TELEMETRY=0 "$openspec_bin" validate --all --strict
