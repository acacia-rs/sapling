# BDS Nether generation (worldgen/nether, 2026-10-17)

Build as [worldgen-bds.md](worldgen-bds.md) (`bedrock_server.exe` 6ab54bee, BDS 1.26.52.3). Ported in
`crates/sapling-worldgen/src/nether/`; judged against a one-worker, seed-0, tick-free BDS (below).

## Verdict

Not the overworld's 1.18 port. `NetherGenerator` is Java 1.12's `ChunkGeneratorHell` density in f32, BDS's
legacy layer biome stack, and the biome JSONs' surface builders, all on MT19937.

| Stage | BDS | Ported | Exact against the canon |
|---|---|---|---|
| Density | 1481e9e90 (vt slot 46): 5x17x5 cells, `LegacyBlendedNoise::fillOctave` | `nether/terrain.rs` | yes |
| Bedrock | `loadChunk` 1481ec000 | `nether/floor.rs` | yes |
| Biomes | layer stack built by the ctor, `fillBiomes` 148251120 | `nether/biome.rs` | yes |
| Surface | `minecraft:overworld` 145809ef0, `minecraft:capped` 145809370 | `nether/surface.rs`, `capped.rs` | yes |
| Carver | `nether_cave_carver_feature` in `pregeneration_pass`, vtable 14a928470 | `nether/carver.rs` | yes |
| Decoration | `decorationPostProcessChunk` 1481ed1f0 (slot 10): hardcoded legacy features and the biomes' feature rules | no | milestone 2 |
| Structures | `structurePostProcessChunk` 1481ecce0 (slot 9): fortress, bastion, ruined portal | no | milestone 2 |

## Generator
Ctor 1481e6d60, vtable 14a923740 (slot 9 structures, 10 decoration, 14 `loadChunk`, 46 density, 53/54 the
`WorldGenerator` pass helpers). One `Random` (MT) on the seed's low 32 bits draws, in order (fields):

