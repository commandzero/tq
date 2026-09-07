#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/tool-versions.sh
export OPENSPEC_TELEMETRY=0
if [[ $(openspec --version) != "$OPENSPEC_VERSION" ]]; then
    echo "Install @fission-ai/openspec@$OPENSPEC_VERSION before checking specifications." >&2
    exit 1
fi
if [[ $# == 0 ]]; then
    exec openspec validate --specs --strict
fi
./scripts/repo-check.sh openspec "$@"
if [[ $(git rev-parse HEAD) != "$(git rev-parse --verify "${2:?Expected BASE HEAD}^{commit}")" ]]; then
    echo "Check out the requested head before validating its specifications." >&2
    exit 1
fi
git diff --exit-code HEAD -- openspec/
openspec validate --specs --strict
