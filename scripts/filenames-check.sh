#!/bin/bash
# Check filenames, not directory components or non-Markdown test fixtures.
set -euo pipefail
root=${1:-.}
inventory=$(mktemp)
trap 'rm -f "$inventory"' EXIT
export LC_ALL=C
failed=0
pattern='^\.?[a-z0-9]+([.-][a-z0-9]+)*$'
for directory in docs tests; do
    status=0
    rg --files --hidden --no-ignore --null -- "$root/$directory" > "$inventory" || status=$?
    # ripgrep returns 1 for an empty tree, and 2 for a scan error.
    if [[ $status -gt 1 ]]; then exit "$status"; fi
    while IFS= read -r -d '' path; do
        name=${path##*/}
        if [[ $directory == tests && $name != *.[mM][dD] ]]; then continue; fi
        if [[ ! $name =~ $pattern ]]; then
            printf 'Filename must be lowercase kebab-case: %q\n' "$path" >&2
            failed=1
        fi
    done < "$inventory"
done
exit "$failed"
