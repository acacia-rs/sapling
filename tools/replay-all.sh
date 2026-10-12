#!/usr/bin/env bash
# Every region of the oracle tests against its one-worker canon, one line each:
# `<region> | <canon> | wrong N missing M structure stage K`, then the sum. The canons of tools/replay-canons.txt
# first, then the structure regions of tools/replay-worlds.txt; logs target/testbox/<prefix>-<canon>.log.
# usage: tools/replay-all.sh <prefix> [canon...]   (names of either list; default: all)
# Prints as each run ends; a run takes one to four minutes, all of them about 80.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
prefix=$1
shift
out=${OUT:-$here/../target/testbox}
canons=$(grep -v '^#' "$here/replay-canons.txt" | cut -d'|' -f1)
worlds=$(grep -v '^#' "$here/replay-worlds.txt" | cut -d'|' -f1)
total=0
for canon in ${*:-$canons $worlds}; do
  if grep -qx "$canon" <<<"$worlds"; then
    region=$(grep -E "^$canon\|" "$here/replay-worlds.txt" | cut -d'|' -f4)
    bash "$here/replay-regions.sh" "$prefix" "$canon" >/dev/null 2>&1
  else
    region=$(grep -E "^$canon\|" "$here/replay-canons.txt" | cut -d'|' -f4)
    bash "$here/replay.sh" "$prefix-$canon" "$canon" test >/dev/null 2>&1
  fi
  log=$out/$prefix-$canon.log
  # The first region the filter kept: a region name that is part of another's fails after its own lines.
  wrong=$(grep -m1 '^  exact blocks:' "$log" 2>/dev/null | awk '{ print $8 }')
  missing=$(grep -m1 '^  missing:' "$log" 2>/dev/null | awk '{ print $2 }')
  stage=$(grep -m1 '^  structure stage:' "$log" 2>/dev/null | awk '{ print $3 }')
  echo "$region | $canon | wrong ${wrong:-?} missing ${missing:-?} structure stage ${stage:-?}"
  total=$((total + ${wrong:-0} + ${missing:-0}))
done
echo "TOTAL wrong + missing $total"
