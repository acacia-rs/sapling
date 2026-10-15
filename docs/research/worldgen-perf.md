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
| origin16 | 43.0 -> 17.2 | 46.2 -> 18.6 | 110.0 -> 45.3 | 899 -> 190 | 4343 -> 1383 |
| strong16 | 96.6 -> 28.0 | 103.6 -> 30.1 | 258.0 -> 77.0 | 1248 -> 215 | 5270 -> 1511 |
| strong8 | 225.6 -> 70.6 | 60.4 -> 19.0 | 152.0 -> 48.8 | 953 -> 148 | 2549 -> 839 |
| village8 | 107.8 -> 43.7 | 28.9 -> 11.7 | 70.6 -> 28.9 | 656 -> 126 | 1821 -> 604 |

Those are round four's quiet measurements (835f265). Round five ran while testbox stayed saturated
by other users' jobs for hours, so it is measured in instructions, which the load does not move
(cycles moved by up to 50% between runs). Instructions at the last commit: origin16 44.3 G,
strong16 72.4, strong8 45.3, village8 28.0 (round four: 45.3, 77.0, 48.8, 28.9). Peak RSS unchanged.

testbox is shared: other users' batch jobs fill most of its twelve threads for hours, and a busy SMT
sibling inflated cycles by up to 80% in round four. `bench.sh` and `tools/bench-ab.sh` (two binaries
kept by `KEEP=1 bench.sh`, timed in turn, best of three) wait for a quiet second before each run.

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

## Fourth round

Measured with `tools/bench-ab.sh` in quiet turns:

- The aquifer keeps each candidate's x and z distance terms for the block column it is asked in:
  1-1.5% fewer cycles, 2.4-3.3% fewer instructions.
- Terrain squashes a block column's eight densities together, so the `/ -24` division runs in lanes:
  2.4-3.2%.
- Chunk columns are 24 copy-on-write sections (chunk.rs):
  - A section is shared (Arc) until written, then owned (Box) and written in place.
  - `share` turns a finished chunk's owned sections into shared ones, and all-air ones into the one
    air section.
  - Reads go through a cached pointer per section.
  - Every copy (carved, staged, post-processed, loaded, earlier-phase states, views) shares what it
    does not write.

  Peak RSS halves (origin16 376 -> 190 MB, strong16 596 -> 215) with cycles flat (+0.3 to +0.8%;
  instructions +4 to +7%). `Arc::make_mut` per write cost 80% (an atomic per write), and matching on
  the variant per read 3-7%; the pointer cache and the owned fast path made it flat.
- The AVX2 lanes gather a permutation entry and its successor at once (7 gathers instead of 14):
  1.5-2.7%.
- Scheduling workers over distant areas (`mt1s`/`mt6s`: six origin16-sized areas 64 chunks apart,
  each thread sweeping whole areas):
  - Six threads spent 92 ms of CPU per column against 43 on one thread. The caches are sized for one
    area, so six live areas evict each other's terrain and regenerate it.
  - With every cache limit ×6 (experiment 48bef2a, not on the branch), six threads spent 45 ms per
    column, the single-thread cost.
  - Wall-time scaling could not be measured: other users' jobs held most of testbox's cores, so the
    six threads got 1.5-2.8 cores.

  Recommendation for the server's chunk scheduler:
  - Give each worker its own area, far enough apart that their nine-phase rings do not meet (about 10
    chunks); workers in one sweep wait on each other's sources (round three).
  - Scale the generator's cache limits with the number of areas in flight. That needs a constructor
    parameter in sapling-worldgen, which is not built yet.

## Fifth round

Measured in instructions with `tools/bench-ab.sh QUIET=0`, best of five; testbox stayed busy.

- `OverworldGenerator::with_areas(seed, areas)` sizes every cache for that many areas generated at
  once; `new(seed)` is one area, unchanged. On six origin16 areas 64 chunks apart (`mt1s`, `mt6s`,
  `mt6sc`):

  | run | Ginstructions | Gcycles | ms per column | peak RSS |
  |---|---|---|---|---|
  | one thread | 347.4 | 253 | 43.5 | 304 MB |
  | six threads, caches for one area | 713.3 | 494 | 47.5 | 610 MB |
  | six threads, `with_areas(.., 6)` | 352.8 | 249 | 23.5 | 1193 MB |

  With caches for six, the work per column is back to one thread's. The wall time halves although
  the box gave the six threads about two cores.
- The connection upgrade's "does this chunk hold a fence, pane, bar or stair" walked all 98304 cells
  through `Canvas::get` once per start. It now scans the chunk's sections directly (a shared all-air
  section once): instructions -1.7 / -5.1 / -5.9 / -2.3% (origin16 / strong16 / strong8 / village8).
- The aquifer reads its twelve-candidate memo in place instead of copying it out per open block:
  -0.7 / -1.4 / -1.8 / -1.2%.
- Measured and dropped: terrain adjustments indexed by 16-row band per column (-0.1 to -0.3%).
- What the cache initialisation closures held: decoration's view reads inlined into each source's
  decoration, and the biome palette order (about 0.6% on origin16).
- Stopped here: the next items are each estimated under 1% on every workload.
  - Terrain block loop: lerping the vein columns in lanes, about 0.5%.
  - Aquifer: what is left is the candidate search itself.
  - Palette order: an early exit once every biome is seen, about 0.6% on origin16.
  - Blended recomputes: about 1% of the fill.

## Server scheduling

