#!/usr/bin/env bash
# usage: run.sh <seed> <out-dir> <x0,z0,x1,z1>...   (overworld chunk coordinates, inclusive); see README.md
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
bds=${WORLDGEN_BDS:-C:/Users/hexay/acacia/acacia/.testserver/bds-worldgen}
tile=${WORLDGEN_TILE:-8}
workers=${WORLDGEN_WORKERS:-6}
settle=${WORLDGEN_SETTLE:-20}
limit=${WORLDGEN_TIMEOUT:-3600}

[ $# -ge 3 ] || { echo "usage: run.sh <seed> <out-dir> <x0,z0,x1,z1>..." >&2; exit 2; }
seed=$1 out=$(realpath -m "$2")
shift 2
regions=""
for region in "$@"; do
  [[ $region =~ ^-?[0-9]+,-?[0-9]+,-?[0-9]+,-?[0-9]+$ ]] || { echo "bad region '$region': want x0,z0,x1,z1" >&2; exit 2; }
  IFS=, read -r x0 z0 x1 z1 <<<"$region"
  [ "$x0" -le "$x1" ] && [ "$z0" -le "$z1" ] || { echo "bad region '$region': x0 > x1 or z0 > z1" >&2; exit 2; }
  regions+="[$region],"
done
# A ticking area holds at most 100 chunks, and each tile's area adds a one-chunk ring.
[ $(((tile + 2) * (tile + 2))) -le 100 ] || { echo "WORLDGEN_TILE must be at most 8" >&2; exit 2; }
if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then echo "$out is not empty" >&2; exit 2; fi

level="wg-$seed-$(date +%s)"
pack="$bds/development_behavior_packs/worldgen_oracle"
rm -rf "$pack"
cp -r "$here/pack" "$pack"
# WORLDGEN_LOAD_TICKS: how long a tile's area may take before it is removed and added again (a retry
# generates its chunks a second time: README, "A repeatable BDS").
# WORLDGEN_KICK=true: a stalled area gets a second area over its missing chunks instead (pack/scripts/main.js).
# WORLDGEN_DIMENSION=nether: the regions are Nether chunks (saved under dimension 1).
printf 'export const REGIONS = [%s];\nexport const TILE = %d;\nexport const WORKERS = %d;\nexport const SETTLE = %d;\nexport const LOAD_TIMEOUT = %d;\nexport const KICK = %s;\nexport const DIMENSION = "%s";\n' \
  "$regions" "$tile" "$workers" "$settle" "${WORLDGEN_LOAD_TICKS:-600}" "${WORLDGEN_KICK:-false}" "${WORLDGEN_DIMENSION:-overworld}" >"$pack/scripts/regions.js"
mkdir -p "$bds/worlds/$level"
echo '[{"pack_id": "7a3d2f60-5c1e-4b8a-9d47-0e6f1b2c3a01", "version": [1, 0, 0]}]' >"$bds/worlds/$level/world_behavior_packs.json"

props="$bds/server.properties"
settings="level-name=$level
level-seed=$seed
level-type=DEFAULT
server-port=${WORLDGEN_PORT:-19190}
server-portv6=$((${WORLDGEN_PORT:-19190} + 1))
online-mode=false
content-log-console-output-enabled=true
max-threads=${WORLDGEN_THREADS:-8}"
keys=$(sed 's/=.*//' <<<"$settings" | paste -sd'|')
{ grep -vE "^($keys)=" "$props" || true; echo "$settings"; } >"$props.new"
mv "$props.new" "$props"

log="$bds/worldgen.log"
: >"$log"
start=$SECONDS
deadline=$((SECONDS + limit))
cd "$bds"
# BDS hangs on `stop` after `save hold`, so stop it plainly once the pack is done and keep stdin open
# until it quits; the deadline also ends this feeder when BDS dies early, or the pipeline would wait forever.
{
  until grep -qE 'WORLDGEN done|Quit correctly' "$log" || [ $SECONDS -ge $deadline ]; do sleep 1; done
  echo stop
  until grep -q 'Quit correctly' "$log" || [ $SECONDS -ge $deadline ]; do sleep 1; done
} | timeout "$limit" ./bedrock_server.exe >"$log" 2>&1 || true
elapsed=$((SECONDS - start))

grep -E 'WORLDGEN (done|.*(error|retry))|ERROR' "$log" | grep -vE 'TRANSPORT|NetherNet|nethernet|how_to|=====|ERROR\] *$' | cut -c1-300 | head -n 40 || true
grep -q 'Quit correctly' "$log" || { echo "BDS did not stop cleanly; the world stays in $bds/worlds/$level, the log in $log" >&2; exit 1; }
grep -q 'WORLDGEN done .* 0 tiles failed' "$log" || { echo "the pack did not touch every tile; the world stays in $bds/worlds/$level, the log in $log" >&2; exit 1; }

version=$(grep -m1 -oE 'Version: [0-9.]+' "$log" | cut -d' ' -f2)
mkdir -p "$out"
cp -r "worlds/$level/." "$out/"
rm -rf "worlds/$level"
printf 'bds %s\nseed %s\ndimension %s\nregions %s\ntile %s\ngenerated %s\nseconds %s\n' \
  "$version" "$seed" "${WORLDGEN_DIMENSION:-overworld}" "$*" "$tile" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$elapsed" >"$out/oracle.txt"
echo "BDS $version, $elapsed s: $(grep -m1 -oE 'WORLDGEN done.*' "$log") -> $out"
