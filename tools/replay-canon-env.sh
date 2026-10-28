#!/usr/bin/env bash
# Prints the ENVS that replay a canon of tools/replay-canons.txt (its one-worker run in ~/oracle-canon/<dir> on
# testbox); they expand there. usage: ENVS="$(tools/replay-canon-env.sh <canon>) ..." tools/oracle.sh ...
# env: NO_STRUCTURE=1 leaves the structure stage unreplayed.
here=$(cd "$(dirname "$0")" && pwd)
line=$(grep -E "^$1\|" "$here/replay-canons.txt") || { echo "unknown canon $1" >&2; exit 2; }
IFS='|' read -r canon _ _ _ _ dir _ <<<"$line"
at="\$HOME/oracle-canon/${dir:-$canon}"
structure="\$(test -f $at/structure-order.txt && echo SAPLING_ORACLE_STRUCTURE_ORDER=$at/structure-order.txt)"
[ -n "${NO_STRUCTURE:-}" ] && structure=""
# <dir>/env.txt: further variables of that run (SAPLING_ORACLE_DEAD_CORAL=<CoralColor>: BDS draws it from the clock).
echo "SAPLING_ORACLE_WORLDS=$at SAPLING_ORACLE_ORDER=$at/order.txt SAPLING_ORACLE_THREAD_SEED=0 $structure \$(cat $at/env.txt 2>/dev/null)"
