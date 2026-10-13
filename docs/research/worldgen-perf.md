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
| origin16 | 43.0 -> 20.9 | 46.2 -> 22.4 | 110.0 -> 53.8 | 899 -> 376 | 4343 -> 2529 |
| strong16 | 96.6 -> 36.8 | 103.6 -> 39.5 | 258.0 -> 97.0 | 1248 -> 595 | 5270 -> 2848 |
| strong8 | 225.6 -> 95.8 | 60.4 -> 25.8 | 152.0 -> 63.6 | 953 -> 418 | 2549 -> 1420 |
| village8 | 107.8 -> 56.2 | 28.9 -> 15.1 | 70.6 -> 36.5 | 656 -> 269 | 1821 -> 1046 |

The largest wins, in order: the preliminary surface computed once per quart column instead of hundreds
of times per chunk (density/surfaces.rs); evicted cache entries taken back while another cache still
holds them (cache.rs: staged chunks held their carved terrain while the carved cache had dropped it,
so terrain was generated again and again); terrain adjustments skipping kernels that are zero at the
block's height; template passes classifying blocks by table instead of by state string; blocks stored
in two bytes (half the memory of every cached chunk); the biome lattice sampling only columns its
blocks read.

## Where the time goes now

`PROFILE`, last commit, self time (strong16 / origin16): the aquifer 13.7% / 7.9%, the blended noise's
octave fills 11.6 / 12.4, Perlin octaves (`sample_split` plus `PerlinNoise::sample`: cave, noodle,
vein, climate) 13.4 / 13.8, the terrain block loop 8.1 / 6.0, the feature dispatch with ore veins
inlined 4.7 / 8.3, the structure stage's reads (`Canvas::get`) 3.8 / -, terrain adjustments 2.4 / -.
Terrain is about half of all cycles; the rest is spread thin over decoration, the structure stage and
the biome zoom.

## Verification (last commit)

- `tools/replay-all.sh fin` (the canons of replay-canons.txt and the structure regions of
  replay-worlds.txt through replay-regions.sh): all 59 regions 0 wrong, 0 missing, structure stage 0.
- Stock oracle `tools/oracle.sh v8` and `ENVS=SAPLING_ORACLE_STILL=1 tools/oracle.sh v8-still`: wrong +
  missing 22540 and 23411, as on all2.
- `tools/testbox.sh lib-fin test -p sapling-worldgen -p sapling-world -p sapling-data --lib`: all pass.

## Tried, no gain

- A per-quart-column cache of the climate sample: neutral once the preliminary surface is cached.
- A branchless top three of the aquifer's twelve candidates: neutral; the branchy insert predicts well.
- A staged-chunk cache of 2048 instead of 10240: peak RSS down by a third to two fifths but 15-45% slower,
  since staged chunks are what keeps carved terrain alive.

## Left

By expected gain: the noise fills (lanes across the cell grid's columns, if the gradient reuse can be
kept per column); the aquifer's per-block work in fully generated adjusted chunks; overlays of writes
instead of whole-chunk copies for decoration's earlier-phase states and post-process results (memory);
the structure stage's neighbour reads; contention of the mutex caches under many threads (unmeasured).
