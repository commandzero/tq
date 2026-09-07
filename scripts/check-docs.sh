#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/tool-versions.sh
if [[ $(okf --version) != "okf $OKF_VERSION "* ]]; then
    echo "Install the pinned validator: cargo install okf --version $OKF_VERSION --locked" >&2
    exit 1
fi
okf validate docs/
./scripts/repo-check.sh docs
