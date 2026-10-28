#!/usr/bin/env bash
# Syncs this worktree to testbox (31 GB, Linux) and runs cargo there; the log comes back as $OUT/<name>.log.
# usage: tools/testbox.sh <name> <cargo args...>   e.g. tools/testbox.sh lib test -p sapling-worldgen --lib
# env: REMOTE_DIR  default work/<this worktree's folder>, so worktrees never share one
#      OUT         default target/testbox
#      ENVS        "K=V ..." for the remote cargo; $HOME and $RUN (the run's remote folder) expand there
#      REF         a git ref to run instead of the working tree
#      FETCH       a path under $RUN to copy back into $OUT as well
#      BATCH       a file of runs `<log name>|<envs>|<cargo args, shell-quoted>`, all in one sync and job, in place
#                  of <cargo args>; each log comes back as $OUT/<log name>.log, the exit code is the highest
#      JOBS        BATCH runs at once after the first (default 1)
# Ends with `exit: <cargo's code>` and exits with it. Use a new <name> per run to keep its log.
# Runs on one REMOTE_DIR queue up. Only files whose content changed are rewritten there, so cargo
# rebuilds what changed whatever the mtimes say.
set -uo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
name=$1
shift
remote=${REMOTE_DIR:-work/$(basename "$root")}
run=$remote/target/testbox
out=${OUT:-$root/target/testbox}
opts="-o ConnectTimeout=20 -o ServerAliveInterval=10"
# testbox's link drops connections intermittently
retry() { for i in 1 2 3 4 5; do "$@" && return 0; echo "retry $i: $1" >&2; sleep 3; done; return 1; }

pack() {
  if [ -n "${REF:-}" ]; then git -C "$root" archive --format=tar.gz "$REF"
  else tar czf - -C "$root" --exclude=target --exclude=.git --exclude=./worlds --exclude=.testserver .; fi
}
upload() { pack | ssh $opts testbox "mkdir -p $run && cat > $run/$name.tgz"; }
cargo_run() { echo "env SAPLING_ORACLE_WORLDS=\$HOME/oracle-worlds RUSTC_WRAPPER=\$HOME/.cargo/bin/sccache $2 \$HOME/.cargo/bin/cargo +stable $3 > \$RUN/$1.log 2>&1"; }
if [ -n "${BATCH:-}" ]; then
  logs=$(cut -d'|' -f1 "$BATCH" | sed 's/$/.log/' | tr '\n' ' ')
  # Only the first run builds; the rest start together, JOBS at a time.
  runs="" i=0
  while IFS='|' read -r log envs args; do
    runs="$runs
{ $(cargo_run "$log" "$envs" "$args"); echo \$? > \$RUN/$log.code; }"
    [ $i -gt 0 ] && runs="$runs &" && [ $((i % ${JOBS:-1})) -eq 0 ] && runs="$runs
wait"
    i=$((i + 1))
  done <"$BATCH"
  runs="$runs
wait
(cd \$RUN && cat $logs) > \$RUN/$name.log
(cd \$RUN && cat ${logs//.log/.code}) | sort -n | tail -n 1 > \$RUN/$name.done"
else
  runs="$(cargo_run "$name" "${ENVS:-}" "$(printf '%q ' "$@")")
echo \$? > \$RUN/$name.done"
fi
# A relaunch after a dropped connection finds the archive gone and leaves the first job alone.
job="cd \$HOME/$remote && [ -f target/testbox/$name.tgz ] || exit 0
RUN=\$PWD/target/testbox
rm -rf \$RUN/$name.src \$RUN/$name.log ${FETCH:+\$RUN/$FETCH} && mkdir \$RUN/$name.src && tar xzf \$RUN/$name.tgz -C \$RUN/$name.src &&
  rsync -rlpc --delete --exclude=/target --exclude=/target-prof \$RUN/$name.src/ . || { echo 'sync failed' > \$RUN/$name.log; echo 97 > \$RUN/$name.done; exit; }
rm -rf \$RUN/$name.src \$RUN/$name.tgz
$runs"
write_job() { printf '%s\n' "$job" | ssh $opts testbox "rm -f $run/$name.done && cat > $run/$name.sh"; }

retry ssh $opts testbox "bash -s -- $remote" <"$root/tools/testbox-target.sh" || exit 1
retry upload || exit 1
retry write_job || exit 1
retry ssh $opts testbox "{ nohup flock $run/lock sh $run/$name.sh </dev/null >/dev/null 2>&1 & }" || exit 1
# Poll with short calls; a single long ssh would die with the link.
until ssh $opts testbox "test -f $run/$name.done" 2>/dev/null; do sleep 10; done
mkdir -p "$out"
retry scp -q $opts "testbox:$run/$name.log" "$out/$name.log"
if [ -n "${FETCH:-}" ]; then
  rm -rf "${out:?}/$FETCH"
  ssh $opts testbox "tar czf - -C $run $FETCH" | tar xzf - -C "$out"
fi
[ -n "${BATCH:-}" ] && ssh $opts testbox "tar czf - -C $run $logs" | tar xzf - -C "$out"
code=$(retry ssh $opts testbox "cat $run/$name.done")
grep -E '^(error|warning: unused)|^\s+--> |^test result|FAILED|panicked|left:|right:' "$out/$name.log" | head -n "${HEAD:-60}"
echo "log: $out/$name.log"
echo "exit: $code"
exit "$code"
