#!/usr/bin/env bash
# Runs `cargo test` on testbox (31 GB, Linux) for machines short on RAM, as acacia's tool of the same name.
# usage: tools/remote-test.sh [cargo test args...]   e.g. tools/remote-test.sh -p sapling-shard --test integration commands::
# Each worktree gets its own remote dir (work/<folder>); REMOTE_DIR picks another.
# TICK_SCALE (default 2) runs the bot tests' servers and bots that many times faster (tests/integration/common):
# 2026-10-24 sapling's bot tests took 47.6 s at 1, 33.5 s at 2, ~30 s at 4 (joins and wall waits don't scale).
# HUMAN_SCALE (default 0.1) shortens the bots' human-looking pauses (acacia-bot's human.rs), most of what's left.
set -uo pipefail
source "$(dirname "$0")/remote-sync.sh"
args=$(printf '%q ' "$@")
retry ssh $opts $host "cd $remote || exit; rm -f test.log test.done; { nohup flock -o target/.busy sh -c 'ACACIA_TICK_SCALE=${TICK_SCALE:-2} ACACIA_HUMAN_SCALE=${HUMAN_SCALE:-0.1} $cargo test $args >test.log 2>&1; echo \$? >test.done' </dev/null >/dev/null 2>&1 & }" || exit 1
# Poll with short calls; a single long ssh would die with the link.
until ssh $opts $host "test -f $remote/test.done" 2>/dev/null; do sleep 5; done
retry ssh $opts $host "cd $remote; grep -E '^(error|warning)|^\s+--> |test result|FAILED|panicked|left:|right:' test.log | head -60; echo exit: \$(cat test.done)"
