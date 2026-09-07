#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/tool-versions.sh
export PATH="$PWD/target/tools/bin:$PATH"
./scripts/check-docs.sh
shellcheck scripts/*.sh benchmarks/cases/*.sh
if [[ $(actionlint -version) != "v$ACTIONLINT_VERSION"$'\n'* ]]; then
    echo "Install Actionlint $ACTIONLINT_VERSION using docs/contributor-checks.md." >&2
    exit 1
fi
actionlint
rustfmt --edition 2024 --check scripts/repo-check.rs
mkdir -p target/repository-checks
rustc --edition=2024 -D warnings --test scripts/repo-check.rs -o target/repository-checks/tests
target/repository-checks/tests
./scripts/test-package-release.sh
