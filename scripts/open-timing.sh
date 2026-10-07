#!/bin/bash
# How long the picker takes to open on this desktop: from `wallpicker` to
# the picker answering over IPC (its window up), and to its library read
# (the grid filling). Opens and closes it OPENS times; the first open after
# a while is the coldest (files not in the page cache).
#
#   cargo build --release -p wallpicker
#   scripts/open-timing.sh [OPENS]
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
opens=${1:-3}
bin=$here/target/release/wallpicker
ui=$here/quickshell
state=$(mktemp -d)
cp -r "${XDG_STATE_HOME:-$HOME/.local/state}/wallpicker" "$state/"
if pgrep -x qs > /dev/null; then
    echo "open-timing: close the picker (and any other Quickshell) first" >&2
    exit 1
fi
ms() { date +%s%3N; }
for i in $(seq "$opens"); do
    t0=$(ms)
    XDG_STATE_HOME=$state "$bin"
    until qs -p "$ui" ipc call wallpicker ready > /dev/null 2>&1; do sleep 0.01; done
    t1=$(ms)
    until [ "$(qs -p "$ui" ipc call wallpicker ready 2>/dev/null)" = "true" ]; do sleep 0.01; done
    t2=$(ms)
    printf "open %s  up after %4s ms  library read after %4s ms\n" "$i" $((t1 - t0)) $((t2 - t0))
    XDG_STATE_HOME=$state "$bin" --toggle
    sleep 1.5
done
rm -r "$state"
