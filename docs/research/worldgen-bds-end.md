# BDS End generation (worldgen/end, 2026-10-18)

Build as [worldgen-bds.md](worldgen-bds.md) (`bedrock_server.exe` 6ab54bee, BDS 1.26.52.3). Ported in
`crates/sapling-worldgen/src/end/` (`EndGenerator`), `feature/end.rs` and `structure/end.rs`, `structure/end_city/`;
judged against a one-worker, seed-0, tick-free BDS (below). Ported from asm-level specs (every draw and f32 op in
order), as the Nether was.

## Verdict

Like the Nether, not a 1.18 port: `TheEndGenerator` is Java 1.12's `ChunkGeneratorEnd` (density, `populate`) and
`MapGenEndCity`/`EndCityPieces`, in f32 and on MT19937, with a few Bedrock changes listed per stage.

| Stage | BDS | Ported | Exact against the canon |
|---|---|---|---|
| Density | `prepareHeights` 148243be0 (vt slot 46) over 1482432f0 | `end/terrain.rs` | yes, first comparison |
| Surface | `minecraft:the_end` builder 1487e4fd0: stone to end stone; the density writes none | nothing to do | yes |
| Biomes | one, `the_end` (9), every column | `EndGenerator::chunk_biomes` | yes |
| Structures | `structurePostProcessChunk` 148244bd0 (slot 9): end cities | `structure/end_city` | yes, first comparison |
| Decoration | `decorationPostProcessChunk` 148245110 (slot 10) | `feature/end.rs` | yes (one fix) |

No bedrock, no carver, no pregeneration pass (slot 53 is a stub), no liquids, no freeze pass.

## Generator
Vtable 14a924380 (the Nether's layout: 9 structures, 10 decoration, 14 `loadChunk` 148242a00, 16
`postProcessMobsAt` 148246230, 46 `prepareHeights`, 56 the surface provider's island height). Ctor 1482414a0, built
by the End dimension's factory 14827aa10. One MT on the seed's low 32 bits draws: +0x168 lower limit (16 improved
octaves, `addOctave` flags 0x101), +0x170 upper limit (16), +0x178 main (8), +0x180 the island `SimplexNoise` (3
`nextFloat`, the 256 shuffle; its offsets are never read), +0x188 a 3-octave PerlinSimplex the surface builder
ignores. Java's 10- and 16-octave depth and scale noises are not drawn.

## Density (1482432f0)
3x33x3 cell corners, origin `(2cx, 0, 2cz)`, `LegacyBlendedNoise::fillOctave` as the Nether's (`fill_square_grid`):
main xz 17.1103 (0x4188e1e5) y 4.277575, limits xz 1368.824 (0x44ab1a5e) y 684.412.
- Island height per cell column (i, j): `X = i + 2cx`, `Z = j + 2cz`, `h = sqrtf(Z² + X²)·-8 + 100` clamped to
  -100..80; for k, l in -12..=12 (x outer) with `kx² + lz² > 4096` (i64) and island simplex at (kx, lz) `< -0.9f`:
  `100 - sqrtf(dz² + dx²)·((|lz|·147 + |kx|·3439) mod 13 + 9)` (u32), `dx = i - 2k`, `dz = j - 2l`, clamped, the
  maximum kept.
- Rows 0..31: `t = (main/10 + 1)·0.5`, the blend as the overworld's, `d = (b + -8) + h`; above row 14
  `s = clamp((y-14)/64)`, `s·-3000 + (1-s)·d`; below row 8 `e = (8-y)/7` (unclamped), `e·-30 + (1-e)·d`. Row 32
  is a scalar tail: `(h + (b + -8))·0.71875 + -843.75`, the folded top slide (the "0.72, -843" of the old notes).
- Interpolation z, x, then y (steps 1/8, 1/8, 1/4, the y = 0 step computed as `step·0 + lower`); positive is
  end stone, y 0..127.

## Decoration (148245110)
The large-feature chunk seed on one MT, nothing reseeds it. In order:
1. The feature rules (slot 54) **first**. `the_end` has no rules of its own, but rules without a biome filter
   reach it: `overworld_underwater_magma_feature`'s `Math.Random(18, 30)` spends one draw of this generator (its
   magma never places). Until the rule tables knew biome 9 (`END_IDS`, scripts/bds_packs.py) every small island sat
   one draw early.
2. Spikes (`getSpikesForLevel` 148273530, recomputed per call): MT(level seed), sizes 0..9 shuffled forward
   (`j = u32 % (i+1)`); centres `(int)(Mth.cos(2a)·42)`, `(int)(Mth.sin(2a)·42)` for `a = i·π/10 - π` (truncated:
   (42,0) (33,24) (12,39) (-12,39) (-33,24) (-42,0) (-33,-24) (-12,-39) (12,-39) (33,-24)); radius `size/3 + 2`,
   height `3·size + 76`, cages on sizes 1 and 2. `SpikeFeature::place` 149d63340 runs in the chunk holding the
   spike's **min corner** (x-r, z-r) and writes the whole spike through the 3x3: obsidian within `d² <= r²+1` below
   the height, air elsewhere in the box above y 65 to height+10, then the iron bar cage. No bedrock, no fire.
