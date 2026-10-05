#!/usr/bin/env python3
"""Run the panel's smoke test under plain Luau.

Noctalia's `require("./x.luau")` and standalone Luau's `require("./x")`
disagree, so the plugin's files are bundled into one script with a small
`require` of its own, after mock.luau's fake host. The output is compared
with smoke.snap.

    tests/run.py            compare with smoke.snap
    tests/run.py --update   write smoke.snap from this run
    LUAU=/path/to/luau tests/run.py

Needs the `luau` binary (https://github.com/luau-lang/luau/releases).
"""

import difflib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
PLUGIN = HERE.parent / "wallpicker"
SNAP = HERE / "smoke.snap"


def luau_string(text: str) -> str:
    return json.dumps(text, ensure_ascii=False)


def module_table(value) -> str:
    """A JSON value as a Luau table literal (translations only: strings and objects)."""
    if isinstance(value, dict):
        items = ", ".join(f"[{luau_string(k)}] = {module_table(v)}" for k, v in value.items())
        return "{ " + items + " }"
    return luau_string(value)


def flatten(prefix: str, value: dict, out: dict) -> None:
    for k, v in value.items():
        key = f"{prefix}.{k}" if prefix else k
        if isinstance(v, dict):
            flatten(key, v, out)
        else:
            out[key] = v


def bundle() -> str:
    modules = sorted(p for p in PLUGIN.glob("*.luau") if p.name not in ("panel.luau", "bar.luau"))
    # Declared first, so the modules (which require each other) see it.
    parts = ["--!nonstrict", "local __modules, __loaded = {}, {}", "local require"]
    for m in modules:
        parts.append(f'__modules["./{m.name}"] = function()\n{m.read_text()}\nend')
    mock_src = (HERE / "mock.luau").read_text()
    parts.append(f"local MOCK = (function()\n{mock_src}\nend)()")
    parts.append(
        "require = function(path)\n"
        "  if __loaded[path] == nil then\n"
        "    local f = __modules[path]\n"
        '    assert(f, "require: no module " .. path)\n'
        "    __loaded[path] = f()\n"
        "  end\n"
        "  return __loaded[path]\n"
        "end"
    )
    strings: dict = {}
    flatten("", json.loads((PLUGIN / "translations" / "en.json").read_text()), strings)
    parts.append(f"MOCK.setTranslations({module_table(strings)})")
    parts.append("local ui, panel, noctalia = MOCK.ui, MOCK.panel, MOCK.noctalia")
    parts.append("math.randomseed(1)")
    parts.append(f"do\n{(PLUGIN / 'panel.luau').read_text()}\nend")
    parts.append(f"do\n{(HERE / 'smoke.luau').read_text()}\nend")
    # `--!nonstrict` only counts on the first line.
    return re.sub(r"(?m)^--!nonstrict\n", "", "\n".join(parts), count=0).replace(
        "local __modules", "--!nonstrict\nlocal __modules", 1
    )


def main() -> int:
    luau = os.environ.get("LUAU", "luau")
    with tempfile.TemporaryDirectory() as tmp:
        script = Path(tmp) / "bundle.luau"
        script.write_text(bundle())
        run = subprocess.run([luau, str(script)], capture_output=True, text=True)
    if run.returncode != 0:
        sys.stderr.write(run.stdout + run.stderr)
        return 1
    got = run.stdout
    if "--update" in sys.argv:
        SNAP.write_text(got)
        print(f"wrote {SNAP.name} ({got.count(chr(10))} lines)")
        return 0
    want = SNAP.read_text() if SNAP.exists() else ""
    if got == want:
        print(f"smoke test: same as {SNAP.name} ({got.count(chr(10))} lines)")
        return 0
    sys.stdout.writelines(difflib.unified_diff(want.splitlines(True), got.splitlines(True), "smoke.snap", "this run"))
    return 1


if __name__ == "__main__":
    sys.exit(main())
