#!/usr/bin/env python3
"""Give the library's Wallhaven wallpapers their metadata (TODO.md, Metadata):
origin, source, uploader and artists, from Wallhaven's API, and take the
artist names out of their tags.

Finds wallhaven-<id>.{jpg,png,webp} under the wallpaper folders and the
download folder, and wallhaven_<id> ones (saved from the site by hand), which
it renames to wallhaven-<id> as wallpicker names its downloads (so they count
as downloaded), unless that name is taken. Asks /api/v1/w/<id> for each
(cached, at most 40 a minute) and shows what would change. Nothing is
written or renamed without --write.

    scripts/wallhaven-metadata.py              # dry run: what would change
    scripts/wallhaven-metadata.py --write      # do it (the files it changes copied first)
    --refresh    ask Wallhaven again instead of using the cached answers
    --overwrite  replace metadata a wallpaper already has (else only fill in)
    --no-rename  leave wallhaven_<id> files named as they are
    --force      write even a library.json from before metadata (version 1)

A renamed file is renamed everywhere wallpicker remembers it too: the
library, what each monitor shows and the history (state.json), the
wallpapers per workspace (config.json) and hyprpaper's config. Reopen the
picker afterwards (a picker kept ready still lists the old names).

Exit codes: 0 done, 1 something failed, 2 usage.
"""

import argparse
import json
import os
import re
import shutil
import sys
import tempfile
import time
import urllib.error
import urllib.request

API = "https://wallhaven.cc/api/v1/w/"
# Wallhaven allows 45 requests a minute; stay under it.
GAP = 60 / 40
# wallhaven-<id> as wallpicker downloads it; wallhaven_<id> saved by hand.
FILE = re.compile(r"^wallhaven([-_])([A-Za-z0-9]{1,32})\.(jpg|png|webp)$")
# As core's uploader_query: what can follow "@" without reading as other syntax.
USERNAME = re.compile(r"^[A-Za-z0-9_.-]{1,64}$")


def xdg(var, fallback):
    return os.environ.get(var) or os.path.expanduser(fallback)


CONFIG = os.path.join(xdg("XDG_CONFIG_HOME", "~/.config"), "wallpicker", "config.json")
LIBRARY = os.path.join(xdg("XDG_DATA_HOME", "~/.local/share"), "wallpicker", "library.json")
CACHE = os.path.join(xdg("XDG_CACHE_HOME", "~/.cache"), "wallpicker", "metadata", "wallhaven")
STATE = os.path.join(xdg("XDG_STATE_HOME", "~/.local/state"), "wallpicker", "state.json")
HYPRPAPER = os.path.join(xdg("XDG_STATE_HOME", "~/.local/state"), "wallpicker", "hyprpaper.conf")


def load_json(path, missing):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return missing


def find_files(settings):
    """id -> [paths], as the library's scan names them (the folder as set,
    joined), and the wallhaven_<id> ones among them."""
    roots = list(settings.get("folders") or [])
    download = settings.get("download_dir")
    if download and download not in roots:
        roots.append(download)
    recursive = settings.get("recursive", True)
    found = {}
    seen = set()
    by_hand = set()
    for root in roots:
        root = os.path.expanduser(root)
        for folder, dirs, names in os.walk(root):
            if not recursive:
                dirs.clear()
            dirs.sort()
            for name in sorted(names):
                m = FILE.match(name)
                path = os.path.join(folder, name)
                if m and path not in seen and os.path.isfile(path):
                    seen.add(path)
                    found.setdefault(m.group(2), []).append(path)
                    if m.group(1) == "_":
                        by_hand.add(path)
    return found, by_hand


def plan_renames(by_hand):
    """old -> new (wallhaven_<id> to wallhaven-<id>, beside it), and the ones
    left as they are because that name is taken."""
    renames, taken = {}, []
    for old in sorted(by_hand):
        folder, name = os.path.split(old)
        new = os.path.join(folder, "wallhaven-" + name[len("wallhaven_"):])
        if os.path.lexists(new) or new in renames.values():
            taken.append(old)
        else:
            renames[old] = new
    return renames, taken


def rename_files(renames):
    """The renames that were done: one whose new name turned up meanwhile, or
    that fails, is left (and said)."""
    done = {}
    for old, new in renames.items():
        # os.rename would replace a file that appeared since the plan.
        if os.path.lexists(new):
            print(f"not renamed, {os.path.basename(new)} exists now: {old}", file=sys.stderr)
            continue
        try:
            os.rename(old, new)
            done[old] = new
        except OSError as e:
            print(f"could not rename {old}: {e}", file=sys.stderr)
    return done


def renamed_in(value, done):
    """`value` (parsed JSON) with every string that's a renamed path, as a
    key or a value, renamed."""
    if isinstance(value, dict):
        return {done.get(k, k): renamed_in(v, done) for k, v in value.items()}
    if isinstance(value, list):
        return [renamed_in(v, done) for v in value]
    if isinstance(value, str):
        return done.get(value, value)
    return value


