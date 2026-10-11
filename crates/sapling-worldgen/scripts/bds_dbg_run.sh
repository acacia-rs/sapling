#!/usr/bin/env bash
# usage: WORLDGEN_BDS=<your BDS copy> bds_dbg_run.sh <seed> <out dir> <dbg config.json> <x0,z0,x1,z1>...
# tools/worldgen-oracle/run.sh with BDS started under bds_dbg.py: generates the chunk regions and leaves
# <out dir>/dbg/ (dbg.log, trace-*.txt), <out dir>/worldgen.log and, after a clean stop, <out dir>/world/.
# One run at a time per BDS copy (a lock dir guards it); ports $WORLDGEN_PORT and the next (default 19290).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
bds=${WORLDGEN_BDS:?set WORLDGEN_BDS to your own BDS copy}
pack_src=$here/../../../tools/worldgen-oracle/pack
seed=$1 out=$(realpath -m "$2") cfg=$(realpath "$3")
shift 3
limit=${WORLDGEN_TIMEOUT:-1800}
port=${WORLDGEN_PORT:-19290}
mkdir "$bds/.dbg-lock" 2>/dev/null || { echo "another run holds $bds/.dbg-lock" >&2; exit 2; }
trap 'rmdir "$bds/.dbg-lock"' EXIT
regions=""
for region in "$@"; do regions+="[$region],"; done
rm -rf "$out"; mkdir -p "$out/dbg"

level="wg-$seed-$(date +%s)"
pack="$bds/development_behavior_packs/worldgen_oracle"
rm -rf "$pack"; cp -r "$pack_src" "$pack"
# EXTRA_PACK: a folder of pack files laid over the oracle pack (scripts/feature_override_pack.py).
if [ -n "${EXTRA_PACK:-}" ]; then cp -r "$EXTRA_PACK"/. "$pack"/; fi
# The slowed server must not miss the load deadline: a retried tile generates its chunks twice.
# WORLDGEN_WORKERS: tiles loaded at once. With 6, the near origin tile 8,8,15,15 stalls and is retried; with 1 it loads.
printf 'export const REGIONS = [%s];\nexport const TILE = 8;\nexport const WORKERS = %d;\nexport const SETTLE = 20;\nexport const LOAD_TIMEOUT = %d;\n' "$regions" "${WORLDGEN_WORKERS:-6}" "${WORLDGEN_LOAD_TICKS:-12000}" >"$pack/scripts/regions.js"
rm -rf "$bds/worlds"; mkdir -p "$bds/worlds/$level"
echo '[{"pack_id": "7a3d2f60-5c1e-4b8a-9d47-0e6f1b2c3a01", "version": [1, 0, 0]}]' >"$bds/worlds/$level/world_behavior_packs.json"
props="$bds/server.properties"
settings="level-name=$level
level-seed=$seed
level-type=DEFAULT
server-port=$port
server-portv6=$((port + 1))
online-mode=false
content-log-console-output-enabled=true
script-watchdog-enable=false
max-threads=${WORLDGEN_THREADS:-8}"
keys=$(sed 's/=.*//' <<<"$settings" | paste -sd'|')
{ grep -vE "^($keys)=" "$props" || true; echo "$settings"; } >"$props.new"
mv "$props.new" "$props"

log="$out/worldgen.log"
: >"$log"
deadline=$((SECONDS + limit))
{
  until grep -qE 'WORLDGEN done|Quit correctly' "$log" || [ $SECONDS -ge $deadline ]; do sleep 1; done
  echo stop
  until grep -q 'Quit correctly' "$log" || [ $SECONDS -ge $deadline ]; do sleep 1; done
} | (cd "$bds" && timeout "$limit" py -3 -s "$here/bds_dbg.py" "$bds/bedrock_server.exe" "$cfg" "$out/dbg") >"$log" 2>&1 || true
grep -E 'WORLDGEN (done|.*(error|retry))' "$log" | head -5 || true
if grep -q 'Quit correctly' "$log"; then cp -r "$bds/worlds/$level" "$out/world"; fi
echo "$(grep -c '^hit' "$out/dbg/dbg.log") hits logged in $out/dbg/dbg.log"
