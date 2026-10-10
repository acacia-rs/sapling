#!/usr/bin/env bash
# Per-region totals of an oracle log, or what changed between two.
# usage: tools/oracle-totals.sh <log>            every region: wrong, missing, their share of the stable cells
#        tools/oracle-totals.sh <before> <after> the regions whose wrong + missing moved, and both sums
awk -v two="${2:+1}" '
FNR == 1 { f++ }
# With --test-threads 1 the first region of a test follows `test <name> ... ` on one line.
/^(test [^ ]+ \.\.\. )?seed -?[0-9]+ / { r = $0; sub(/^test [^ ]+ \.\.\. /, "", r); sub(/:$/, "", r); if (!(r in seen)) { seen[r] = 1; order[++n] = r } }
/^  exact blocks:/ { stable[f, r] = $3; wrong[f, r] = $8; has[f, r] = 1 }
/^  missing: / { missing[f, r] = $2 }
# Cells where our structure stage and BDS disagree on a structure block (tests/oracle_support/structures.rs).
/^  structure stage: / { stage[f, r] = $3 }
END {
  for (i = 1; i <= n; i++) {
    r = order[i]; a = wrong[1, r] + missing[1, r]; b = wrong[2, r] + missing[2, r]
    if (!two) {
      printf "%-44s wrong %6d  missing %6d  structure stage %5d  %.4f%% of %d stable\n", r, wrong[1, r], missing[1, r], stage[1, r], 100 * a / stable[1, r], stable[1, r]
      sum_a += a; cells += stable[1, r]; stage_a += stage[1, r]
    } else if (!has[1, r] || !has[2, r]) {
      printf "%-44s only in the %s log\n", r, has[1, r] ? "first" : "second"
    } else {
      moved = stage[1, r] != stage[2, r] ? sprintf("  structure stage %d -> %d", stage[1, r], stage[2, r]) : ""
      if (a != b || moved != "") printf "%-44s %6d -> %6d (%+d)%s\n", r, a, b, b - a, moved
      sum_a += a; sum_b += b; both++; stage_a += stage[1, r]; stage_b += stage[2, r]
    }
  }
  if (!two) printf "%-44s wrong + missing %d, %.4f%% of %d stable; structure stage %d\n", "TOTAL " n " regions", sum_a, cells ? 100 * sum_a / cells : 0, cells, stage_a
  else printf "%-44s %6d -> %6d (%+d)  structure stage %d -> %d\n", "TOTAL " both " regions in both", sum_a, sum_b, sum_b - sum_a, stage_a, stage_b
}' "$@"
