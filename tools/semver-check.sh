#!/usr/bin/env bash
# Checks sapling-api's public API against a git revision (default origin/master) with
# cargo-semver-checks on testbox. Install it there once: `cargo binstall cargo-semver-checks`.
# usage: tools/semver-check.sh [rev]   (REMOTE_DIR as for remote-test.sh)
set -uo pipefail
source "$(dirname "$0")/remote-sync.sh"
rev=${1:-origin/master}
base="$remote-baseline"
git -C "$root" archive --format=tar "$rev" | retry ssh $opts $host "rm -rf $base && mkdir -p $base && tar xf - -C $base" || exit 1
retry ssh $opts $host "cd $remote || exit; rm -f semver.log semver.done; { nohup flock -o target/.busy sh -c '$cargo semver-checks -p sapling-api --baseline-root ~/$base >semver.log 2>&1; echo \$? >semver.done' </dev/null >/dev/null 2>&1 & }" || exit 1
until ssh $opts $host "test -f $remote/semver.done" 2>/dev/null; do sleep 10; done
retry ssh $opts $host "cd $remote; grep -vE '^ +(Compiling|Checking|Documenting|Building|Parsing|Finished)' semver.log | tail -60; echo exit: \$(cat semver.done)"
