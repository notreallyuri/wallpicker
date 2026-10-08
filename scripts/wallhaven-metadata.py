#!/usr/bin/env python3
"""Give the library's Wallhaven wallpapers their metadata (TODO.md, Metadata):
origin, source, uploader and artists, from Wallhaven's API, and take the
artist names out of their tags.

Finds wallhaven-<id>.{jpg,png,webp} under the wallpaper folders and the
download folder, asks /api/v1/w/<id> for each (cached, at most 40 a minute),
and shows what would change. Nothing is written without --write.

    scripts/wallhaven-metadata.py              # dry run: what would change
    scripts/wallhaven-metadata.py --write      # do it (library.json copied first)
    --refresh    ask Wallhaven again instead of using the cached answers
    --overwrite  replace metadata a wallpaper already has (else only fill in)
    --force      write even a library.json from before metadata (version 1)

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
FILE = re.compile(r"^wallhaven-([A-Za-z0-9]{1,32})\.(jpg|png|webp)$")
# As core's uploader_query: what can follow "@" without reading as other syntax.
USERNAME = re.compile(r"^[A-Za-z0-9_.-]{1,64}$")


def xdg(var, fallback):
    return os.environ.get(var) or os.path.expanduser(fallback)


CONFIG = os.path.join(xdg("XDG_CONFIG_HOME", "~/.config"), "wallpicker", "config.json")
LIBRARY = os.path.join(xdg("XDG_DATA_HOME", "~/.local/share"), "wallpicker", "library.json")
CACHE = os.path.join(xdg("XDG_CACHE_HOME", "~/.cache"), "wallpicker", "metadata", "wallhaven")


def load_json(path, missing):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return missing


def find_files(settings):
    """id -> [paths], as the library's scan names them (the folder as set, joined)."""
    roots = list(settings.get("folders") or [])
    download = settings.get("download_dir")
    if download and download not in roots:
        roots.append(download)
    recursive = settings.get("recursive", True)
    found = {}
    seen = set()
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
                    found.setdefault(m.group(1), []).append(path)
    return found


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
            entry[field] = value
    return entry, dropped


def distinct_tags(items):
    return {t.lower() for e in items.values() for t in e.get("tags") or []}


def write_library(plans, overwrite):
    """Apply `plans` (path -> (meta, artist names)) to library.json as it is
    now (a tag edited meanwhile is kept), atomically."""
    library = load_json(LIBRARY, {"items": {}})
    items = library.setdefault("items", {})
    for path, (meta, names) in plans.items():
        items[path], _ = change(items.get(path, {}), meta, names, overwrite)
    folder = os.path.dirname(LIBRARY)
    os.makedirs(folder, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=folder, prefix=".library.", suffix=".json")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(library, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, LIBRARY)


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--write", action="store_true", help="save the changes (else a dry run)")
    p.add_argument("--refresh", action="store_true", help="ask Wallhaven again, not the cache")
    p.add_argument("--overwrite", action="store_true", help="replace metadata already there")
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

    files = find_files(settings)
    print(f"{sum(map(len, files.values()))} Wallhaven files ({len(files)} wallpapers)")
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
            before = items.get(path, {})
            after, dropped = change(before, meta, names, args.overwrite)
            if after == before:
                continue
            changed[path] = after
            plans[path] = (meta, names)
            who = ", ".join(a["name"] for a in meta["artists"]) or "-"
            up = meta["uploader"]["name"] if meta["uploader"] else "-"
            line = f"[{i}/{len(files)}] {os.path.basename(path)}  artists: {who}  uploader: {up}"
            if meta["source"]:
                line += "  source: yes"
            if dropped:
                line += "  drops tags: " + ", ".join(dropped)
            print(line)

    after = dict(items)
    after.update(changed)
    print()
    print(f"{len(changed)} entries change; asked Wallhaven {api.asked} times (the rest cached)")
    print(f"distinct tags: {len(distinct_tags(items))} → {len(distinct_tags(after))}")
    if gone:
        print(f"no longer on Wallhaven ({len(gone)}): " + ", ".join(gone))
    if failed:
        print(f"could not ask ({len(failed)}, run again): " + ", ".join(failed))

    if not args.write:
        print("\ndry run: nothing written (--write saves it)")
    elif changed:
        if os.path.exists(LIBRARY):
            copy = LIBRARY + time.strftime(".before-metadata-%Y%m%d-%H%M%S")
            shutil.copy2(LIBRARY, copy)
            print(f"\nthe old library.json is at {copy}")
        write_library(plans, args.overwrite)
        print("written")
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\ninterrupted: nothing written (what was asked is cached)", file=sys.stderr)
        sys.exit(1)
