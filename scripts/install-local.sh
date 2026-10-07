#!/bin/sh
# Install a release build for your user: `wallpicker` (the picker's
# backend, the daemon and the command line) in ~/.local/bin, the picker (the
# Quickshell interface) in ~/.local/share/wallpicker/ui, a launcher entry and
# its icon. Build first: cargo build --release -p wallpicker
set -eu
cd "$(dirname "$0")/.."

bin="$HOME/.local/bin/wallpicker"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
apps="$data/applications"
icons="$data/icons/hicolor/512x512/apps"
ui="$data/wallpicker/ui"

# A build older than its sources would pair the new interface with an old
# backend (methods it doesn't have yet).
release=target/release/wallpicker
stale=$(find crates Cargo.lock -newer "$release" -type f 2>/dev/null | head -n 1 || true)
if [ ! -e "$release" ] || [ -n "$stale" ]; then
    echo "install-local: $release is missing or older than ${stale:-its sources}; run \`cargo build --release -p wallpicker\` first" >&2
    exit 1
fi

install -Dm755 "$release" "$bin"
# The picker, copied over the last one.
mkdir -p "$ui"
cp -r quickshell/. "$ui/"
install -Dm644 packaging/wallpicker.png "$icons/wallpicker.png"
mkdir -p "$apps"
# A full path: launchers (and Noctalia) often run with a PATH that doesn't
# have ~/.local/bin.
sed "s|^Exec=wallpicker|Exec=$bin|" packaging/wallpicker.desktop > "$apps/wallpicker.desktop"
chmod 644 "$apps/wallpicker.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database "$apps" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$(dirname "$(dirname "$icons")")" 2>/dev/null || true
echo "installed: $bin, $ui, $apps/wallpicker.desktop"
