#!/bin/sh
set -eu

repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"
cargo_bin=${CARGO:-cargo}

# Criterion's test mode runs each case once, including its correctness oracle.
# cargo bench uses the optimized bench profile even with --test.
"$cargo_bin" bench --locked --bench event_stream \
    -p tq-core -p tq-toon -p tq-formats -p tq-cli -- --test
