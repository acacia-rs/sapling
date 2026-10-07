#!/usr/bin/env bash
# Runs a built probe example against the drill_server example, then stops it: the same drill as
# tools/bds-drill.sh, on sapling. Build both examples first.
# usage: tools/sapling-drill.sh <port> <example name> <output file>
set -uo pipefail
port=$1 out=$(realpath -m "$3")
examples="$(dirname "$0")/../target/debug/examples"
log="$out.server.log"
"$examples/drill_server.exe" "$port" >"$log" 2>&1 &
server=$!
n=0
until grep -q 'started' "$log" || [ "$n" -ge 60 ]; do sleep 0.5; n=$((n + 1)); done
"$examples/$2.exe" "127.0.0.1:$port" >"$out" 2>&1
echo "probe exit $?" >>"$out"
kill "$server" 2>/dev/null
wait "$server" 2>/dev/null
echo "probe: $(tail -1 "$out"), $(wc -l <"$out") lines in $out; server log $log"
grep -E 'ERROR|WARN|panicked' "$log" | head -20 || true
