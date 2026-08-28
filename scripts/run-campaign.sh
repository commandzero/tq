#!/bin/sh
set -eu

campaign="${1:-}"
profile="${2:-}"

# Keep benchmark outputs in the sibling archive checkout when it is present.
# TQ_BENCHMARK_ARCHIVE_ROOT can override discovery for CI and other layouts.
benchmark_archive_root="${TQ_BENCHMARK_ARCHIVE_ROOT:-}"
if [ -z "$benchmark_archive_root" ]; then
    search_root=$PWD
    while [ "$search_root" != "/" ]; do
        candidate="$search_root/tq-benchmarks"
        if [ -d "$candidate/.git" ]; then
            benchmark_archive_root=$candidate
            break
        fi
        parent=$(dirname "$search_root")
        [ "$parent" = "$search_root" ] && break
        search_root=$parent
    done
fi
benchmark_archive_root="${benchmark_archive_root:-benchmarks}"
work_root="$benchmark_archive_root/.work"

case "$campaign:$profile" in
    compatibility:smoke|compatibility:full)
        mkdir -p target/compatibility
        cargo build --quiet --release -p tq-cli
        TQ_BIN="${TQ_BIN:-$PWD/target/release/tq}"
        export TQ_BIN
        exec cargo run --quiet -p tq-test-support --bin tq-compat -- run \
            --profile "$profile" --json "target/compatibility/$profile.json"
        ;;
    benchmark:smoke)
        mkdir -p "$work_root"
        cargo build --quiet --release -p tq-cli
        TQ_BIN="${TQ_BIN:-$PWD/target/release/tq}"
        export TQ_BIN
        exec cargo run --quiet -p tq-test-support --bin tq-bench -- run \
            --profile smoke --output "$work_root/smoke.json" --max-samples 1 \
            --case benchmark.startup --case benchmark.parse-discard \
            --case benchmark.scalar-extraction --case benchmark.event-stream
        ;;
    benchmark:standard|benchmark:large)
        mkdir -p "$work_root"
        cache_root="${TQ_CORPUS_CACHE:-$work_root/corpus}"
        corpus_origin="${TQ_CORPUS_ORIGIN:-refreshed}"
        if [ "$corpus_origin" = refreshed ]; then
            refresh_json="$(mktemp "${TMPDIR:-/tmp}/tq-corpus.XXXXXX")"
            trap 'rm -f "$refresh_json"' EXIT HUP INT TERM
            cargo run --quiet -p tq-test-support --bin tq-corpus -- \
                refresh tests/corpus/sources "$cache_root" "$profile" >"$refresh_json"
            TQ_BENCH_MANIFESTS="$(jq -r '.manifests | join(":")' "$refresh_json")"
            export TQ_BENCH_MANIFESTS
            rm -f "$refresh_json"
            trap - EXIT HUP INT TERM
        fi
        cargo build --quiet --release -p tq-cli
        TQ_BIN="${TQ_BIN:-$PWD/target/release/tq}"
        export TQ_BIN
        exec cargo run --quiet -p tq-test-support --bin tq-bench -- run \
            --profile "$profile" --output "$work_root/$profile.json" \
            --cache-root "$cache_root" \
            --origin "$corpus_origin"
        ;;
    benchmark:stack-overflow)
        mkdir -p "$work_root"
        cargo build --quiet --release -p tq-cli
        TQ_BIN="${TQ_BIN:-$PWD/target/release/tq}"
        export TQ_BIN
        exec cargo run --quiet -p tq-test-support --bin tq-stack-overflow -- run \
            --scenario-dir tests/stack-overflow \
            --output "$work_root/stack-overflow.json" \
            --report "$benchmark_archive_root/stack-overflow.md"
        ;;
    fuzz:default)
        seconds="${TQ_FUZZ_SECONDS:-10}"
        root="$PWD"
        if command -v cargo-fuzz >/dev/null 2>&1; then
            cargo_fuzz="cargo-fuzz"
        elif [ -x "$root/target/cargo-fuzz/bin/cargo-fuzz" ]; then
            cargo_fuzz="$root/target/cargo-fuzz/bin/cargo-fuzz"
        else
            echo "cargo-fuzz is required (install with: cargo install cargo-fuzz)" >&2
            exit 69
        fi
        for target in query_parser toon_decoder bytecode_decode vm_program cli_args automatic_plan recursive_interpolation user_functions regex_date_platform; do
            RUSTUP_TOOLCHAIN="${TQ_FUZZ_TOOLCHAIN:-nightly}" \
                "$cargo_fuzz" run --fuzz-dir "$root/tests/fuzz" "$target" -- \
                -max_total_time="$seconds" \
                -timeout=5 \
                -max_len=65536
        done
        exit 0
        ;;
    *)
        echo "campaign runner: unsupported campaign '$campaign' profile '$profile'" >&2
        exit 64
        ;;
esac
