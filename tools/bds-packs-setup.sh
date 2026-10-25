#!/usr/bin/env bash
# Makes a BDS copy for the pack drills (bds_pack_probe, bds_command_probe functions): a required
# resource pack padded to span several download chunks and a behavior pack of probe functions, both
# applied to its world. Run on testbox: bash tools/bds-packs-setup.sh [from dir] [to dir] [port]
# (default ~/work/bds-mechanisms, ~/work/bds-packs, 19290). Then drill it with KEEP_WORLD=1, as a
# fresh world would lose its pack lists.
set -euo pipefail
from=${1:-$HOME/work/bds-mechanisms} to=${2:-$HOME/work/bds-packs} port=${3:-19290}
mkdir -p "$to"
rsync -a --exclude=/worlds --exclude=drill.log "$from/" "$to/"
sed -i -e "s/^server-port=.*/server-port=$port/" -e "s/^server-portv6=.*/server-portv6=$((port + 1))/" \
  -e 's/^level-name=.*/level-name=packs/' -e 's/^texturepack-required=.*/texturepack-required=true/' "$to/server.properties"

rp=$to/resource_packs/probe_rp bp=$to/behavior_packs/probe_bp
rm -rf "$rp" "$bp"
mkdir -p "$rp" "$bp/functions/probe" "$bp/functions/deep/a"
manifest() { # name uuid module-type module-uuid
  printf '{"format_version":2,"header":{"name":"%s","description":"sapling probe","uuid":"%s","version":[1,0,0],"min_engine_version":[1,21,0]},"modules":[{"type":"%s","uuid":"%s","version":[1,0,0]}]}\n' "$1" "$2" "$3" "$4"
}
manifest probe_rp 5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001 resources 5d9a0a52-5d2f-4b47-9a43-34c0e2b1f002 >"$rp/manifest.json"
manifest probe_bp 5d9a0a52-5d2f-4b47-9a43-34c0e2b1f003 data 5d9a0a52-5d2f-4b47-9a43-34c0e2b1f004 >"$bp/manifest.json"
# Fixed bytes: a new pack would change the hash a capture shows.
head -c 3000000 /dev/zero | tr '\0' 'p' >"$rp/padding.bin"

f=$bp/functions
printf '# a comment\n\nsay hello\nscoreboard players add @s fn 1\n' >"$f/probe/hello.mcfunction"
printf 'function probe/hello\nsay after\n' >"$f/probe/nested.mcfunction"
printf 'testfor @e[type=pig]\nsay still\n' >"$f/probe/fails.mcfunction"
printf 'notacommand\nsay bad\n' >"$f/probe/bad.mcfunction"
printf '/say slash\n' >"$f/probe/slash.mcfunction"
printf 'scoreboard players add @s loops 1\nfunction probe/self\n' >"$f/probe/self.mcfunction"
printf 'say later\n' >"$f/probe/later.mcfunction"
printf 'scoreboard players add ticks tk 1\n' >"$f/probe/tick.mcfunction"
printf 'say upper\n' >"$f/probe/Upper.mcfunction"
: >"$f/probe/empty.mcfunction"
printf '   say indented   \n' >"$f/probe/spaced.mcfunction"
printf 'say deep\n' >"$f/deep/a/b.mcfunction"
printf '{"values":["probe/tick"]}\n' >"$f/tick.json"

world=$to/worlds/packs
rm -rf "$world"
mkdir -p "$world"
echo '[{"pack_id":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001","version":[1,0,0]}]' >"$world/world_resource_packs.json"
echo '[{"pack_id":"5d9a0a52-5d2f-4b47-9a43-34c0e2b1f003","version":[1,0,0]}]' >"$world/world_behavior_packs.json"
echo "bds-packs ready in $to on port $port"
