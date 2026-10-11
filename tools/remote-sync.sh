#!/usr/bin/env bash
# Copies the worktree's sources to testbox's $REMOTE_DIR (default work/sapling), keeping its build cache.
# Sourced by remote-test.sh and remote-drill.sh, which use `retry`, `opts` and `remote` after.
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
remote=${REMOTE_DIR:-work/sapling}
opts="-o ConnectTimeout=20 -o ServerAliveInterval=10"
# testbox's link drops connections intermittently
retry() { for i in 1 2 3 4 5; do "$@" && return 0; echo "retry $i: $1" >&2; sleep 3; done; return 1; }

# Clear old sources (keep the build cache) so files deleted locally don't linger remotely.
retry ssh $opts testbox "mkdir -p $remote && find $remote -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +"
# Captures and notes stay behind: nothing built or run there reads them, and they are most of the tree.
tar czf - -C "$root" --exclude=target --exclude=.git --exclude=./research --exclude=./docs . | retry ssh $opts testbox "tar xzf - -C $remote" || exit 1
