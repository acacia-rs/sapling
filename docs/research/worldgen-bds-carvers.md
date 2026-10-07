# BDS carvers (static RE, crates/sapling-worldgen/src/carver)

Companion to [worldgen-bds.md](worldgen-bds.md) (pipeline, RNGs, labels). They run in `loadChunk` after the surface
(steps 11 and 12); lava lakes follow in `decorationPostProcessChunk`.

- Large-feature chunk seed (cave `place` 148281d10, canyon `apply` 14827f840, decoration post-process): MT seeded with
  the low 32 bits of the world seed; `a = nextInt()`, `b = nextInt()`, forced odd as `(v − (v>>31)) | 1` = `v/2*2+1`;
  `seed(x,z) = (z·b' + x·a') ^ levelSeed`. Since `nextInt()` ≥ 0 this equals `|1`. Source chunks x outer, z inner,
  ±8 around the target.
- Vanilla data: only `overworld_extra_cave_carver_feature` (pregeneration_pass, overworld tags) places a carver
  (`underground_extra_cave_carver_feature`: skip 66, height_limit 103). The unreferenced
  `underground_cave_carver_feature` (skip 33, height_limit 236) also runs (oracle: +3k matched cells, no extras);
  where its rule comes from is unmapped. Both see the same chunk seeds (each rule reseeds with the level seed).
- `CaveFeature::addFeature` 148284810: count `nextInt(nextInt(nextInt(40)+1)+1)`, then `nextInt(skip) != 0` drops it;
  per cave z=`nextInt(16)`, y=`getCaveY`, x=`nextInt(16)` (MSVC right-to-left args), then horizontal/vertical radius
  multipliers and floor level (each `min+(max−min)·f` when min≠max); room on `nextInt(4)==0`, then `nextInt(4)` extra
  tunnels; tunnel yaw `2(fπ)`, pitch `2(f−0.5)/8`, thickness, length `112−nextInt(28)`.
  Ctor defaults 1482817d0: +0x70 {0.9, 0.75, 0.92, 0.7}, +0x80 6, +0x84 4, +0x88 40, +0x8c 4, +0x90 4, +0x94 1 (cached).
- 1.18 config 14a925940 (picked unless legacy flags): y `nextInt(h)+8`, thickness `2f+f` ×`(3f·f+1)` 1 in 10, local y
  0-based (offset −64), no y<11 lava. Legacy 14a925970: y `nextInt(nextInt(h)+8)`, thickness `2f+f`, lava below 11.
- `addTunnel` 148283530: child MT from parent `nextInt()`; branch step `u%(len/2)+len/4`; steep 1 in 6 → pitch decay
  0.92 else 0.7; per step 6 raw draws before anything else; pitch/yaw change decay 0.9/0.75, gains 2 and 4, step 0.1;
  branch (thickness > 1) at mid with `0.5u+0.5`, yaw ∓π/2, pitch/3; skip segment 1 in 4. Tunnel segments carry raw
  radii (multipliers unused); rooms (`addRoom` 1482830a0: `1+6f`, own MT: y_scale then length) apply them.
- Cached mode (+0x94): segments per source chunk; per target: `(dz²+dx²) − (len−step)² ≤ (t+2+16)²`, centre within
  `2hr+16` of the chunk middle, then `carveBlocks` 148283e70: y from top down, `dy=((y−1)+0.5−cy)/vr`, `dy > floor`,
  `(dy²+dx²)+dz² < 1`; `canReplaceBlock` 148281400 (below); `Aquifer::computeSubstance` at y−64: barrier
  > 0 keeps the block, else air at/above the fluid level, fluid below. Water-boundary abort only without an aquifer.
- Canyon (`addFeature` 14827faa0, carve vt0 148280880), configs 14a925900/910/920 {y offset, max y, flags, chance}: new
  worlds use 920 {64, 312, any-biome, no giant, flat start, 1/100}: x, y=`nextInt(58)+74` + skip, skip, z, yaw, pitch
  `(f−0.5)/4`, thickness `3f₂+3f₁`, giant float; child seed → two MTs: walk + `rs[0..128)`, fresh one for `rs[128..320)`;
  length `120−u%30`; 9 draws per step; out-of-range step ends the canyon; `dy²/6 + rs[y−2]·(dz²+dx²) < 1`.
  Legacy 910 {64, 120, biome test, giant 1/20 ×2 and y-scale 40, `nextInt(nextInt(40)+8)+84`, 1/150}.
- Block names: the globals compared are `VanillaBlockTypeIds` HashedStrings (0x30 apart, hash first), named from
  their initialiser 1425f41e0 (`movabs rax, hash; mov [id], rax; lea r9, "minecraft:…"`; a few short names have no
  `lea r9` and are placed by neighbours). Static lists are `std::vector<const HashedString*>` filled in the same
  function. Config 14a925940: +0x18 `canReplaceBlock(block, above)`, +0x20 grass test 148281750.
- Cave `canReplaceBlock` 148281400: stone, dirt, coarse dirt, podzol, grass, the 16 *glazed* terracottas, the 4
  sandstones and 4 red sandstones, mycelium, snow layer, packed ice, deepslate, calcite, sand/red/suspicious sand,
  gravel, rooted dirt, tuff, iron and copper ores (both) and raw blocks. Not bedrock, granite, terracotta, mud,
  snow, ice. Legacy config's test is 148281190.
- Canyon test 148280800: list 14ccd27e8 (54, Java's carver replaceables): stone, granite, diorite, andesite, tuff,
  deepslate, calcite, dirt, coarse dirt, mud, podzol, mycelium, grass, rooted dirt, terracotta and the 16 stained,
  sandstones, sands, iron/copper ores and raw blocks, snow, snow layer, packed ice, gravel, sulfur, cinnabar.
- Fix-ups, per replaceable cell (run even when an aquifer barrier keeps it). Cave, before emptying: blocks y+1..y+3
  all in the sand list → y+1 becomes plain `sandstone` (red sand too). After: original grass or mycelium (148281750)
  over dirt/coarse dirt (14ccdb858) → the dirt becomes that block. Canyon: a per-column flag set by any grass block met
  in the ellipsoid (replaceable or not); while set, dirt/coarse dirt below each carved cell becomes grass. No sandstone.
- Oracle (carver cells only; cells next to feature blocks, decorated afterwards, or on noise-cave floors counted apart):
  seed 20261003 near origin 99.53% precision / 99.44% recall, far out 99.61 / 99.41, seed −4172144997902289642 99.86 /
  99.86; no extra cells left (all were bedrock). Residue: liquid flow and aquifer fluid differences (wrong fluid), ore
  blobs over aquifer-barrier cells, ~50–130-cell missed pockets per region (unexplained; eroded badlands has a dome in
  terracotta at y 94–99 no carver of ours reaches). Iceberg cut-outs and geodes in carved cells are filed as later.
- Old growth pine taiga (biomes world, ~−5310 72 5450): the shape and carver misses sit under a structure (barrel,
  campfire, chest, straw bed, grass path); BDS's terrain adaptation, not carvers.
