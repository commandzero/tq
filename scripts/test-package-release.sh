#!/bin/bash
# Exercise packaging boundaries in a disposable repository with a fake compiler.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$(mktemp -d "${TMPDIR:-/tmp}/tq-package-test.XXXXXX")
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/repo/scripts" "$fixture/bin"
cp "$root/scripts/package-release.sh" "$fixture/repo/scripts/"
cp "$root/rust-toolchain.toml" "$fixture/repo/"
printf '#!/bin/bash\nexit 0\n' > "$fixture/repo/scripts/preflight.sh"
printf 'MIT fixture license\n' > "$fixture/repo/LICENSE"
printf '## [0.4.0] - 2026-09-06\n' > "$fixture/repo/CHANGELOG.md"
printf '/target/\n' > "$fixture/repo/.gitignore"
cat > "$fixture/bin/cargo" <<'MOCK'
#!/bin/bash
set -euo pipefail
case "$1" in
    pkgid) printf 'path+file:///fixture/crate#%s\n' "${PACKAGE_VERSION:-0.4.0}" ;;
    test) exit 0 ;;
    build)
        mkdir -p target/aarch64-apple-darwin/release
        cat > target/aarch64-apple-darwin/release/tq <<'BINARY'
#!/bin/bash
if [[ ${1:-} == --version ]]; then
    echo "tq ${BINARY_VERSION:-0.4.0} (TOON v3; jq target 1.8.x; revision fixture)"
else
    echo "${QUERY_RESULT:-42}"
fi
BINARY
        chmod +x target/aarch64-apple-darwin/release/tq
        ;;
    *) exit 1 ;;
esac
MOCK
printf '#!/bin/bash\nprintf "host: aarch64-apple-darwin\\n"\n' > "$fixture/bin/rustc"
printf '#!/bin/bash\necho "fixture OS"\n' > "$fixture/bin/sw_vers"
chmod +x "$fixture/bin/"* "$fixture/repo/scripts/"*.sh
export PATH="$fixture/bin:$PATH"
cd "$fixture/repo"
git init -q
git config user.name 'Release fixture'
git config user.email 'fixture@example.invalid'
git add .
git -c commit.gpgsign=false commit -qm fixture
git -c tag.gpgsign=false tag v0.4.0

expect_failure() {
    if "$@" > "$fixture/output" 2>&1; then
        echo "Expected release packaging to reject: $*" >&2
        cat "$fixture/output" >&2
        exit 1
    fi
}

expect_failure ./scripts/package-release.sh not-a-version
expect_failure ./scripts/package-release.sh v0.4.0-rc.01
expect_failure ./scripts/package-release.sh v0.4.0-rc.
expect_failure ./scripts/package-release.sh v0.5.0
expect_failure env PACKAGE_VERSION=0.3.0 ./scripts/package-release.sh v0.4.0
printf 'dirty\n' >> LICENSE
expect_failure ./scripts/package-release.sh v0.4.0
git restore LICENSE
expect_failure env BINARY_VERSION=0.3.0 ./scripts/package-release.sh v0.4.0
rm -rf target/release-artifacts
expect_failure env QUERY_RESULT=wrong ./scripts/package-release.sh v0.4.0
rm -rf target/release-artifacts
./scripts/package-release.sh v0.4.0
archive=target/release-artifacts/tq-v0.4.0-aarch64-apple-darwin.tar.gz
before=$(shasum -a 256 "$archive")
expect_failure ./scripts/package-release.sh v0.4.0
[[ $(shasum -a 256 "$archive") == "$before" ]]
[[ $(tar -tzf "$archive") == $'tq\nLICENSE\nrelease.txt' ]]
echo "Release package checks pass."
