#!/usr/bin/env bash
# Runs a built probe example against the drill_server example, then stops it: the same drill as
# tools/bds-drill.sh, on sapling. Build both examples first.
# usage: tools/sapling-drill.sh <port> <example name> <output file> [probe args...]
# Probes whose waits follow ACACIA_TICK_SCALE (examples/shared/clock.rs) drill a server ticking TICK_SCALE
# (default 2) times faster; others run at 20/s. A capture whose server fell behind is marked LAGGED: it
# skipped ticks, which only matters when a settle ends before the world settles.
# 2026-10-24: water, lava, kill, shove, concrete and pistons gave the same motion at 1x, 2x and 4x (4x with
# 3 drills at once; with 6 a debug server lagged enough to change kill).
set -uo pipefail
port=$1 out=$(realpath -m "$3")
case $2 in
  bds_command_probe) export ACACIA_TICK_SCALE=${TICK_SCALE:-2} ;;
  *) export ACACIA_TICK_SCALE=1 ;;
esac
examples="$(dirname "$0")/../target/debug/examples"
log="$out.server.log"
exe=$([ -f "$examples/drill_server.exe" ] && echo .exe)
"$examples/drill_server$exe" "$port" >"$log" 2>&1 &
server=$!
n=0
until grep -q 'started' "$log" || [ "$n" -ge 60 ]; do sleep 0.5; n=$((n + 1)); done
"$examples/$2$exe" "127.0.0.1:$port" "${@:4}" >"$out" 2>&1
echo "probe exit $?" >>"$out"
kill "$server" 2>/dev/null
wait "$server" 2>/dev/null
start=$(grep -o '^## drill starts at [0-9]*' "$out" | grep -o '[0-9]*$')
behind=$(grep 'fell behind its tick' "$log" | grep -o '20[0-9-]*T[0-9:.]*Z' | while read -r at; do [ "$(date -d "$at" +%s%3N)" -ge "${start:-0}" ] && echo; done | wc -l)
[ "$behind" -gt 0 ] && echo "LAGGED: the server fell behind its tick during the drill at ${ACACIA_TICK_SCALE}x; if this capture differs, rerun with TICK_SCALE=1" >>"$out"
echo "probe: $(tail -1 "$out"), $(wc -l <"$out") lines in $out; server log $log"
grep -E 'ERROR|WARN|panicked' "$log" | head -20 || true
