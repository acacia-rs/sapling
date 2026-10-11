#!/usr/bin/env bash
# The oracle totals of the replay worlds in tools/replay-worlds.txt, each against its own one-worker run
# under the full replay (structure order, decoration order, thread draws): one line per region,
# `<name> <region>: wrong N missing M structure stage K`. Logs: target/testbox/<prefix>-<name>.log.
# usage: tools/replay-regions.sh <prefix> [name...]   (default: every world of the list that is on testbox)
#        tools/replay-regions.sh <prefix> check <name>  examples/structure_replay_check over that region
#        tools/replay-regions.sh <prefix> rc <name>     examples/replay_check over it
# env: ENVS adds to the replay's (CHECK: SHOW, CELLS, NEAR); the rest as tools/testbox.sh.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
prefix=$1
shift
out=${OUT:-$here/../target/testbox}
row() { grep -E "^$1\|" "$here/replay-worlds.txt" || { echo "unknown replay world $1" >&2; exit 2; }; }
if [ "${1:-}" = check ] || [ "${1:-}" = rc ]; then
  IFS='|' read -r name world _ _ box <<<"$(row "$2")"
  example=structure_replay_check
  [ "$1" = rc ] && example=replay_check
  bash "$here/replay-ex.sh" "$prefix-$name-$1" "$name" "$example" "${SEED:-20261003}" "$world" $box | cut -c1-"${WIDTH:-600}"
  exit
fi
have=$(ssh -o ConnectTimeout=20 testbox "ls replay-worlds")
names=${*:-$(cut -d'|' -f1 "$here/replay-worlds.txt")}
for name in $names; do
  grep -qx "$name" <<<"$have" || continue
  IFS='|' read -r _ _ test region _ <<<"$(row "$name")"
  REGION=$region ENVS="$(bash "$here/replay-env.sh" "$name") ${ENVS:-}" bash "$here/oracle.sh" "$prefix-$name" "$test" >/dev/null 2>&1
  log=$out/$prefix-$name.log
  # The first region the filter kept: a longer name holding it fails after it on unsaved chunks.
  wrong=$(grep -m1 '^  exact blocks:' "$log" | awk '{ print $8 }')
  missing=$(grep -m1 '^  missing:' "$log" | awk '{ print $2 }')
  stage=$(grep -m1 '^  structure stage:' "$log" | cut -d' ' -f5- | cut -c1-160)
  echo "$name $region: wrong ${wrong:-?} missing ${missing:-?} structure stage ${stage:-?}"
done