def write_atomic(path, text):
    folder = os.path.dirname(path)
    os.makedirs(folder, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=folder, prefix="." + os.path.basename(path) + ".")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(text)
    os.replace(tmp, path)


def write_json(path, value):
    write_atomic(path, json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def rename_references(done):
    """What each monitor shows, the history, the wallpapers per workspace and
    hyprpaper's config: the renamed files' new paths. Returns the files changed."""
    changed = []
    for path in (STATE, CONFIG):
        before = load_json(path, None)
        if before is None:
            continue
        after = renamed_in(before, done)
        if after != before:
            write_json(path, after)
            changed.append(path)
    try:
        with open(HYPRPAPER, encoding="utf-8") as f:
            text = f.read()
    except FileNotFoundError:
        return changed
    # Longest first: a path that's the start of another can't eat it.
    new_text = text
    for old in sorted(done, key=len, reverse=True):
        new_text = new_text.replace(old, done[old])
    if new_text != text:
        write_atomic(HYPRPAPER, new_text)
        changed.append(HYPRPAPER)
    return changed


class Wallhaven:
    def __init__(self, key, refresh):
        self.key = key
        self.refresh = refresh
        self.last = 0.0
        self.asked = 0

    def info(self, wid):
        """The wallpaper's `data`, or None when Wallhaven doesn't have it any more."""
        cached = os.path.join(CACHE, wid + ".json")
        if not self.refresh:
            hit = load_json(cached, None)
            if hit is not None:
                return hit.get("data")
        body = self.ask(wid)
        os.makedirs(CACHE, exist_ok=True)
        with open(cached, "w", encoding="utf-8") as f:
            json.dump(body, f)
        return body.get("data")

    def ask(self, wid):
        headers = {"User-Agent": "wallpicker-metadata"}
        if self.key:
            headers["X-API-Key"] = self.key
        for attempt in range(4):
            wait = self.last + GAP - time.monotonic()
            if wait > 0:
                time.sleep(wait)
            self.last = time.monotonic()
            self.asked += 1
            req = urllib.request.Request(API + wid, headers=headers)
            try:
                with urllib.request.urlopen(req, timeout=30) as resp:
                    return json.load(resp)
            except urllib.error.HTTPError as e:
                if e.code == 404:
                    return {"data": None}
                if e.code == 401:
                    raise SystemExit("Wallhaven refused the API key (401): check Settings → Wallhaven.")
                if e.code == 429 and attempt < 3:
                    print("  Wallhaven says slow down; waiting a minute", file=sys.stderr)
                    time.sleep(60)
                    continue
                raise
        raise RuntimeError("still rate-limited")


def clean(name):
    """As core's normalize_tag: whitespace trimmed and collapsed."""
    return " ".join((name or "").split())


def metadata(wid, data):
    """The entry's metadata (TODO.md's shape), and the artist names and aliases (lowercase)."""
    artists = []
    names = set()
    for t in data.get("tags") or []:
        name = clean(t.get("name"))
        if t.get("category") != "Artists" or not name:
            continue
        artists.append({"name": name, "query": f"id:{t['id']}"})
        names.add(name.lower())
        for alias in (t.get("alias") or "").split(","):
            if clean(alias):
                names.add(clean(alias).lower())
    uploader = None
    u = data.get("uploader") or {}
    # A removed account comes back as "deleted", in the group "deleted".
    if u.get("username") and u.get("group") != "deleted":
        name = u["username"]
        uploader = {"name": name, "query": "@" + name if USERNAME.match(name) else None}
    source = data.get("source") or ""
    meta = {
        "origin": {"provider": "wallhaven", "id": wid},
        "source": source if re.match(r"^https?://", source) else None,
        "uploader": uploader,
        "artists": artists,
    }
    return meta, names


def change(entry, meta, artist_names, overwrite):
    """`entry` with the metadata and without the artist tags; what was dropped."""
    entry = dict(entry)
    entry.setdefault("favorite", False)
    tags = entry.get("tags") or []
    dropped = [t for t in tags if clean(t).lower() in artist_names]
    entry["tags"] = [t for t in tags if clean(t).lower() not in artist_names]
    for field, value in meta.items():
        have = entry.get(field)
        if overwrite or have is None or have == []:
            # As wallpicker writes it: what a wallpaper doesn't have, left out.
            if value is None or value == []:
                entry.pop(field, None)
            else:
                entry[field] = value
    return entry, dropped


def distinct_tags(items):
    return {t.lower() for e in items.values() for t in e.get("tags") or []}


def merged(a, b):
    """Two entries for one file (an old name's and the new one's): favorite
    if either is, every tag once, the metadata of the one that has it."""
    out = dict(b)
    for k, v in a.items():
        if out.get(k) in (None, [], "") and v not in (None, []):
            out[k] = v
    out["favorite"] = bool(a.get("favorite")) or bool(b.get("favorite"))
    tags = list(b.get("tags") or [])
    for t in a.get("tags") or []:
        if clean(t).lower() not in {clean(x).lower() for x in tags}:
            tags.append(t)
    out["tags"] = tags
    return out


def write_library(plans, done, overwrite):
    """library.json as it is now (a tag edited meanwhile is kept), with the
    renamed files' entries under their new names and `plans` (path ->
    (meta, artist names)) applied, atomically."""
    library = load_json(LIBRARY, {"items": {}})
    items = library.setdefault("items", {})
    for old, new in done.items():
        if old in items:
            entry = items.pop(old)
            items[new] = merged(entry, items[new]) if new in items else entry
    for path, (meta, names) in plans.items():
        items[path], _ = change(items.get(path, {}), meta, names, overwrite)
    write_json(LIBRARY, library)


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--write", action="store_true", help="save the changes (else a dry run)")
    p.add_argument("--refresh", action="store_true", help="ask Wallhaven again, not the cache")
    p.add_argument("--overwrite", action="store_true", help="replace metadata already there")
    p.add_argument("--no-rename", action="store_true", help="leave wallhaven_<id> files as they are")
    p.add_argument("--force", action="store_true", help="write a version-1 library.json anyway")
    args = p.parse_args()

    settings = load_json(CONFIG, {})
    library = load_json(LIBRARY, {"items": {}})
    items = library.get("items") or {}
    if args.write and library.get("version", 1) < 2 and not args.force:
        print(
            "library.json is from before metadata (version 1): wallpicker would drop the\n"
            "metadata on its next save. Install the version with metadata (TODO.md, step 1)\n"
            "and open the picker once, then run this again (or --force).",
            file=sys.stderr,
        )
        return 1

    files, by_hand = find_files(settings)
    renames, taken = ({}, []) if args.no_rename else plan_renames(by_hand)
    print(f"{sum(map(len, files.values()))} Wallhaven files ({len(files)} wallpapers, {len(by_hand)} named wallhaven_<id>)")
    api = Wallhaven(settings.get("wallhaven_api_key"), args.refresh)

    changed = {}
    plans = {}
    gone, failed = [], []
    for i, (wid, paths) in enumerate(sorted(files.items()), 1):
        try:
            data = api.info(wid)
        except (urllib.error.URLError, RuntimeError, ValueError) as e:
            failed.append(wid)
            print(f"[{i}/{len(files)}] {wid}: could not ask Wallhaven: {e}", file=sys.stderr)
            continue
        if data is None:
            gone.append(wid)
            continue
        meta, names = metadata(wid, data)
        for path in paths:
            # Planned under its new name, if it gets one.
            new = renames.get(path, path)
            before = items.get(path, {})
            after, dropped = change(before, meta, names, args.overwrite)
            if after == before and new == path:
                continue
            changed[new] = after
            plans[new] = (meta, names)
            who = ", ".join(a["name"] for a in meta["artists"]) or "-"
            up = meta["uploader"]["name"] if meta["uploader"] else "-"
            name = os.path.basename(path)
            if new != path:
                name += " → " + os.path.basename(new)
            line = f"[{i}/{len(files)}] {name}  artists: {who}  uploader: {up}"
            if meta["source"]:
                line += "  source: yes"
            if dropped:
                line += "  drops tags: " + ", ".join(dropped)
            print(line)

    after = {k: v for k, v in items.items() if k not in renames}
    after.update(changed)
    print()
    print(f"{len(changed)} entries change; asked Wallhaven {api.asked} times (the rest cached)")
    print(f"distinct tags: {len(distinct_tags(items))} → {len(distinct_tags(after))}")
    if gone:
        print(f"no longer on Wallhaven ({len(gone)}): " + ", ".join(gone))
    if failed:
        print(f"could not ask ({len(failed)}, run again): " + ", ".join(failed))
    if renames:
        print(f"{len(renames)} file{'s' if len(renames) != 1 else ''} renamed wallhaven_<id> → wallhaven-<id>")
    if taken:
        print(f"left as they are, the wallhaven-<id> name is taken ({len(taken)}; Find duplicates?): "
              + ", ".join(os.path.basename(t) for t in taken))

    if not args.write:
        print("\ndry run: nothing written or renamed (--write does it)")
    elif changed:
        # A copy of each file this can change, beside it.
        stamp = time.strftime(".before-metadata-%Y%m%d-%H%M%S")
        print()
        for path in (LIBRARY, STATE, CONFIG, HYPRPAPER) if renames else (LIBRARY,):
            if os.path.exists(path):
                shutil.copy2(path, path + stamp)
                print(f"copied {path} to {os.path.basename(path) + stamp}")
        done = rename_files(renames)
        # A rename that didn't happen: its metadata under the name it kept.
        for old, new in renames.items():
            if old not in done and new in plans:
                plans[old] = plans.pop(new)
        for path in rename_references(done) if done else []:
            print(f"renamed paths in {path}")
        write_library(plans, done, args.overwrite)
        print(f"written ({len(done)} renamed)" if renames else "written")
        if done:
            print("reopen the picker: one kept ready still lists the old names")
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\ninterrupted: nothing written (what was asked is cached)", file=sys.stderr)
        sys.exit(1)
