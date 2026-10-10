#!/usr/bin/env python3
"""Live two-monitor preset/restart check. Requires clear desktops, grim and Pillow.
Uses an isolated XDG profile and daemon socket; always removes its test surfaces.
"""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
from PIL import Image

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=Path('target/release/wallpicker'))
parser.add_argument('--adjustments-only', action='store_true', help='Check that zoom/pan updates never animate')
parser.add_argument('--monitors', nargs=2, default=['DP-1', 'DP-2'])
args = parser.parse_args()
binary = args.binary.resolve()
names = args.monitors
root = Path(tempfile.mkdtemp(prefix='wallpicker-presets-live-'))
env = os.environ.copy()
for key, folder in [('XDG_CONFIG_HOME', 'config'), ('XDG_STATE_HOME', 'state'),
                    ('XDG_DATA_HOME', 'data'), ('XDG_CACHE_HOME', 'cache')]:
    env[key] = str(root / folder)
    (root / folder / 'wallpicker').mkdir(parents=True)
# Separate abstract socket, same real Wayland display and runtime directory.
env['HYPRLAND_INSTANCE_SIGNATURE'] = 'preset-test-' + root.name
stubs = root / 'bin'
stubs.mkdir()
(stubs / 'noctalia').write_text('#!/bin/sh\nexit 1\n')
(stubs / 'noctalia').chmod(0o755)
env['PATH'] = str(stubs) + ':' + env['PATH']
config = root / 'config/wallpicker/config.json'
state_path = root / 'state/wallpicker/state.json'
settings = {'wallpaper_backend': 'wallpicker',
            'transition': {'kind': 'left', 'duration_ms': 3000 if args.adjustments_only else 400, 'fps': 60},
            'slideshow': {'enabled': False}, 'workspaces': {'enabled': False},
            'span_layout': {names[0]: {'x': 0, 'y': 0}, names[1]: {'x': 600, 'y': 0}},
            'span_monitors': names, 'span_bezel': 0}
config.write_text(json.dumps(settings))
for filename, colors in [('separate.png', [(255, 0, 0), (0, 255, 0)]),
                         ('span.png', [(0, 0, 255), (255, 255, 0)])]:
    image = Image.new('RGB', (400, 200), colors[0])
    image.paste(colors[1], (200, 0, 400, 200))
    image.save(root / filename)
initial = {'current': {n: str(root / 'separate.png') for n in names},
           'adjust': {names[0]: {'flip': {'h': True, 'v': False}},
                      names[1]: {'frame': {'zoom': 200, 'x': 750, 'y': 500,
                                           'width': 1920, 'height': 1080}}}}
state_path.write_text(json.dumps(initial))
address = '\0wallpicker-daemon:' + env.get('XDG_RUNTIME_DIR', '') + ':' + env['HYPRLAND_INSTANCE_SIGNATURE']
daemon = None
log = (root / 'daemon.log').open('w')


