#!/usr/bin/env bash
# Runs a probe example on testbox against its Linux BDS (~/work/bds-mechanisms, port 19182) or
# against sapling's drill_server, and copies the capture back.
# usage: tools/remote-drill.sh bds|sapling <example name> <local output file> [probe words...]
# Set REMOTE_DIR per worktree as for remote-test.sh. Probe arguments are single words.
# BDS_DIR and BDS_PORT pick another BDS copy (one drill at a time per copy); SAPLING_PORT the drill
# server's port (default 19183); SAPLING_DRILL_WORLD=strongholds its overworld (see drill_server.rs).
set -uo pipefail
target=$1 example=$2 out=$3 words="${*:4}"
source "$(dirname "$0")/remote-sync.sh"
case $target in
  bds) run="bash tools/bds-drill.sh ${BDS_DIR:-~/work/bds-mechanisms} ${BDS_PORT:-19182} $example drill.txt $words" build="--example $example" ;;
  sapling) run="SAPLING_DRILL_WORLD=${SAPLING_DRILL_WORLD:-flat} bash tools/sapling-drill.sh ${SAPLING_PORT:-19183} $example drill.txt $words" build="--example $example --example drill_server" ;;
  *) echo "first argument: bds or sapling" >&2; exit 2 ;;
esac
retry ssh $opts testbox "cd $remote || exit; rm -f drill.log drill.done drill.txt; { nohup sh -c '~/.cargo/bin/cargo +stable build -q -p sapling $build && $run; echo \$? >drill.done' >drill.log 2>&1 </dev/null & }" || exit 1
# Poll with short calls; a single long ssh would die with the link.
until ssh $opts testbox "test -f $remote/drill.done" 2>/dev/null; do sleep 15; done
retry ssh $opts testbox "cd $remote; tail -n 20 drill.log"
retry scp $opts "testbox:$remote/drill.txt" "$out"
