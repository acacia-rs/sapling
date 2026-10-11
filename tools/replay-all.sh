#!/usr/bin/env bash
# The oracle totals of every canon in tools/replay-canons.txt that names a test region, one line each
# (`<canon> <region> wrong N missing M`); logs target/testbox/<prefix>-<canon>.log.
# usage: tools/replay-all.sh <prefix> [canon...]
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
prefix=$1
shift
canons=${*:-$(awk -F'|' '$4 != "-" { print $1 }' "$here/replay-canons.txt")}
for canon in $canons; do
  # A region name that is part of another's (badlands) fails on that one's unsaved chunks after its own line.
  line=$(bash "$here/replay.sh" "$prefix-$canon" "$canon" test 2>&1 | grep -E '^seed |^error' | sort -r | head -n 1)
  echo "$canon ${line:-no result}"
done
