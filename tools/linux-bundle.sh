#!/bin/bash
# Packs the Linux build with icon, desktop entry and an install script.
#   tools/linux-bundle.sh            -> dist/MORSEHUNTER-<version>-linux-<arch>.tar.gz
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
arch=$(uname -m)
dir="MORSEHUNTER-$version-linux-$arch"
stage="dist/$dir"

cargo build --release --locked -p morsehunter

rm -rf dist && mkdir -p "$stage"
cp target/release/morsehunter "$stage/"
strip "$stage/morsehunter"
cp assets/icon-256.png "$stage/morsehunter.png"
cp README.md LICENSE NOTICE "$stage/"

cat > "$stage/morsehunter.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=MORSEHUNTER
Comment=CW contest simulator after Morse Runner by VE3NEA
Exec=morsehunter
Icon=morsehunter
Terminal=false
Categories=Education;HamRadio;
StartupWMClass=morsehunter
DESKTOP

cat > "$stage/install.sh" <<'INSTALL'
#!/bin/sh
# Installs for this user only: ~/.local/bin, the icon and the menu entry.
#   ./install.sh             install or update
#   ./install.sh --remove    take it away again (settings and scores stay)
set -e
cd "$(dirname "$0")"
bin="$HOME/.local/bin"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
if [ "$1" = "--remove" ]; then
    rm -f "$bin/morsehunter" "$icons/morsehunter.png" "$apps/morsehunter.desktop"
    echo "removed"
    exit 0
fi
mkdir -p "$bin" "$icons" "$apps"
install -m 755 morsehunter "$bin/morsehunter"
install -m 644 morsehunter.png "$icons/morsehunter.png"
sed "s|^Exec=.*|Exec=$bin/morsehunter|" morsehunter.desktop > "$apps/morsehunter.desktop"
command -v update-desktop-database >/dev/null && update-desktop-database "$apps" 2>/dev/null || true
echo "installed to $bin/morsehunter"
case ":$PATH:" in *":$bin:"*) ;; *) echo "note: $bin is not in PATH, the menu entry works all the same" ;; esac
INSTALL
chmod 755 "$stage/install.sh"

tar -C dist -czf "dist/$dir.tar.gz" "$dir"
rm -rf "$stage"
ls -la dist
