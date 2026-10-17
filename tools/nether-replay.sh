#!/usr/bin/env bash
# The Nether's regions against their one-worker, seed-0 canon on testbox (~/oracle-canon/<dir>: the world, its -run2
# twin and <world>-buffers, the chunk buffers loadChunk committed; tools/worldgen-oracle/README.md, "The Nether").
# Prints each region's totals as tools/oracle-totals.sh reads them, then the stage lines.
# usage: tools/nether-replay.sh <name> [dir (default nether)]   env: REGION, as tools/oracle.sh
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
name=$1
dir=${2:-nether}
out=${OUT:-$here/../target/testbox}
envs="${ENVS:-} SAPLING_ORACLE_WORLDS=\$HOME/oracle-canon/$dir"
[ -n "${REGION:-}" ] && envs="$envs SAPLING_ORACLE_REGION=$(printf '%q' "$REGION")"
ENVS=$envs bash "$here/testbox.sh" "$name" test --release -p sapling-worldgen --test nether_oracle -- --ignored --nocapture --test-threads 1 > /dev/null
code=$?
log=$out/$name.log
grep -E '^error|^test result|panicked' "$log" | tail -n 5
bash "$here/oracle-totals.sh" "$log"
grep -E '(^|\.\.\. )seed|^  stages:' "$log" | sed 's/^test [^ ]* \.\.\. //'
echo "exit: $code"
exit "$code"
