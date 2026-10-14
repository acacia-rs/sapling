#!/usr/bin/env bash
# Two binaries kept by `KEEP=1 tools/bench.sh <name>` timed in turn, A B A B..., REPS times per workload
# under the testbox lock: the best cycles and instructions of each and B's change. Turns cancel most of
# what other users' load does to a single run.
# usage: tools/bench-ab.sh <a> <b> [workload...]   (default: origin16 strong16 strong8 village8; REPS=3)
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
a=$1 b=$2
shift 2
remote=${REMOTE_DIR:-work/$(basename "$root")}
run=$remote/target/testbox
out=${OUT:-$root/target/testbox}
name=ab-$a-$b
opts="-o ConnectTimeout=20 -o ServerAliveInterval=10"
declare -A load=(
  [origin16]="20261003 -8 -8 7 7"
  [strong16]="20261003 -97 -130 -82 -115"
  [strong8]="20261003 -93 -126 -86 -119"
  [village8]="20261003 -66 4 -59 11"
)
# Each A B turn waits for a second in which the box is under a fifth busy (other users' batch jobs);
# QUIET=0 skips the wait when the box stays busy for hours (then trust instructions over cycles).
job="cd \$HOME/$run && rm -f $name.out
quiet() { while :; do set -- \$(head -1 /proc/stat); u=\$((\$2 + \$3 + \$4)); i=\$5; sleep 1; set -- \$(head -1 /proc/stat); b=\$((\$2 + \$3 + \$4 - u)); [ \$((b * 100 / (b + \$5 - i))) -lt 20 ] && return; sleep 20; done; }"
for w in ${*:-origin16 strong16 strong8 village8}; do
  for _ in $(seq "${REPS:-3}"); do
    [ "${QUIET:-1}" = 1 ] && job="$job
quiet"
    for n in $a $b; do
      job="$job
echo '== $n $w' >> $name.out
taskset -c ${CORE:-3} perf stat -x, -e cycles,instructions ./$n.bin ${load[$w]:?unknown workload $w} >> $name.out 2>&1"
    done
  done
done
job="$job
echo 0 > $name.done"
printf '%s\n' "$job" | ssh $opts testbox "rm -f $run/$name.done && cat > $run/$name.sh" || exit 1
ssh $opts testbox "{ nohup flock $run/lock sh $run/$name.sh </dev/null >/dev/null 2>&1 & }" || exit 1
until ssh $opts testbox "test -f $run/$name.done" 2>/dev/null; do sleep 10; done
mkdir -p "$out"
scp -q $opts "testbox:$run/$name.out" "$out/$name.out"
awk -F, -v a="$a" -v b="$b" '
  /^== / { split($0, f, " "); n = f[2]; w = f[3]; if (!(w in seen)) { seen[w] = 1; order[++k] = w } }
  /^allocations/ { split($0, h, " "); hash[n, w] = h[6] }
  $3 == "cycles" { c = $1 / 1e9; if (!((n, w) in cyc) || c < cyc[n, w]) cyc[n, w] = c }
  $3 == "instructions" { i = $1 / 1e9; if (!((n, w) in ins) || i < ins[n, w]) ins[n, w] = i }
  END {
    for (j = 1; j <= k; j++) {
      w = order[j]
      printf "%-9s %7.2f -> %7.2f Gcyc (%+.1f%%)  %7.2f -> %7.2f Ginstr (%+.1f%%)  hash %s %s\n", w, cyc[a, w], cyc[b, w], 100 * (cyc[b, w] / cyc[a, w] - 1), ins[a, w], ins[b, w], 100 * (ins[b, w] / ins[a, w] - 1), hash[a, w] == hash[b, w] ? "same" : "DIFFERS", hash[b, w]
    }
  }' "$out/$name.out" | tee "$out/$name.bench"
