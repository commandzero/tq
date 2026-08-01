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
    benchmark:smoke|benchmark:standard|benchmark:large|fuzz:default)
        ;;
    *)
        echo "campaign runner: unsupported campaign '$campaign' profile '$profile'" >&2
        exit 64
        ;;
esac

echo "campaign runner: '$campaign/$profile' is staged but its harness is not implemented yet" >&2
exit 2
