#!/bin/sh
set -eu

repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

cargo_bin=${CARGO:-cargo}
openspec_bin=${OPENSPEC:-openspec}
okf_bin=${OKF:-okf}

"$cargo_bin" fmt --all --check
"$cargo_bin" check --workspace --all-targets
"$cargo_bin" clippy --workspace --all-targets --all-features -- -D warnings
"$cargo_bin" test --workspace
if ! "$okf_bin" --version 2>/dev/null | rg -Fq "okf 0.2.7 "; then
    echo "Install the pinned documentation validator: cargo install okf --version 0.2.7 --locked" >&2
    exit 1
fi
"$okf_bin" validate docs/
if [ "$(OPENSPEC_TELEMETRY=0 "$openspec_bin" --version 2>/dev/null)" != "1.11.0" ]; then
    echo "Install the pinned OpenSpec CLI: npm install --global --save-exact @fission-ai/openspec@1.11.0" >&2
    exit 1
fi
OPENSPEC_TELEMETRY=0 "$openspec_bin" validate --all --strict
