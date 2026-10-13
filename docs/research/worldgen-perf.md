# sapling-worldgen performance

Branch `worldgen/perf8` (from all2 1333c09): every change keeps the output block-exact (the
float operations, draw order and iteration order are untouched); each commit message carries its own
before and after.

## Measuring

`tools/bench.sh <name>` (`REF=<commit>` for a commit) syncs, builds `examples/timing` and runs four
workloads on one testbox core under one lock, printing ms per column, `perf stat` cycles and
instructions, peak RSS (`/usr/bin/time`), heap allocations and a hash of every generated block:

| workload | chunks | what it is |
|---|---|---|
| origin16 | -8..7 x -8..7 | mixed biomes around the origin |
| strong16 | -97..-82 x -130..-115 | the stronghold canon's area, connected |
| strong8 | -93..-86 x -126..-119 | the same, cold and isolated (the nine-phase ring) |
| village8 | -66..-59 x 4..11 | the village canon |

The hash must not move; a change that moves it is not a speed change. `PROFILE=<workload>` records a
call graph instead (line tables in `target-prof`); `perf annotate -l` on that binary ranks lines.

## Result

Baseline (1333c09) against the branch's last commit, cycles and peak RSS from `perf stat` and `time`:

| workload | ms/column | Gcycles | Ginstructions | peak RSS MB | MB allocated |
|---|---|---|---|---|---|
| origin16 | 43.0 -> 19.4 | 46.2 -> 20.9 | 110.0 -> 52.2 | 899 -> 377 | 4343 -> 2540 |
| strong16 | 96.6 -> 32.2 | 103.6 -> 34.6 | 258.0 -> 89.9 | 1248 -> 595 | 5270 -> 2864 |
| strong8 | 225.6 -> 82.8 | 60.4 -> 22.2 | 152.0 -> 57.7 | 953 -> 417 | 2549 -> 1431 |
| village8 | 107.8 -> 50.4 | 28.9 -> 13.5 | 70.6 -> 34.1 | 656 -> 269 | 1821 -> 1053 |

The largest wins, in order: the preliminary surface computed once per quart column instead of hundreds
of times per chunk (density/surfaces.rs); evicted cache entries taken back while another cache still
holds them (cache.rs: staged chunks held their carved terrain while the carved cache had dropped it,
so terrain was generated again and again); terrain adjustments skipping kernels that are zero at the
block's height; template passes classifying blocks by table instead of by state string; blocks stored
in two bytes (half the memory of every cached chunk); the biome lattice sampling only columns its
blocks read.

## Noise lanes and the aquifer (second round)

- The blended noise fills its 25 cell columns row by row (noise/blended.rs). Every column decides
  gradient reuse alike, row by row: the last row of a column leaves its lattice index for the next.
  So the columns run together with the same per-sample operations. An octave whose first row would
  reuse the previous column's gradients (low-frequency limit octaves) keeps BDS's column-by-column
  order. This gave 5-6% fewer cycles.
- The aquifer answers air at once where all twelve candidate cells share one fluid status. Then every
  pressure is 0 and so is the barrier, and at or above the level the update flag does not matter. Above
  sea level the twelve statuses are compared once per grid cell. This gave 7-9% on the structure
  workloads, whose adjusted chunks generate every row up to y 255.

## Where the time goes now

`PROFILE`, last commit, self time (strong16 / origin16): Perlin octaves (`sample_split` plus
`PerlinNoise::sample`: cave, noodle, vein, climate) 15.0% / 14.5%, the terrain block loop with the
inlined blended fill 9.2 / 7.8, the aquifer 9.0 / 8.0, the blended gradient recomputes 5.5 / 6.8, and
the feature dispatch with ore veins inlined 5.9 / 9.5. Then the structure stage's reads
(`Canvas::get`) 4.0 / -, terrain adjustments 2.8 / -, and decoration's view reads - / 3.0.

## Verification (last commit)

- `tools/replay-all.sh` (the canons of replay-canons.txt and the structure regions of
  replay-worlds.txt through replay-regions.sh): all 59 regions 0 wrong, 0 missing, structure stage 0.
- Stock oracle `tools/oracle.sh v9` and `ENVS=SAPLING_ORACLE_STILL=1 tools/oracle.sh v9-still`: wrong +
  missing 22540 and 23411, as on all2.
- `tools/testbox.sh lib-fin2 test -p sapling-worldgen -p sapling-world -p sapling-data --lib`: all pass.

## Tried, no gain

- A per-quart-column cache of the climate sample: neutral once the preliminary surface is cached.
- A branchless top three of the aquifer's twelve candidates: neutral; the branchy insert predicts well.
- A staged-chunk cache of 2048 instead of 10240: peak RSS down by a third to two fifths but 15-45% slower,
  since staged chunks are what keeps carved terrain alive.
- Perlin and normal noise sampled in eight lanes: first the vein and noodle noises of a cell column's
  kept corners, then the whole cavifier per column. Every lane was bit-identical (a test compared them),
  but the timing was neutral to 2% slower with more instructions. The permutation-table chains and
  gradient lookups are scalar gathers on the SSE2 baseline, and the core already overlaps independent
  scalar samples.

## Left

By expected gain: the Perlin octaves with gathers (only on a target with AVX2, so not exact-portable
work); the blended gradient recomputes; the aquifer's nearest-centre search below sea level; overlays of writes
instead of whole-chunk copies for decoration's earlier-phase states and post-process results (memory);
the structure stage's neighbour reads; contention of the mutex caches under many threads (unmeasured).
