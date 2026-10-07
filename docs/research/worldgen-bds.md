# BDS world generation (static RE)

Build: `bds-molang/bedrock_server.exe`, PE stamp `6ab54bee` (bds-blocks is the same build). Tool: bdsre (scratch copy),
LeviLamina headers (`src/mc/world/level/levelgen/**`) for class layouts. VAs are absolute (image base 0x140000000).

## Verdict

The overworld terrain is a **C++ port of Java 1.18.0/1.18.1's `NoiseSampler` era** (hardcoded noises + `TerrainShaper`
splines + Cavifier/NoodleCavifier/OreVeinifier/Aquifer), **not** the data-driven 1.18.2+ noise router. Same noise
names, same octave/amplitude tables, same Xoroshiro128++ seeding by MD5 of the name. Two systematic deviations:
- **All noise math is `float`** (`mulss/addss`, `floorf`), Java uses `double`. No Java `wrap()` (3.3554432e7) either.
  Expect near-identical terrain near origin, drifting at large |x|,|z|; not bit-exact with Java.
- Everything outside the terrain-shape pipeline (bedrock floor, legacy lakes/dungeons/canyons, decoration seeding,
  feature rules) uses **Bedrock's MT19937 `Random`** with 32-bit seeds, not Java's LCG.

| Stage | Source | Java parity | Conf. |
|---|---|---|---|
| Climate noises (temp, veg, cont., erosion, ridge, offset, jagged) | hardcoded, `OverworldNoises3d::make` | identical params/seeding, float math | high |
| Terrain spline (offset/factor/jaggedness) | hardcoded `TerrainShaper` | Java 1.18.1 `TerrainShaper.overworld` port | high |
| Base 3D density (`LegacyBlendedNoise` "minecraft:terrain") | hardcoded | Java BlendedNoise, 684.412/80/160 | high |
| Caves (cheese, spaghetti 2D/3D, entrances, pillars, noodles) | hardcoded `Cavifier`/`NoodleCavifier` | Java 1.18.1 NoiseSampler names/structure | med-high |
| Aquifers | hardcoded `Aquifer` | Java 1.18.1 NoiseBasedAquifer (13 surface sample offsets) | medium |
| Ore veins | in `_prepareHeights` | Java 1.18.1 OreVeinifier, ported and matched | high |
| Deepslate transition | `DepthBasedBlockSupplier` "minecraft:depth_based_layer" | Java's vertical-gradient rule, own name; ported and matched | high |
| Biomes | hardcoded `OverworldBiomeBuilder` + `RTree` + `BlendedMultiNoiseBiomeProvider`, 3D (`BiomeSource3d`) | Java OverworldBiomeBuilder port + Bedrock extras (sulfur caves, 1.21.60 plateau variant) | medium |
| Surface | **Bedrock** data-driven: biome JSON `minecraft:surface_builder` → `BiomeSurfaceSystem` / `VanillaSurfaceBuilders` | **not** Java SurfaceRules; uses PerlinSimplex surface noise + material-adjustment noise | high |
| Bedrock floor | hardcoded, MT seeded by chunk pos only (no world seed) | **Bedrock-only** | high |
| Carvers | 2 `cave_carver_feature`s in `pregeneration_pass` + hardcoded canyon | legacy pre-1.18 style, **not** Java carvers; ported, oracle 99.5% precision/recall | high |
| Lakes / monster rooms / freeze | hardcoded in `decorationPostProcessChunk` | Java-legacy shapes, MT RNG; lava lakes ported | high (lakes) |
| Features | data-driven `feature_rules` + `features` JSON, 12 named passes, MT per rule | **Bedrock-only** | high |
| Structures | legacy `StructureFeature` + jigsaw v2 (`br::worldgen`, `worldgen/structures` JSON) | separate system | low (not mapped) |

## Class hierarchy (LeviLamina headers + vtables)

