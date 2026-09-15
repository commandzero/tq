#!/bin/sh
set -eu

campaign="${1:-}"
if [ "$campaign" = benchmark ]; then
    profile="${2:-rapid}"
else
    profile="${2:-}"
fi

# Keep benchmark outputs in the sibling archive checkout when it is present.
# TQ_BENCHMARK_ARCHIVE_ROOT can override discovery for CI and other layouts.
benchmark_archive_root="${TQ_BENCHMARK_ARCHIVE_ROOT:-}"
if [ -z "$benchmark_archive_root" ]; then
    search_root=$PWD
    while [ "$search_root" != "/" ]; do
        candidate="$search_root/tq-benchmarks"
        if [ -e "$candidate/.git" ]; then
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

cargo_bin=${CARGO:-cargo}
host_tq=${TQ_HOST_TQ:-}

cleanup_cargo_output() {
    rm -f "${cargo_output:-}"
}

cleanup_cargo_output_and_exit() {
    signal_status=$1
    cleanup_cargo_output
    trap - EXIT HUP INT TERM
    exit "$signal_status"
}

# Cargo's metadata has no target-triple field, and running a workspace binary
# to parse that metadata can execute a cross-target binary. Resolve the paths
# Cargo actually emitted instead, using a host tq parser for Cargo's JSON
# message stream. The caller may provide reviewed binaries explicitly.
host_tq_parser() {
    if [ -z "$host_tq" ]; then
        host_tq=$(command -v tq 2>/dev/null || true)
    fi
    if [ -z "$host_tq" ] || [ ! -x "$host_tq" ]; then
        echo "campaign runner: a host tq parser is required (set TQ_HOST_TQ)" >&2
        return 69
    fi
    printf '%s\n' "$host_tq"
}

cargo_artifact_path() {
    artifact_output=$1
    artifact_name=$2
    # shellcheck disable=SC2016
    if artifact_path=$("$(host_tq_parser)" -i jsonl --arg name "$artifact_name" -r \
        'select(.reason == "compiler-artifact" and .target.name == $name and .executable != null) | .executable' \
        "$artifact_output"); then
        :
    else
        return $?
    fi
    case "$artifact_path" in
        ""|null|*"
"*)
            echo "campaign runner: Cargo did not emit one executable for $artifact_name" >&2
            return 69
            ;;
    esac
    printf '%s\n' "$artifact_path"
}

build_tq_cli() {
    if [ -n "${TQ_BIN:-}" ]; then
        "$cargo_bin" build --quiet --release --locked -p tq-cli
        export TQ_BIN
        return
    fi
    cargo_output=$(mktemp "${TMPDIR:-/tmp}/tq-cargo-build.XXXXXX")
    trap cleanup_cargo_output EXIT
    trap 'cleanup_cargo_output_and_exit 129' HUP
    trap 'cleanup_cargo_output_and_exit 130' INT
    trap 'cleanup_cargo_output_and_exit 143' TERM
    if "$cargo_bin" build --release --locked -p tq-cli --bin tq \
        --message-format=json-render-diagnostics >"$cargo_output"; then
        :
    else
        status=$?
        cleanup_cargo_output
        trap - EXIT HUP INT TERM
        return "$status"
    fi
    if TQ_BIN=$(cargo_artifact_path "$cargo_output" tq); then
        :
    else
        status=$?
        cleanup_cargo_output
        trap - EXIT HUP INT TERM
        return "$status"
    fi
    cleanup_cargo_output
    trap - EXIT HUP INT TERM
    export TQ_BIN
}

