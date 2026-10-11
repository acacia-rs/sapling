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
  Between chunks something does: a worker that had spent about 32.2k draws at startup (8054 recipe UUIDs,
  141189020, four draws each) stood at 32266 at badlands' first decoration, at 32213 at the birch run's, and
  53 draws further than our count 29 chunks into badlands; in forest runs the gaps exceed a whole state
  (624). The consumer is not that UUID helper (no call after startup) and was not identified.
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

Wrong + missing of 4x4 regions against their own one-worker worlds (16 chunks, 1.57M cells; the temple and
mansion regions are larger), with the phase model, with the run's order alone, and with the thread generator:

| region | phase model | run's order | order + thread draws | glow lichen on BDS's cell |
|---|---|---|---|---|
| pale garden | 1216 | 1216 | 0 | 37 of 37 |
| bamboo jungle | 1044 | 1039 | 25 | 24 of 24 |
| old growth pine taiga | 684 | 680 | 37 | 21 of 21 |
| badlands | 607 | 1080 | 157 | 58 of 58 |
| old growth birch forest | 893 | 893 | 827 | 70 of 71 |
| jungle temple (5x5) | 3010 | 3010 | 2764 | 58 of 58 |

(Mansion was not run: its 9x9 region is four tiles and the logged run did not finish in the session.)

The jungle temple does not come down, and that is a finding: in this run source 78,6 was decorated at rank 23
and 78,8 at rank 138, so 78,6 saw no leaves of 78,8, and BDS still has the jungle tree's log at 1271,72,107
where our 78,6 hangs a vine. That one cell is `replay_check`'s first line; 79,6's 1295 cells follow it.
residue-lush.md's fix for the stock oracle ("78,8 before 78,6", 3104 -> 389) made our walker avoid the cell by
another route; the difference is in `VinesFeature` or what it reads for source 78,6 over bare terrain, and a
one-worker run with a breakpoint on 149d99a70 in that chunk is the next step.

The order alone changes little outside sculk and makes sculk worse (a faithful order with guessed veins); the
thread draws are what moves lush caves, through one lichen cell that a patch or pool then meets. The regions'
"lush cave residue" of the stock oracle was this, not the lush code.

## What the replay found
- **Veins in water.** A sculk vein set into a water source keeps the water in the liquid layer; removed, it
  leaves the source (source 594,-348: a spring at 9513,-10,-5566 that our vein had dried up; BDS's writes there
  are ours one for one, and its water runs 35 blocks down). `Block::into_water` now covers veins and
  `View::set_block_quiet` turns air over a wet vein or lichen back into water.
- **Veins and lichen stop water.** The fall ends on a dry vein at 9514,-21,-5566 and spreads from there; our
  liquid pass washed it away (`liquid/flow.rs`, `blocking`). `_isLiquidBlocking` (148b5ee50) asks the block's
  byte +0xba (2: blocking) before the shape test; the table behind it was not read out, so only the two
  multiface blocks are covered.
- On the stock oracle sets (all2 817f552 -> edge3, wrong / missing; `tools/oracle.sh`, logs `base` and
  `final`, with `ENVS=SAPLING_ORACLE_STILL=1` for the still twins) these two changes are the whole difference:

| world | ticked: wrong | missing | still: wrong | missing |
|---|---|---|---|---|
| near origin | 1486 -> 1342 | 230 -> 220 | 874 -> 716 | 185 -> 176 |
| far out | 260 | 28 | 150 | 20 |
| negative seed | 78 -> 76 | 10 | 80 -> 78 | 46 |
| biome regions | 5396 -> 5256 | 1100 -> 1094 | 5370 -> 5356 | 1173 -> 1145 |
| tree regions | 2651 -> 2643 | 814 -> 812 | 2021 -> 2014 | 571 -> 568 |
| structures | 15480 -> 14816 | 4685 -> 4569 | 16753 -> 16321 | 5106 -> 5013 |
| all 59 regions, wrong + missing | 32218 -> 31126 | | 32349 -> 31603 | |

  By region: pillager outpost -494 / -224, near origin -154 / -167, frozen peaks -163 / -151, trail ruins
  -99 / -100, ancient city -95 / -178, mansion -80 / -11; up: stony peaks +4 / +107, cherry grove +19 / +19,
  badlands +16 / +20 (sculk regions swing by that much on one cell: see residue.md, "sculk2").
