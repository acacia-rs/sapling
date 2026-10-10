#!/usr/bin/env bash
# The regions of an oracle test as run.sh arguments.
# usage: tools/worldgen-oracle/regions.sh <test fn>   e.g. regions.sh blocks_match_bds_trees
here=$(cd "$(dirname "$0")" && pwd)
awk -v test="fn $1()" '
index($0, test) { on = 1; next }
on && /^}/ { exit }
on { while (match($0, /Region::new\("[^"]*", *-?[0-9]+, *-?[0-9]+, *-?[0-9]+, *-?[0-9]+\)/)) {
       r = substr($0, RSTART, RLENGTH); $0 = substr($0, RSTART + RLENGTH)
       sub(/^Region::new\("[^"]*", */, "", r); sub(/\)$/, "", r); gsub(/ /, "", r); printf "%s ", r } }
END { print "" }' "$here/../../crates/sapling-worldgen/tests/oracle.rs"
