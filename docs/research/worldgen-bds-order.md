# What BDS's overworld depends on besides the seed (2026-10-10, worldgen/edge3)

Companion to [worldgen-bds-residue.md](worldgen-bds-residue.md) and worldgen-bds-features.md ("Decoration
order"). Earlier rounds called part of the oracle's residue "run order" or "thread-local floor" without being
able to hold either still. This round did: a BDS with one worker and a fixed thread seed repeats itself, its
decoration order and thread draws can be logged, and sapling can replay both. What is then left is the port's.

## What the output is a function of
Blocks of a generated chunk depend on, in this order of weight (badlands 4x4 tile, cells of 1.57M that differ
between two loads of the same seed and the same requests; all on a tick-free BDS):

| loads compared | cells | what varies |
|---|---|---|
| stock BDS, tile alone, 5 runs (10 pairs) | 1730 to 2594 | everything below |
| stock BDS, tile with five others, 2 runs | 1036 | everything below |
| one worker, entropy seeds, 2 runs | 2086 | the thread generator's seed |
| 8 threads (5 workers), thread seed 0, 2 runs | 1106 | the workers' race alone |
| one worker, thread seed 0: 2 plain runs, 2 under a light debugger | 0 | nothing |
| the same under 24k scheduler breakpoints / 48k draw breakpoints | 1 / 203 | which requests were pending when |

1. **The seed.** Terrain, biomes, structure starts and every feature rule's own generator.
2. **Which chunks are requested, and when.** A ticking area or player asks for chunks; BDS generates a box
   around them (a 4x4 tile with its one-chunk ring loads 21x19 chunks, runs structures in 19x17 and
   decorates 17x15). The scheduler holds the pending {chunk, stage} entries in a hash set and launches them
   sorted by priority; without a player every priority is 0, so the order is the set's iteration order under
   an unstable sort (below). It changes with the box, with what else is pending, and with how far the worker
   had got when the main thread queued the next request.
3. **The workers' race.** With more than one worker (max-threads 8 gives 5), tasks whose 3x3 lock fails go back
   to the queue: 3608 and 5927 decoration entries for 241 decorations in two loads, and 276 of 2426 pairs of
   chunks whose 3x3 overlap ran in the other order. Structure post-processing flips 190 of 1186 neighbour
   pairs, loading 47 of 1478. Neighbouring decorations never flipped: the parity phases held in every log
   (856 of 856 pairs, one worker or five).
4. **Each worker's thread generator.** A `Random` seeded from system entropy on first use (1411889d0), drawn
   by `MultifaceFeature` for its face order and spread direction, and by other code on the same thread.
5. **A retried or re-requested chunk.** A chunk that unloads unsaved is generated again when it is requested
   again: a run whose tile missed the pack's load deadline loaded 398 of 399 chunks twice and decorated 132
   twice, writing into saved neighbours a second time (1107 cells off the same run without the retry).

The oracle's twins agree better (329 cells) than any pair above because they were lucky, not because their
load pattern is special: two stock runs of the tile differ by a thousand cells or more.

## The scheduler
`ChunkSource::_spawnChunkGenerationTasks` (1409ddfc0) moves the pending vector (+0x70) into a hash set of
{ChunkPos, state}, asks `Dimension` +0x3b0 (vt+8, 141877550) for each entry's priority, sorts (140a0cbd0,
`std::sort`) and launches from the back. Logged live at the call's return (1409dea92): 23859 entries over a
whole tile load, every priority 0, for all five waiting states (0 load, 2, 4 structures, 6 decoration, 8).
`1409ddb10` runs it from whichever thread finishes a task while fewer than 2N tasks are in flight, up to 6N
(N workers). A chunk's state moves by compare-and-swap (1409e1fa0): 0 -> 1 loading, 4 -> 5 structures (back
to 2 when its lock fails), 6 -> 7 decoration (back to 6), 8 -> 9. `_checkForUnblockingChunks` (1409e09d0)
queues a neighbour in state 2, 6 or 8 once all nine chunks around it have reached its state. The lock
(1438df8f0) is nine `try_lock`s and holds no order. Nothing in this code is a parity rule, and with one worker
no lock ever fails, yet neighbours still decorate in parity phases: where that comes from was not found.

So there is no same-phase rule to port. The order of same-phase chunks is the iteration order of an
unordered set that every request, requeue and unblocking rewrites, cut into batches by timing. It is repeatable
only when one worker runs alone and the main thread is not slowed.

## A repeatable BDS
```
py -3 -I crates/sapling-worldgen/scripts/bds_still.py <copy>/bedrock_server.exe    # no game-time ticks
py -3 -I crates/sapling-worldgen/scripts/bds_seed0.py <copy>/bedrock_server.exe    # thread generators seeded 0
WORLDGEN_BDS=<copy> WORLDGEN_THREADS=1 tools/worldgen-oracle/run.sh <seed> <out> <x0,z0,x1,z1>
```
`max-threads=1` leaves one worker; `bds_seed0.py` turns the entropy call in `Random::Random()` into nops.
Such a run's world is a function of the seed and the regions (four of four loads identical). Its decoration
order still comes from that run's scheduler, so to replay it the order has to be logged in the same run:
```
WORLDGEN_BDS=<copy> WORLDGEN_THREADS=1 crates/sapling-worldgen/scripts/bds_dbg_run.sh <seed> <out> \
    crates/sapling-worldgen/scripts/data/dbg_canon.json <x0,z0,x1,z1>
py -3 -I crates/sapling-worldgen/scripts/dbg_loads.py <out>/dbg/dbg.log --order <out>/order.txt
```
`dbg_loads.py` prints per stage the chunks, repeats, threads and parity breaks (two logs: the pairs that
flipped), and writes `x z draws` per decorated chunk. A log with a chunk decorated twice is a retried tile
(`WORLDGEN_LOAD_TICKS`, 12000 under the debugger): discard it. The draw breakpoints slow the worker enough to
reorder the tail of a load, so the logged run's world is its own: compare against `<out>/world`, not against
a plain run.

