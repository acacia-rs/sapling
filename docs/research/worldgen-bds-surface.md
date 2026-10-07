# BDS floor, deepslate, veins, surface and biome tree (static RE, crates/sapling-worldgen)

Companion to [worldgen-bds.md](worldgen-bds.md) (pipeline, RNGs, labels): `loadChunk` steps 6 to 10.

### Bedrock floor (loadChunk 1482117a4..148211860)
`for x in 0..16 { for z in 0..16 { n = rand.nextInt(4); fill y -64 ..= -64+n+1 } }` (2 to 5 layers) with
`bedrock[infiniburn_bit=0]`, on the chunk MT of step 3 (seed `cx*0x14609048 + cz*0x7EBE2D5`, nothing drawn from it
before). The surface builders continue on the same MT. Confirmed 100% on all oracle worlds.

### Deepslate and ore veins (`_prepareHeights` 14822fc96..1482300af)
Solid blocks only (density > 0 after the aquifer barrier). Base: y < 0 deepslate; 0 <= y <= 8 deepslate unless
`at(x,y,z).nextFloat() >= (y/8 > 1 ? 0 : 1 - y/8)`; else stone. Positional random: `fromHashOf(
"minecraft:depth_based_layer").forkPositional()` off the world factory, `Mth.getSeed` with `sar`.
Veins: Java 1.18.1 `OreVeinifier` exactly (Java's constants), positional `fromHashOf("minecraft:ore")
.forkPositional()`. Noises `ore_veininess` -8 [1] at (x,y,z)*1.5, `ore_vein_a/b` -7 [1] at *4 (cell corners,
clamped to [-1,1], interpolated like the density), `ore_gap` -5 [1] at the block. Vein table (OreVeinifier ctor
1481f0380): copper (veininess > 0) y 0..50 `copper_ore`/`raw_copper_block`/`granite`; iron y -60..-8
`deepslate_iron_ore`/`raw_iron_block`/`tuff`. The interpolator buffers default to 1.0 (noodle 14822ed79, veins
1481f0380), so corners below -150 read 1.0, not 0. Vein granite inside the surface run is replaced by the
foundation (see the overworld builder), which is why copper filler vanishes under shallow sea floors.

### Surface (`buildChunkSurface` 14580d280)
- Per column, x outer, z inner. Biome: `LevelChunk::getBiome` at the heightmap (one above the highest non-air
  block, fluids included) (`findBiomesAndPrepareChunkParams` 14580c660). Lower bound: `getPreliminarySurfaceLevel`
  of the quart column (4x4 cache 144cd2f80) minus 8, clamped to [-64, 255]; builders work from the heightmap down.
- Depth noise: `minecraft:surface` PerlinSimplex, 4 octaves (Xoroshiro, offsets `nextFloat*256` x,y,z in order),
  `sum simplex2(x*k/8 + xo, z*k/8 + zo) * 0.55/k`, k = 1, 1/2, 1/4, 1/8 (loadChunk 148211560). Simplex 2D/3D are
  Java's in f32 (145e27a70/145e27d50) except `floor(v) = (int)v - (v <= 0)`.
- Material adjustments (143907ca0): noise = generator +0xd50, 3 octaves of 3D simplex from an MT on the low 32
  seed bits after 20202 raw draws (offsets drawn z,y,x, unused), `sum simplex3(p*s*k)/k * 1/7`, k = 1, 1/2, 1/4,
  p = (x, 0, z). In range `[min, max)` replaces the listed materials; later adjustments win.
- Overworld builder (145809ef0) = Java 1.17 `DefaultSurfaceBuilder` with: depth `(int)(mt.nextFloat()*0.25 +
  noise/3 + 3)`; sea level = 1 + top of the last water/lava run above, reset to 0x800003e8 by air (dry columns
  never see "underwater"); replaceable = stone, granite, diorite, andesite (+ polished); first replaceable:
  y in [sea-4, sea+1] takes top/mid, y < sea-1 takes mid, or sea floor (then foundation) when
  `y == lower+1 || y < sea - (sea_floor_depth + depth)`; after the run every replaceable becomes the foundation;
  a run ending on sand/red sand/suspicious sand continues as sandstone for `nextInt(4) + max(0, y - sea)`;
  air/water/lava under a run turns sand/red sand above it to sandstone/red sandstone, gravel to the foundation;
  water becomes the sea material. Steep (mountain_parameters): north = `h(z+1) - h(z-1) > 3`, south the reverse,
  east = `h(x-1) - h(x+1) > 3`, west the reverse (h = top non-air, neighbours clamped to the chunk); replaces
  top and mid at every block.
- Mesa (1487dcb80, init 1487dd550; 1487db280 is an older variant): Java 1.17 badlands with 192 bands from
  MT(seed32) (`generateBands` with nextInt(10)+6 runs, white `nextInt(7)+9` times), band offset
  `round(clay_bands_offset(x,0,z)*4)`, NormalNoises `clay_bands_offset` -8 [1], `badlands_surface` -6 [1,1,1],
  `badlands_pillar` -2 [1;4], `badlands_pillar_roof` -8 [1] from `Xoroshiro(seed32 zero-extended).fork`.
  Fixed sea 63; skips columns whose top is fluid; only `stone` counts; plain/orange/band split at y 63..255.
- Frozen ocean (1487e4350, init 1487e18e0): Java 1.17 icebergs with `iceberg_pillar` -6 [1;4] (x1.28),
  `iceberg_surface` -6 [1;3], `iceberg_pillar_roof` -3 [1] (x1.17) from `Xoroshiro(seed32 sign-extended).fork`,
  positional `at(x,0,z)` for the snow/ice draws. Only where `min(|surface*8.25|, pillar*15) > 1.8` does it call
  the overworld builder; other frozen-ocean columns keep bare stone and draw nothing from the chunk MT.
- Swamp (1487e4d10, init 1487e4870): 2 simplex octaves from MT(seed32), `sum simplex2(x/4*k, z/4*k)/k`; if > 0 and
  the top is below y 63, the topmost non-water block down to `63 - max_puddle_depth` becomes water; then overworld.
- Biome parameters come from the vanilla behaviour packs (`behavior_packs/vanilla*/__brarchive/biomes.brarchive`,
  newest pack wins), transcribed by `crates/sapling-worldgen/scripts/surface_to_rust.py` (the BDS folder is not
  in the repo, so no build-time read).

### Biome tree (`OverworldBiomeBuilder::addBiomes` 148230a70, `RTree::create` 1480610f0)
- Entries: Java's builder order and slices (mid 1481f9850, high 1482009c0, peaks 148206880, low 14820bae0,
  valleys inline), Bedrock ids in u16 tables on the stack, version-gated: 1.21.60+ plateau table (pale garden in
  `PLATEAU[2][4]`), swamp/mangrove split in valleys; `MIDDLE_VARIANT[1][0]` = 195 (dappled forest) resolves to
  nothing without its experiment. Underground: dripstone, lush, sulfur caves (c [-0.19, 0.55], e [0.45, 1.0],
  d [0.2, 0.9], w [-1.1, -0.85]), deep dark. 7594 entries = cubiomes btree262's leaves.
- Ranges quantize through f64 (`Climate::quantizeCoord` 14805fad0): -0.45f is -4499, Java -4500.
- Build (148060510): Java's, but sorts with MSVC `std::sort` (148089fd0: introsort, insertion <= 32, ninther
  > 40); comparator chains (min+max)/2 of params p..p+6 (abs when sorting buckets); < 7 children sort by
  `sum (min+max)/2` (no abs, unlike Java); bucket size `pow(6, floorf(logf(n - 0.01f)/1.791759f))`.
  Search (1480616f0): depth-first, descend while `distance < best`, ties keep the earlier leaf. With this tree
  biomes match every oracle block.
