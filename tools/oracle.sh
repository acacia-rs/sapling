#!/usr/bin/env bash
# Runs sapling-worldgen's oracle test of this worktree on testbox and prints every region's totals.
# usage: tools/oracle.sh <name> [test filter]   e.g. tools/oracle.sh base        (every blocks_match_bds_* test)
#                                                    tools/oracle.sh t1 blocks_match_bds_trees
# env: REGION=<substring> keeps the regions whose name contains it; DUMP=1 fetches the per-cell dump to
#      $OUT/<name>.dump; the rest as tools/testbox.sh. `tools/oracle-totals.sh <before log> <after log>`
#      lists the regions that moved.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
name=$1
filter=${2:-blocks_match_bds}
out=${OUT:-$here/../target/testbox}
envs=${ENVS:-}
[ -n "${REGION:-}" ] && envs="$envs SAPLING_ORACLE_REGION=$(printf '%q' "$REGION")"
if [ -n "${DUMP:-}" ]; then
  envs="$envs SAPLING_ORACLE_DUMP=\$RUN/$name.dump"
  export FETCH=$name.dump
fi
# One test at a time: each already uses every core for its chunks, and the log keeps one order.
ENVS=$envs bash "$here/testbox.sh" "$name" test --release -p sapling-worldgen --test oracle -- --ignored --nocapture --test-threads 1 "$filter" > /dev/null
code=$?
log=$out/$name.log
grep -E '^error|^\s+--> |^test .*(ok|FAILED)|^test result|panicked' "$log" | tail -n 30
bash "$here/oracle-totals.sh" "$log"
echo "log: $log"
echo "exit: $code"
exit "$code"
