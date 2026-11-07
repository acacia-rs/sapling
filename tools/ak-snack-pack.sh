#!/usr/bin/env bash
# Makes a BDS copy with a behavior pack defining `ak:snack` (a custom food with a cooldown) for the
# `ak1snack` drill: bash tools/ak-snack-pack.sh [from dir] [to dir] [port] (default ~/work/bds-parity1,
# ~/work/bds-ak-snack, 19614). Drill it with KEEP_WORLD=1; the source BDS is left untouched.
set -euo pipefail
from=${1:-$HOME/work/bds-parity1} to=${2:-$HOME/work/bds-ak-snack} port=${3:-19614}
mkdir -p "$to"
rsync -a --exclude=/worlds --exclude=drill.log "$from/" "$to/"
sed -i -e "s/^server-port=.*/server-port=$port/" -e "s/^server-portv6=.*/server-portv6=$((port + 1))/" "$to/server.properties"
world=$(sed -n 's/^level-name=//p' "$to/server.properties")

bp=$to/behavior_packs/ak_snack_bp
rm -rf "$bp"
mkdir -p "$bp/items"
printf '{"format_version":2,"header":{"name":"ak_snack","description":"sapling probe","uuid":"7a3c1e52-0d4b-4f6a-9b1e-2c5a6d7e8f01","version":[1,0,0],"min_engine_version":[1,21,40]},"modules":[{"type":"data","uuid":"7a3c1e52-0d4b-4f6a-9b1e-2c5a6d7e8f02","version":[1,0,0]}]}\n' >"$bp/manifest.json"
cat >"$bp/items/snack.json" <<'JSON'
{
  "format_version": "1.21.40",
  "minecraft:item": {
    "description": { "identifier": "ak:snack", "menu_category": { "category": "items" } },
    "components": {
      "minecraft:max_stack_size": 64,
      "minecraft:food": { "nutrition": 4, "saturation_modifier": 0.6, "can_always_eat": false },
      "minecraft:use_modifiers": { "use_duration": 1.6, "movement_modifier": 0.35 },
      "minecraft:use_animation": "eat",
      "minecraft:cooldown": { "category": "ak_snack", "duration": 3.0 }
    }
  }
}
JSON

rm -rf "$to/worlds/$world"
mkdir -p "$to/worlds/$world"
echo '[{"pack_id":"7a3c1e52-0d4b-4f6a-9b1e-2c5a6d7e8f01","version":[1,0,0]}]' >"$to/worlds/$world/world_behavior_packs.json"
echo "bds-ak-snack ready in $to on port $port (world $world)"
