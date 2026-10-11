#!/usr/bin/env bash
# Replays a one-worker BDS run on testbox (docs/research/worldgen-bds-order.md, "Replaying a run"). The run is
# ~/oracle-canon/<canon>/ there: its world under the oracle test's world name, a -run2 link, order.txt.
# usage: tools/replay.sh <name> <canon> test                     the region's oracle totals against that run
#        tools/replay.sh <name> <canon> rc                       examples/replay_check over the region
#        tools/replay.sh <name> <canon> ex <example> [args...]   any example under the replay env
# env: as tools/oracle.sh and tools/remote-example.sh. Canons are listed in tools/replay-canons.txt.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
name=$1 canon=$2 mode=$3
shift 3
line=$(grep -E "^$canon\|" "$here/replay-canons.txt") || { echo "unknown canon $canon" >&2; exit 2; }
IFS='|' read -r _ world test region box <<<"$line"
export ENVS="${ENVS:-} SAPLING_ORACLE_WORLDS=\$HOME/oracle-canon/$canon SAPLING_ORACLE_ORDER=\$HOME/oracle-canon/$canon/order.txt SAPLING_ORACLE_THREAD_SEED=0"
case $mode in
  test) REGION=$region bash "$here/oracle.sh" "$name" "$test" ;;
  rc) bash "$here/remote-example.sh" "$name" replay_check 20261003 "$world" $box ;;
  ex) bash "$here/remote-example.sh" "$name" "$@" ;;
  *) echo "mode: test, rc or ex" >&2; exit 2 ;;
esac