def cli(*arguments):
    result = subprocess.run([str(binary), *arguments], env=env, capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise AssertionError(f'{arguments}: {result.stderr or result.stdout}')
    return json.loads(result.stdout) if result.stdout.strip() else None


def status():
    with socket.socket(socket.AF_UNIX) as client:
        client.settimeout(3)
        client.connect(address)
        client.sendall(b'{"cmd":"status"}\n')
        return json.loads(client.makefile().readline())


def settled():
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        try:
            screens = status().get('screens') or []
            if all(any(s['monitor'] == n and s.get('pixels') and not s['fading'] for s in screens) for n in names):
                return screens
        except (OSError, ValueError):
            pass
        time.sleep(0.05)
    raise AssertionError('Test renderer did not settle; see ' + str(root / 'daemon.log'))


def start():
    global daemon
    daemon = subprocess.Popen([str(binary), 'daemon'], env=env, stdout=log, stderr=log)
    settled()


def stop():
    global daemon
    if daemon is not None:
        daemon.terminate()
        try:
            daemon.wait(timeout=5)
        except subprocess.TimeoutExpired:
            daemon.kill()
            daemon.wait(timeout=5)
        daemon = None


def check_pixels(label, colors):
    settled()
    # Allow the compositor's layer mapping animation to finish too.
    time.sleep(0.5)
    for name, expected in zip(names, colors):
        shot = root / f'{label}-{name}.png'
        subprocess.run(['grim', '-o', name, str(shot)], check=True, timeout=10)
        image = Image.open(shot).convert('RGB')
        w, h = image.size
        assert (w, h) == (1920, 1080), f'{name}: this fixture expects 1920x1080'
        for fraction, wanted in zip([0.25, 0.75], expected):
            got = image.getpixel((int(w * fraction), int(h * 0.6)))
            assert max(abs(a-b) for a, b in zip(got, wanted)) < 15, f'{label} {name}: got {got}, expected {wanted}; {shot}'
    print(label + ': both monitors match expected pixels', flush=True)


def apply(saved):
    before = json.loads(state_path.read_text())
    report = cli('presets', 'apply', saved['id'], '--json')
    assert set(report['applied']) == set(names) and not report['skipped'] and not report['failed'], report
    after = json.loads(state_path.read_text())
    for name in names:
        source = saved['monitors'][name]['wallpaper']
        assert after['current'][name] == source
        if before['current'][name] != source:
            assert after['history'][name][-1] == before['current'][name]
    return after


def check_adjustments():
    start()
    for frame in ['180,65,40', '220,70,60']:
        subprocess.run([str(binary), 'adjust', names[1], '--frame', frame],
                       env=env, check=True, capture_output=True, timeout=30)
        assert all(not s['fading'] for s in status()['screens']), 'Single-monitor adjustment animated'
    print('Single-monitor zoom/pan updates: immediate', flush=True)
    def span(frame):
        subprocess.run([str(binary), 'span', str(root / 'span.png'), '--monitors', ','.join(names), '--frame', frame],
                       env=env, check=True, capture_output=True, timeout=30)
    span('100,50,50')
    assert all(s['fading'] for s in status()['screens']), 'New span did not use the normal transition'
    settled()
    for frame in ['150,65,40', '200,70,60']:
        span(frame)
        assert all(not s['fading'] for s in status()['screens']), 'Span adjustment animated'
    assert json.loads(config.read_text())['transition']['kind'] == 'left'
    print('PASS: native zoom/pan updates are immediate; new wallpaper transitions preserved', flush=True)


try:
    if args.adjustments_only:
        check_adjustments()
    else:
        a = cli('presets', 'save', 'Separate adjusted', '--json')
        start()
        separate_colors = [[(0, 255, 0), (255, 0, 0)], [(0, 255, 0), (0, 255, 0)]]
        span_colors = [[(255, 255, 0)] * 2, [(0, 0, 255)] * 2]
        check_pixels('separate-startup', separate_colors)
        subprocess.run([str(binary), 'span', str(root / 'span.png'), '--monitors', ','.join(names), '--flip', 'h'],
                       env=env, check=True, capture_output=True, timeout=30)
        b = cli('presets', 'save', 'Spanned mirrored', '--json')
        check_pixels('span-saved', span_colors)
        state = apply(a)
        assert state.get('span') is None
        assert state['adjust'][names[0]]['flip']['h']
        assert state['adjust'][names[1]]['frame']['zoom'] == 200
        check_pixels('separate-applied', separate_colors)
        state = apply(b)
        assert set(state['span']['monitors']) == set(names)
        assert state['span']['look']['flip']['h']
        check_pixels('span-applied', span_colors)
        stop()
        start()
        check_pixels('span-restarted', span_colors)
        state = apply(a)
        check_pixels('separate-after-restart', separate_colors)
        stop()
        start()
        check_pixels('separate-restarted', separate_colors)
        state = apply(b)
        check_pixels('span-after-second-restart', span_colors)
        saved = json.loads(config.read_text())
        assert saved['wallpaper_backend'] == settings['wallpaper_backend']
        assert saved['transition'] == settings['transition']
        assert not saved['slideshow']['enabled'] and not saved['workspaces']['enabled']
        print('PASS: separate/spanned presets, adjustments, history, transitions and two daemon restarts', flush=True)

finally:
    stop()
    log.close()
    print('Test daemon stopped; original wallpaper surfaces revealed. Artifacts: ' + str(root), flush=True)
