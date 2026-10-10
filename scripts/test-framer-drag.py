#!/usr/bin/env python3
"""Headless slow-drag regression at high zoom. Requires ui-check.sh's tools and Pillow."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
from PIL import Image

repo = Path(__file__).resolve().parent.parent
root = Path(tempfile.mkdtemp(prefix='wallpicker-framer-drag-'))
image = root / 'drag-fixture.png'
Image.new('RGB', (2742, 1518), (70, 120, 160)).save(image)
seed = root / 'seed.py'
seed.write_text('''import json,os,sys
from pathlib import Path
path=Path(os.environ['XDG_STATE_HOME'])/'wallpicker/state.json'
state=json.loads(path.read_text())
state['current']['HEADLESS-1']=sys.argv[1]
state.setdefault('adjust',{})['HEADLESS-1']={'flip':{'h':len(sys.argv)>2,'v':False},'frame':{'zoom':330,'x':500,'y':500,'width':1920,'height':1080}}
path.write_text(json.dumps(state))
''')
calls = [
    f'!python3 {seed} {image}', '!sleep 1', 'dialog adjust HEADLESS-1', 'state',
    'drag 960 350 954 350', 'state',
    'drag 954 350 960 350', 'state',
    'drag 960 350 960 344', 'state',
    'drag 960 344 960 350', 'state',
    '!wtype -k Escape', f'!python3 {seed} {image} mirror', '!sleep 1',
    'dialog adjust HEADLESS-1', 'drag 960 350 954 350', 'state',
    'drag 954 350 960 350', 'state',
]
env = os.environ.copy()
env['CHECK_FOLDERS'] = str(root)
env['CHECK_EXTRA_TIME'] = '15'
result = subprocess.run([str(repo / 'scripts/ui-check.sh'), str(root / 'drag.png'), *calls],
                        cwd=repo, env=env, capture_output=True, text=True, timeout=90)
(root / 'result.log').write_text(result.stdout + result.stderr)
assert result.returncode == 0, result.stdout + result.stderr
assert (root / 'drag.png').exists(), 'Headless session ended before the screenshot'
frames = [json.loads(line.split(' -> ', 1)[1])['adjust']['HEADLESS-1']['frame']
          for line in result.stdout.splitlines() if line.startswith('state -> ')]
assert len(frames) == 7, result.stdout
assert frames[1]['x'] > frames[0]['x'], frames
assert abs(frames[2]['x'] - 500) <= 1, frames
assert frames[3]['y'] > frames[2]['y'], frames
assert abs(frames[4]['y'] - 500) <= 1, frames
assert frames[5]['x'] < 500, frames
assert abs(frames[6]['x'] - 500) <= 1, frames
print('PASS: slow horizontal/vertical drags, reverse drags and mirrored dragging at 330% zoom')
print('Artifacts:', root)
