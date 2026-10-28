#!/usr/bin/env bash
# Prints the ENVS that replay a run sent by tools/replay-world.sh: its worlds, its structure order and,
# when the run logged decorations (scripts/data/dbg_canon.json), its decoration order and thread seed 0.
# They expand on testbox. usage: ENVS="$(tools/replay-env.sh <name>) ..." tools/remote-example.sh ...
dir="\$HOME/replay-worlds/$1"
echo "SAPLING_ORACLE_WORLDS=$dir SAPLING_ORACLE_STRUCTURE_ORDER=$dir/structure-order.txt \$(test -f $dir/order.txt && echo SAPLING_ORACLE_ORDER=$dir/order.txt SAPLING_ORACLE_THREAD_SEED=0)"
