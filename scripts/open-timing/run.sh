#!/bin/bash
# Times cold opens of the window: the app starts in the tray, then the window
# is opened and closed (destroyed) a few times with --toggle, as a keybind
# does. Each open prints its timeline from the moment it was asked for.
#
#   cargo tauri build --no-bundle --features automation
#   scripts/open-timing/run.sh [OPENS]
#
# Headless (cage), a throwaway profile made from yours (no API key, a
# stand-in hyprctl); the thumbnail cache is your real one.
set -e
here=$(cd "$(dirname "$0")" && pwd)
opens=${1:-3}
tmp=$(mktemp -d)
cp "$here/../../target/release/wallpicker-app" "$tmp/wallpicker-auto"
mkdir -p "$tmp/config/wallpicker" "$tmp/state" "$tmp/data" "$tmp/bin"
run=/tmp/wallpicker-memory-$UID
mkdir -p "$run" && chmod 700 "$run"
python3 - "$tmp" <<'PY'
import json, os, sys
tmp = sys.argv[1]
s = json.load(open(os.path.expanduser('~/.config/wallpicker/config.json')))
s['wallhaven_api_key'] = None
s['wallpaper_backend'] = 'hyprpaper'
s['workspaces'] = {'enabled': False, 'remember': False, 'wallpapers': {}}
s['close_to_tray'] = True
json.dump(s, open(f'{tmp}/config/wallpicker/config.json', 'w'))
PY
cp -r ~/.local/state/wallpicker "$tmp/state/" 2>/dev/null || true
cp -r ~/.local/share/wallpicker "$tmp/data/" 2>/dev/null || true
cat > "$tmp/bin/hyprctl" <<'SH'
#!/bin/sh
case "$*" in *monitors*) echo '[{"name":"DP-1","width":1920,"height":1080,"x":0,"y":0,"focused":true}]';; *workspaces*) echo '[]';; *) echo ok;; esac
SH
chmod +x "$tmp/bin/hyprctl"
cat > "$tmp/cycle.sh" <<SH
#!/bin/sh
"$tmp/wallpicker-auto" --tray > "$tmp/out.log" 2>&1 &
sleep 3
for i in \$(seq 1 $opens); do
  "$tmp/wallpicker-auto" --toggle; sleep 14
  "$tmp/wallpicker-auto" --toggle; sleep 3
done
SH
chmod +x "$tmp/cycle.sh"
env -u HYPRLAND_INSTANCE_SIGNATURE -u WAYLAND_DISPLAY -u DISPLAY PATH="$tmp/bin:$PATH" \
  XDG_CONFIG_HOME="$tmp/config" XDG_STATE_HOME="$tmp/state" XDG_DATA_HOME="$tmp/data" XDG_RUNTIME_DIR="$run" \
  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 WALLPICKER_AUTOMATE="$here/collect.js" \
  timeout $((opens * 18 + 15)) dbus-run-session -- cage -- "$tmp/cycle.sh" >/dev/null 2>&1 || true
python3 - "$tmp/out.log" <<'PY'
import json, sys
opens = [json.loads(l.split(' ', 1)[1]) for l in open(sys.argv[1]) if l.startswith('open-timing ')]
if not opens:
    sys.exit('no timings (did the window open?)')
rows = [('window built', lambda o: o['built'] - o['asked']),
        ('page starts loading', lambda o: o['page']['origin'] - o['asked'])]
at = lambda key: (lambda o: o['page']['origin'] - o['asked'] + o['page']['marks'][key] if key in o['page']['marks'] else None)
rows += [('page ready (DOM)', lambda o: o['page']['origin'] - o['asked'] + o['page']['dom_ready'] if o['page']['dom_ready'] else None),
         ('UI code starts (wasm)', at('main')),
         ('first paint', lambda o: o['page']['origin'] - o['asked'] + o['page']['paint']['first-paint'] if 'first-paint' in o['page']['paint'] else None),
         ('UI drawn (mounted)', at('mounted')), ('first request sent', at('fetching')),
         ('monitors', at('monitors')), ('settings', at('settings')), ('state', at('state')), ('catalog', at('catalog')),
         ('library scanned', at('scan')), ('tiles created', at('tiles')), ('tiles on screen', at('tiles-painted')), ('first thumbnail', at('first-thumbnail')),
         ('thumbnails on screen done', lambda o: o['page']['origin'] - o['asked'] + o['page']['thumbnails_done'] if o['page']['thumbnails_done'] else None)]
print(f"{'ms after asking':28}" + ''.join(f'{"open " + str(i + 1):>9}' for i in range(len(opens))))
for name, f in rows:
    print(f'{name:28}' + ''.join(f'{(f(o) if f(o) is not None else "-"):>9}' for o in opens))
print(f"{'thumbnails loaded':28}" + ''.join(f"{o['page']['thumbnails']:>9}" for o in opens))
for r in opens[0]['page']['resources']:
    print(f"  resource {r['name']}: {r['start']}-{r['end']} ms after page start")
PY
rm -rf "$tmp" 2>/dev/null || true