# The coordinator and worker must come from the same collector build. Cargo
# running a selected coordinator binary does not build its sibling worker.
build_benchmark_worker() {
    if [ -n "${TQ_BENCH_WORKER:-}" ]; then
        "$cargo_bin" build --quiet --release --locked -p tq-test-support --bin tq-bench-worker
        export TQ_BENCH_WORKER
        return
    fi
    cargo_output=$(mktemp "${TMPDIR:-/tmp}/tq-cargo-worker-build.XXXXXX")
    trap cleanup_cargo_output EXIT
    trap 'cleanup_cargo_output_and_exit 129' HUP
    trap 'cleanup_cargo_output_and_exit 130' INT
    trap 'cleanup_cargo_output_and_exit 143' TERM
    if "$cargo_bin" build --release --locked -p tq-test-support --bin tq-bench-worker \
        --message-format=json-render-diagnostics >"$cargo_output"; then
        :
    else
        status=$?
        cleanup_cargo_output
        trap - EXIT HUP INT TERM
        return "$status"
    fi
    if TQ_BENCH_WORKER=$(cargo_artifact_path "$cargo_output" tq-bench-worker); then
        :
    else
        status=$?
        cleanup_cargo_output
        trap - EXIT HUP INT TERM
        return "$status"
    fi
    cleanup_cargo_output
    trap - EXIT HUP INT TERM
    export TQ_BENCH_WORKER
}

