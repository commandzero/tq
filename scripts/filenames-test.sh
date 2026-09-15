#!/bin/bash
# Exercise repository filename policy through its command-line interface.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$(mktemp -d "${TMPDIR:-/tmp}/tq-filenames.XXXXXX")
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/docs/.hidden" "$fixture/tests/nested"
bash "$root/scripts/filenames-check.sh" "$fixture"
touch "$fixture/docs/jq-1.8.md" "$fixture/docs/.hidden/data-v1.toon" \
    "$fixture/tests/nested/README.txt" "$fixture/tests/nested/good-name.md"
bash "$root/scripts/filenames-check.sh" "$fixture"
for name in README.md bad_name.md 'bad name.svg' $'bad\nname.md' UPPER.PNG trailing-.md double--separator.md; do
    touch "$fixture/docs/.hidden/$name"
    if bash "$root/scripts/filenames-check.sh" "$fixture" > "$fixture/output" 2>&1; then
        printf 'Expected rejection of docs filename: %q\n' "$name" >&2
        exit 1
    fi
    rm "$fixture/docs/.hidden/$name"
done
touch "$fixture/tests/nested/README.MD"
if bash "$root/scripts/filenames-check.sh" "$fixture" > "$fixture/output" 2>&1; then
    echo 'Expected rejection of uppercase test Markdown filename.' >&2
    exit 1
fi
rm "$fixture/tests/nested/README.MD"
mkdir "$fixture/docs/ignored"
printf 'ignored/\n' > "$fixture/docs/.ignore"
touch "$fixture/docs/ignored/UPPER.md"
if bash "$root/scripts/filenames-check.sh" "$fixture" > "$fixture/output" 2>&1; then
    echo 'Expected ignored files to remain subject to filename policy.' >&2
    exit 1
fi
rm "$fixture/docs/ignored/UPPER.md"
bash "$root/scripts/filenames-check.sh" "$fixture"
if bash "$root/scripts/filenames-check.sh" "$fixture/missing" > "$fixture/output" 2>&1; then
    echo 'Expected a missing directory to fail, not pass as an empty tree.' >&2
    exit 1
fi
echo 'Filename policy tests pass.'
