#!/usr/bin/env bash
# Makes a BDS copy for bds_loot_probe: a behavior pack holding loot/row.mcstructure (tools/loot_structures.py),
# applied to its world. Run on testbox: bash tools/bds-loot-setup.sh [from dir] [to dir] [port]
# (default ~/work/bds-mechanisms, ~/work/bds-loot, 19292). Drill it with KEEP_WORLD=1, as a fresh
# world would lose its pack list.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
from=${1:-$HOME/work/bds-mechanisms} to=${2:-$HOME/work/bds-loot} port=${3:-19292}
mkdir -p "$to"
rsync -a --exclude=/worlds --exclude=drill.log "$from/" "$to/"
sed -i -e "s/^server-port=.*/server-port=$port/" -e "s/^server-portv6=.*/server-portv6=$((port + 1))/" \
  -e 's/^level-name=.*/level-name=loot/' "$to/server.properties"
bp=$to/behavior_packs/loot_bp
rm -rf "$bp"
mkdir -p "$bp"
printf '{"format_version":2,"header":{"name":"loot_bp","description":"sapling loot probe","uuid":"7c1e0a52-5d2f-4b47-9a43-34c0e2b1f101","version":[1,0,0],"min_engine_version":[1,21,0]},"modules":[{"type":"data","uuid":"7c1e0a52-5d2f-4b47-9a43-34c0e2b1f102","version":[1,0,0]}]}\n' >"$bp/manifest.json"
python3 "$here/loot_structures.py" "$bp"
world=$to/worlds/loot
rm -rf "$world"
mkdir -p "$world"
echo '[{"pack_id":"7c1e0a52-5d2f-4b47-9a43-34c0e2b1f101","version":[1,0,0]}]' >"$world/world_behavior_packs.json"
echo "bds-loot ready in $to on port $port"
