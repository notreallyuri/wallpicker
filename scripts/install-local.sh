#!/bin/sh
# Install release builds for your user: `wallpicker` (the command line and
# the Quickshell interface's backend) and `wallpicker-app` (the old window)
# in ~/.local/bin, the interface, a launcher entry and its icon. Build first:
#   cargo build --release -p wallpicker
#   cargo tauri build --no-bundle   (the window; a plain cargo build of it
#                                    would load the dev server)
set -eu
cd "$(dirname "$0")/.."

bin="$HOME/.local/bin/wallpicker"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/512x512/apps"

# A build older than its sources would pair the new interface with an old
# backend (methods it doesn't have yet). `check BINARY HOW SOURCES...`.
check() {
    built=$1 how=$2
    shift 2
    stale=$(find "$@" -newer "$built" -type f 2>/dev/null | head -n 1 || true)
    if [ ! -e "$built" ] || [ -n "$stale" ]; then
        echo "install-local: $built is missing or older than ${stale:-its sources}; run \`$how\` first" >&2
        exit 1
    fi
}
check target/release/wallpicker "cargo build --release -p wallpicker" crates Cargo.lock
check target/release/wallpicker-app "cargo tauri build --no-bundle" crates src-tauri/src src-tauri/Cargo.toml ui/src Cargo.lock

install -Dm755 target/release/wallpicker "$bin"
# Beside it: `wallpicker` hands the window's arguments to it.
install -Dm755 target/release/wallpicker-app "$(dirname "$bin")/wallpicker-app"
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
echo "installed: $bin, $(dirname "$bin")/wallpicker-app, $apps/wallpicker.desktop, $ui"
