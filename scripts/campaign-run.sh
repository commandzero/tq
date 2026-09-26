#!/bin/sh
set -eu
quick_entry_seconds=$(date +%s)
quick_compile_seconds=0

campaign="${1:-}"
suite="${2:-}"
if [ "$campaign" = benchmark ]; then
    suite="${suite:-natural-corpus}"
    profile="${3:-standard}"
elif [ "$campaign:$suite" = compatibility:stack-overflow ]; then
    profile="${3:-standard}"
else
    profile="$suite"
fi

if [ "$campaign" = benchmark ] || [ "$campaign:$suite" = compatibility:stack-overflow ]; then
    case "$profile" in
        quick|standard|extended) ;;
        *)
            echo "campaign runner: unsupported profile '$profile' (quick|standard|extended)" >&2
            exit 64
            ;;
    esac
    if [ -n "${4:-}" ]; then
        echo "campaign runner: usage: campaign-run.sh $campaign SUITE [quick|standard|extended]" >&2
        exit 64
    fi
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
quick_install_root=

cleanup_cargo_output() {
    rm -f "${cargo_output:-}"
    if [ -n "${quick_install_root:-}" ]; then
        rm -rf "$quick_install_root"
    fi
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
    if [ "${TQ_CAMPAIGN_COMPILED:-}" = 1 ]; then
        return
    fi
    if [ -n "${TQ_BIN:-}" ]; then
        cargo_started=$(date +%s)
        "$cargo_bin" build --quiet --release --locked -p tq-cli
        quick_compile_seconds=$((quick_compile_seconds + $(date +%s) - cargo_started))
        export TQ_BIN
        return
    fi
    cargo_output=$(mktemp "${TMPDIR:-/tmp}/tq-cargo-build.XXXXXX")
    trap cleanup_cargo_output EXIT
    trap 'cleanup_cargo_output_and_exit 129' HUP
    trap 'cleanup_cargo_output_and_exit 130' INT
    trap 'cleanup_cargo_output_and_exit 143' TERM
    cargo_started=$(date +%s)
    if "$cargo_bin" build --release --locked -p tq-cli --bin tq \
        --message-format=json-render-diagnostics >"$cargo_output"; then
        quick_compile_seconds=$((quick_compile_seconds + $(date +%s) - cargo_started))
        :
    else
        status=$?
        cleanup_cargo_output
        trap - EXIT HUP INT TERM
        return "$status"
    fi
    if [ "${TQ_CAMPAIGN_QUICK_BUILD:-}" = 1 ]; then
        TQ_CLI_CARGO_OUTPUT=$cargo_output
        export TQ_CLI_CARGO_OUTPUT
        trap - EXIT HUP INT TERM
        return
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
    if [ "${TQ_CAMPAIGN_COMPILED:-}" = 1 ]; then
        return
    fi
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

# Compile every executable before starting quick's shared wall-clock budget.
# Install the guard into an owned staging root using Cargo's workspace cache,
# then transfer cleanup of that root to the guard on exec.
build_quick_support() {
    cargo_output=$(mktemp "${TMPDIR:-/tmp}/tq-cargo-quick-build.XXXXXX")
    trap cleanup_cargo_output EXIT
    trap 'cleanup_cargo_output_and_exit 129' HUP
    trap 'cleanup_cargo_output_and_exit 130' INT
    trap 'cleanup_cargo_output_and_exit 143' TERM
    set -- --bin tq-quick --bin tq-bench --bin tq-corpus --bin tq-bench-worker
    if [ "$suite" = stack-overflow ]; then
        set -- "$@" --bin tq-stack-overflow
    fi
    cargo_started=$(date +%s)
    "$cargo_bin" build --release --locked -p tq-test-support "$@" \
        --message-format=json-render-diagnostics >"$cargo_output"
    quick_compile_seconds=$((quick_compile_seconds + $(date +%s) - cargo_started))
    TQ_SUPPORT_CARGO_OUTPUT=$cargo_output
    export TQ_SUPPORT_CARGO_OUTPUT
}

discover_quick_support() {
    if [ -n "${TQ_CLI_CARGO_OUTPUT:-}" ]; then
        TQ_BIN=$(cargo_artifact_path "$TQ_CLI_CARGO_OUTPUT" tq)
    fi
    TQ_BENCH_BIN=$(cargo_artifact_path "$TQ_SUPPORT_CARGO_OUTPUT" tq-bench)
    TQ_CORPUS_BIN=$(cargo_artifact_path "$TQ_SUPPORT_CARGO_OUTPUT" tq-corpus)
    TQ_BENCH_WORKER=${TQ_BENCH_WORKER:-$(cargo_artifact_path "$TQ_SUPPORT_CARGO_OUTPUT" tq-bench-worker)}
    if [ "$suite" = stack-overflow ]; then
        TQ_STACK_OVERFLOW_BIN=$(cargo_artifact_path "$TQ_SUPPORT_CARGO_OUTPUT" tq-stack-overflow)
        export TQ_STACK_OVERFLOW_BIN
    fi
    rm -f "${TQ_CLI_CARGO_OUTPUT:-}" "$TQ_SUPPORT_CARGO_OUTPUT"
    export TQ_BIN TQ_BENCH_BIN TQ_CORPUS_BIN TQ_BENCH_WORKER
}

run_support() {
    support_binary=$1
    shift
    if [ "${TQ_CAMPAIGN_COMPILED:-}" = 1 ]; then
        case "$support_binary" in
            tq-bench) "$TQ_BENCH_BIN" "$@" ;;
            tq-corpus) "$TQ_CORPUS_BIN" "$@" ;;
            tq-stack-overflow) "$TQ_STACK_OVERFLOW_BIN" "$@" ;;
            *) echo "unsupported precompiled helper: $support_binary" >&2; return 64 ;;
        esac
    else
        "$cargo_bin" run --quiet --release --locked -p tq-test-support \
            --bin "$support_binary" -- "$@"
    fi
}

