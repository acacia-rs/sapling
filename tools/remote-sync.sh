#!/usr/bin/env bash
# Copies the worktree's sources (or REF's, a git ref) to testbox's $REMOTE_DIR (default work/<this worktree's
# folder>), keeping its build cache, which lives on tmpfs (tools/testbox-target.sh).
# Sourced by remote-test.sh, remote-drill.sh and remote-drills.sh, which use `retry`, `opts`, `remote` and `cargo` after.
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
remote=${REMOTE_DIR:-work/$(basename "$root")}
opts="-o ConnectTimeout=20 -o ServerAliveInterval=10"
# For a remote shell; sccache refills a new or evicted target.
cargo="env RUSTC_WRAPPER=\$HOME/.cargo/bin/sccache \$HOME/.cargo/bin/cargo +stable"
# testbox's link drops connections intermittently
retry() { for i in 1 2 3 4 5; do "$@" && return 0; echo "retry $i: $1" >&2; sleep 3; done; return 1; }

retry ssh $opts testbox "bash -s -- $remote" <"$root/tools/testbox-target.sh" || exit 1
# Captures and notes stay behind: nothing built or run there reads them, and they are most of the tree.
if [ -n "${REF:-}" ]; then
  stage=$remote.ref
  git -C "$root" archive --format=tar.gz "$REF" -- . ':!research' ':!docs' | retry ssh $opts testbox "rm -rf $stage && mkdir -p $stage && tar xzf - -C $stage" || exit 1
  # Untracked build data (sapling-data's captured tables) comes from the working tree.
  git -C "$root" ls-files -z -o -i --exclude-standard -- crates | grep -zv -e /target/ -e __pycache__ \
    | tar czf - -C "$root" --null -T - | retry ssh $opts testbox "tar xzf - -C $stage" || exit 1
  # An archive's files all carry the commit's time, so only changed content is rewritten (it gets a new
  # mtime): cargo then rebuilds what differs from the last ref built here.
  retry ssh $opts testbox "rsync -rlpc --delete --exclude=/target $stage/ $remote/ && rm -rf $stage" || exit 1
else
  # Clear old sources (keep the build cache) so files deleted locally don't linger remotely.
  retry ssh $opts testbox "find $remote -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +"
  tar czf - -C "$root" --exclude=target --exclude=.git --exclude=./research --exclude=./docs . | retry ssh $opts testbox "tar xzf - -C $remote" || exit 1
fi