- **Left, each the first difference of its run** (badlands, `replay_check`): sculk sensors BDS grows on a floor
  that water has reached (9515,-45,-5567, 9516,-45,-5568, 9520,-45,-5566 with its water kept: sources
  593,-348 and 595,-348 after 594,-348's fall), one vein of source 593,-349 (9507,-34,-5572), then a fall
  our source 595,-349 continues down column 9515,-5568 where BDS leaves it at y -23.

## replay4: the first sources, fixed (2026-10-11, worldgen/replay4)
Each region's first differing source was compared with a one-worker run of its own (new canons `r-*`, logged
with the breakpoints named below). `tools/replay.sh <name> <canon> test|rc|ex` runs the oracle test,
`replay_check` or any example against a canon; `tools/replay-all.sh` every canon of `tools/replay-canons.txt`;
`tools/worldgen-oracle/canon-upload.sh` turns a run into a canon on testbox. `replay_check` also lists
`unwritten` cells: what the run has that no source of ours wrote (a BDS feature we lack, or a structure cell).

Wrong + missing against the one-worker worlds (the `q-*` canons of edge3; the `r-*` reruns agree):

| region | edge3 | replay4 | what is left |
|---|---|---|---|
| pale garden | 0 | 0 | |
| bamboo jungle | 25 | 0 | |
| badlands | 157 | 7 | 13 surface cells of a mineshaft (structure stage) |
| jungle temple | 2764 | 18 | a mineshaft's cells under chunk 81,4 (structure stage) |
| old growth pine taiga | 37 | 37 | one ceiling patch at the region's west edge, below |
| old growth birch forest | 827 | 818 | a mineshaft corridor under the clay pool, below |

