#!/usr/bin/env bash
set -eu

# The strict campaign must never silently substitute a host package-manager jq.
# A release job either supplies the reviewed executable explicitly or downloads
# a target-specific artifact whose digest is supplied alongside its URL.
sha256_file() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    else
        echo "cannot verify pinned jq reference: no SHA-256 utility is available" >&2
        exit 69
    fi
}

if [ -n "${TQ_JQ:-}" ]; then
    reference_jq=$TQ_JQ
elif [ -n "${TQ_REFERENCE_JQ:-}" ]; then
    reference_jq=$TQ_REFERENCE_JQ
elif [ -n "${TQ_REFERENCE_JQ_URL:-}" ]; then
    expected_sha256=${TQ_REFERENCE_JQ_SHA256:-}
    if [ -z "$expected_sha256" ]; then
        echo "a jq artifact URL requires TQ_REFERENCE_JQ_SHA256" >&2
        exit 69
    fi
    reference_jq="${TQ_REFERENCE_JQ_OUTPUT:-${PWD}/target/reference-build/jq/jq}"
    if [ ! -e "$reference_jq" ]; then
        reference_dir=$(dirname "$reference_jq")
        mkdir -p "$reference_dir"
        if ! command -v curl >/dev/null 2>&1; then
            echo "cannot provision jq artifact: curl is unavailable" >&2
            exit 69
        fi
        # Keep the temporary file beside the destination so the final rename
        # is atomic and cannot cross a filesystem boundary.
        temporary_jq=$(mktemp "$reference_dir/.tq-reference-jq.XXXXXX")
        cleanup() {
            rm -f "$temporary_jq"
        }
        trap cleanup EXIT HUP INT TERM
        curl --fail --silent --show-error --location --retry 2 \
            "$TQ_REFERENCE_JQ_URL" -o "$temporary_jq"
        actual_sha256=$(sha256_file "$temporary_jq")
        if [ "$actual_sha256" != "$expected_sha256" ]; then
            echo "downloaded jq artifact SHA-256 mismatch: $reference_jq" >&2
            echo "expected $expected_sha256, got $actual_sha256" >&2
            exit 69
        fi
        chmod 755 "$temporary_jq"
        mv "$temporary_jq" "$reference_jq"
        trap - EXIT HUP INT TERM
    fi
else
    reference_jq="${PWD}/target/reference-build/jq/jq"
    if [ ! -f "$reference_jq" ]; then
        reference_jq="${PWD}/../jq/jq"
    fi
fi

if [ ! -f "$reference_jq" ] || [ ! -x "$reference_jq" ]; then
    echo "pinned jq reference is not usable or unprovisioned: $reference_jq" >&2
    echo "set TQ_JQ (or TQ_REFERENCE_JQ) to the reviewed executable" >&2
    exit 69
fi

if [ -n "${TQ_REFERENCE_JQ_SHA256:-}" ]; then
    actual_sha256=$(sha256_file "$reference_jq")
    if [ "$actual_sha256" != "$TQ_REFERENCE_JQ_SHA256" ]; then
        echo "pinned jq reference SHA-256 mismatch: $reference_jq" >&2
        echo "expected $TQ_REFERENCE_JQ_SHA256, got $actual_sha256" >&2
        exit 69
    fi
fi

if [ "$#" -eq 0 ]; then
    printf '%s\n' "$reference_jq"
    exit 0
fi

export TQ_JQ="$reference_jq"
exec "$@"
