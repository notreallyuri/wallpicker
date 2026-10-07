#!/bin/sh
# Install a release build for your user: the binary in ~/.local/bin, a
# launcher entry and its icon. Build first with `cargo tauri build --no-bundle`
# (a plain `cargo build --release` would load the dev server).
set -eu
cd "$(dirname "$0")/.."

bin="$HOME/.local/bin/wallpicker"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/512x512/apps"

# A release build older than the Rust sources would pair the new interface
# with an old backend (methods it doesn't have yet).
release=target/release/wallpicker
stale=$(find crates src-tauri/src src-tauri/Cargo.toml Cargo.lock -newer "$release" -type f 2>/dev/null | head -n 1 || true)
if [ ! -e "$release" ] || [ -n "$stale" ]; then
    echo "install-local: $release is missing or older than ${stale:-the sources}; run \`cargo tauri build --no-bundle\` first" >&2
    exit 1
fi

install -Dm755 "$release" "$bin"
# The Quickshell interface (`wallpicker ui`), copied over the last one.
ui="${XDG_DATA_HOME:-$HOME/.local/share}/wallpicker/ui"
mkdir -p "$ui"
cp -r quickshell/. "$ui/"
install -Dm644 src-tauri/icons/icon.png "$icons/wallpicker.png"
mkdir -p "$apps"
# A full path: launchers (and Noctalia) often run with a PATH that doesn't
# have ~/.local/bin.
sed "s|^Exec=wallpicker|Exec=$bin|" packaging/wallpicker.desktop > "$apps/wallpicker.desktop"
chmod 644 "$apps/wallpicker.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database "$apps" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$(dirname "$(dirname "$icons")")" 2>/dev/null || true
echo "installed: $bin, $apps/wallpicker.desktop, $ui"
