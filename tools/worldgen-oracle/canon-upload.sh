#!/usr/bin/env bash
# Turns a one-worker debugger run (scripts/bds_dbg_run.sh with scripts/data/dbg_canon.json's breakpoints) into a
# canon on testbox for tools/replay.sh: ~/oracle-canon/<canon>/{<world>, <world>-run2 -> <world>, order.txt}.
# usage: canon-upload.sh <run dir> <canon> <world name>      e.g. canon-upload.sh out/jt q-jt 20261003-structures
# Prints dbg_loads.py's report first: a run with a chunk decorated twice (a retried tile) is no oracle, discard it.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
run=$1 canon=$2 world=$3
[ -d "$run/world" ] || { echo "no saved world in $run" >&2; exit 1; }
report=$(py -3 -I "$here/../../crates/sapling-worldgen/scripts/dbg_loads.py" "$run/dbg/dbg.log" --order "$run/order.txt")
echo "$report" | head -n 12
tar czf - -C "$run" world order.txt | ssh -o ConnectTimeout=20 testbox "rm -rf oracle-canon/$canon && mkdir -p oracle-canon/$canon && cd oracle-canon/$canon && tar xzf - && mv world $world && ln -s $world $world-run2 && wc -l order.txt"
