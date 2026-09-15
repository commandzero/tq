#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/tools-versions.sh
okf_bin=${OKF:-okf}
version=$("$okf_bin" --version)
if [[ $version != "okf $OKF_VERSION" && $version != "okf $OKF_VERSION "* ]]; then
    echo "Install the pinned validator: cargo install okf --version $OKF_VERSION --locked" >&2
    exit 1
fi
"$okf_bin" validate docs/
"$okf_bin" lint docs/
bash scripts/filenames-check.sh
