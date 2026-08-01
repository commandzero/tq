#!/bin/sh
set -eu

tasks_file="openspec/changes/build-tq-mvp/tasks.md"

if [ ! -f "$tasks_file" ]; then
    echo "baseline gate: missing $tasks_file" >&2
    exit 2
fi

pending="$(
    awk '
        /^## 2[.]/ { in_baseline = 1 }
        /^## 6[.]/ { in_baseline = 0 }
        in_baseline && /^- \[ \]/ { print }
    ' "$tasks_file"
)"

if [ -n "$pending" ]; then
    echo "baseline gate: blocked; complete every task in sections 2-5 first" >&2
    printf '%s\n' "$pending" >&2
    exit 1
fi

echo "baseline gate: passed"