```
ChunkSource
└─ WorldGenerator : ChunkSource, IPreliminarySurfaceProvider          (levelgen/WorldGenerator.h)
   ├─ OverworldGenerator (abstract)            vtable 14a923a80 (65 slots, pure = 14a5bc9f0)
   │  ├─ OverworldGenerator2d                  vtable 14a923f60, ctor 1482180e0  (legacy 2D worlds)
   │  └─ OverworldGeneratorMultinoise          vtable 14a924170, ctor 14821f6f0  (current overworld)
   ├─ NetherGenerator                          vtable 14a923740, ctor 1481e6d60
   ├─ TheEndGenerator                          vtable 14a924380 (stored by 1482414a0 / 148242770)
   ├─ FlatWorldGenerator, VoidGenerator        (headers only; not mapped)
   └─ DensityFunctionWorldGenerator            (headers only; Bedrock's own data-driven density system, see below)
```
RTTI for game classes is stripped; names come from `std::function` lambda RTTI (`loadChunk@OverworldGenerator`,
`InstancedThreadLocal<ThreadData@{Overworld,Nether,TheEnd}Generator>`, `resolveBuilderImplementation@BiomeSurfaceSystem`,
`setBeardifier/setBeardAndShaver/setBury@FeatureTerrainAdjustments`, `BlendingDataFactory<BlendingBiomeProvider{2D,3D}>`),
source paths (`levelgen/v1/ChunkBlenderFactory.cpp`, `biome/BiomeSource3d.cpp`, `levelgen/v2/StructureInstance.cpp`)
and the LeviLamina headers.

`OverworldGeneratorMultinoise::Noises` (header): `LegacyBlendedNoise mTerrainNoise`, `CavifierNoises` (16 NormalNoise),
`AquiferNoises` (4 + positional factory), `NoodleCavifierNoises` (4), `OreVeinifierNoises` (4 + factory),
`PerlinSimplexNoise mBlendNoise`, `PerlinSimplexNoise mSurfaceNoise`, `OverworldNoises3d mCommonNoise` (7 NormalNoise +
`TerrainShaper`), `DepthBasedBlockSupplier mDeepslateSupplier`. Plus `mBiomeSource`, `mCavifier`, `mMaterialAdjNoise`,
`mChunkBlender(Factory)` (old-chunk blending, Java `Blender` analogue).
NormalNoise type: `NormalNoiseImpl<0, MultiOctaveNoiseImpl<0, ParityImprovedNoiseImpl<0>>>`.

Density grid: `MultidimensionalArray<float,5,5,41>` (4-wide xz cells, 8-tall y cells → 320 blocks);
`ThreadData::blockBuffer` = 16×16×320 `Block*`; generation column is y −64..255 (`0x140` passed as height).

## RNGs

| RNG | Evidence | Used for |
|---|---|---|
| Xoroshiro128++ | `0x9E3779B97F4A7C15`, `0x6A09E667F3BCC909`, Stafford mix13 `0xBF58476D1CE4E5B9`/`0x94D049BB133111EB`, `rol 17/49, shl 21` (14821fa49) | world seed → `upgradeSeedTo128bit` → `forkPositional`; every named noise; aquifer/ore positional randoms; nextFloat = bits24·2⁻²⁴ (5.960464e-08 in `_prepareHeights`) |
| MD5 | TLS hasher vtable (+0x228 update/final), result bytes byte-swapped to two BE u64, XOR seedLo/seedHi (1481d9b10) | Java `fromHashOf(name)`; zero state → golden/silver fallback (Java Xoroshiro ctor) |
| `"octave_"+i` | inline imm `0x5f657661` ("ave_") in 145e24e60 | per-octave `fromHashOf`, as Java PerlinNoise |
| MT19937 (`Random`, 2560 B) | init `0x6C078965`, `0x9908B0DF`, `0x9D2C5680`; lazy init, standard outputs; `nextInt(n)` = `u32 % n` (0 for n ≤ 0, 141188e50), `nextInt()` = `u32 >> 1`, `nextFloat` = `(float)(u32·2⁻³² as f64)` (141188db0), `nextBoolean` = bit 27 (141188bc0), `consumeCount` (vt+0x40) skips n | bedrock floor, carvers, canyons, decoration, structures post-process, nether/end legacy noises |
| Java LCG `0x5DEECE66D` | 40+ sites (14593d6cb…, 1436d3762…) | not on the overworld terrain path (likely legacy structures/`JavaRandom` helpers; unmapped) |
| Java pos hash `0x2FC20F`/`0x6EBFFF5`/`0x285B825` | 23 sites | `Mth.getSeed(x,y,z)` analogue (block-variant/positional); unmapped |

## Named noises (all `minecraft:` strings present; seeded Xoroshiro `fromHashOf`)

