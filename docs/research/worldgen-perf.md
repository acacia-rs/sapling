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
| origin16 | 43.0 -> 18.2 | 46.2 -> 19.5 | 110.0 -> 44.5 | 899 -> 376 | 4343 -> 2555 |
| strong16 | 96.6 -> 29.6 | 103.6 -> 31.6 | 258.0 -> 76.5 | 1248 -> 596 | 5270 -> 2887 |
| strong8 | 225.6 -> 74.9 | 60.4 -> 20.1 | 152.0 -> 49.0 | 953 -> 418 | 2549 -> 1447 |
| village8 | 107.8 -> 46.3 | 28.9 -> 12.4 | 70.6 -> 28.7 | 656 -> 270 | 1821 -> 1065 |

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

## Third round

- Blended gradient recomputes take the permutation chain's x and y steps once per grid x (five columns
  share it): about 1%.
- The structure canvas's map of writes past the chunk uses FxHash and is skipped while empty: cycles
  flat, instructions down 1.5% on the structure workloads.
- Decoration's view keeps its nine chunks in OnceCells (no RefCell borrow per read) and locates blocks
  by shift and mask: 2.5-3.5%.
- Cave, noodle and vein noises are sampled in eight lanes per cell column, past each gate. AVX2 is
  chosen at run time (noise/avx2.rs; the build stays baseline x86-64 and FMA is never enabled):
  - `round_ps` floors exactly as `f32::floor`.
  - The permutation lookups are integer gathers.
  - A gradient component times p is p, -p or p's sign alone, by masks.
  - Lanes are bit-identical to the scalar sample (noise/tests.rs, special values included).

  This gave 3-5% fewer cycles and 11-14% fewer instructions. With the gradient table as float gathers
  it was 10% slower than scalar; the portable lanes without AVX2 were neutral.
- Threads (`tools/bench.sh mt mt1 mt6`, origin16 on one generator, six cores): 18.4 -> 10.1 ms per
  column with 6 threads, cycles +8%, about 2 cores busy. perf shows no lock functions and few context
  switches: the threads wait on each other's OnceLock'd sources in the shared sweep, not on cache
  mutexes. Spreading threads over distant areas would scale; the mutexes need no change.

## Where the time goes now

`PROFILE`, last commit, self time (strong16 / origin16):

- the AVX2 noise lanes 9.7% / 9.9% (plus scalar Perlin for climate and surface, about 2%);
- the terrain block loop 9.8 / 7.7;
- the aquifer 9.4 / 9.2;
- the feature dispatch with ore veins inlined 6.1 / 9.7;
- the blended fill 5.6 / 7.1;
- the structure stage's reads 3.4 / -;
- decoration's view reads - / 3.6.

## Verification (last commit, eb1acb4)

- `tools/replay-all.sh fin3` (the canons of replay-canons.txt and the structure regions of
  replay-worlds.txt through replay-regions.sh): all 59 regions 0 wrong, 0 missing, structure stage 0.
- Stock oracle `tools/oracle.sh v10` and `ENVS=SAPLING_ORACLE_STILL=1 tools/oracle.sh v10-still`: wrong +
  missing 22540 and 23411, as on all2.
- `tools/testbox.sh lib-fin3 test -p sapling-worldgen -p sapling-world -p sapling-data --lib`: all pass.

## Tried, no gain

- A per-quart-column cache of the climate sample: neutral once the preliminary surface is cached.
- A branchless top three of the aquifer's twelve candidates: neutral; the branchy insert predicts well.
- A staged-chunk cache of 2048 instead of 10240: peak RSS down by a third to two fifths but 15-45% slower,
  since staged chunks are what keeps carved terrain alive.
- Perlin and normal noise sampled in eight lanes: first the vein and noodle noises of a cell column's
  kept corners, then the whole cavifier per column. Every lane was bit-identical (a test compared them),
  but the timing was neutral to 2% slower with more instructions. The permutation-table chains and
  gradient lookups are scalar gathers on the SSE2 baseline, and the core already overlaps independent
  scalar samples. (The AVX2 path above is what paid.)
- The aquifer's shared-status answer extended below sea level (barrier 0, the update flag from the two
  smallest distances) was mixed: origin -1.6%, strong16 +1.3%. Comparing the twelve statuses of every
  cell costs statuses the full search never computes.
- The loaded chunk sharing the staged one's Arc when no post-process wrote it: peak RSS -4 to -9%, but
  cycles +1.5 to 2% in two runs.
- Write overlays for decoration's earlier-phase states and the post-process results were not built.
  heaptrack (strong16) puts the peak in carved terrain the staged chunks hold (about 230 MB), the
  post-process results (about 140 MB) and copies made by writes into staged chunks (about 60 MB). An
  overlay needs a layered read path in the view, the canvas and the structure features, each of which
  now reads plain chunks. The cheaper sharing above already cost cycles.

## Left

By expected gain:

1. The noise lanes' permutation gathers.
2. The terrain block loop.
3. The aquifer's candidate search.
4. A layered chunk representation for the staged and post-process states (memory).
5. Thread scheduling that spreads a server's workers over distant areas.
