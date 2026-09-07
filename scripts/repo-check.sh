#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p target/repository-checks
rustc --edition=2024 -D warnings scripts/repo-check.rs -o target/repository-checks/repo-check
exec target/repository-checks/repo-check "$@"
