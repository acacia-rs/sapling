#!/usr/bin/env bash
# Times generation workloads with examples/timing on testbox, pinned to one core: ms per column, perf stat cycles
# and instructions, peak RSS, heap allocations and a hash of every block (must not move under a speed change).
# usage: tools/bench.sh <name> [workload...]   workloads below (default: all); summary in $OUT/<name>.bench
# env: PROFILE=<workload>: perf record that one instead (line tables, own target dir), report in $OUT/<name>.perf.txt
#      REF: a git ref to time instead of the working tree; CORE (default 3).
# Sync, build and runs are one job under the worktree's testbox lock, so a queued build cannot swap the binary.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
name=$1
shift
remote=${REMOTE_DIR:-work/$(basename "$root")}
run=$remote/target/testbox
out=${OUT:-$root/target/testbox}
opts="-o ConnectTimeout=20 -o ServerAliveInterval=10"
retry() { for i in 1 2 3 4 5; do "$@" && return 0; echo "retry $i: $1" >&2; sleep 3; done; return 1; }
declare -A load=(
  [origin16]="20261003 -8 -8 7 7"
  [strong16]="20261003 -97 -130 -82 -115"
  [strong8]="20261003 -93 -126 -86 -119"
  [village8]="20261003 -66 4 -59 11"
)
workloads=${*:-origin16 strong16 strong8 village8}
target=target
envs=""
if [ -n "${PROFILE:-}" ]; then
  workloads=$PROFILE
  target=target-prof
  envs="CARGO_TARGET_DIR=\$HOME/$remote/target-prof CARGO_PROFILE_RELEASE_DEBUG=line-tables-only"
fi
pack() {
  if [ -n "${REF:-}" ]; then git -C "$root" archive --format=tar.gz "$REF"
  else tar czf - -C "$root" --exclude=target --exclude=.git --exclude=worlds --exclude=.testserver .; fi
}
bin=\$RUN/$name.bin
job="cd \$HOME/$remote && RUN=\$PWD/target/testbox && cd \$RUN && rm -f $name.out $name.perf.txt
rm -rf $name.src && mkdir $name.src && tar xzf $name.tgz -C $name.src && rm $name.tgz &&
  rsync -rlpc --delete --exclude=/target --exclude=/target-prof $name.src/ \$HOME/$remote/ && rm -rf $name.src || { echo 97 > $name.bdone; exit; }
(cd \$HOME/$remote && env $envs \$HOME/.cargo/bin/cargo +stable build --release -p sapling-worldgen --example timing) > $name-build.log 2>&1 || { echo 98 > $name.bdone; exit; }
cp \$HOME/$remote/$target/release/examples/timing $bin"
for w in $workloads; do
  args=${load[$w]:?unknown workload $w}
  if [ -n "${PROFILE:-}" ]; then
    job="$job
taskset -c ${CORE:-3} perf record -q -g --call-graph dwarf,16384 -F 499 -o $name.data $bin $args >> $name.out 2>&1
perf report -i $name.data --no-children --stdio --percent-limit 0.5 -g none 2>/dev/null | grep -v '^#' | grep -v '^$' | head -n 80 > $name.perf.txt
perf report -i $name.data --children --no-inline --stdio --percent-limit 1 -g none 2>/dev/null | grep -v '^#' | grep -v '^$' | head -n 80 >> $name.perf.txt
rm -f $name.data"
  else
    job="$job
echo \"== $w\" >> $name.out
taskset -c ${CORE:-3} /usr/bin/time -f 'maxrss_kb %M' perf stat -x, -e cycles,instructions $bin $args >> $name.out 2>&1"
  fi
done
job="$job
rm -f $bin
echo 0 > $name.bdone"
upload() { pack | ssh $opts testbox "mkdir -p $run && cat > $run/$name.tgz"; }
write_job() { printf '%s\n' "$job" | ssh $opts testbox "rm -f $run/$name.bdone && cat > $run/$name.bench.sh"; }
retry upload || exit 1
retry write_job || exit 1
retry ssh $opts testbox "{ nohup flock $run/lock sh $run/$name.bench.sh </dev/null >/dev/null 2>&1 & }" || exit 1
until ssh $opts testbox "test -f $run/$name.bdone" 2>/dev/null; do sleep 10; done
mkdir -p "$out"
code=$(retry ssh $opts testbox "cat $run/$name.bdone")
if [ "$code" != 0 ]; then
  scp -q $opts "testbox:$run/$name-build.log" "$out/$name-build.log"
  grep -E '^error' -A12 "$out/$name-build.log" | head -n 40
  echo "failed: $code (log $out/$name-build.log)"
  exit 1
fi
scp -q $opts "testbox:$run/$name.out" "$out/$name.out"
if [ -n "${PROFILE:-}" ]; then
  scp -q $opts "testbox:$run/$name.perf.txt" "$out/$name.perf.txt"
  cat "$out/$name.out"
  echo "profile: $out/$name.perf.txt"
  exit 0
fi
awk -F, '
  /^== / { w = $0; sub(/^== /, "", w) }
  /ms per column/ { split($0, a, " "); ms = a[3] }
  /^allocations/ { split($0, a, " "); allocs = a[2]; amb = a[4]; hash = a[6] }
  $3 == "cycles" { cyc = $1 / 1e9 }
  $3 == "instructions" { ins = $1 / 1e9 }
  /^maxrss_kb/ { split($0, a, " "); printf "%-9s %8s ms/col  %7.2f Gcyc  %7.2f Ginstr  %6.0f MB rss  %10s allocs %s MB  hash %s\n", w, ms, cyc, ins, a[2] / 1024, allocs, amb, hash }
' "$out/$name.out" | tee "$out/$name.bench"
