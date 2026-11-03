#!/usr/bin/env bash
# The pre-push gate: every crate's lib and tests and the BDS probe examples, through the build lane. Failures are
# re-run alone, one at a time: the box is shared (load 90-120 from other projects on 2026-11-03), and wall-clock
# waits there miss while the test is sound. Prints each failure as `real` or `load`; push only on `exit: 0`.
set -uo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
remote=${REMOTE_DIR:-work/$(basename "$root")}
args=(-p sapling-api -p sapling-shard -p sapling-leveldb -p sapling-world -p sapling-data -p sapling -p sapling-net -p sapling-worldgen
  --lib --tests --example bds_command_probe --example bds_loot_probe)
run() { /srv/agents/work/acacia/tools/lane.sh build 2 bash "$root/tools/remote-test.sh" "$@" 2>&1 | grep -oP '^exit: \K\d+' | tail -n 1; }
failed() { ssh testbox "grep -oP '^test \K\S+(?= \.\.\. FAILED\$)' $remote/test.log" | sort -u; }

code=$(run --no-fail-fast "${args[@]}")
if [ "${code:-1}" = 0 ]; then echo "exit: 0"; exit 0; fi
names=$(failed)
if [ -z "$names" ]; then
  ssh testbox "grep -E '^error(\[|:)' $remote/test.log" | sort -u | head -n 20
  echo "exit: ${code:-1} (no failing test named: a build error)"
  exit "${code:-1}"
fi
# shellcheck disable=SC2086
run --no-fail-fast "${args[@]}" -- --exact --test-threads 1 $names >/dev/null
real=$(failed)
for name in $names; do grep -qxF "$name" <<<"$real" && echo "real $name" || echo "load $name"; done
if [ -n "$real" ]; then echo "exit: 101"; exit 101; fi
echo "exit: 0 (every failure passed alone)"
