#!/bin/bash
# Runs the Quickshell interface headless (cage; as a window: cage has no
# layer-shell for the floating panel) with a throwaway profile made
# from yours (no API key, a stand-in hyprctl with two 1920x1080 monitors),
# then the given test-hook calls, a screenshot, and the log's warnings.
# Fails if QML reported errors or warnings. CHECK_FOLDERS=a:b uses those
# wallpaper folders instead of yours; CHECK_PREFS='{...}' sets interface
# choices (services/Prefs.qml) to start with.
#
#   cargo build
#   scripts/ui-check.sh [OUT.png] ["click 1845 136" "wheel 900 600 3" ...]
#
# Test hooks (windows/TestHooks.qml): click X Y, rightclick X Y, wheel X Y
# NOTCHES, scrollY X Y, probe X Y, contents X Y, state; real input through
# QtTest: press X Y BUTTON (1 left, 2 right), move X Y, drag X1 Y1 X2 Y2,
# roll X Y NOTCHES, spin X Y NOTCHES GAP_MS; range X Y. Coordinates are in
# the window's content. A call starting with ! is a command run in the
# session, e.g. "!wtype -M ctrl -k f -m ctrl" for keys, "!sleep 5" to wait
# (then CHECK_EXTRA_TIME=SECONDS keeps the session up that much longer).
# The session has its own cache for adjusted copies (yours are left alone).
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-ui-check.png}
shift || true
tmp=$(mktemp -d)
run=/tmp/wallpicker-ui-check-$UID
mkdir -p "$run" && chmod 700 "$run"
mkdir -p "$tmp/config/wallpicker" "$tmp/state" "$tmp/data" "$tmp/bin" "$tmp/cache/wallpicker"
# Your caches (thumbnails...), but not the adjusted copies: adjusting prunes
# the ones its own state doesn't use, which would be yours.
for entry in ~/.cache/wallpicker/*; do
    [ "$(basename "$entry")" = transformed ] || ln -s "$entry" "$tmp/cache/wallpicker/"
done
python3 - "$tmp" <<'PY'
import json, os, sys
tmp = sys.argv[1]
s = json.load(open(os.path.expanduser('~/.config/wallpicker/config.json')))
s['wallhaven_api_key'] = None
s['wallpaper_backend'] = 'hyprpaper'
s['workspaces'] = {'enabled': False, 'remember': False, 'wallpapers': {}}
# CHECK_FOLDERS (colon-separated): other wallpaper folders, e.g. a throwaway
# one with test pictures.
if os.environ.get('CHECK_FOLDERS'):
    s['folders'] = os.environ['CHECK_FOLDERS'].split(':')
json.dump(s, open(f'{tmp}/config/wallpicker/config.json', 'w'))
PY
cp -r ~/.local/state/wallpicker "$tmp/state/" 2>/dev/null || true
cp -r ~/.local/share/wallpicker "$tmp/data/" 2>/dev/null || true
# CHECK_PREFS='{"localLayout":"folders"}': interface choices to start with.
if [ -n "$CHECK_PREFS" ]; then
    python3 - "$tmp/state/wallpicker/ui.json" <<'PY'
import json, os, sys
try:
    prefs = json.load(open(sys.argv[1]))
except (OSError, ValueError):
    prefs = {}
prefs.update(json.loads(os.environ['CHECK_PREFS']))
os.makedirs(os.path.dirname(sys.argv[1]), exist_ok=True)
json.dump(prefs, open(sys.argv[1], 'w'))
PY
fi
cat > "$tmp/bin/hyprctl" <<'SH'
#!/bin/sh
case "$*" in *monitors*) echo '[{"name":"DP-1","width":1920,"height":1080,"x":0,"y":0,"focused":true},{"name":"DP-2","width":1920,"height":1080,"x":1920,"y":0,"focused":false}]';; *workspaces*) echo '[]';; *) echo ok;; esac
SH
chmod +x "$tmp/bin/hyprctl"
{
    echo '#!/bin/bash'
    echo 'wlr-randr --output HEADLESS-1 --custom-mode 1920x1080'
    echo "WALLPICKER_DEBUG_WHEEL=${WALLPICKER_DEBUG_WHEEL:-} WALLPICKER_TEST=1 WALLPICKER_WINDOW=1 WALLPICKER_BIN='$here/target/debug/wallpicker' qs -p '$here/quickshell' > '$tmp/ui.log' 2>&1 &"
    echo 'sleep 6'
    # wtype's first key after startup is lost (its keyboard isn't set up
    # yet): a harmless one first.
    echo 'wtype -k Shift_L 2>/dev/null; sleep 0.3'
    for call in "$@"; do
        if [ "${call#!}" != "$call" ]; then
            # A command run in the session (wtype for keys).
            echo "( ${call#!} ) >> '$tmp/calls.txt' 2>&1; echo \"$call\" >> '$tmp/calls.txt'"
        else
            echo "echo \"$call -> \$(qs -p '$here/quickshell' ipc call wallpicker-test $call 2>&1)\" >> '$tmp/calls.txt'"
        fi
        echo 'sleep 0.6'
    done
    echo "grim '$tmp/shot.png'"
} > "$tmp/inside.sh"
chmod +x "$tmp/inside.sh"
env -u HYPRLAND_INSTANCE_SIGNATURE -u WAYLAND_DISPLAY -u DISPLAY PATH="$tmp/bin:$PATH" \
    XDG_CONFIG_HOME="$tmp/config" XDG_STATE_HOME="$tmp/state" XDG_DATA_HOME="$tmp/data" XDG_CACHE_HOME="$tmp/cache" XDG_RUNTIME_DIR="$run" \
    WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 \
    timeout $((20 + ${CHECK_EXTRA_TIME:-0} + $#)) dbus-run-session -- cage -- "$tmp/inside.sh" >/dev/null 2>&1 || true
cat "$tmp/calls.txt" 2>/dev/null || true
cp "$tmp/shot.png" "$out" 2>/dev/null && echo "screenshot: $out"
problems=$(grep -E "ERROR|WARN|caused by" "$tmp/ui.log" | grep -vE "radv|lsfg|portal|not a conformant" || true)
if [ -n "$problems" ]; then
    echo "QML problems:"; echo "$problems"; exit 1
fi
echo "no QML errors or warnings"
