# Bedrock worldgen prior art

Researched 2026-10-06. Question: what reimplements BDS world generation, and how close is
Bedrock 1.18+ terrain to Java's?

Legend: **[code]** = I read the source. **[readme]** = README/page claim, not checked.
"Activity" = last push per the GitHub API on 2026-10-06.

## Bottom line

- No open project reproduces full Bedrock 1.18+ chunks (terrain, carvers, features, structures)
  seed-accurately. Every Bedrock tool found does one of two things: it reuses **Java** biome
  and terrain code (cubiomes) for the noise part, or it adds Bedrock MT19937 math for
  structure **positions** and a few feature-RNG details.
- Terrain shape and biomes: port Java's 1.18+ noise router (Java code or a Rust port of it such as
  Pumpkin's). Mojang states that 64-bit seeds give the same world on both editions.
- Anything that is decorated or carved needs Bedrock-specific work that no project has done in
  full. Treat BDS as the oracle.

## Java/Bedrock terrain parity facts (primary sources)

| Fact | Source |
|---|---|
| BE 1.18.30: "Worlds can now be created with 64-bit seeds … These seeds can be used between Bedrock Edition and Java Edition to produce the same world (MCPE-144994, MCPE-148168)"; "Non-numeric seed UI inputs now produce the same seeds as Java Edition" | [minecraft.net 1.18.30 changelog](https://www.minecraft.net/en-us/article/minecraft-1-18-30-bedrock) (fetched, quoted) |
| BE beta 1.18.0.22: "Replaced the random number generator used in world generation"; 1.18.0.24 reshuffled seeds again. Java 21w41a likewise replaced the worldgen RNG (Xoroshiro), so the BE change was presumably the convergence step | [World seed history](https://minecraft.wiki/w/World_seed) |
| Wiki: all 64-bit seeds "generate the same terrain and biomes in both", but "structures, features (i.e. decorators), carver caves, and mob spawns will generate differently". The wiki itself tags this {{Verify}} | [World seed](https://minecraft.wiki/w/World_seed) |
| "Different generated structure, terrain feature and cave carver locations between editions" (MCPE-154323) | [Parity issue list, Unsorted/World generation](https://minecraft.wiki/w/Parity_issue_list) |
| BE: "The Nether only uses the bottom 32 bits of the world seed, rather than all 64 like the Overworld does" (MCPE-154947) | Parity issue list |
| BE: canyons "generally larger and deeper"; icebergs twice as often; ore can replace polished andesite/diorite/granite; huge mushrooms in swamps; dying trees; dead-coral reefs; End small-island placement differs | Parity issue list (BE-only section) |
| JE only: andesite/diorite/granite blobs can replace deepslate and tuff (MC-242090) → BE ore/stone-blob passes differ | Parity issue list (JE-only section) |
| Biome-attribute differences (temperatures of dripstone/lush caves, meadow and cherry colors, eroded-badlands band colors MC-240977) are colors and attributes. Placement is shared | Parity issue list |
| Bedrock floor: wiki tags "distribution of bedrock is dependent on the world seed" as **JE only**. So BE's Y=-64..-60 floor probably does not use Java's seeded positional RNG. **Unverified; check against BDS.** | [Bedrock](https://minecraft.wiki/w/Bedrock) |
| Structures on BE: per-region MT19937 seeded with `rx*341873128712 + rz*132897987541 + seed + salt`, truncated to 32 bits, so only the low 32 bits of the seed matter. Exceptions: trail ruins and trial chambers use Java placement (full seed) | [code] ChunkBiomesGUI `Bfinders.c/h`, BedrockUtil `ChunkRandom.java`; [readme] MCBE-seedcracker, SeedFinder |
| BE buried treasure sits at chunk-local 8 (Java uses 9). Nether fortress/bastion regions are 480 blocks (Java 432) with 1/3 fortress odds (Java 2/5) | Parity issue list |
| Slime chunks on BE: MT19937 seeded with `(cx*0x1f1f1f1f) ^ cz` (522133279 = 0x1f1f1f1f); slime chunk if `next_u32 % 10 == 0`. Independent of the world seed | [code] BedrockUtil `getSlimeSeed`; rbedrock docs; chunkbase credits protolambda/jocopa3 |
| Aquifers and ore veins: no BE-specific divergence documented anywhere I found. Both come from the noise router, so presumably shared. **Unverified** | the absence on Aquifer/Ore_vein wiki pages is the only evidence |

### Bedrock RNG primitives (from code)

- MT19937, 32-bit seed, standard init (`1812433253`) and tempering. Draw kinds: `next_u32`;
  `nextInt()` = `u32 >> 1`; `nextInt(n)` = `u32 % n`; float = `u32 / 2^32`. [code] ChunkBiomesGUI
  `Brng.h`, rbedrock `src/random.c`. Note: ChunkBiomesGUI takes a power-of-two fast path
  (`& (n-1)`), which equals `% n` for unsigned values.
- Chunk population seed: seed MT with the world seed, draw `a`, `b` (`nextInt()`), force odd,
  then `(cx*a + cz*b) ^ worldSeed`. **The "force odd" rule is disputed.** BedrockUtil and the
  commented-out ChunkBiomesGUI code use `|1` (ChunkBiomesGUI marks it "unverified"). featurelab
  states it is `v/2*2+1` (round toward zero), which differs for odd negatives. featurelab's
  version is the only one that claims measurement against a game build (1.26.50/1.26.60).
  **Resolved (BDS 1.26.52 decompile, 2026-10-07):** BDS compiles `v/2*2+1`, but `nextInt()` is `u32 >> 1`, never
  negative, so both forms agree (worldgen-bds.md, Carvers).
- Carver/mineshaft seed: the same, without forcing odd. [code] BedrockUtil `getCarverSeed`,
  ChunkBiomesGUI `bedrockChunkGenerateRnd`.
- Per-feature-entry seed: `s ^ (fnv1_32(name) + 0x9E3779B9 + (s<<6) + (s>>2))` (Boost
  hash_combine). The name hash is FNV-1 64-bit (multiply then xor), truncated, and an empty
  string hashes to 0. Two independent generators are built from the result: one for positions,
  one for the placed feature. [code] featurelab `random/decorationseed.go`. BedrockUtil has the
  same combine as `getDecorationSeed` with an integer salt.

## Projects, ranked by usefulness to a Rust BDS-parity port

| # | Project | Lang / license | Activity | BE target | Covers | Accuracy evidence | Reuse for Rust |
|---|---|---|---|---|---|---|---|
| 1 | [Pumpkin-MC/Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) `crates/pumpkin-world/src/generation` | Rust / GPL-3.0 | 2026-10-06 | **Java** 1.21.x | noise router, aquifers, ore veins, surface rules, carvers, features, structures | [code] tests compare chunks to vanilla Java dumps with a mismatch allowance (`proto_chunk_test.rs`, surface allows ≤6000 mismatches) plus a density fingerprint test | High for terrain, since BE terrain = JE terrain. GPL-3.0: read it, or reuse it only if sapling can be GPL |
| 2 | [xpple/cubiomes](https://github.com/xpple/cubiomes) (Cubitect fork) | C / MIT | 2026-10-02 | **Java** | biomes, 1.18+ final density (`terrainnoise.c`: spaghetti, cheese, noodle, pillars, preliminary surface, `generateRegion`), carvers, ores, ore veins, strongholds, loot | [readme] lists the additions; tests in `tests/` | High: MIT, compact C, one function per density term. Good as a reference for the noise router or as a test oracle via `cubiomes-sys` |
| 3 | [stirante/featurelab](https://github.com/stirante/featurelab) | Go engine + TS UI / MIT | 2026-10-04 | BE 1.26.50–1.26.60 | data-driven **feature** placement (scatter, ore, tree, geode, carver features, feature rules, Molang) on a bench; Bedrock decoration-seed derivation, FNV-1 name hash, two-stream split | [code] seeding comments claim the derivation follows the game; docs mark which claims were reproduced and which are uncertain (`docs/site/engine/coverage.md`) | High for the feature/decoration pass, and the license allows it. Bench only: no terrain and no world |
| 4 | [MZEEN2424/ChunkBiomesGUI](https://github.com/MZEEN2424/ChunkBiomesGUI) (from [Nel-S/Chunkbiomes](https://github.com/Nel-S/Chunkbiomes)) | C/C++ / custom LICENSE: Chunkbase-ported code is reserved to its authors | 2025-01-27 | BE 1.14–1.21 | MT19937; region salts, spacings and offsets for all BE structures; uniform vs triangular placement; mineshaft; ravine; biomes via **unmodified Java cubiomes** | [code] read `Brng.h`, `Bfinders.c/h`. Several items marked "unverified" | Constants and formulas are directly usable. Rewrite them yourself, because the license is murky |
| 5 | [FragrantResult186/BedrockUtil](https://github.com/FragrantResult186/BedrockUtil) | Java / no license | 2026-05-22 | BE | ChunkRandom (population, decoration, carver, region, stronghold, fortress, end-city and slime seeds); **mineshaft and stronghold piece generators**; per-structure configs by version | [code] `ChunkRandom.java`, `GridStronghold.java`; tests in `TestStructures.java` | Good reference for seed formulas and BE stronghold/mineshaft pieces. No license, so read it and don't copy |
| 6 | [zebedelu/SeedFinder](https://github.com/zebedelu/SeedFinder) | C + Python + Lua / Zlib (API) | 2026-10-05 | BE 1.18+ | 17 structure types, BE biome gates (`core/viability.c`), 64-bit cracker using Java-style trail ruins/trial chambers | [readme] measured cracking runs (`docs/verification/bedrock64-empirical.md`); credits Chunkbase | Medium: permissive license, structure-position layer only |
| 7 | [Alist2930/MCBE-seedcracker](https://github.com/Alist2930/MCBE-seedcracker) | C (+OpenCL) / none | 2026-09-30 | BE 1.18–26.3x | low-32 structure cracking (MT), high-32 via cubiomes biomes | [readme] gives the region-seed formula (low-32 form of the same constants) | Confirms that biomes use Java cubiomes on BE. Low value otherwise |
| 8 | [Earthcomputer/bedrockified](https://github.com/Earthcomputer/bedrockified) | Java (1.13.1 jarmod) / MIT | 2019-06-26 | BE ~1.13 | BedrockRandom, BE seed, BE ocean layers, shipwreck, stronghold, spawn point, slime chunks | [readme] claims parity for biomes, structure locations, slimes and spawn. Chunkbase credits it as the basis of its BE support | Historical; the pre-1.18 biome layer stack is obsolete. `BedrockSpawnPoint` and `BedrockStrongholdStructure` are still worth a read |
| 9 | [bedrock-dev/MCBEStructureFinder](https://github.com/bedrock-dev/MCBEStructureFinder) (xiaohengying/hhhxiao) | C/C++ / none | 2025-07-19 | BE 1.18+ branch | structure finders (village, stronghold, temples, monument, mansion, portal, treasure, outpost, fortress, bastion), world spawn | [readme] says it is "being refactored, currently unusable". Chunkbase cites it | Low; read `be_finder.cpp` for spawn and stronghold only |
| 10 | [PowerNukkitX](https://github.com/PowerNukkitX/PowerNukkitX) `level/generator` | Java / LGPL-3.0 | 2026-10-06 | BE 1.21.x server | full 1.18-style pipeline: density functions, splines, aquifer, ore veins, carvers, BE-named ore features, jigsaw structures | [code] noises have vanilla octave/amplitude parameters but are seeded through `RandomSourceProvider.fork()` order and `NukkitRandom` (MT) per chunk, not vanilla's per-name positional seeding. **Vanilla-like, not seed-accurate** | Low for parity. Shows a full 1.18 pipeline running inside a Bedrock server |
| 11 | [KCodeYT/VanillaGenerator](https://github.com/KCodeYT/VanillaGenerator) | Java / Apache-2.0 | 2023-02-12 | BE 1.19.60 | runs a real BDS and clones its chunks into PNX | [readme] | The "BDS as generator" approach; also usable as an oracle harness pattern |
| 12 | [HimmelKreis4865/JavaGen](https://github.com/HimmelKreis4865/JavaGen) | PHP / Apache-2.0 | 2024-04-27 | PMMP | proxies chunks from a **Java** server | [readme] | Same idea using a Java server: terrain matches, decoration doesn't |
| 13 | [reedacartwright/cubiomes `bedrock`](https://github.com/reedacartwright/cubiomes/tree/bedrock) | C / MIT | 2021-08-07 | BE 1.16–1.17 | BE biome layers (1.16), BE nether. Commit: "Output is consistent with chunkbase" | [code] compare API: 6 commits ahead, 143 behind upstream | Pre-1.18 only. Obsolete for sapling |
| 14 | [reedacartwright/rbedrock](https://github.com/reedacartwright/rbedrock) | R + C / see repo | 2026-10-05 | BE | BE MT19937 API (`bedrock_random_*`), seed helpers, world/LevelDB I/O | [code] `src/random.c` | Small, clean MT reference; useful as a test vector source |
| 15 | [birdieindaklubb/PE-BE-Terrain-Verifier](https://github.com/birdieindaklubb/PE-BE-Terrain-Verifier) (+BE-Pillar/Dungeon-Cracker) | C++ / MIT | 2026-09-24 | PE 0.6.1, 0.9.0, 1.1.5 | pre-1.18 PE terrain density, surfaces, native cave carver, Nether/End | [readme] parity-check target against a private "PE-BE-Worldgen-Lib" | Pre-1.18 only. Its PE cave carver could hint at how BE carvers work today. Unverified |

### Checked and ruled out (no vanilla-BE terrain)

| Project | Finding |
|---|---|
| [Cubitect/cubiomes](https://github.com/Cubitect/cubiomes), [cubiomes-viewer](https://github.com/Cubitect/cubiomes-viewer) | Java only. Code search found no `bedrock` or `MC_BE` in upstream. Last push 2024-11-10 |
| [df-mc/dragonfly](https://github.com/df-mc/dragonfly) (Go) | `server/world/generator/flat.go` only [code] |
| [AllayMC/Allay](https://github.com/AllayMC/Allay) | built-in generator is flat (`FlatNoiser.java`); "vanilla-like" terrain comes from the Terra plugin (custom config packs) [code+docs] |
| [CloudburstMC/Nukkit](https://github.com/CloudburstMC/Nukkit) `Normal.java` | old Java-1.12-style octave noise (684.412 scale) seeded with NukkitRandom/SplittableRandom; not BE-accurate [code] |
| [pmmp/PocketMine-MP](https://github.com/pmmp/PocketMine-MP) `world/generator/normal` | custom simplex generator; not vanilla |
| [wode490390/BetterVanillaGenerator](https://github.com/wode490390/BetterVanillaGenerator) | Nukkit plugin, Java-1.12-style port; last push 2022 |
| [bedrock-crustaceans/bedrock-rs](https://github.com/bedrock-crustaceans/bedrock-rs) | protocol/world I/O; no generator |
| [Mojang/bedrock-samples](https://github.com/Mojang/bedrock-samples) | ships 89 vanilla `biomes/*.json` (climate, legacy `overworld_generation_rules`, `surface_builder`, `multinoise_generation_rules`) but **no** vanilla `features/` or `feature_rules/`, and no noise settings |
| Chunkbase (closed) | BE support credits bedrockified. Its warning: "On Bedrock Edition, predictions get slightly less reliable millions of blocks away from 0, 0", and some structures fail in game more often on BE ([seed map](https://www.chunkbase.com/apps/seed-map)) |
| crates.io | `cubiomes`/`cubiomes-sys` (Rust wrapper of upstream, Java only); `tungsten-mc` (Java datapack → Rust/CUDA density compiler, repo has no license); `mt19937` (generic MT). No Bedrock worldgen crate |
| Bedrock Pattern Finder (CurseForge) | Java 1.21+ only (`RandomSplitter` bedrock floor) |

## Implications for sapling

1. **Terrain and biomes:** port the Java 1.18+/current noise router (Xoroshiro128++, md5-named
   positional seeding, density functions, aquifer, ore veins, surface rules). Pin the Java
   version that matches the BDS version. Use xpple/cubiomes (MIT) or Pumpkin (GPL, read only) as
   references. Validate against BDS chunk dumps, not Java, because the parity claim carries a
   wiki {{Verify}}.
2. **Bedrock RNG layer:** MT19937 plus the seed derivations above. Settle the `|1` vs `/2*2+1`
   question with the BDS oracle before relying on either.
3. **Unexplored in public projects, so BDS reverse-engineering is needed:** carver caves and
   canyons (BE-specific; canyons are larger), feature decoration order and per-biome lists
   (vanilla feature JSON is not published), BE structure pieces beyond stronghold and mineshaft,
   bedrock floor pattern, Nether 32-bit seeding, End islands.
4. **Cheap first wins:** slime chunks, structure start positions, spawn point. Several
   independent implementations agree on these.
