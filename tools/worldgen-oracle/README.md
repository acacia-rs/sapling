# worldgen-oracle

Has a real Bedrock Dedicated Server generate a seed's world and save every chunk of given regions, so
`sapling-worldgen` can be diffed against it block for block and biome for biome. No player joins.

```
tools/worldgen-oracle/run.sh <seed> <out-dir> <x0,z0,x1,z1>...      # overworld chunk coordinates, inclusive
cargo run -p worldgen-oracle --release -- <world> check <x0,z0,x1,z1>...        # every chunk saved and untouched
cargo run -p worldgen-oracle --release -- <world> <block x> <block z>           # a column: blocks, liquids, biome ids by y
cargo run -p worldgen-oracle --release -- <world> diff <other> <x0,z0,x1,z1>... # differing cells, by block pair
```

`run.sh` uses `.testserver/bds-worldgen` of the main checkout (`$WORLDGEN_BDS` overrides; ports
19190/19191), a copy of the test BDS made for this tool. It sets `level-name` (fresh per run),
`level-seed`, `level-type=DEFAULT` and four more `server.properties` keys, installs `pack/` with the
regions, runs BDS until the pack logs `WORLDGEN done`, sends `stop`, and moves the world folder to
`<out-dir>` (must be empty) with an `oracle.txt`: BDS version, seed, regions, seconds. The console log is
`<bds>/worldgen.log`; a failed run leaves its world in `<bds>/worlds/`. One run at a time per BDS copy.
A run takes 20-90 s for 64 chunks and 80-105 s for 1088; BDS startup is 15-55 s of that.

## What a run guarantees

- Every chunk of the regions is saved. The pack loads them in tiles of 8x8 chunks (`$WORLDGEN_TILE`,
  6 at once), each inside a ticking area one chunk wider, waits until every chunk of the area is
  loaded (re-adding an area that stalls), then fills the bedrock floor (y = -64) with stone and back:
  BDS only saves chunks that changed since generation. `run.sh` fails unless every tile reported ok
  and BDS logged `Quit correctly`.
- The touch leaves no trace. y = -64 is all bedrock in a generated overworld chunk; the pack reads the
  floor before, between and after the two fills (all bedrock, all stone, all bedrock) and fails the tile
  otherwise, and `check` re-reads it from the saved world.
- Terrain, surface and biomes are reproducible: two runs, and runs with 8x8 against 3x3 tiles, agree on
  every biome cell and on all blocks but the decoration below.

## Limits

- BDS decoration is not deterministic. Two runs of the same seed and tiling differ in 0.002% (seed
  20261003) to 0.03% (lush caves) of block cells, in features that cross chunk borders and land in an
  order-dependent way: glow lichen, lush-cave clay/moss/dripleaf/vines, moss carpet, grass. Each oracle
  world has a `-run2` twin: a cell where the two disagree is not ground truth (`diff` lists them).
- Loaded chunks tick while a tile is processed. The pack sets `randomtickspeed 0`, `domobspawning`,
  `doweathercycle`, `dofiretick`, `dodaylightcycle` false and clears the weather, but scheduled ticks still
  run: generated water and lava may have flowed (the twins also differ in water/lava/obsidian/stone cells).
- Chunks in the ring around a region may be saved too; only the requested ones are guaranteed.
- Overworld only, default world type, no experiments. A ticking area holds at most 100 chunks, hence
  tiles of at most 8x8.

## Oracle worlds

In the git-ignored `worlds/oracle/`, generated 2026-10-06 by BDS 1.26.52.3 (Windows; the version is in
each `oracle.txt` and the server log, not in `release-notes.txt`):

| Folder | Seed | Regions |
|---|---|---|
| `20261003`, `20261003-run2` | `20261003` | `-16,-16,15,15` and, for float drift, `100000,100000,100007,100007` |
| `<each of the above>-still`, `-still-run2` (2026-10-10) | as above | the same regions, no game-time ticks (below); `SAPLING_ORACLE_STILL=1` makes the oracle test read them |
| `-4172144997902289642`, `-4172144997902289642-run2` | `-4172144997902289642` | `-4,-4,3,3` |
| `20261003-biomes`, `20261003-biomes-run2` (2026-10-07) | `20261003` | 20 regions of 4x4 chunks, one per surface builder or material case, listed in `crates/sapling-worldgen/tests/oracle.rs` (`blocks_match_bds_biomes`); picked with `examples/find_biomes.rs` |
| `20261003-trees`, `20261003-trees-run2` (2026-10-07) | `20261003` | 20 regions of 4x4 chunks in the tree biomes, listed in `blocks_match_bds_trees` |
| `20261003-structures`, `20261003-structures-run2` (2026-10-07) | `20261003` | 16 regions (807 chunks) around one start of each structure kind, BDS `/locate` answers; listed in `blocks_match_bds_structures` |

The `-still` worlds come from a BDS copy patched by `crates/sapling-worldgen/scripts/bds_still.py`: its loaded
chunks never run their tick queues, so the saved blocks are worldgen's alone (no liquid flushed in game, one
bubble column cell per magma block). Make more with `WORLDGEN_BDS=<patched copy> run.sh <seed> <out>
$(regions.sh <test fn>)`; `WORLDGEN_PORT` moves the server off 19190.

Rebuild one with the `run.sh` line above and the seed and regions from this table. The `sapling-worldgen` oracle
test and examples read them from `$SAPLING_ORACLE_WORLDS` (default: `worlds/oracle` of the checkout), so a worktree
can point at the main checkout's copy.

## Running the oracle test

On testbox, which holds the worlds in `~/oracle-worlds`; each script's header lists its env:

```
tools/oracle.sh <name> [test filter]                 # every region's wrong and missing cells; log in target/testbox/<name>.log
tools/oracle-totals.sh <before log> <after log>      # the regions that moved between two runs
tools/remote-example.sh <name> <example> [args...]   # a release example's output
tools/testbox.sh <name> <cargo args...>              # anything else, e.g. `lib test -p sapling-worldgen --lib`
```

A worktree syncs to its own remote folder, and runs on it queue up. The test generates a region's chunks
on every core (`SAPLING_ORACLE_THREADS`) and compares them in order on one, which is what bounds a run:
all six `blocks_match_bds_*` tests took 193 s against 444 s before (2026-10-09, testbox shared with other work).