`OverworldNoises3d::make` 1481d9b10 — firstOctave and amplitudes read from the code, all equal to Java 1.18:

| noise | firstOctave | amplitudes |
|---|---|---|
| temperature | −10 | 1.5, 0, 1, 0, 0, 0 |
| vegetation | −8 | 1, 1, 0, 0, 0, 0 |
| continentalness | −9 | 1, 1, 2, 2, 2, 1, 1, 1, 1 |
| erosion | −9 | 1, 1, 0, 1, 1 |
| ridge ("minecraft:ridge", built inline) | −7 | 1, 2, 1, 0, 0, 0 |
| offset | −3 | 1, 1, 1, 0 |
| jagged | −16 | 1 ×16 |

NormalNoise valueFactor = `(1/6) / (0.1·(1 + 1/(maxOct−minOct+1)))`, INPUT_FACTOR 1.018127 (f32) in 145e26660: Java formula.

`OverworldGeneratorMultinoise::Noises::make` 148220f20 order: `minecraft:blending`, `minecraft:terrain` (positional fork
for LegacyBlendedNoise, as Java), `minecraft:surface`, `aquifer_barrier`, `aquifer_fluid_level_floodedness`,
`aquifer_fluid_level_spread`, `aquifer_lava`, `minecraft:aquifer` (positional), CavifierNoises (148234150:
`cave_cheese`, `cave_layer`, `cave_entrance`, `spaghetti_3d_1/2`, `spaghetti_3d_rarity/thickness`,
`spaghetti_2d`, `spaghetti_2d_elevation/modulator/thickness`, `spaghetti_roughness(_modulator)`, `pillar`,
`pillar_rareness`, `pillar_thickness`), `ore_veininess`, `ore_vein_a/b`, `ore_gap`, `noodle`, `noodle_thickness`,
`noodle_ridge_a/b`, then `OverworldNoises3d::make`, then `minecraft:depth_based_layer`.
Absent vs Java 1.18.2+: `minecraft:shift`, `surface_secondary`, `*_large`, nether/surface-rule noises
(`clay_bands_offset`, `badlands_surface`, `iceberg_surface` exist but belong to Bedrock surface builders 1487dd550/1487e18e0).

TerrainShaper (`TerrainShaper::buildOverworld` 148068d20, `getErosionFactor` 14806ff80): erosion points −0.6, −0.5,
−0.35, −0.25, 0.03; weirdness ±0.2 / ±0.05; ridges (0.45→1.56, 0.7) and (−0.7, −0.15→1.37) — exact Java 1.18.1
`TerrainShaper`. Bedrock adds debug names ("Erosion Factor for base value", "Ridge spline for", "WeirdnessSpline for {}
jaggedness factor", "M-spline for continentalness", "Natural variation 3D noise 1/2", "Weirdness/Ridges Shattered",
"Extra 3D Peak Noise") — labels only, the spline values match Java.

BlendedNoise (14822d4b0 overworld, 1481e9e90 nether, 1482171a0 2D, 1482432f0 end): 684.412, 8.55515 (=/80),
4.277575 (=/160); nether 2053.236 (y_scale 3), 34.22 (=/60); end 1368.824 (xz_scale 2). Java noise-settings values.

## Chunk pipeline

ChunkSource tasks (strings): `_launchGenerationTask` → `loadChunk`; `_launchStructurePostProcessingTask` →
`structurePostProcessChunk`; decoration post-process → `decorationPostProcessChunk`; `_lightingTask` (all in 1409ddfc0 /
140a17440). Generation is per chunk, neighbours only needed from post-process on (`ChunkViewSource`).

### `OverworldGenerator::loadChunk` 148211300 (vtable slot 14)
1. `getBiomeSource()` (slot 48); `createNoiseCache(chunkPos)` (slot 59, 14822cb10): per-quart climate cache
   (`ChunkLocalNoiseCache::CacheEntry`: shiftedX/Z, continentalness, weirdness, erosion, offset, factor, jag, temp, humidity).
