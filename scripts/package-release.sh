#!/bin/bash
# Build and verify one native archive. This command never publishes artifacts.
set -euo pipefail
cd "$(dirname "$0")/.."
tag=${1:?Usage: package-release.sh vVERSION}
version=${tag#v}
version_pattern='^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'
if [[ ! $tag =~ $version_pattern ]]; then
    echo "Expected a v-prefixed release version." >&2
    exit 1
fi
if [[ $version == *-* ]]; then
    IFS=. read -r -a identifiers <<< "${version#*-}"
    for identifier in "${identifiers[@]}"; do
        if [[ $identifier =~ ^[0-9]+$ && $identifier == 0?* ]]; then
            echo "Numeric prerelease identifiers must not have leading zeros." >&2
            exit 1
        fi
    done
fi
if [[ -n $(git status --porcelain) ]]; then
    echo "Release packaging requires a clean checkout of the reviewed tag." >&2
    exit 1
fi
commit=$(git rev-parse HEAD)
if [[ $(git rev-parse --verify "refs/tags/$tag^{commit}") != "$commit" ]]; then
    echo "Check out $tag before packaging." >&2
    exit 1
fi
for package in tq-core tq-toon tq-formats tq-cli; do
    package_id=$(cargo pkgid --locked -p "$package")
    package_version=${package_id##*#}
    if [[ ${package_version##*@} != "$version" ]]; then
        echo "$package does not match $tag." >&2
        exit 1
    fi
done
found=false
while IFS= read -r line; do
    if [[ $line == "## [$version] - "????-??-?? ]]; then found=true; fi
done < CHANGELOG.md
if [[ $found != true ]]; then
    echo "Add the dated $version changelog section before tagging." >&2
    exit 1
fi
RUSTUP_TOOLCHAIN=$(awk -F '"' '/^channel =/ {print $2}' rust-toolchain.toml)
export RUSTUP_TOOLCHAIN
host=$(rustc -vV | awk '/^host:/ {print $2}')
case "$host" in
    aarch64-apple-darwin|x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu) ;;
    *) echo "No validated archive recipe for $host." >&2; exit 1 ;;
esac
./scripts/preflight.sh
cargo test --locked -p tq-core regex_date_platform_release_host_contract
cargo test --locked -p tq-cli ambient
cargo build --locked --release --target "$host" -p tq-cli
mkdir -p target/release-artifacts
archive="tq-$tag-$host.tar.gz"
destination="$PWD/target/release-artifacts/$archive"
if [[ -e $destination || -e $destination.sha256 ]]; then
    echo "Refusing to replace existing release artifacts: $destination" >&2
    exit 1
fi
stage=$(mktemp -d "${TMPDIR:-/tmp}/tq-release.XXXXXX")
trap 'rm -rf "$stage"' EXIT
cp "target/$host/release/tq" "$stage/tq"
cp LICENSE "$stage/LICENSE"
{
    printf 'tag: %s\ncommit: %s\ntarget: %s\nfeatures: default\n' "$tag" "$commit" "$host"
    rustc -vV
    uname -a
    if [[ $host == *linux* ]]; then ldd --version; else sw_vers; fi
} > "$stage/release.txt"
tar -czf "$destination" -C "$stage" tq LICENSE release.txt
mkdir "$stage/extracted"
tar -xzf "$destination" -C "$stage/extracted"
if [[ ! -x $stage/extracted/tq || ! -s $stage/extracted/LICENSE ]]; then
    echo "Extracted archive is missing an executable or license." >&2
    exit 1
fi
reported_version=$("$stage/extracted/tq" --version)
if [[ $reported_version != "tq $version" && $reported_version != "tq $version ("* ]]; then
    echo "Extracted binary version does not match $tag: $reported_version" >&2
    exit 1
fi
actual=$(printf '{"answer":42}\n' | "$stage/extracted/tq" -i json -o json -c '.answer')
if [[ $actual != 42 ]]; then
    echo "Extracted binary failed the JSON query smoke test." >&2
    exit 1
fi
(cd target/release-artifacts && shasum -a 256 "$archive" > "$archive.sha256" && shasum -a 256 -c "$archive.sha256")
printf 'Verified native archive: %s\n' "$destination"