if [ "${TQ_CAMPAIGN_COMPILED:-}" = 1 ] && [ "${TQ_CAMPAIGN_QUICK_BUILD:-}" = 1 ]; then
    echo "campaign runner: compilation complete; starting bounded quick execution" >&2
    discover_quick_support
fi

if [ "${TQ_CAMPAIGN_COMPILED:-}" != 1 ]; then
    case "$campaign:$suite" in
        benchmark:smoke|benchmark:natural-corpus|benchmark:large-input|compatibility:stack-overflow)
            if [ "$profile" = quick ] || { [ "$campaign" = benchmark ] && [ "${SAMPLING:-}" = quick ]; }; then
                TQ_CAMPAIGN_QUICK_BUILD=1
                export TQ_CAMPAIGN_QUICK_BUILD
                build_tq_cli
                quick_install_root=$(mktemp -d "${TMPDIR:-/tmp}/tq-quick-install.XXXXXX")
                trap cleanup_cargo_output EXIT
                trap 'cleanup_cargo_output_and_exit 129' HUP
                trap 'cleanup_cargo_output_and_exit 130' INT
                trap 'cleanup_cargo_output_and_exit 143' TERM
                : > "$quick_install_root/.tq-quick-install"
                cargo_started=$(date +%s)
                "$cargo_bin" install --path crates/tq-test-support --locked --bin tq-quick \
                    --root "$quick_install_root" --no-track
                quick_compile_seconds=$((quick_compile_seconds + $(date +%s) - cargo_started))
                build_quick_support
                TQ_CAMPAIGN_COMPILED=1
                export TQ_CAMPAIGN_COMPILED
                quick_budget=${CAMPAIGN_BUDGET_SECONDS:-50}
                case "$quick_budget" in
                    *[!0-9]*|"") echo "campaign runner: invalid quick budget" >&2; exit 64 ;;
                esac
                if [ "$quick_budget" -lt 1 ] || [ "$quick_budget" -gt 50 ]; then
                    echo "campaign runner: quick budget must be between 1 and 50 seconds" >&2
                    exit 64
                fi
                if [ "$campaign" = benchmark ]; then
                    session_dir=$(mktemp -d "${TMPDIR:-/tmp}/tq-benchmark-${suite}-quick.XXXXXX")
                else
                    session_dir=$(mktemp -d "${TMPDIR:-/tmp}/tq-stack-overflow-quick.XXXXXX")
                fi
                TQ_QUICK_REPORT_PATH=$session_dir/report.json
                export TQ_QUICK_REPORT_PATH
                quick_elapsed=$(( $(date +%s) - quick_entry_seconds - quick_compile_seconds ))
                quick_remaining=$((quick_budget - quick_elapsed))
                if [ "$quick_remaining" -lt 1 ]; then
                    exit 124
                fi
                exec "$quick_install_root/bin/tq-quick" --cleanup-install-root "$quick_install_root" \
                    --budget-seconds "$quick_remaining" -- "$0" "$campaign" "$suite" "$profile"
            fi
            ;;
    esac
fi

