#!/usr/bin/env bash
# Sends a logged one-worker BDS run (crates/sapling-worldgen/scripts/bds_dbg_run.sh) to testbox for the
# replay: its world as ~/replay-worlds/<name>/<world> with a -run2 link, and its orders beside it
# (structure-order.txt; order.txt when the log has decorations). Then, e.g.:
#   REGION="frozen ocean" ENVS="SAPLING_ORACLE_WORLDS=\$HOME/replay-worlds/<name> \
#     SAPLING_ORACLE_STRUCTURE_ORDER=\$HOME/replay-worlds/<name>/structure-order.txt" tools/oracle.sh <log name> blocks_match_bds_biomes
# usage: tools/replay-world.sh <run dir> <name> <world>   e.g. tools/replay-world.sh $S/fo fo 20261003-biomes
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
run=$1 name=$2 world=$3
log=$run/dbg/dbg.log
[ -d "$run/world" ] || { echo "$run/world missing: the run did not stop cleanly" >&2; exit 1; }
orders=(--structure-order "$run/structure-order.txt")
if grep -q '^hit decorate ' "$log"; then orders+=(--order "$run/order.txt"); fi
py -3 -I "$here/../crates/sapling-worldgen/scripts/dbg_loads.py" "$log" "${orders[@]}"
remote=replay-worlds/$name
tar czf - -C "$run" world $(cd "$run" && ls structure-order.txt order.txt 2>/dev/null) |
  ssh -o ConnectTimeout=20 testbox "rm -rf $remote && mkdir -p $remote && tar xzf - -C $remote && mv $remote/world $remote/$world && ln -s $world $remote/$world-run2 && ls $remote"