- **The chunk top is one above the terrain.** `VinesFeature` walks y 64 up to `LevelChunk` +0xf88, which is the
  heightmap's maximum (one above the top block) until something is set higher, then that block's y. Ours used
  the top block's y for both, so over chunks with nothing above the terrain BDS's walk had one more cell:
  logged per call (149d99d3d, bp = the bound; 149d99d60 a column move, 149d99fe0 a vine with the cell at
  rsp+0x4c), source 78,6's 50 calls differ first at the walk from 1268,118, 72 against our 71, where BDS moves the column
  once more (four draws) and never hangs the vine at 1271,72,107 that kept 79,6's tree from growing. The temple
  falls from 2764 to 22, bamboo jungle from 25 to 0 (its ring source's podzol), the stock temple 3103 -> 357.
  lush2's "78,8 before 78,6" was a coincidence, as edge3 suspected.
- **Sculk growth looks at the type above.** `SculkBlockBehavior` (148b41810) lets a sensor or shrieker grow under
  air or under any block of type water; ours asked for a water source, so floors a fall had reached grew
  nothing. A growth set into a source keeps it (`SculkSensorWet`), into flowing water it does not (saved
  column 9515,-5567: sensor at -45 with no liquid, depth-1 water over it at -44).
- **Sculk wakes no water** and **water blocking is a per-block byte**: worldgen-bds-liquids.md, "Blocking".
  Badlands 157 -> 54 with the growth test, -> 7 with quiet sculk writes.
- **A blob's bubble column is still water.** `partially_exposed_blob` cells beside the one bubble column cell an
  earlier magma block of the same blob started were taken for enclosed (temple source 80,8, try 12: 12 of 14
  writes on BDS's cells, the two extra beside a column). Read off the saved cells, not traced live.
- **Old growth birch is the mineshaft.** The first pool (source -309,285, try 17 at -4944,0,4568) logged cell
  by cell (patch 143bc8310, accepted cell 143bc8468): 71 of 73 columns agree; BDS takes -4944,4574 and
  -4943,4574 with their ground at y 4, where our mineshaft has rails at y 5 and BDS air (`unwritten`:
  `RailEastWest>air` from -4944,5,4574, `Air>rail` at y 6), and skips -4944,4575. One column more or less
  shifts every later draw of the source. So lush2 was right and edge3's "port difference in the pool" wrong;
  the next step is the mineshaft's (worldgen/shaft4), then `tools/replay.sh x r-ob test`.
- **Pine taiga** is unchanged in both of its runs: source -334,341's ceiling patch at -5342,-13,5459 has 50
  cells against BDS's 51; the two that differ (-5346,-12,5459 for our -11, and -5345,-11,5459 missing) lie in
  chunk -335,341, which neither run saved.

Stock oracle, wrong + missing over all 59 regions (all2 0b54505 -> replay4; logs `base`, `m1` and `-still`):
ticked 30977 -> 27006, still 31436 -> 27534. Jungle temple 3103 -> 357 (still 3075 -> 329), stronghold 1919 ->
1047 (1759 -> 887), pillager outpost 1305 -> 1170 (2560 -> 2567), ancient city 8490 -> 8432 (7859 -> 7736),
near origin 1536 -> 1481 (866 -> 805), mangrove swamp -47, bamboo jungle -25; up: badlands 1780 -> 1820
(1873 -> 1910), whose stock worlds are run-order sculk.

## Replaying the structure stage (2026-10-11, worldgen/shaft4)
`scripts/data/dbg_struct.json` logs a run's `loadChunk` and `postProcessStructureFeatures` entries;
`dbg_loads.py <dbg.log> --structure-order <file>` writes them in order (`load x z` and `x z` lines) and
`SAPLING_ORACLE_STRUCTURE_ORDER=<file>` makes the generator run the structure stage that way
(`structured/replay.rs`): chunks post-processed in the run's order, a mineshaft piece reading its neighbours
as they are at that moment, floors past the chunk landing at once, spawner flags kept per piece, and
mineshaft starts visited in the start map's order (below). `tools/replay-world.sh <run dir> <name> <world>`
sends a run to testbox, `tools/replay-oracle.sh <name> <test fn> <region> <box>` prints the oracle totals with
the phase model and with the replay and then `examples/structure_replay_check`: the differing cells grouped by
the chunk and piece that wrote ours last, in the run's order (`NEAR=x,y,z` lists the writes around a cell).
Other kinds still place per chunk from carved terrain under the replay (shipwrecks and portals re-run their
start's placement per chunk, which a moving state breaks); `SAPLING_REPLAY_LIVE_KINDS=1` lets villages,
strongholds, mansions and cities read the state too.

Call for call: `scripts/data/dbg_shaft.json` adds every mineshaft piece's `postProcess` (piece box, chunk box,
the chunk generator's index: the `Random` handed to pieces holds its generator at +8, so the index is at
+0x9d4), each cobweb call and each support read; `examples/structure_replay_draws` prints ours and
`scripts/dbg_shaft_cmp.py <dbg.log> <ours>` names the first piece whose box or draw position differs per chunk.
On the mangrove swamp run all 155 chunks with mineshaft pieces agree piece for piece and draw for draw.

Structure-stage cells against each run's own world (`structure stage:` of the oracle test):

| region | phase model, before | replay, before | phase model, after | replay, after | what is left under replay |
|---|---|---|---|---|---|
| frozen ocean | 206 | 202 | | 0 | |
| swamp | 121 | 72 | | 0 | |
| mangrove swamp | 334 | 245 | | 47 | water that flowed into corridors at decoration (rails washed off): liquid stage |
| badlands (dark oak) | | | 0 | 0 | |
| igloo, snowy plains | | | 57 | 1 | the igloo's waterlogged bars |
| ancient city | | | 312 | 6 | 2 tuff a feature laid over city deepslate, 4 liquid cells |
| stronghold | | | 278 | 67 (61 with live kinds) | below |

"Before" is the branch point; the three small worlds were measured there, the others only after the fix.
What the replay found:
- **A post-process sees 3x3 chunks.** Its `BlockSource` is the view of the chunk and its eight neighbours; a
  corridor is up to 20 long, so a piece met in one chunk has cells two chunks out. There `getBlock` is air (a
  support's test fails, no `nextInt(4)` and torch floats) and `getChunk` is null (the cobweb's interior test
  fails before its float is drawn; floors and rails likewise). We read any chunk. Found from per-piece draw
  positions: BDS drew 4 to 8 fewer per corridor exactly where a section lay two chunks away. This was most of
  the "mineshaft drift": it does not depend on order (stock oracle structure stage 1983 -> 865).
- **Start order.** `foreachIntersectingStructureStart` walks the list of the feature's
  `std::unordered_map<ChunkPos, start>` (MSVC: a new key goes in front of its bucket's entries, or at the list's
  end when the bucket is empty; 8 buckets, times 8 on growth, entries re-placed by the same rule).
  `createBlueprints` fills it as chunks load: every chunk within 8 not scanned before, x outer, tested and
  filed. The hash is FNV-1a over each int's bytes, combined as boost's `hash_combine`
  (`structure/start_map.rs`). So two mineshafts in one chunk run in load order, not in any order of their
  positions: in the mangrove run chunk -504,513 ran start -504,510 before -501,513 and chunk -501,514 ran
  -500,514 before -501,513. Only a replay has the load order; without one we keep descending x, z.
- **Fences.** A mineshaft piece keeps the fences its own post-process placed (they are still the piece's
  unjoined fence block, which `canBeReplaced` compares by state) and replaces a library's railing. Keeping
  every lone fence by state made the stronghold run worse (67 -> 111); keeping none left a cobweb over a
  support's fence in the city run.
- **Village foundation columns are order**: with the state read live, 6 of the 9 cells of the stronghold
  run's village go; 3 of one house stay (not traced).
- Stronghold run, left: 31 cells of water or lava that flowed at decoration, 4 fence joins, 9 village cells,
  and one chunk (-93,-124) where BDS left out a spider corridor (piece 118 of start -89,-124: its webs and
  fence are ours only, in an open cave with a lava spring beside it). Next step: a `dbg_shaft.json` run of
  that region and whether piece 118 has cobweb calls in that chunk (none: `isInInvalidLocation` saw a liquid
  we do not have at structure time).

Stock oracle, worldgen/all2 (with replay4 merged) -> shaft4 merged with it, wrong + missing and structure
stage over all 59 regions (logs `all2`, `fin`, `all2-still`, `fin-still`):

| set | wrong + missing | structure stage | of it mineshaft |
|---|---|---|---|
| ticked | 27006 -> 25104 | 1904 -> 780 | 1831 -> 701 |
| still | 27534 -> 25552 | 2075 -> 903 | 2000 -> 834 |

By region (ticked; still): old growth birch 804 -> 20; 805 -> 21 (its clay pool was read through a mineshaft
that the view fix moved), stronghold -261; -415, igloo snowy plains -243; -253, ancient city -175; -119,
ocean ruins -146; -142, mangrove swamp -118; -101, swamp -52; -53, frozen ocean -38, near origin -33; -35.
One region rose: far out +5 (two mineshafts whose neighbour pairs the phase order gets wrong). Mansion 26,
village 2, ruined portal 2 and temple 2 are unchanged: not measured under replay (their one-worker runs did
not finish in the session).

What a mineshaft chunk's blocks depend on besides the seed: (1) which chunks loaded first, through the start
map's order when two starts meet in a chunk; (2) which of its eight neighbours were post-processed before
it: their pieces' air over a support's span, their floors' planks under its corridors, and whether a spider
corridor's spawner is already placed; nothing beyond the 3x3. A default for (2) was tried: neighbours never
seen as placed is worse than our phase order on the stock oracle (865 -> 1005), so the phase model stays; any
fixed rule is right for about half of BDS's neighbour pairs.

## replay5: feature and liquid stage, region by region (2026-10-11, worldgen/replay5, interim)
A canon now carries its structure order too and may hold several regions: one debugger run over four far-apart
4x4 regions takes about 15 minutes, a third of that BDS's start (`canon-upload.sh <run> <dir> <world>
[<world>...]`, the `dir` field of `tools/replay-canons.txt`; `SAPLING_ORACLE_REGION==<name>` keeps exactly one
region). `examples/slice` replays and takes several boxes; `examples/feature_trace` prints the liquid stage
(`queue`/`route` per scheduled tick, `tick` per run tick, a line per generation).

Wrong + missing against each region's one-worker world (stock oracle beside it for scale):

| region | stock, ticked | replay, branch point | replay, now | what is left |
|---|---|---|---|---|
| stony peaks (two runs: `stony`, `stony2`) | 2113 | 60 | 0 | |
| jagged peaks | 542 | 3 | 0 | |
| savanna plateau | 474 | 221 | 0 | |
| windswept gravelly hills | 328 | 3 | 0 | |
| frozen peaks | 299 | | 0 | |
| cherry grove | 282 | | 0 | |
| savanna | 132 | | 0 | |
| dark forest | 164 | | 0 (6 with the canon's first run) | |
| mangrove swamp | 28 | (47, shaft4's run) | 0 (2 at first) | |
| eroded badlands | 57 | | 0 | |
| wooded badlands | 56 | | 0 (45 at first) | |
| windswept hills | 62 | | 0 | |
| snowy slopes | 29 | | 0 | |
| grove | 19 | | 0 | |
| swamp | 21 | | 0 | |
| badlands | 1813 | 0 | 0 | |
| old growth pine taiga | 573 | 0 | 0 | |
| pale garden | 744 | 0 | 0 | |
| bamboo jungle | 552 | 0 | 0 | |
| old growth birch forest | 20 | 53 (`r-ob`: no structure order) | 0 (`birch2`) | |

(Badlands' 7 and pine taiga's 37 of replay4 were gone at the branch point: all2's merge of shaft4's 3x3 view.)

What the replay found:
- **Feature writes wake no liquid**: worldgen-bds-liquids.md, "Where ticks come from".
- **Lava hardens by a neighbour's update only, and a dried cell flows down once** (stony peaks 31 -> 0):
  worldgen-bds-liquids.md, "How they run".
- **A catalyst needs an opaque solid block below.** `SculkPatchFeature::place` reads the block's byte +0x71
  where we asked `is_solid`; a sculk sensor under the patch origin got a catalyst from us and none from BDS
  (stony peaks -3631,-29,4131, jagged peaks -6448,-12,-6074). A probe read +0x71 for the default block of all
  1477 types: it is the solid-blocking property (`BlockType` +0x130 bit 0x40000) without leaves, glass, ice,
  mangrove roots, farmland, paths, copper grates, pistons, chorus and light blocks, plus honey, jigsaw and
  structure blocks (`Block::is_opaque_solid`). The patch's start test reads the same byte.
- **Lava runs through azaleas, dripleaf, rails, snow layers and coral**: the `isLavaBlocking` overrides read
  per type (worldgen-bds-liquids.md, "Lava"); dark forest, 3 cells.
- **Leaf litter does not lie on leaves.** Three litter cells of ours sat on a leaves block over grass
  (dark forest -9427,87,6563); the bed now asks the block below for support from above
  (`supports(up, Any)`; the kind BDS asks is a guess, leaves refuse every kind).
- **The lava lake belongs to its chunk's decoration.** `decorationPostProcessChunk` places the lake, then the
  monster rooms, then the feature rules, all in the 3x3 as it is. sapling laid every lake over the finished
  terrain apart from the sources, so no feature ever saw one: in wooded badlands BDS's source 299,-556 sets a
  water spring (4784,62,-8887) in the wall of the cavity that chunk 298's lake had opened, and its water turns
  twenty lava cells to obsidian (45 cells). `feature/lake.rs` now runs it as the source's first step on the
  post-process generator the rooms continue; `OverworldGenerator::decorate` is left as the judge's lake-only
  stage.
- **Monster rooms test the material.** A room keeps non-solid blocks in its walls; a mineshaft's rails
  (material 24, not solid) became cobblestone in ours (mangrove swamp, two cells). Webs, torches and ladders
  share the material.
- Mangrove swamp's 47 liquid cells of shaft4 (water into mineshaft corridors) are gone with the liquid rules.
- Savanna plateau's 221 cells (sculk patch try 130 of ring source 563,-391 onwards) went with the two changes
  above; which of them it was is not separated.
- **Sulfur spikes are waterloggable** like pointed dripstone (gravelly hills -2001,-5,-5561, three cells).

## What a server can make canonical
Not BDS's order: it is not a function of the seed and the position, and following it would make a chunk depend
on a chain of same-phase chunks without end. sapling keeps its own canonical order (parity phases, same-phase
sources blind to each other, face and spread choices fixed), which is one of the outcomes BDS's race allows in
kind but equals no particular load. What can be made canonical is the judge: a one-worker, seed-0 world with
its logged order is exact ground truth, so a remaining difference there is a port difference and has a first
source. The stock oracle worlds stay useful for breadth; their run-order and thread-local cells are no longer
an argument for or against a change.

## The four unexplained chunks
(As edge3 left them; bamboo jungle and the birch pool are settled in "replay4" above.) Each was traced in a one-worker run of its region with the replay, not in the stock oracle's cells:
- Pale garden -469,-286: nothing left. The region is block-exact (0 of 1.57M cells).
- Pine taiga -334,339 (clay): order and thread draws. The region falls from 684 to 37; what is left starts at
  source -335,340 (two moss cells of BDS's under our tuff at -5338,-15,5463) and source -334,341 (22 cells).
- Bamboo jungle (source 175,349 in the racy run; there its first differing try was 113 at 2801,74,5598, a
  fern of BDS's in the stalk's cell, with neighbours 174,348 and 175,348 exact and 175,350 already off): the
  region falls from 1044 to 25, and 14 of those are one thing, a port difference in ring source 174,349.
  Its bamboo tries 6 (2784,75,5585) and 3 (2785,75,5587) lay podzol at x 2782..2783 where BDS has dirt and
  grass: the rule's generator is off by then, in cells the 4x4 region does not save. A one-worker run of
  172..175 x 348..350 would show the first try.
- Old growth birch -309,285 (clay pool): a port difference, not mineshaft drift. 827 of 893 cells stay under
  the replay with every lichen placed. BDS has a clay pool around -4930,8,4558 that no source of ours places;
  it first shows under source -308,284's moss patch (try 91 at -4927,11,4557 writes moss where BDS kept clay
  and water), then granite of -310,284 and -309,284 replaces stone that is clay in BDS, and -309,285's own
  pool (351 cells) follows. `replay_check` lists only cells we write, so the source that should have placed
  that pool is not named yet: next step, list BDS's clay in the saved chunks no source of ours wrote.
