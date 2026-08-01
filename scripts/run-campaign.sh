#!/bin/sh
set -eu

campaign="${1:-}"
profile="${2:-}"

case "$campaign:$profile" in
    compatibility:smoke|compatibility:full)
        mkdir -p target/compatibility
        exec cargo run --quiet -p tq-test-support --bin tq-compat -- run \
            --profile "$profile" --json "target/compatibility/$profile.json"
        ;;
    benchmark:smoke)
        mkdir -p target/benchmarks
        exec cargo run --quiet -p tq-test-support --bin tq-bench -- run \
            --profile smoke --output target/benchmarks/smoke.json --max-samples 1 \
            --case benchmark.startup --case benchmark.parse-discard \
            --case benchmark.scalar-extraction --case benchmark.event-stream
        ;;
    benchmark:standard|benchmark:large)
        mkdir -p target/benchmarks
        exec cargo run --quiet -p tq-test-support --bin tq-bench -- run \
            --profile "$profile" --output "target/benchmarks/$profile.json" \
            --cache-root "${TQ_CORPUS_CACHE:-target/corpus}" \
            --origin "${TQ_CORPUS_ORIGIN:-refreshed}"
        ;;
    fuzz:default)
        ;;
    *)
        echo "campaign runner: unsupported campaign '$campaign' profile '$profile'" >&2
        exit 64
        ;;
esac

echo "campaign runner: '$campaign/$profile' is staged but its harness is not implemented yet" >&2
exit 2
