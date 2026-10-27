#!/usr/bin/env bash
# Every block type's map colour as BDS draws it: the pack (pack/) lays each block type's default
# state on a grid at `/scriptevent mapcol:grid`, and bds_command_probe's `mapgrid` drill makes a map
# of it on a fresh flat world of testbox's BDS copy. Writes the drill's capture to <out> and the
# pack's grid (`MAPCOL cell x z id ...`) to <out>.cells.
# usage: tools/map-colours/run.sh <local output file> [mapgrid|mapgrid-raised]
# (BDS_DIR default work/bds-maps, BDS_PORT 19320)
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
out=$1 bds=${BDS_DIR:-work/bds-maps} host=${REMOTE_HOST:-testbox}
opts="-o ConnectTimeout=20 -o ServerAliveInterval=10"
world=$(ssh $opts $host "grep -E '^level-name=' $bds/server.properties | cut -d= -f2") || exit 1
ssh $opts $host "rm -rf $bds/development_behavior_packs/map_colours $bds/worlds/$world && mkdir -p $bds/worlds/$world \
  && echo '[{\"pack_id\": \"3c1f5e20-8a4b-4d7e-b9a1-6d2e0f4c5a01\", \"version\": [1, 0, 0]}]' >$bds/worlds/$world/world_behavior_packs.json" || exit 1
scp -r $opts "$here/pack" "$host:$bds/development_behavior_packs/map_colours" >/dev/null || exit 1
KEEP_WORLD=1 BDS_DIR="~/$bds" BDS_PORT=${BDS_PORT:-19320} "$here/../remote-drill.sh" bds bds_command_probe "$out" "${2:-mapgrid}"
ssh $opts $host "grep -oE 'MAPCOL .*' $bds/drill.log" >"$out.cells"
wc -l <"$out.cells"
