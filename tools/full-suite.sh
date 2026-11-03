#!/usr/bin/env bash
# The pre-push gate: every crate's lib and tests and the BDS probe examples, through the build lane. Prints each
# failing test and the `exit:` line; push only on `exit: 0`.
set -uo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
remote=${REMOTE_DIR:-work/$(basename "$root")}
out=$(/srv/agents/work/acacia/tools/lane.sh build 2 bash "$root/tools/remote-test.sh" --no-fail-fast \
  -p sapling-api -p sapling-shard -p sapling-leveldb -p sapling-world -p sapling-data -p sapling -p sapling-net -p sapling-worldgen \
  --lib --tests --example bds_command_probe --example bds_loot_probe 2>&1)
ssh testbox "grep -E '\.\.\. FAILED\$|^error(\[|:)' $remote/test.log" | sort -u
grep -E '^exit:' <<<"$out" | tail -n 1
code=$(grep -oP '^exit: \K\d+' <<<"$out" | tail -n 1)
exit "${code:-1}"