2. `WorldGenerator::preProcessStructures` 14196b7b0, `prepareStructureFeatureBlueprints` 14196b5f0.
3. Seed thread MT: `seed = cx*0x14609048 + cz*0x7EBE2D5` (u32 wrap). **No world seed.**
4. Fill 16×16×320 buffer with air; `tryMakeAquifer` (slot 58, 14822c900 → `Aquifer::Aquifer` 1481df940).
5. `biomeSource->fillBiomes(levelChunk, noiseCache)` (vt+0x18; 3D biomes, `BiomeSource3d`).
6. `_prepareHeights` (slot 62, 14822eb60): density cells (`generateDensityCellsForChunk` 14822d010 → 14822d4b0),
   trilinear interpolation, Cavifier/noodles, aquifer fluid pick, ore veins, deepslate supplier.
7. Bedrock floor (worldgen-bds-surface.md).
8. Depth noise (slot 60 `minecraft:surface`): PerlinSimplex octaves, coord·0.125·k, amp 0.55/k, k halves per octave → 16×16 buffer.
9. `BiomeSurfaceSystem::findBiomesAndPrepareChunkParams` 14580c660 → `buildChunkSurface` 14580d280
   (per-biome surface builders from biome JSON; resolved by `resolveBuilderImplementation(LevelSeed64)`).
10. If an experiment/level flag (vt+0xbe0) is set: second surface pass per vertical run of 3D biomes (underground biomes).
11. `decorateWorldGenLoadChunk` (slot 53, 1481ebd30 → `BiomeDecorationSystem::decorateLargeFeature` 14580c470):
    the `pregeneration_pass` ("cave_carver_feature is the only valid feature in pregeneration_pass").
12. Hardcoded canyons (`CanyonFeature::apply` 14827f840): +0x268 (vt 14a98ad98) only in legacy worlds, then +0x258
    (vt 14a925930). Steps 11-12: worldgen-bds-carvers.md.
13. Commit block volume to `LevelChunk`, set generator, state → generated (1409f8eb0, 1409e29f0, 1409e1fa0).

### `structurePostProcessChunk` 148213a20 (slot 9)
MT seeded `levelSeed32`, `a=nextInt()/2*2+1`, `b=nextInt()/2*2+1`, `seed' = levelSeed ^ (cx*b + cz*a)` (pairing of a/b
to x/z: verify), then `postProcessStructureFeatures` 14196b2c0 (legacy structures) and `postProcessStructures`
14196c5f0 (jigsaw v2).

### `decorationPostProcessChunk` 1482143a0 (slot 10)
MT reseeded with the large-feature chunk seed (worldgen-bds-carvers.md). Then, in order:
- Water lake (1/4, 1/32 under vt+0xbe0; not in desert ids 2/0x11): only with the legacy flag, so never in new worlds.
- Lava lake (if generator +0x18c is 0): `nextInt(8)==0`, x=`nextInt(9)`, y=`nextInt(nextInt(120)+8)` (absolute),
  z=`nextInt(9)`; skipped if y < 60 (dimension +0xcc = 63, minus 3); y > 63 also needs `nextInt(10)==0`. Placed at
  (cx·16+3+x, y, cz·16+3+z) by `LakeFeature::place` 149e923b0 = Java legacy lake in f32: corner −8, descend while the
  corner is air, base = y−4, `nextInt(4)+4` blobs (sizes `f*6+3`, `f*4+2`, `f*6+3`, centres `((16−s)−2)*f+1+s/2`,
  `((8−s)−4)*f+2+s/2`), border: liquid at grid y≥4 or non-solid non-lava below fails; lava below grid y 4, air above.
  Oracle: the one lava lake in the oracle regions matches cell for cell.
- Monster rooms (`MonsterRoomFeature::place` 149e9dae0): if minY < 0, 4 tries at y `nextInt(|minY+6|)+minY+6`, then 10
  (else 8) tries at y `nextInt(genHeight)`; x/z `nextInt(16)+(c·16|8)`. Not ported.
- Feature rules (vt slot 54 via cfg dispatch), MT reseeded again with the chunk seed.
- Freeze pass per column: heightmap top; `Biome::shouldFreeze` 14119e860 on top−1 (temperature ≤ 0.15, in build range,
  light < 10, water source) → ice; `shouldSnow` 14119e740 on top → snow layer. `Biome::getTemperature` 14119c8c0 has a
  noise path for a biome component (two PerlinSimplex sums, 0.05 and 0.2 scale, `< 0.3`) and a per-column chunk cache
  above sea level + 1 (+17 in legacy). Not ported: no frozen water in the oracle regions to test against.
