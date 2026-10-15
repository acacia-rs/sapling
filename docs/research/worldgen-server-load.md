# Generating for a server: load scheduling

Branch `worldgen/perf8`. How a server's chunk loads reach sapling-worldgen, and how they are scheduled
over the I/O threads. Generator speed and memory are in worldgen-perf.md.

## Before (c37749d)

`Generated::new` built one `OverworldGenerator` with caches for one area; `World::with_storage` started
two I/O threads on one FIFO channel. Each tick a shard's `send_chunks` runs `View::take_ready` per
player: the nearest 2 x `columns_per_tick` (16) unsent columns of its view circle are asked about, and
those not loaded are queued once. So every player keeps up to 16 loads queued, players interleave in
the FIFO, and both threads cycle through every player's area: N players' areas live at once in caches
sized for one.

## Server scheduling

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
run is unpinned on testbox's 12 threads. Baseline (a696f63) and the branch (eb60507, before spawn
priority) were run back to back under the same load (other users' jobs held most cores; QUIET=0):

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
- Spawns: in the baseline every player crept in together. With areas the first n players spawn in
  seconds and later ones wait for a free worker (views8 mean spawn 19.2 s against 36.4). In earlier,
  less loaded runs the FIFO with 4 areas spawned its last player at 14 s, against 17 s by area.
  Spawn priority (below) addresses this.
- Memory trade-off: the caches are bounded by n areas, not by players. Peak RSS is about 250 MB per
  area above one (views8: 404 MB on one area, 1120 on four; views16 1316, near its plateau). The
  world's own loaded columns, about 50 MB per player view, come on top. Caches for 8 areas
  (`views8a8`, 8 workers) filled views8 in 14 s against 24 s in the same run, with 613 G and 1478 MB.
  More areas buy wall time with memory, not CPU.

## Spawn priority

`World::request_urgent`: sapling-shard's `send_chunks` asks for a joining player's columns within the
spawn radius this way. The I/O threads take urgent loads before any area's, area by area as well
(load_queue.rs). views8 / views16 against no priority (25a45ce), under the same load:
- last spawn 27.9 -> 10.2 s / 42.5 -> 21.0 s;
- mean spawn 13.8 -> 6.5 s / about 24 -> 13 s;
- Ginstr 609 -> 687 / 1231 -> 1595. Every player joins at once here. The spawn rings' surroundings
  are evicted before a worker comes back to fill those views, and are generated again. Players who
  join one at a time do not pay this.

Taking urgent loads in plain order was worse for everyone. All eight rings were generated
interleaved over eight areas, so every player spawned at 14 to 16 s (instructions +14% / +48%).

## Left

- Wall times, once testbox is quiet.
- Spawn priority costs instructions when many players join at once. Caches with room for a few spawn
  rings beyond the n areas would keep their surroundings.
