#!/bin/bash
# The open picker's memory on this desktop (PSS: shared pages split fairly):
# the interface (Quickshell) and its backend (`wallpicker serve`). Opens it
# RUNS times through `wallpicker` (at PLACE: local, online, collections...),
# waits for it to settle, measures, closes it. The headless session renders
# in software and reads ~50 MB high, so this runs on the real screen.
#
#   cargo build --release -p wallpicker
#   scripts/memory.sh [RUNS] [PLACE]
#
# Your interface choices are copied (the picker's changes don't stay); the
# backend reads your real library and settings.
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
runs=${1:-3}
place=${2:-local}
bin=$here/target/release/wallpicker
settle=${SETTLE:-6}
state=$(mktemp -d)
cp -r "${XDG_STATE_HOME:-$HOME/.local/state}/wallpicker" "$state/"
pss() { awk '/^Pss:/{print int($2/1024)}' "/proc/$1/smaps_rollup" 2>/dev/null || echo 0; }
if pgrep -x qs > /dev/null; then
    echo "memory: close the picker (and any other Quickshell) first" >&2
    exit 1
fi
for i in $(seq "$runs"); do
    XDG_STATE_HOME=$state "$bin" --open "$place"
    sleep "$settle"
    q=$(pgrep -x qs | head -1)
    s=$(pgrep -P "$q" -f "wallpicker serve" | head -1)
    printf "run %s  interface %4s MB  backend %3s MB  total %4s MB\n" "$i" "$(pss "$q")" "$(pss "$s")" $(( $(pss "$q") + $(pss "$s") ))
    XDG_STATE_HOME=$state "$bin" --toggle
    sleep 1
done
rm -r "$state"