Biome feature rules run through `decorateWorldGenPostProcess` (slot 54, 14821f400 → `BiomeDecorationSystem::decorate`
14580bb90 → `decorateBiome` 14580b170 per pass); exact caller of slot 54 not pinned.

### Feature placement (data-driven)
- Passes (`FeatureRegistry` statics, strings at a75e2c8/a9d02b9): `pregeneration_pass`, `first_pass`,
  `before_underground_pass`, `underground_pass`, `after_underground_pass`, `before_surface_pass`, `surface_pass`,
  `after_surface_pass`, `before_sky_pass`, `sky_pass`, `after_sky_pass`, `final_pass`. `mLargeFeaturePasses` vs
  `mSmallFeaturePasses`.
- Data: `definitions/feature_rules` (`minecraft:feature_rules`), features via `FeatureRegistry::loadFromDefinitions`;
  `AutomaticFeatureRules`; conditions are Molang (`variable.is_legacy` set in decorateBiome).
- Per-rule RNG: `s = chunkRandom.seed; s ^= ruleHash + 0x9E3779B9 + (s<<6) + (s>>2)` (boost hash_combine), fresh MT
  per rule (14580b52a..b5c4).
- Feature types seen: Scatter, Search, SnapToSurface, StructureTemplate, VegetationPatch, WeightedRandom, Aggregate,
  Sequence, ConditionalList, BeardAndShaver, RectLayout, ScanSurface, SurfaceRelativeThreshold, HeightDifferenceFilter,
  CaveCarver / NetherCaveCarver / UnderwaterCaveCarver, LegacyTree/VanillaTree, Speleothem, UnderwaterCave.
- `FeatureTerrainAdjustments::setBeardifier / setBeardAndShaver / setBury` → Java `Beardifier` analogue for structures.

### Ported stages
Floor, deepslate, ore veins, surface builders and the biome tree: [worldgen-bds-surface.md](worldgen-bds-surface.md).
Cave carvers and canyons: [worldgen-bds-carvers.md](worldgen-bds-carvers.md).

## Data-driven vs hardcoded

- **Hardcoded:** density/caves/aquifers/ore veins/terrain shaper/climate noises/biome parameter table, bedrock floor,
  lakes, dungeons, canyons.
- **Biome JSON components** (`BiomeJsonDocumentGlue::registerComponent`): `climate`, `multinoise_generation_rules`,
  `overworld_generation_rules` (legacy layer biomes), `overworld_height` (noise_type/params, 2D gen),
  `mountain_parameters`, `surface_builder` (Capped, NoiseGradient, TheEnd, Mesa/FrozenOcean…), `surface_material_adjustments`,
  `replace_biomes`, `frozen_noise_based`, `tags`, `village_type`. Vanilla pack biomes thus drive surface + features.
- `DensityFunction*` classes (Add/Clamp/Cache/LinearUpscale/NormalNoise/Spline/YClampedGradient…, `DensityFunctionRegistry`,
  `DensityFunctionWorldGenerator`) exist in headers: Bedrock's own data-driven density system, apparently not used by
  `OverworldGeneratorMultinoise` (unmapped).

## Nether / End (brief)

- Nether (ctor 1481e6d60): `PerlinNoise(IRandom&)` / `PerlinSimplexNoise` built from an **MT** `Random(seed)`
  (145e29d90/145e2a5e0), BlendedNoise with Java nether scales (684.412, y 2053.236, factors 80/60). Shape parity with
  Java's legacy-random nether is plausible, seed parity is not (Java uses its LCG there).
- End (14a924380): BlendedNoise ×2 xz (1368.824), constants −3000, 0.72, −843.0, 1/64, −8, 7, −30 in 1482432f0;
  `TheEndSurfaceSettings` surface builder. Not further mapped.

## Hand labels (scratch copy `names.user.json`, merge into bdsre)

