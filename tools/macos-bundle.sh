#!/bin/bash
# Builds "MORSEHUNTER.app" as one binary for Apple Silicon and Intel, and a disk
# image to hand out. Runs on macOS (lipo, sips, iconutil, codesign, hdiutil).
#   tools/macos-bundle.sh            -> dist/MORSEHUNTER-<version>-macos.dmg
set -euo pipefail
cd "$(dirname "$0")/.."

name="MORSEHUNTER"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
app="dist/$name.app"

for t in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$t" >/dev/null
    MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release --locked -p morsehunter --target "$t"
done

rm -rf dist && mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/morsehunter" \
    target/aarch64-apple-darwin/release/morsehunter \
    target/x86_64-apple-darwin/release/morsehunter

# the icon in all the sizes macOS asks for
set_dir=$(mktemp -d)/icon.iconset
mkdir -p "$set_dir"
for s in 16 32 128 256 512; do
    sips -z $s $s assets/icon.png --out "$set_dir/icon_${s}x${s}.png" >/dev/null
    sips -z $((s * 2)) $((s * 2)) assets/icon.png --out "$set_dir/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$set_dir" -o "$app/Contents/Resources/icon.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$name</string>
    <key>CFBundleDisplayName</key><string>$name</string>
    <key>CFBundleIdentifier</key><string>de.dl8ug.morsehunter</string>
    <key>CFBundleExecutable</key><string>morsehunter</string>
    <key>CFBundleIconFile</key><string>icon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.education</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSHumanReadableCopyright</key><string>MPL-2.0, after Morse Runner by VE3NEA</string>
</dict>
</plist>
PLIST
cp LICENSE NOTICE "$app/Contents/Resources/" 2>/dev/null || true

# no Apple developer ID: an ad-hoc signature, which Apple Silicon needs to start it at all
codesign --force --deep --sign - "$app"
codesign --verify --verbose "$app"

stage=$(mktemp -d)
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
hdiutil create -volname "$name" -srcfolder "$stage" -ov -format UDZO "dist/MORSEHUNTER-$version-macos.dmg"
ls -la dist