3. Beyond 64 chunks (`cx² + cz² > 4096`), at the island height of cell (1, 1) (148244960):
   - below -20 and `u32 % 14 == 0`: a small island (`EndIslandFeature` 144c12ac0) at x, y 55..70, z drawn in that
     order, a second at the **same** spot when `u32 & 3 == 0` (Java draws a new one);
   - above 40: `u32 % 5` chorus plants. The x draw comes before the count test, so the draw that fails it is the
     gateway's `% 700`. A plant needs air at the light heightmap (the terrain's: nothing recomputes it while the
     world generates) and end stone under it; `growTreeRecursive` 148b29a60 as Java's (N, E, S, W; flowers age 5).
   - the gateway (1/700): at `heightmap + 3 + u32 % 7`, Java's shape with `infiniburn_bit=1` bedrock, exit
     (100, 50, 0) in its block entity. BDS skips it when a chunk holding (x±1, z±1) is Loaded (state 11); those are
     this chunk and its +x/+z neighbours, and by the scheduler's unblocking rule ([worldgen-bds-order.md](
     worldgen-bds-order.md), a chunk leaves state 8 only once all nine around it are decorated) none can be while
     this one decorates. Not modelled; the canon's one gateway placed.
4. The tick queues (nothing schedules a tick).

`postProcessMobsAt` (slot 16) spawns each spike's crystal and sets bedrock on its top; it never ran on the
player-free oracle (no bedrock on any spike), so `EndGenerator` places neither. The dragon fight (End tick 148276910)
returns unless a player is within 200 blocks of the origin: no exit portal, dragon or main-island gateways in a
fresh End. The (100, 48, 0) obsidian platform is built on arrival, not by worldgen.

## End cities
`EndCityFeature` (vtable 14a926270) is the End's only structure; post-processes seed as the overworld's.
- Grid 20/11, salt 10387313, triangular (`structure::end::GRID`); start test (148313230): `cx² + cz² > 4096`,
  `floor(island height) >= 0` (Java asks > 40), and `getYPosForStructure` (1483134a0) >= 60: the least of one
  above the top block at (7, 7) and three corners 5 off toward the start's turn, on the chunk's raw terrain (Java
  takes the top block itself; cities stand a block higher). The turn is MT(cx + cz·10387313)'s first draw `& 3`.
- Pieces: Java 1.12's generators draw for draw on the grid's MT after its four draws (tower, fat tower, bridge,
  house, `recursiveChildren`, the overlap test on the `u32 >> 1` kind, depth 8, the ship once per city), the boxes
  span size + 1. Placement is clipped per chunk, insert pieces skip air, data markers keep the cell; "Elytra" sets a
  `frame` block (facing 3, 4, 2, 5 by turn); chests draw loot seeds only. No connection upgrade pass.
- Templates: Java `.nbt` files under `structures/endcity`, appended to templates.bin from a probe (scripts/
  template_probe; worldgen-bds-templates.md). Seen there: end rods come out with every horizontal facing reversed
  (Java east to 4, west 5, south 2, north 3); a SkullType 5 skull is a dragon head; a brewing stand takes bottle
  bit a/b/c from Java slot 0/1/2 (the overworld families keep the old slot 1/2/3 reading, which the probe says puts
  the igloo's stand one bit off: unchecked against an overworld oracle, left alone).
- `/locate structure end_city` (region rings, first hit): `EndGenerator::locate_city`, BDS's six answers in
  `end::tests`.

## Oracle and canon
- `WORLDGEN_DIMENSION=the_end` in `tools/worldgen-oracle/run.sh` and `scripts/bds_dbg_run.sh` (README, "The End");
  the pack touches the top layer (y 255, air). Two one-worker seed-0 tick-free runs of the regions: 0 differing cells.
- Buffers: `scripts/data/dbg_end_buffers.json`, the commit call 1482431de, `[rdx]` 16x16x128 `Block*` as the
  Nether's; 2560 chunks dumped, all equal to `EndGenerator::terrain_blocks`.
- Logged run: `scripts/data/dbg_end_canon.json`, with `WORLDGEN_KICK=true WORLDGEN_LOAD_TICKS=1200` (one tile near
  the origin stalls under the debugger with chunks never requested; the kick loads them).
- Canon `~/oracle-canon/end` (testbox): `20261003-end` (the logged run's world), its `-run2` link, `20261003-end-buffers`,
  `order.txt`, `structure-order.txt`. `tools/end-replay.sh <name>` runs `tests/end_oracle.rs`.
- Regions (chunks): main island -8,-8..7,7 (all ten spikes, both cages); end city with a ship 82,-2..87,9 (start
  84,5, `/locate` from 1000,0); outer islands with chorus 60,-4..67,3; void 20,-2..23,1; end gateway 72,-9..76,-5
  (the gateway at 1188,65,-97, found by scanning 4096 outer chunks).

Residue against the canon (`end-replay.sh`), cells:

| region | terrain only (a248609) | + decoration, + cities |
|---|---|---|
| main island | 40645 (the spikes) | 0 |
| end city with a ship | 4042 (city 3124, chorus 918) | 0 |
| outer islands | 782 (islands, chorus) | 0 |
| void | 0 | 0 |
| end gateway | - | 0 |

The terrain stage was 0 in every region from the start; the decoration's only fix was the rules' draw. Three more
cities of the stock repeatable runs (-1192,56 with a ship and 55 pieces, -264,-1224, -216,1016) also match cell for
cell. Seed width: every End MT takes the seed's low 32 bits; a repeatable run of seed -4172144997902289642
(chunks -4,-4..3,3 and 60,-4..67,3) matches cell for cell.

## For the End dimension
- `EndGenerator::new(seed)`, `chunk_blocks`, `chunk_biomes` (as `NetherGenerator`; y 0..255, terrain 0..127),
  `spikes()` (positions, radii, heights, cages: the crystals and their bedrock are not placed), `island_height`,
  `locate_city`.
- Not generation, left to the dimension: the arrival platform at (100, 48, 0), crystals on the spikes (BDS's
  `postProcessMobsAt` with a player around), the dragon fight and its exit portal, the gateway exits' block entities.