```
148211300 OverworldGenerator::loadChunk                     1482143a0 OverworldGenerator::decorationPostProcessChunk
148213a20 OverworldGenerator::structurePostProcessChunk     14821f6f0 OverworldGeneratorMultinoise::OverworldGeneratorMultinoise
148220f20 OverworldGeneratorMultinoise::Noises::make        14822eb60 OverworldGeneratorMultinoise::_prepareHeights
14822d010 OverworldGeneratorMultinoise::generateDensityCellsForChunk
14822c900 OverworldGeneratorMultinoise::tryMakeAquifer      14822cb10 OverworldGeneratorMultinoise::createNoiseCache
14821f400 OverworldGeneratorMultinoise::decorateWorldGenPostProcess
1482180e0 OverworldGenerator2d::OverworldGenerator2d        1481e6d60 NetherGenerator::NetherGenerator
1481d9b10 OverworldNoises3d::make                           148234150 CavifierNoises::make
148068d20 TerrainShaper::buildOverworld                     14806ff80 TerrainShaper::getErosionFactor
1481df940 Aquifer::Aquifer                                  145e26660 NormalNoise::getValue
14580c660 BiomeSurfaceSystem::findBiomesAndPrepareChunkParams
14580d280 BiomeSurfaceSystem::buildChunkSurface             14580bb90 BiomeDecorationSystem::decorate
14580b170 BiomeDecorationSystem::decorateBiome              14580c470 BiomeDecorationSystem::decorateLargeFeature
1481ebd30 WorldGenerator::decorateWorldGenLoadChunk         1481ebe30 WorldGenerator::decorateWorldGenPostProcess
14196b2c0 WorldGenerator::postProcessStructureFeatures      14196c5f0 WorldGenerator::postProcessStructures
14196b7b0 WorldGenerator::preProcessStructures              14196b5f0 WorldGenerator::prepareStructureFeatureBlueprints
```
Unlabelled but identified: 145e24e60 multi-octave noise from positional factory ("octave_"+i), 148263740 ThreadData
accessor.

Carver labels (2026-10-07): 14827f840/14827faa0/148280880 `CanyonFeature::{apply,addFeature,carveEllipsoid}`,
148281d10/148284810/1482830a0/148283530/148283e70/148281400 `CaveFeature::{place,addFeature,addRoom,addTunnel,
carveBlocks,canReplaceBlock}`, 1482810b0/148281070/1482817b0/148281770/148281160 `CaveFeature::{getThickness,
getThicknessLegacy,getCaveY,getCaveYLegacy,getTunnelLength}`, 1482817d0 `CaveCarverFeature::CaveCarverFeature`,
14580bd50 `BiomeDecorationSystem::_decorateLargeFeatureRules`, 149e923b0/149e91f50 `LakeFeature::{place,LakeFeature}`,
149e9dae0 `MonsterRoomFeature::place`, 14119e860/14119e740/14119c8c0 `Biome::{shouldFreeze,shouldSnow,getTemperature}`,
141188be0 `Random::_genRandInt32`, 140874110/140874140/1408741c0/140874390 `IRandom::{nextInt,nextIntUnbounded,
nextFloat,consumeCount}`.

## Unknowns / next

1. Float-vs-double drift: measure BDS vs a Java-1.18.1 double port at large coords before choosing what sapling ports
   (port the float version for Bedrock parity).
2. Cavifier/Aquifer/OreVeinifier internals not decompiled constant-by-constant (only spot checks); `_prepareHeights`
   14822eb60 and 14822d4b0 are the functions to transcribe.
3. Biome placement: when JSON `multinoise_generation_rules` override the hardcoded builder (never, for vanilla).
4. Surface builders not ported: Capped, TheEnd, NoiseGradient (sulfur caves), the subsurface second pass
   (step 10, flag vt+0xbe0); the legacy mesa variant 1487db280 (which worlds pick it?); `Biome::getTemperature`
   (base temperature used for ice/iceberg checks).
