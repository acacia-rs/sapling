#!/usr/bin/env bash
# Replays a one-worker BDS run on testbox (docs/research/worldgen-bds-order.md, "Replaying a run"). The run is
# ~/oracle-canon/<dir>/ there: its world under the oracle test's world name, a -run2 link, order.txt and, for
# runs uploaded since replay5, structure-order.txt (then the structure stage is replayed too; NO_STRUCTURE=1 skips it).
# usage: tools/replay.sh <name> <canon> test                     the region's oracle totals against that run
#        tools/replay.sh <name> <canon> rc                       examples/replay_check over the region
#        tools/replay.sh <name> <canon> ex <example> [args...]   any example under the replay env
# env: as tools/oracle.sh and tools/remote-example.sh.
# tools/replay-canons.txt: canon|world|test fn|region|box|dir (default: the canon)|seed (default 20261003).
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
name=$1 canon=$2 mode=$3
shift 3
line=$(grep -E "^$canon\|" "$here/replay-canons.txt") || { echo "unknown canon $canon" >&2; exit 2; }
IFS='|' read -r _ world test region box dir seed <<<"$line"
at="\$HOME/oracle-canon/${dir:-$canon}"
structure="\$(test -f $at/structure-order.txt && echo SAPLING_ORACLE_STRUCTURE_ORDER=$at/structure-order.txt)"
[ -n "${NO_STRUCTURE:-}" ] && structure=""
# <dir>/env.txt: further variables of that run (SAPLING_ORACLE_DEAD_CORAL=<CoralColor>: BDS draws it from the clock).
export ENVS="${ENVS:-} SAPLING_ORACLE_WORLDS=$at SAPLING_ORACLE_ORDER=$at/order.txt SAPLING_ORACLE_THREAD_SEED=0 $structure \$(cat $at/env.txt 2>/dev/null)"
case $mode in
  test) REGION="=$region" bash "$here/oracle.sh" "$name" "$test" ;;
  rc) bash "$here/remote-example.sh" "$name" replay_check "${seed:-20261003}" "$world" $box ;;
  ex) bash "$here/remote-example.sh" "$name" "$@" ;;
  *) echo "mode: test, rc or ex" >&2; exit 2 ;;
esac
