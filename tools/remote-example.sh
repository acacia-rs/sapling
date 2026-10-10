#!/usr/bin/env bash
# Runs a release example of this worktree on testbox (oracle worlds at $SAPLING_ORACLE_WORLDS) and prints its output.
# usage: tools/remote-example.sh <name> <example> [args...]   e.g. tools/remote-example.sh col biome_column 20261003 20261003 0 0
# env: PKG (default sapling-worldgen), LINES_MAX (default 200); the rest as tools/testbox.sh.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
name=$1
example=$2
shift 2
bash "$here/testbox.sh" "$name" run --release -p "${PKG:-sapling-worldgen}" --example "$example" -- "$@" > /dev/null
code=$?
log=${OUT:-$here/../target/testbox}/$name.log
grep -E '^error' -A12 "$log" | head -n 40
awk 'seen { print } /^ +Running `/ { seen = 1 }' "$log" | head -n "${LINES_MAX:-200}"
echo "log: $log"
echo "exit: $code"
exit "$code"
