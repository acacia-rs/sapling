#!/usr/bin/env bash
# Runs one probe example's drills against sapling's drill_server on testbox, each word on its own
# server and port at once, and copies the captures back as <local dir>/<word>.txt (and .log).
# usage: tools/remote-drills.sh <example name> <local dir> <word>...   e.g. tools/remote-drills.sh bds_command_probe ab/new water lava shove
# env: JOBS drills at once (default all), SAPLING_PORT the first port (default 19300), SAPLING_DRILL_WORLD as
# for remote-drill.sh, TICK_SCALE as for sapling-drill.sh.
# One build for all words; tools/remote-drill.sh runs one drill (or one against BDS).
set -uo pipefail
example=$1 out=$2
shift 2
source "$(dirname "$0")/remote-sync.sh"
words=$(printf '%q ' "$@")
job="rm -rf drills && mkdir drills && $cargo build -q -p sapling --example $example --example drill_server || exit 98
i=0
for word in $words; do
  while [ \$(jobs -rp | wc -l) -ge ${JOBS:-$#} ]; do wait -n; done
  TICK_SCALE=${TICK_SCALE:-} SAPLING_DRILL_WORLD=${SAPLING_DRILL_WORLD:-flat} bash tools/sapling-drill.sh \$((${SAPLING_PORT:-19300} + i)) $example drills/\$word.txt \$word >drills/\$word.log 2>&1 &
  i=\$((i + 1))
done
wait"
printf '%s\n' "$job" | retry ssh $opts $host "cat > $remote/drills.sh" || exit 1
retry ssh $opts $host "cd $remote || exit; rm -f drills.log drills.done; { nohup flock target/.busy sh -c 'bash drills.sh; echo \$? >drills.done' >drills.log 2>&1 </dev/null & }" || exit 1
# Poll with short calls; a single long ssh would die with the link.
until ssh $opts $host "test -f $remote/drills.done" 2>/dev/null; do sleep 5; done
mkdir -p "$out"
retry ssh $opts $host "tail -n 20 $remote/drills.log"
ssh $opts $host "tar czf - -C $remote/drills ." | tar xzf - -C "$out" || exit 1
for word in "$@"; do echo "$word: $(tail -1 "$out/$word.txt" 2>/dev/null), $(wc -l <"$out/$word.txt" 2>/dev/null) lines"; done
code=$(retry ssh $opts $host "cat $remote/drills.done")
grep -h '^LAGGED' "$out"/*.txt 2>/dev/null | head -1
echo "exit: $code"
exit "$code"