case "$campaign:$suite" in
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
        if [ -n "${TQ_QUICK_REPORT_PATH:-}" ]; then
            report_path=$TQ_QUICK_REPORT_PATH
        elif [ "$profile" = quick ] || [ "$profile" = standard ] || [ "${SAMPLING:-}" = quick ]; then
            session_dir="$(mktemp -d "${TMPDIR:-/tmp}/tq-benchmark-smoke-${profile}.XXXXXX")"
            report_path="$session_dir/report.json"
        else
            report_path="$work_root/smoke-extended.json"
        fi
        set --
        if [ -n "${TQ_TIMING_CALIBRATION:-}" ]; then
            set -- "$@" --timing-calibration "$TQ_TIMING_CALIBRATION"
        fi
        if [ -n "${SAMPLING:-}" ]; then
            set -- "$@" --sampling "$SAMPLING"
        fi
        if [ -n "${CAMPAIGN_BUDGET_SECONDS:-}" ]; then
            set -- "$@" --campaign-budget-seconds "$CAMPAIGN_BUDGET_SECONDS"
        fi
        if [ -n "${CASE_BUDGET_SECONDS:-}" ]; then
            set -- "$@" --case-budget-seconds "$CASE_BUDGET_SECONDS"
        fi
        run_support tq-bench run \
            --suite smoke --profile "$profile" --output "$report_path" \
            --case benchmark.startup --case benchmark.parse-discard \
            --case benchmark.scalar-extraction --case benchmark.event-stream \
            --case benchmark.object-deep-merge "$@"
        ;;
    benchmark:natural-corpus|benchmark:large-input)
        mkdir -p "$work_root"
        cache_root="${TQ_CORPUS_CACHE:-$work_root/corpus}"
        corpus_origin="${TQ_CORPUS_ORIGIN:-frozen}"
        corpus_suite="$suite"
        build_tq_cli
        build_benchmark_worker
        run_support tq-bench --preflight-only --suite "$suite" --profile "$profile"
        if [ -z "${TQ_BENCH_MANIFESTS:-}" ]; then
            refresh_json="$(mktemp "${TMPDIR:-/tmp}/tq-corpus.XXXXXX")"
            trap 'rm -f "$refresh_json"' EXIT HUP INT TERM
            if [ "$corpus_origin" = refreshed ]; then
                corpus_command=refresh
            else
                corpus_command=prepare
            fi
            run_support tq-corpus \
                "$corpus_command" tests/corpus/sources "$cache_root" "$corpus_suite" >"$refresh_json"
            TQ_BENCH_MANIFESTS="$("$(host_tq_parser)" -r '.manifests | join(":")' "$refresh_json")"
            export TQ_BENCH_MANIFESTS
            rm -f "$refresh_json"
            trap - EXIT HUP INT TERM
        fi
        set --
        if [ -n "${TQ_QUICK_REPORT_PATH:-}" ]; then
            report_path=$TQ_QUICK_REPORT_PATH
        elif [ "$profile" = quick ] || [ "$profile" = standard ] || [ "${SAMPLING:-}" = quick ]; then
            session_dir="$(mktemp -d "${TMPDIR:-/tmp}/tq-benchmark-${suite}-${profile}.XXXXXX")"
            report_path="$session_dir/report.json"
        else
            report_path="$work_root/$suite-$profile.json"
        fi
        if [ -n "${TQ_TIMING_CALIBRATION:-}" ]; then
            set -- "$@" --timing-calibration "$TQ_TIMING_CALIBRATION"
        fi
        if [ -n "${BENCHMARK_MODE:-}" ]; then
            set -- "$@" --mode "$BENCHMARK_MODE"
        fi
        if [ -n "${SAMPLING:-}" ]; then
            set -- "$@" --sampling "$SAMPLING"
        fi
        if [ -n "${CAMPAIGN_BUDGET_SECONDS:-}" ]; then
            set -- "$@" --campaign-budget-seconds "$CAMPAIGN_BUDGET_SECONDS"
        fi
        if [ -n "${CASE_BUDGET_SECONDS:-}" ]; then
            set -- "$@" --case-budget-seconds "$CASE_BUDGET_SECONDS"
        fi
        run_support tq-bench run \
            --suite "$suite" --profile "$profile" --output "$report_path" \
            --cache-root "$cache_root" \
            --origin "$corpus_origin" "$@"
        ;;
    compatibility:stack-overflow)
        mkdir -p "$work_root"
        build_tq_cli
        build_benchmark_worker
        set --
        if [ -n "${TQ_QUICK_REPORT_PATH:-}" ]; then
            set -- --output "$TQ_QUICK_REPORT_PATH"
        elif [ "$profile" = standard ]; then
            session_dir="$(mktemp -d "${TMPDIR:-/tmp}/tq-stack-overflow-standard.XXXXXX")"
            set -- --output "$session_dir/report.json"
        elif [ "$profile" = extended ]; then
            # Uncalibrated pages remain diagnostic evidence in the archive.
            set -- --output "$work_root/stack-overflow.json" \
                --report-dir "$work_root/stack-overflow-pages"
        fi
        run_support tq-stack-overflow run \
            --profile "$profile" --scenario-dir tests/stack-overflow "$@"
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
        echo "campaign runner: unsupported campaign '$campaign' suite '$suite' profile '$profile'" >&2
        exit 64
        ;;
esac
