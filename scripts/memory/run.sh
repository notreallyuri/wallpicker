#!/bin/bash
# Replays a browsing session (session.js) in a headless copy of the app and
# records its memory every second, per process and by kind, to a CSV.
#
#   cargo tauri build --no-bundle --features automation
#   scripts/memory/run.sh [VARIANT] [OUT.csv]
#
# VARIANT: default, nofx (no blur or animations), noimg (images hidden).
# Needs cage. Uses a throwaway profile made from yours (no API key, a stand-in
# hyprctl, workspaces off); the thumbnail cache is your real one.
set -e
here=$(cd "$(dirname "$0")" && pwd)
variant=${1:-default}
out=${2:-memory-$variant.csv}
tmp=$(mktemp -d)
bin=$here/../../target/release/wallpicker
cp "$bin" "$tmp/wallpicker-auto"
mkdir -p "$tmp/config/wallpicker" "$tmp/state" "$tmp/data" "$tmp/bin"
# A fixed runtime folder, kept between runs: services on the run's own
# session bus (document portal, gvfs) mount themselves in it. Short, for
# socket paths.
run=/tmp/wallpicker-memory-$UID
mkdir -p "$run" && chmod 700 "$run"
python3 - "$tmp" <<'PY'
import json, os, sys
tmp = sys.argv[1]
s = json.load(open(os.path.expanduser('~/.config/wallpicker/config.json')))
s['wallhaven_api_key'] = None
s['wallpaper_backend'] = 'hyprpaper'
s['workspaces'] = {'enabled': False, 'remember': False, 'wallpapers': {}}
json.dump(s, open(f'{tmp}/config/wallpicker/config.json', 'w'))
PY
cp -r ~/.local/state/wallpicker "$tmp/state/" 2>/dev/null || true
cp -r ~/.local/share/wallpicker "$tmp/data/" 2>/dev/null || true
cat > "$tmp/bin/hyprctl" <<'SH'
#!/bin/sh
case "$*" in *monitors*) echo '[{"name":"DP-1","width":1920,"height":1080,"x":0,"y":0,"focused":true}]';; *workspaces*) echo '[]';; *) echo ok;; esac
SH
chmod +x "$tmp/bin/hyprctl"
(echo "const VARIANT = '$variant';"; cat "$here/session.js") > "$tmp/session.js"
python3 "$here/record.py" "$out" wallpicker-auto 95 &
rec=$!
env -u HYPRLAND_INSTANCE_SIGNATURE -u WAYLAND_DISPLAY -u DISPLAY PATH="$tmp/bin:$PATH" \
  XDG_CONFIG_HOME="$tmp/config" XDG_STATE_HOME="$tmp/state" XDG_DATA_HOME="$tmp/data" XDG_RUNTIME_DIR="$run" \
  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 WALLPICKER_AUTOMATE="$tmp/session.js" \
  timeout 105 dbus-run-session -- cage -- "$tmp/wallpicker-auto" --open local >/dev/null 2>&1 || true
wait $rec
rm -rf "$tmp"
python3 - "$out" <<'PY'
import csv, sys
rows = [r for r in csv.DictReader(open(sys.argv[1])) if int(r['web']) > 0]
late = [r for r in rows if int(r['secs']) > 12] or rows
print(f"{sys.argv[1]}: peak {max(int(r['total']) for r in late)} MB while browsing; "
      f"page process working memory peak {max(int(r['web_anon']) for r in late)} MB, at the end {late[-1]['web_anon']} MB")
PY
