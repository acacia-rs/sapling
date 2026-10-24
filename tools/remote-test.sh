#!/usr/bin/env bash
# Runs `cargo test` on testbox (31 GB, Linux) for machines short on RAM, as acacia's tool of the same name.
# usage: tools/remote-test.sh [cargo test args...]   e.g. tools/remote-test.sh -p sapling-shard --test commands
# Each worktree gets its own remote dir (work/<folder>); REMOTE_DIR picks another.
set -uo pipefail
source "$(dirname "$0")/remote-sync.sh"
args=$(printf '%q ' "$@")
retry ssh $opts testbox "cd $remote || exit; rm -f test.log test.done; { nohup flock target/.busy sh -c '$cargo test $args >test.log 2>&1; echo \$? >test.done' </dev/null >/dev/null 2>&1 & }" || exit 1
# Poll with short calls; a single long ssh would die with the link.
until ssh $opts testbox "test -f $remote/test.done" 2>/dev/null; do sleep 5; done
retry ssh $opts testbox "cd $remote; grep -E '^(error|warning)|^\s+--> |test result|FAILED|panicked|left:|right:' test.log | head -60; echo exit: \$(cat test.done)"
