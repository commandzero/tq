#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/tools-versions.sh
export OPENSPEC_TELEMETRY=0
openspec_bin=${OPENSPEC:-openspec}
if [[ $("$openspec_bin" --version) != "$OPENSPEC_VERSION" ]]; then
    echo "Install @fission-ai/openspec@$OPENSPEC_VERSION before checking specifications." >&2
    exit 1
fi
if [[ $# == 0 ]]; then
    exec "$openspec_bin" validate --specs --strict
fi
if [[ $(git rev-parse HEAD) != "$(git rev-parse --verify "${2:?Expected BASE HEAD}^{commit}")" ]]; then
    echo "Check out the requested head before validating its specifications." >&2
    exit 1
fi
git diff --exit-code HEAD -- openspec/
archives=$(./scripts/repo-check.sh openspec "$@")
if [[ -n $archives ]]; then
    # Export only associated archives from committed HEAD. The native CLI checks
    # every archive in its working directory, even when given an item name.
    stage=$(mktemp -d "${TMPDIR:-/tmp}/tq-openspec.XXXXXX")
    trap 'rm -rf "$stage"' EXIT
    while IFS= read -r archive; do
        mkdir -p "$stage/$archive"
        git show "HEAD:$archive/tasks.md" > "$stage/$archive/tasks.md"
    done <<< "$archives"
    (cd "$stage" && "$openspec_bin" validate --archived)
fi
"$openspec_bin" validate --specs --strict
