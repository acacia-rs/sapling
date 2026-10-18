#!/usr/bin/env bash
# Runs a built probe example against a BDS on a fresh world, then stops the server.
# usage: tools/bds-drill.sh <bds dir> <port> <example name> <output file> [probe args...]
# The BDS dir's server.properties names the port and the world (level-name), which is wiped first.
# Works with the Windows server (bedrock_server.exe) and the Linux one (bedrock_server).
set -uo pipefail
bds=$(realpath "$1") port=$2 out=$(realpath -m "$4")
exe=$([ -f "$bds/bedrock_server.exe" ] && echo .exe)
probe=$(realpath "$(dirname "$0")/../target/debug/examples/$3$exe")
world=$(grep -E '^level-name=' "$bds/server.properties" | cut -d= -f2)
rm -rf "$bds/worlds/$world"
log="$bds/drill.log"
: >"$log"
cd "$bds" || exit 1
{
  n=0
  # A loaded testbox took 64 s to start one (2026-10-18).
  until grep -q 'Server started' "$log" || [ "$n" -ge 360 ]; do sleep 0.5; n=$((n + 1)); done
  "$probe" "127.0.0.1:$port" "${@:5}" >"$out" 2>&1
  echo "probe exit $?" >>"$out"
  echo stop
  sleep 5
} | LD_LIBRARY_PATH=. timeout 600 "./bedrock_server$exe" >"$log" 2>&1
echo "server exit $?, probe: $(tail -1 "$out"), $(wc -l <"$out") lines in $out"
grep -E 'ERROR|WARN' "$log" | head -20
