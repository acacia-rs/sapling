#!/usr/bin/env bash
# A region of a one-worker run sent by tools/replay-world.sh, against that run's world: the oracle
# test's totals with our phase model and with the run's structure order, then the first differing
# chunks and pieces under the replay (examples/structure_replay_check).
# usage: tools/replay-oracle.sh <name> <test fn> <region name> <x0> <z0> <x1> <z1>
#   e.g. tools/replay-oracle.sh fo blocks_match_bds_biomes "frozen ocean" 474 570 477 573
# env: SEED (default 20261003), SKIP_PHASE=1 leaves the phase-model run out, CHECK_ENVS for the check
#      (SHOW, CELLS, NEAR); the rest as tools/testbox.sh. Logs: target/testbox/<name>-{phase,replay,check}.log
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
name=$1 test=$2 region=$3
shift 3
dir="\$HOME/replay-worlds/$name"
worlds="SAPLING_ORACLE_WORLDS=$dir"
order="SAPLING_ORACLE_STRUCTURE_ORDER=$dir/structure-order.txt"
out=${OUT:-$here/../target/testbox}
world=$(ssh -o ConnectTimeout=20 testbox "ls replay-worlds/$name | grep -v -e -run2 -e order")
# The first region the filter kept (a longer name holding it would panic after it: its chunks are not saved).
totals() { for line in "exact blocks:" "missing:" "structure stage:"; do grep -m1 "^  $line" "$out/$1.log" | cut -c1-200; done; }
if [ -z "${SKIP_PHASE:-}" ]; then
  REGION=$region ENVS="$worlds" bash "$here/oracle.sh" "$name-phase" "$test" >/dev/null 2>&1
  echo "phase model:"; totals "$name-phase"
fi
REGION=$region ENVS="$worlds $order" bash "$here/oracle.sh" "$name-replay" "$test" >/dev/null 2>&1
echo "replay:"; totals "$name-replay"
ENVS="$worlds $order ${CHECK_ENVS:-}" bash "$here/remote-example.sh" "$name-check" structure_replay_check "${SEED:-20261003}" "$world" "$@" | cut -c1-"${WIDTH:-700}"