How a server reached the generator (c37749d): `Generated::new` built one `OverworldGenerator` with caches
for one area; `World::with_storage` started two I/O threads on one FIFO channel. Each tick a shard's
`send_chunks` runs `View::take_ready` per player: the nearest 2 x `columns_per_tick` (16) unsent
columns of its view circle are asked about, and those not loaded are queued once. So every player
keeps up to 16 loads queued, players interleave in the FIFO, and both threads cycle through every
player's area: N players' areas live at once in caches sized for one.

What changed:
- `WorldStorage::load_threads` (default 2) sets a world's I/O threads. `Generated::with_areas(inner,
  seed, n)` answers n and builds `OverworldGenerator::with_areas(seed, n)`. `Generated::new` picks
  half the hardware threads, at most 4.
- The I/O threads take saves and player jobs first, then loads by area (sapling-world load_queue.rs).
  A worker takes the oldest load within 20 chunks of its last one. With none for two ticks, it takes
  the oldest load near no other worker, else the oldest load anywhere (it helps).
  - So at most n areas are generated at once, and a player's loads keep the shard's nearest-first
    order.
  - Without the two-tick wait, a worker whose area ran dry between ticks left it half done. Its cache
    was evicted and later regenerated: views16 cost 1615 G instead of 1236 G, and one player filled
    last.

`tools/bench.sh views<p>[a<n>]` (sapling-world examples/fill_views) has p players 64 chunks apart in
x. Each fills a radius-8 circle (197 columns) as a shard asks, in 50 ms ticks, through a `World`. The
run is unpinned on testbox's 12 threads. Baseline (a696f63) and the branch were run back to back under
the same load (other users' jobs held most cores; QUIET=0):

| run | fill s | spawn s (last) | Ginstr | CPU s | peak RSS MB |
|---|---|---|---|---|---|
| views1, base | 5.4 | 2.2 | 59.2 | 7.8 | 197 |
| views1, 4 areas | 4.4 | 1.9 | 58.0 | 7.7 | 277 |
| views4, base | 158 | 13.7 | 1603 | 272 | 365 |
| views4, 4 areas | 17.4 | 12.1 | 282 | 38.9 | 709 |
| views8, base | 385 | 47.0 | 3838 | 647 | 458 |
| views8, 4 areas | 45.1 | 35.6 | 616 | 89.4 | 1120 |
| views16, 4 areas | 82.7 | 78.3 | 1236 | 179 | 1316 |

- The hash of every block is the same for each workload on every schedule and thread count (2, 4, 8
  and 16 threads, FIFO or by area).
- Reference with one worker and one area's caches, filling the same areas in turn (`views4a1`,
  `views8a1`): 280 and 601 G. The four workers by area cost no more. The areas differ in cost, so
  p x views1 is the wrong reference.
- Steps, in Ginstr for views4 / views8:
  - baseline: 1603 / 3838;
  - areas passed through with the FIFO kept: 285 / 886;
  - by area without the wait: 282 / 707;
  - with the wait: 277-282 / 616.
- Spawns: in the baseline every player crept in together. Now the first n players spawn in seconds
  and later ones wait for a free worker (views8 mean spawn 19.2 s against 36.4). The last spawn comes
  sooner than the baseline's here. In earlier, less loaded runs the FIFO with 4 areas spawned its
  last player at 14 s, against 17 s by area.
- Memory trade-off: the caches are bounded by n areas, not by players. Peak RSS is about 250 MB per
  area above one (views8: 404 MB on one area, 1120 on four; views16 1316, near its plateau). The
  world's own loaded columns, about 50 MB per player view, come on top. Caches for 8 areas
  (`views8a8`, 8 workers) filled views8 in 14 s against 24 s in the same run, with 613 G and 1478 MB.
  More areas buy wall time with memory, not CPU.

## Where the time goes now

`PROFILE`, round four's last commit, self time (strong16 / origin16):

- the terrain block loop 8.7% / 6.7%;
- the aquifer 8.1 / 6.8;
- the blended fill 6.8 / 7.8;
- the AVX2 noise lanes 5.7 / 5.8;
- the feature dispatch with ore veins inlined 4.9 / 7.7;
- the structure stage's reads 4.8 / -;
- terrain adjustments 4.0 / -;
- cache initialisation closures 3.6 / 5.7;
- decoration's view reads - / 3.7.

## Verification (last commit's code, 30a7356)

Run in a second testbox folder (`REMOTE_DIR=work/sapling-worldgen-perf8-verify`) while the bench
folder's lock waited for quiet:

- `tools/replay-all.sh fin5` (the canons of replay-canons.txt and the structure regions of
  replay-worlds.txt through replay-regions.sh): all 59 regions 0 wrong, 0 missing, structure stage 0.
- Stock oracle `tools/oracle.sh v12` and `ENVS=SAPLING_ORACLE_STILL=1 tools/oracle.sh v12-still`: wrong +
  missing 22540 and 23411, as on all2.
- `tools/testbox.sh lib-fin5 test -p sapling-worldgen -p sapling-world -p sapling-data --lib`: all pass.

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
- Per-chunk write overlays were not built: the copy-on-write sections of round four give the memory
  without a layered read path in the view, the canvas and the structure features.
- `chunk.rs` sections with `Arc::make_mut` on every write: +80% cycles, from an atomic per write
  (owned sections are now written in place).

## Left

By expected gain:

Every item left is under about 1% (fifth round). Larger gains would need structural change, not
tuning:

1. Re-measuring the fifth round's cycles, and the server scheduling's wall times, once testbox is quiet.
2. The structure start maps (structure/mod.rs, templated.rs: mineshafts, villages, v2 pieces...) are
   unbounded HashMaps: a long-running server's memory grows with the ground explored.
3. Spawn priority across players: players beyond n wait for a whole area. A shard could pass a
   priority (a load inside a joining player's spawn radius first) without growing the live areas.
