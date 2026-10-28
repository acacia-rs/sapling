#!/usr/bin/env bash
# Every region of the oracle tests against its one-worker canon, one line each:
# `<region> | <canon> | wrong N missing M structure stage K`, then the sum. The canons of tools/replay-canons.txt
# first, then the structure regions of tools/replay-worlds.txt; logs target/testbox/<prefix>-<canon>.log.
# usage: tools/replay-all.sh <prefix> [canon...]   (names of either list; default: all)
# One sync and one testbox job for all of them (tools/testbox.sh BATCH); env: ENVS adds to every run's,
# JOBS runs at once (default 3: one run keeps ~4 of testbox's 12 cores busy).
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
prefix=$1
shift
out=${OUT:-$here/../target/testbox}
canons=$(grep -v '^#' "$here/replay-canons.txt" | cut -d'|' -f1)
worlds=$(grep -v '^#' "$here/replay-worlds.txt" | cut -d'|' -f1)
have=$(ssh -o ConnectTimeout=20 testbox "ls replay-worlds")
batch=$(mktemp) rows=$(mktemp)
trap 'rm -f "$batch" "$rows"' EXIT
args="test --release -p sapling-worldgen --test oracle -- --ignored --nocapture --test-threads 1"
for canon in ${*:-$canons $worlds}; do
  if grep -qx "$canon" <<<"$worlds"; then
    grep -qx "$canon" <<<"$have" || continue
    IFS='|' read -r _ _ test region _ <<<"$(grep -E "^$canon\|" "$here/replay-worlds.txt")"
    envs=$(bash "$here/replay-env.sh" "$canon") filter=$region
  else
    IFS='|' read -r _ _ test region _ <<<"$(grep -E "^$canon\|" "$here/replay-canons.txt")"
    envs=$(bash "$here/replay-canon-env.sh" "$canon") filter="=$region"
  fi
  echo "$prefix-$canon|$envs ${ENVS:-} SAPLING_ORACLE_REGION=$(printf '%q' "$filter")|$args $test" >>"$batch"
  echo "$canon|$region" >>"$rows"
done
BATCH=$batch JOBS=${JOBS:-3} bash "$here/testbox.sh" "$prefix" >/dev/null
total=0
while IFS='|' read -r canon region; do
  log=$out/$prefix-$canon.log
  # The first region the filter kept: a region name that is part of another's fails after its own lines.
  wrong=$(grep -m1 '^  exact blocks:' "$log" 2>/dev/null | awk '{ print $8 }')
  missing=$(grep -m1 '^  missing:' "$log" 2>/dev/null | awk '{ print $2 }')
  stage=$(grep -m1 '^  structure stage:' "$log" 2>/dev/null | awk '{ print $3 }')
  echo "$region | $canon | wrong ${wrong:-?} missing ${missing:-?} structure stage ${stage:-?}"
  total=$((total + ${wrong:-0} + ${missing:-0}))
done <"$rows"
echo "TOTAL wrong + missing $total"