5. Who calls slot 54; which rule places `underground_cave_carver_feature`; structure post-process a/b pairing
   (assumed as the large-feature seed's).
6. Structures (legacy `StructureFeature` vs jigsaw `br::worldgen`, `worldgen/structure_sets`) unmapped.
7. Lighting/heightmap stage and `OverworldGenerator2d` selection rule (legacy worlds, `no_legacy_worldgen` 14393d890).

## Terrain port findings (crates/sapling-worldgen, 2026-10-06)

- Densities are Java's x128: the shaper factor carries 128 (shaper +0x1fc), BlendedNoise skips Java's /128,
  slides target -10/15, the cave threshold is 200; `_prepareHeights` scales back with 0.005 (= 0.64/128).
- f32 op order that matters: smoothstep = `((t*6-15)*t+10) * (t*t*t)`; NormalNoise second half scales the input
  by 1.0181268 before the octave factor; blended octaves take `nextFloat()*256` offsets, NormalNoise octaves
  `(float)(nextDouble()*256)`. Biome climate is quantized as `(long)((double)f * 10000)`.
- BlendedNoise fills the 5x41x5 cell grid per octave (`LegacyBlendedNoise::fillOctave` 145e23fc0) and reuses the
  previous sample's gradients while the y lattice index is unchanged, across columns too (port it as is).
- Not Java: noodles have no y-limit in the cells (block test `y >= minY+4` instead), corners below -150 skip caves and
  zero the noodle/vein noises, the global fluid picker is water at sea level 63 below everything but y < -54 lava,
  Java 1.19's deep-dark aquifer test is present, pillars clamp after the min (max(min(..), pillar)).
- Biomes are per block: the tree is sampled on an 8-block lattice (warm-started up each column) and every block takes
  the nearest jittered lattice point (`java.util.Random` step on `Mth.getSeed` ^ low 32 seed bits, offsets x0.9).
  BDS saves sub-chunks above the highest non-empty one as biome copies of it.
- Former gap (fixed, see Biome tree): BDS builds its RTree from its own leaf list with MSVC `std::sort` (`RTree::Node::sort` 148060320,
  build 148060510, create 1480610f0); cubiomes' Java tree (btree262) let exact-boundary ties
  (quantized value == shared range end) then resolve differently (0.075% of blocks near origin).

New labels: 14822d4b0 `_computeDensityCells`, 14822e160 `getPreliminarySurfaceLevel`, 145e24e60
`MultiOctaveNoise::MultiOctaveNoise`, 145e26950 `ParityImprovedNoise::noise`, 145e24940/145e29ee0/145e23fc0
`LegacyBlendedNoise::{LegacyBlendedNoise,addOctave,fillOctave}`, 1481e10d0 `Cavifier::carve`, 1481dfe30/1481e0af0
`Aquifer::{computeSubstance,computeFluid}`, 1481def00/1481df0e0/1481df660 `OverworldNoises3d::{sampleQuart,
sampleClimate,terrainInfo}`, 14822cb50 `ChunkLocalNoiseCache::ChunkLocalNoiseCache`, 148078730/148096a00
`CubicSpline::{apply,applyMultipoint}`, 14805f140 `BiomeSource3d::fillBiomes`, 14805eb90 `fillLattice`, 148a73c40/
148a72d90/142f67a10/148a72910 `BiomeVolume::{fuzzyZoom,latticeJitter,nearestCorner,uniformCellMask}`, 14805fad0/
14805fb30/1480616f0/14805fc20 `Climate::{quantizeCoord,RTreeNode::distance,RTree::search,TargetPoint}`.

Milestone 2a labels: 148230a70 `OverworldBiomeBuilder::addBiomes`, 1481f9850/1482009c0/148206880/14820bae0
`::{addMidSlice,addHighSlice,addPeaks,addLowSlice}`, 1481f95f0 `::addSurfaceBiome`, 14805fa80 `Climate::Parameter::span`,
14805fd00 `Climate::parameters`, 1480610f0/148060510/148060320/14805ff10 `RTree::{create,build,Node::sort,
Node::makeSubTree}`, 14808bb60 `RTree::Node::sort::less`, 148089fd0/14808ced0 `std::sort<RTree::Node>` (by params,
by key), 145809ef0/145809370/1487dcb80/1487e4350/1487e4d10 `VanillaSurfaceBuilders::{Overworld,Capped,Mesa,
FrozenOcean,Swamp}SurfaceBuilder::buildSurfaceAt`, 1487dd550/1487e18e0/1487e4870 `::{Mesa,FrozenOcean,Swamp}
SurfaceBuilder::init`, 143907ca0 `SurfaceMaterialAdjustments::evaluate`, 1481f0380 `OreVeinifier::OreVeinifier`,
145e2a730 `PerlinSimplexNoise::addOctave`, 145e27a70/145e27d50 `SimplexNoise::{getValue2D,getValue3D}`,
141188be0/141188db0/141188e50/141188bc0 `Random::{_genRandInt32,nextFloat,nextInt,nextBoolean}`.