## The thread generator
- `MultifaceFeature::place` (143b83e50) copies its face list (floor 0, ceiling 1, wall 2, 5, 3, 4, pushed by
  the loader's three setters 143be5950/59e0/5a70: sculk vein 0 1 2 5 3 4, glow lichen 1 2 5 3 4) and shuffles
  it forward: for i in 1..n, swap with `raw % (i + 1)`. It tries the origin with that order, then each face's
  neighbour cell in that order; before each neighbour it shuffles a second list (the faces without the
  opposite one) that it then does not use.
- The spread (1465e9360) shuffles six directions the same way, starting from up, down, north, south, west, east
  (1403747f0), and takes the first that `getSpreadFromFaceTowardDirection` accepts.
- Nothing else draws from it while a chunk decorates (17 forest and cave chunks, every draw index contiguous).
  Between chunks something does: a worker that had spent 32216 draws on 8054 recipe UUIDs at startup stood at
  32266 at badlands' first decoration and 53 draws further than our count 29 chunks later; in forest runs the
  gaps exceed a whole state (624). The consumer is not `Random`'s UUID helper (141189020: no call after
  startup) and was not identified.
- So the replay takes the generator's position per chunk from the log: `dbg_canon.json` logs the value of each
  face-shuffle draw (143b84073, eax), `dbg_loads.py` finds each chunk's first one in MT19937(0)'s stream.
  `feature/thread.rs` then draws exactly as BDS does inside the chunk.

## Replaying a run
`SAPLING_ORACLE_ORDER=<order.txt>` makes the oracle test and `feature_trace`, `try_check`, `replay_check`
decorate in that order, every source over all earlier ones; `SAPLING_ORACLE_THREAD_SEED=0` adds the thread
generator. The run's world goes to testbox as `<dir>/<test world>` with a `-run2` link beside it and
`SAPLING_ORACLE_WORLDS=<dir>`. `examples/replay_check <seed> <world> <x0> <z0> <x1> <z1>` lists per source, in
the run's order, the cells it wrote last and how many the run has as another block: the first source listed is
where to look, with `feature_trace` beside a `dbg_sculk.py`/`dbg_calls.py` log of that chunk in the same run.

Wrong + missing against one-worker worlds of single regions (REPLAY_TABLE)

## What the replay found
- **Veins in water.** A sculk vein set into a water source keeps the water in the liquid layer; removed, it
  leaves the source (source 594,-348: a spring at 9513,-10,-5566 that our vein had dried up; BDS's writes there
  are ours one for one, and its water runs 35 blocks down). `Block::into_water` now covers veins and
  `View::set_block_quiet` turns air over a wet vein or lichen back into water.
- **Veins and lichen stop water.** The fall ends on a dry vein at 9514,-21,-5566 and spreads from there; our
  liquid pass washed it away (`liquid/flow.rs`, `blocking`). `_isLiquidBlocking` (148b5ee50) asks the block's
  byte +0xba (2: blocking) before the shape test; the table behind it was not read out, so only the two
  multiface blocks are covered.
- On the ticked and still oracle sets these two took 32218 -> 31126 and 32349 -> 31603 (below).
- **Left, each the first difference of its run** (badlands, `replay_check`): sculk sensors BDS grows on a floor
  that water has reached (9515,-45,-5567, 9516,-45,-5568, 9520,-45,-5566 with its water kept: sources
  593,-348 and 595,-348 after 594,-348's fall), one vein of source 593,-349 (9507,-34,-5572), then a fall
  our source 595,-349 continues down column 9515,-5568 where BDS leaves it at y -23.

## What a server can make canonical
Not BDS's order: it is not a function of the seed and the position, and following it would make a chunk depend
on a chain of same-phase chunks without end. sapling keeps its own canonical order (parity phases, same-phase
sources blind to each other, face and spread choices fixed), which is one of the outcomes BDS's race allows in
kind but equals no particular load. What can be made canonical is the judge: a one-worker, seed-0 world with
its logged order is exact ground truth, so a remaining difference there is a port difference and has a first
source. The stock oracle worlds stay useful for breadth; their run-order and thread-local cells are no longer
an argument for or against a change.

## The four unexplained chunks
All four are lush-cave chunks next to chunks the oracle did not save, and all four differ in other cells in a
one-worker run of their region, so none is a fixed error of the port at that chunk:
- Bamboo jungle 175,349: its bamboo rule first differs at try 113 (2801,74,5598), where BDS has a fern and no
  stalk. The neighbours 174,348 and 175,348 match BDS write for write; 175,350, on the saved box's edge,
  already has 137 stalk cells BDS lacks. The cause lies beyond the saved chunks, as res6 found.
- Old growth birch -309,285, pale garden -469,-286, pine taiga -334,339: the clay pools and moss patches
  differ from the first lush source of their runs on (REPLAY_LUSH).