case "$campaign:$profile" in
    compatibility:strict)
        mkdir -p target/compatibility
        build_tq_cli
        "$cargo_bin" build --quiet --release --locked -p tq-test-support
        # The preflight refuses PATH/package-manager jq substitutions. The
        # Rust comparator then validates the executable and runtime identity
        # against the host-specific reviewed pin before executing any case.
        exec ./scripts/reference-jq-provision.sh \
            "$cargo_bin" run --quiet --release --locked -p tq-test-support --bin tq-manual-compare -- \
            --markdown-dir "target/compatibility/jq-manual" \
            "target/compatibility/strict.toon"
        ;;
    compatibility:smoke|compatibility:full)
        mkdir -p target/compatibility
        build_tq_cli
        exec "$cargo_bin" run --quiet -p tq-test-support --bin tq-compat -- run \
            --profile "$profile" --json "target/compatibility/$profile.json"
        ;;
    benchmark:smoke)
        mkdir -p "$work_root"
        build_tq_cli
        build_benchmark_worker
        set --
        if [ -n "${TQ_TIMING_CALIBRATION:-}" ]; then
            set -- --timing-calibration "$TQ_TIMING_CALIBRATION"
        fi
        exec "$cargo_bin" run --quiet --release -p tq-test-support --bin tq-bench -- run \
            --profile smoke --output "$work_root/smoke.json" --max-samples 1 \
            --case benchmark.startup --case benchmark.parse-discard \
            --case benchmark.scalar-extraction --case benchmark.event-stream \
            --case benchmark.object-deep-merge "$@"
        ;;
    benchmark:rapid|benchmark:standard|benchmark:large)
        if [ "$profile" = standard ] && [ -z "${TQ_TIMING_CALIBRATION:-}" ]; then
            echo "standard publication requires TQ_TIMING_CALIBRATION pointing to a verified native validation summary" >&2
            exit 64
        fi
        mkdir -p "$work_root"
        cache_root="${TQ_CORPUS_CACHE:-$work_root/corpus}"
        corpus_origin="${TQ_CORPUS_ORIGIN:-frozen}"
        corpus_profile="$profile"
        build_tq_cli
        build_benchmark_worker
        "$cargo_bin" run --quiet --release --locked -p tq-test-support --bin tq-bench -- \
            --preflight-only --profile "$profile"
        if [ -z "${TQ_BENCH_MANIFESTS:-}" ]; then
            refresh_json="$(mktemp "${TMPDIR:-/tmp}/tq-corpus.XXXXXX")"
            trap 'rm -f "$refresh_json"' EXIT HUP INT TERM
            if [ "$corpus_origin" = refreshed ]; then
                corpus_command=refresh
            else
                corpus_command=prepare
            fi
            "$cargo_bin" run --quiet --release -p tq-test-support --bin tq-corpus -- \
                "$corpus_command" tests/corpus/sources "$cache_root" "$corpus_profile" >"$refresh_json"
            TQ_BENCH_MANIFESTS="$("$(host_tq_parser)" -r '.manifests | join(":")' "$refresh_json")"
            export TQ_BENCH_MANIFESTS
            rm -f "$refresh_json"
            trap - EXIT HUP INT TERM
        fi
        set --
        if [ "$profile" = standard ]; then
            set -- --markdown-dir "$PWD/docs/tests/comparison"
        fi
        if [ -n "${TQ_TIMING_CALIBRATION:-}" ]; then
            set -- "$@" --timing-calibration "$TQ_TIMING_CALIBRATION"
        fi
        exec "$cargo_bin" run --quiet --release -p tq-test-support --bin tq-bench -- run \
            --profile "$profile" --output "$work_root/$profile.json" \
            --cache-root "$cache_root" \
            --origin "$corpus_origin" "$@"
        ;;
    benchmark:extra-large)
        mkdir -p "$work_root"
        cache_root="${TQ_CORPUS_CACHE:-$work_root/corpus}"
        corpus_origin="${TQ_CORPUS_ORIGIN:-frozen}"
        build_tq_cli

        if [ -n "${TQ_BENCH_MANIFEST:-}" ]; then
            manifest="$TQ_BENCH_MANIFEST"
        elif [ -n "${TQ_BENCH_MANIFESTS:-}" ]; then
            manifest=""
            old_ifs=$IFS
            IFS=:
            for candidate in $TQ_BENCH_MANIFESTS; do
                if [ "$("$(host_tq_parser)" -r '.source_id' "$candidate")" = microsoft-us-buildings-georgia ]; then
                    manifest="$candidate"
                    break
                fi
            done
            IFS=$old_ifs
        else
            refresh_json="$(mktemp "${TMPDIR:-/tmp}/tq-corpus.XXXXXX")"
            trap 'rm -f "$refresh_json"' EXIT HUP INT TERM
            if [ "$corpus_origin" = refreshed ]; then
                corpus_command=refresh
            else
                corpus_command=prepare
            fi
            "$cargo_bin" run --quiet --release -p tq-test-support --bin tq-corpus -- \
                "$corpus_command" tests/corpus/sources "$cache_root" large >"$refresh_json"
            manifest="$("$(host_tq_parser)" -r '.manifests[] | select(endswith("/microsoft-us-buildings-georgia/manifest.json"))' "$refresh_json" | head -n 1)"
            rm -f "$refresh_json"
            trap - EXIT HUP INT TERM
        fi

        if [ -z "${manifest:-}" ] || [ ! -f "$manifest" ]; then
            echo "extra-large benchmark requires the microsoft-us-buildings-georgia manifest" >&2
            exit 69
        fi
        if [ "$("$(host_tq_parser)" -r '.source_id' "$manifest")" != microsoft-us-buildings-georgia ]; then
            echo "extra-large benchmark manifest is not microsoft-us-buildings-georgia: $manifest" >&2
            exit 69
        fi
        source_path="$("$(host_tq_parser)" -r '.artifacts.source_json.path' "$manifest")"
        if [ -z "$source_path" ] || [ "$source_path" = null ]; then
            echo "extra-large benchmark manifest has no source JSON artifact: $manifest" >&2
            exit 69
        fi
        input="$cache_root/$source_path"
        if [ ! -f "$input" ]; then
            echo "extra-large benchmark source is missing: $input" >&2
            exit 69
        fi
        exec benchmarks/cases/parallel-selected-json.sh \
            "$input" "$TQ_BIN" "$work_root/parallel-selected-json/$(date +%Y-%m-%d)"
        ;;
    benchmark:stack-overflow)
        mkdir -p "$work_root"
        build_tq_cli
        build_benchmark_worker
        # This runner currently records RSS controls but has no calibrated
        # publication gate. Keep its diagnostic pages in the evidence archive.
        exec "$cargo_bin" run --quiet --release --locked -p tq-test-support --bin tq-stack-overflow -- run \
            --scenario-dir tests/stack-overflow \
            --output "$work_root/stack-overflow.json" \
            --report-dir "$work_root/stack-overflow-pages"
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
        for target in query_parser toon_decoder bytecode_decode vm_program cli_args automatic_plan recursive_interpolation user_functions regex_date_platform native_input; do
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
