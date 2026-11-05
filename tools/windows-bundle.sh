#!/bin/bash
# Packs the Windows build into a zip: unpack anywhere and start morsehunter.exe.
#   tools/windows-bundle.sh          on Windows (Git Bash), as the Release workflow does
#   TARGET=x86_64-pc-windows-gnu tools/windows-bundle.sh
#                                    from Linux, with cargo-zigbuild and llvm-windres
#   -> dist/MORSEHUNTER-<version>-windows-x86_64.zip
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
dir="MORSEHUNTER-$version-windows-x86_64"
stage="dist/$dir"

if [ -n "${TARGET:-}" ]; then
    # the icon needs a resource compiler, llvm-windres does it for the GNU target
    export "RC_${TARGET//-/_}=${RC:-llvm-windres}"
    cargo zigbuild --release --locked --target "$TARGET" -p morsehunter
    exe="target/$TARGET/release/morsehunter.exe"
else
    cargo build --release --locked -p morsehunter
    exe="target/release/morsehunter.exe"
fi

rm -rf dist && mkdir -p "$stage"
cp "$exe" "$stage/"
cp README.md LICENSE NOTICE "$stage/"

if command -v 7z >/dev/null; then
    (cd dist && 7z a -tzip -bso0 "$dir.zip" "$dir")
else
    (cd dist && zip -qr "$dir.zip" "$dir")
fi
rm -rf "$stage"
ls -la dist
