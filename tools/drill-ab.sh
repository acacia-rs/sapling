#!/usr/bin/env bash
# A/B of sapling drills: BASE (a git ref, default origin/master) against this working tree, both sides at once
# on testbox (tools/remote-drills.sh), then the watcher's motion packets compared (tools/drill_motion.py).
# usage: tools/drill-ab.sh <example name> <local dir> <word>...   e.g. tools/drill-ab.sh bds_command_probe ab water shove
# Captures land in <dir>/base/<word>.txt and <dir>/new/<word>.txt; exits 1 if any motion differs.
# water, lava, shove and kill repeat exactly run to run; tnt, hit and chase don't, so one A/B of them proves nothing.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
example=$1 out=$2
shift 2
mkdir -p "$out"
new_dir=${REMOTE_DIR:-work/$(basename "$(dirname "$here")")}
REF=${BASE:-origin/master} REMOTE_DIR=$new_dir-base SAPLING_PORT=19350 bash "$here/remote-drills.sh" "$example" "$out/base" "$@" >"$out/base.log" 2>&1 &
base=$!
REMOTE_DIR=$new_dir SAPLING_PORT=19300 bash "$here/remote-drills.sh" "$example" "$out/new" "$@" >"$out/new.log" 2>&1
new_code=$?
wait "$base"
base_code=$?
[ "$base_code" = 0 ] && [ "$new_code" = 0 ] || { tail -n 8 "$out/base.log" "$out/new.log"; echo "drills failed: base $base_code, new $new_code"; exit 2; }
grep -H '^LAGGED' "$out"/base/*.txt "$out"/new/*.txt 2>/dev/null
py -3 -I "$here/drill_motion.py" "$out" base,new "$@"