| field | noise | octaves | used by |
|---|---|---|---|
| +0x168 | improved (`addOctave` 145e29ee0, flags 0x101 as the overworld's) | 16 | density, lower limit |
| +0x170 | improved | 16 | density, upper limit |
| +0x178 | improved | 8 | density, main |
| +0x180 | improved | 4 | nothing ported (unknown) |
| +0x198 | PerlinSimplex (145e2a730, flag 0: offsets drawn z, y, x) | 4 | surface depth |
| +0x188 | improved | 10 | Java's scale noise: filled, never used |
| +0x190 | improved | 16 | Java's depth noise: filled, never used |
| +0x1a0 | PerlinSimplex | 3 | surface material adjustments |

## Density (1481e9e90)
- `fillOctave(origin (4cx, 0, 4cz), 5 x 17 x 5)`, scales (xz, y): main 8.55515 (0x4108e1e5), 34.220596
  (0x4208e1e4); limits 684.412 (0x442b1a5e), 2053.2358 (0x450053c6). Octave k at scale·2^-k, divided by 2^-k
  (`noise/blended.rs` `fill_grid`, the overworld's code generic over the row count).
- Blend as the overworld: `t = (main/10 + 1)·0.5`, `low = min/512`, `(high-low)·t + low` clamped by t.
- Minus the row offset `2·cos(6π·y/17)` through the Mth table (index `(y·π·6/17)·K + 16384`), less
  `10·(4-t)³` for `t = min(y, 16-y) < 4`. Rows 14, 15, 16: `v·0x3f2aaaaa + 0xc0555556`, `v·0x3eaaaaaa +
  0xc0d55556`, `-10`.
- Interpolation z, x, then y in BDS's order (`(b-a)·t + a`, t = k/4, k/8); positive → **stone**, else y < 32
  → **water**, else air. The surface builders make them netherrack and lava.

## Bedrock (loadChunk)
Thread MT seeded `cx·0x9939f508 + cz·0xf1565bd5` (u32). Every column (x outer, z inner) `nextInt(4)+2` floor
layers from y 0, then every column `nextInt(5)+1` ceiling layers down from y 127. The surface continues on it.

## Biomes (layers, 2D)
Built at the end of the ctor; `fillBiomes` asks the zoom layer for the chunk's 16x16 and sets each column whole.
- Noise layers (vtable 14a9246a0, init 14824a450, fill 14824a9a0): layer i (temperature, humidity, altitude,
  weirdness) seeds an MT with `seed32 + i` and builds two 2-octave `PerlinNoise` (145e27060: `addOctave`, norm
  1/Σ(1/f)). At quart (x, z): `(b·norm + a·norm)·v`, a at `(x/64, 0, z/64)`, b at `(x·1.0181268/64, 0, …)`,
  `v = 0.16666667 / ((1/(2+1) + 1)·0.1)`. Sampling 145e27220 = `ImprovedNoise` per octave, `/f` summed.
- Multi-noise layer (14824b460): biomes with `minecraft:multinoise_generation_rules`, registry (id) order hell
  8, soulsand valley 178, crimson 179, warped 180, basalt deltas 181; distance `dt²+dh²+da²+dw²+weight²` summed
  in that order, strict `<`, hell the default.
- Voronoi zoom (14824b970/14824ff80, layer salt -0x794549289d90b428): Java 1.14's, in f32. Layer world seed
  `s = s·(s·M+A) + salt` three times from the world seed (Java's LCG constants); block (X, Z) reads the quart
  cell of (X-2, Z-2) and its +x, +z, +xz neighbours, corner jitter `((nextInt(1024)/1024) - 0.5)·3.6` (+4 for
  the far corners) from `initChunkSeed(4qx, 4qz)`, nearest corner by Java's strict comparisons.

## Surface (loadChunk's column loop)
Per column (x outer, z inner) the builder of the column biome, parameters (1481ec6aa..): the chunk MT, the
+0x1a0 adjustment noise, a **fixed** sea level 32 (+0x10 = 1; the overworld tracks its sea), the column start 127
(no heightmap), lower bound 0, the depth noise `Σ simplex(x·f·0.0625 + xo, z·f·0.0625 + zo)·0.55/f`.
- Default builder (hell, crimson, warped): the overworld's (`surface/overworld.rs`, `Sea::Fixed`), y 127 down
  to 1, one depth draw per column, replaceable = stone family. Adjustments: kept per column by the noise at
  (x, 0, z) (`min <= v < max`, scale 1 when absent), applied **per y** within their `height_range` (Molang
  with `variable.sea_level`, `roundf`, clamped to 0..127; absent = the whole column): the base is the raw
  materials with the ranges holding y 127 applied. Hell: gravel / soul sand at y 29..34. The floating-sand fix is
  off under a netherrack foundation.
- Capped (soulsand valley, basalt deltas; `CappedSurfaceBuilder`, not Java's): two depth draws (floor, then
  ceiling). Floor and ceiling material: the highest of one single-octave noise per material (seeds
  `seed32 + i`, floors then ceilings) at `(x/16, 32, z/16)`, first on ties, no sampling for a lone material.
  Beach: the first floor noise at `(x/32, 109, z/32)` > 0 (soul sand valley: gravel at y 27..32 of floors).
  Walking y 0..127 up: solid → open at y fills the solid cells below `y-1` downward for `d_floor`; open → solid
  fills `d_ceil` cells upward from y; then stone → foundation, water → sea. Water and lava are open, bedrock solid.

## Carver (`nether_cave_carver_feature`)
Factory 14837c610, vtable 14a928470 (`place` 149d577e0, `addFeature` 149d589b0, `addRoom` 149d57a90,
`addTunnel` 149d57b70). Java 1.12's `MapGenCavesHell` in f32: the overworld's large-feature seeds, sources ±8,
count `nextInt(nextInt(nextInt(10)+1)+1)`, skip `nextInt(5) != 0` drawn always, origin z, y `nextInt(128)`, x;
room `nextInt(4) == 0` (thickness `6f+1`, y scale 0.5, a one-ellipsoid tunnel) then `nextInt(4)+1` tunnels;
yaw `2fπ`, pitch `(2(f-0.5))/8`, thickness `2((f+f)+f)`, y scale 0.5; `height_limit`/`y_scale` JSON fields
unread. Tunnels walk per target chunk (no segment cache) with the Mth table: a step beyond the target's reach
ends the tunnel, a box with lava within two cells is skipped, cells (y 2..120) of netherrack, dirt, coarse
dirt or grass become air; no aquifer, no fix-ups.
The Mth table is the UCRT's `sinf`, read live: `mth.rs` carries the 85 entries that differ from the rounded sine.

## Oracle and canon
- `WORLDGEN_DIMENSION=nether` in `tools/worldgen-oracle/run.sh` and `scripts/bds_dbg_run.sh` (README, "The
  Nether"). 225 chunks in 15-60 s; two one-worker seed-0 tick-free runs: 0 differing cells.
- Buffers: `scripts/data/dbg_nether_buffers.json` breaks on the commit call 1481eca38 and dumps `[rdx]`
  (16x16x128 `Block*`), `dbg_block_ptrs.py` names the pointers. Every buffer cell held a default state.
- Canon `~/oracle-canon/nether` (testbox): the world `20261003-nether`, its twin, `20261003-nether-buffers`
  (2450 chunks: the regions and BDS's generation ring). `tools/nether-replay.sh` runs `tests/nether_oracle.rs`.
- Regions (chunks), picked with `/locate` (`execute in nether positioned … run locate biome minecraft:<id>`;
  plain biome ids are a syntax error): nether wastes and basalt deltas -4,-4..3,3; warped forest -24,0..-21,3;
  crimson forest -24,-24..-21,-21; soul sand valley -8,-28..-5,-25; fortress 12,-1..19,6 (start 240, 32);
  bastion -24,12..-18,18 (start -328, 248).
- Second canon `~/oracle-canon/nether-gaps` (2026-10-21, same recipe, `nether_gaps_match_bds`): ruined portal west
  -18,-2..-15,1, ruined portal north-west -21,-17..-19,-15, soul sand valley fossils -8,8..-4,11; the `-run2` twin is
  the buffers run's world. Judge with `tools/nether-replay.sh <name> nether-gaps`.

Residue against the canon (`nether-replay.sh`, 2026-10-17), cells:

| region | shape, bedrock, surface, carvers, biomes | later: structures | later: features |
|---|---|---|---|
| nether wastes and basalt deltas | 0 | 0 | 59826 |
| warped forest | 0 | 0 | 19638 |
| crimson forest | 0 | 0 | 23538 |
| soul sand valley | 0 | 0 | 17294 |
| fortress | 0 | 35270 | 81572 |
| bastion | 0 | 17045 | 73631 |

With the structure stage (2026-10-17, worldgen/nether-bastion) every region is 0 but one fortress feature cell.

Every loaded buffer of the run (2450 chunks) equals ours; the "later" cells are what BDS set after `loadChunk`.

## Milestone 2
- Decoration (1481ed1f0): Java 1.12 hell decoration on the decoration MT (lava springs `nextInt(120)`-style
  draws, fire, glowstone, mushrooms via 149d57200, quartz/gold/debris/magma/blackstone/gravel ores), then the
  biome feature rules (crimson/warped fungi and vines, basalt pillars and deltas, soul sand valley fossils and
  underground soul sand); the liquid pass (lava springs flow at decoration). The canon needs a logged run
  (`dbg_canon.json`'s overworld VAs replaced by the Nether's decoration and lock entries) for its order.
- Structures (1481ecce0): fortress (legacy pieces) and bastion (jigsaw), ruined portals; grid salts and
  spacing ([worldgen-prior-art.md](worldgen-prior-art.md): 480-block regions, 1/3 fortress odds, unverified).

## Bastions and ruined portals
`structure/bastion` and `ruined_portal::place_nether`, run by `structure/nether.rs` after the fortress.
- Bastion (148311de0 -> 141edf3d0): rotation, start pool (`nextInt(4)` of bridge, treasure, hoglin stable,
  units), element, then the ancient city's queued placer (depth 15, 80) with an empty anchor, which skips the
  start shuffle; start piece at y 32; netherrack plugs down to y 0. Data: `jigsaw/pools_bastion.txt`
  (`scripts/jigsaw_extract/jbastion.py`), templates in templates.bin (bastion appended, 54 states probed).
- One position test (`bastion_treasure_room_highrampart`, axis Y, 0..0.05 over 0..100 from the start
  piece's origin): one `nextFloat` once input and location pass.
- The mob pieces are Bedrock `.mcstructure`s whose air BDS does not place; their jigsaw's final state
  `minecraft:blackstone_bricks` names no block (air). `small_stables/inner_4`, `outer_4` are BDS names
  without templates (no blocks).
- Nether portal: `y = nextInt(85) + 15`, air kept below 32, heights under the roof (140a08bb0 walks down
  through the solid roof first), no mossiness, the blackstone sweep; aged walls keep their default state;
  placed only in its start chunk's post-process.
- Judged: canon bastion region 0 wrong after decoration; the two portals at chunks -16,0 and -20,-16 are 0
  wrong in the second canon (below).

## The code-registered feature rules
Ported in `feature/basalt.rs` (basalt columns 14827c230, pillar 14827ca10, delta 148294fc0), `fungus.rs`
(huge fungus 143b7ded0), `nether_vines.rs` (weeping 14829d8a0, twisting 149d66ee0, vine column 1425e9460) and
`nether_fossil.rs` (149d5bcb0). With them the four non-structure regions are exact (features and liquids 0).
- Read as the binary says, then judged: the delta's cells sit at the search's air y (it looked like it would
  only fill pockets; it matches). Twisting tries centre 5 blocks to −x/−z of the origin.
- The fungus's `canBeBuiltOver` is false for vines: a hat skips another cluster's vine cell and draws nothing for
  it (one wrong cell there desyncs the rest of the hat).
- Every vine cell is the vine block with its own drawn age; BDS has no `_plant` block in these.
- The Nether plants' rule (1465ec270, 14715cd80): nylium, soul soil or the soil list 0x14ccd3328.
- The fossil leaves its templates' air out (they hold more air than bone; placing it dug holes in soul soil
  that lava then filled). Judged 0 in the second canon's fossil region, dried ghasts included.
- The ghast's gate (149d5c3bb): `140247750` is `SemVersion` less-than (major, minor, patch as u16, then the
  pre-release strings; byte +7 means "any"). The ghast is skipped when the level's base game version is below a
  static version, so a world made by this BDS always passes; an old world upgraded in place would not.

Seed widths, checked on seed -4172144997902289642 (two 4x4 regions, 974 chunk buffers and 4096 biome columns, all
equal): every MT (density, noise layers, capped noises, carver, bedrock) takes the low 32 bits; the layers' LCG
world seed takes all 64.
