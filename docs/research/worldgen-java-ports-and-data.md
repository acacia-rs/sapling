# Java 1.18+ worldgen ports and data sources

Researched 2026-10-06. Scope: open reimplementations of **Java** 1.18+ worldgen, the data each
side exposes, and the Java/Bedrock differences that matter for a port. Bedrock-specific projects
are in `worldgen-prior-art.md`.

Legend: **[code]** = I cloned and read the source. **[readme]** = README or API claim, not checked.
Activity = last push per GitHub on 2026-10-06. sapling is MIT (`sapling-worldgen/Cargo.toml:7`).

## Bottom line

- Three Rust projects generate real blocks through the full Java pipeline (noise, aquifers, veins,
  surface, carvers, features, structures): **Pumpkin** (GPL-3.0), **worldgen-rs** (Apache-2.0)
  and **BCore** (MIT). **SteelMC** (AGPL-3.0) has the noise, surface and structure stages.
  **mcrs** (MIT) has a large worldgen workspace too.
- License limits what we can copy. GPL and AGPL code (Pumpkin, SteelMC) cannot go into MIT
  sapling. Use them as references only. We can copy from Apache-2.0 worldgen-rs (keep its
  LICENSE/NOTICE) and from MIT deepslate, cubiomes, BCore and mcrs (keep the copyright line).
- Recommended base: **worldgen-rs** as the structural template. It is data-driven, permissive,
  reads datapack JSON at runtime, covers every stage through features, and targets ≥1.21.9.
  Check its numbers against **deepslate** (MIT, the reference DF interpreter) and against
  Pumpkin's vanilla chunk-dump tests (read the tests, don't copy them).
- Bedrock exposes almost none of its worldgen as JSON. The vanilla behavior pack has only
  `biomes/*.biome.json`, with no features, feature rules, structures or noise. Java's datapack
  JSON is the only machine-readable source for the noise router, and since 1.18 Bedrock terrain
  and biomes match Java seed-for-seed (see §4).

## 1. Projects that produce blocks

| Project | Lang | License | Activity | MC target | Size |
|---|---|---|---|---|---|
| [Pumpkin-MC/Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) `crates/pumpkin-world/src/generation` | Rust | **GPL-3.0** | 2026-10-06, very active | Java 26.3 (version enum up to `V_26_3`) | ~42k lines in generation/ |
| [garfxld/worldgen-rs](https://github.com/garfxld/worldgen-rs) | Rust (nightly) | **Apache-2.0** | 2026-08-22, one squashed commit | ≥1.21.9 (shelf blocks, pale_garden present) | ~65k lines |
| [HVHBIGNAME/BCore](https://github.com/HVHBIGNAME/BCore) `crates/bcore-worldgen` | Rust | **MIT** | 2026-10-05, active | Java 26.1 (moving to 26.2) | ~62k lines in worldgen |
| [Steel-Foundation/SteelMC](https://github.com/Steel-Foundation/SteelMC) `steel-worldgen` | Rust | **AGPL-3.0** | 2026-10-06, 742★ | Java 26.2 | ~27k lines |
| [andreypfau/mcrs](https://github.com/andreypfau/mcrs) `crates/mcrs_minecraft_worldgen*` | Rust (Bevy) | **MIT** | 2026-10-05 | recent Java | ~106k lines over 12 worldgen crates |
| [misode/deepslate](https://github.com/misode/deepslate) `src/worldgen` | TypeScript | **MIT** | 2026-09-28 (v0.27.2) | tracks current Java | ~3.9k lines in worldgen |
| [Minestom/VanillaReimplementation](https://github.com/Minestom/VanillaReimplementation) | Java | Apache-2.0 | **archived** | 1.19-1.20 era | datapack loader + DF |
| [picawawa4000/mcmapper](https://github.com/picawawa4000/mcmapper) / [dpreader](https://github.com/picawawa4000/dpreader) | C++ | GPL-3.0 | archived / 2025-11 | 1.21.4 | DFs, biomes, a few structures |
| [Earthcomputer/fluffy-parakeet](https://github.com/Earthcomputer/fluffy-parakeet) | Rust | **none** (all rights reserved) | 2024-07, stale | 1.21 | datapack interpreter, ~6.6k lines |
| [Cubitect/cubiomes](https://github.com/Cubitect/cubiomes) | C | MIT | 2024-11 | up to `MC_1_21_WD` | biomes and structure positions only |
| [EnderKill98](https://github.com/EnderKill98/cubiomes-rs) / [villevilli](https://github.com/villevilli/cubiomes-rs) cubiomes-rs | Rust FFI | MIT / other | 2023 / 2025-03 | inherits cubiomes | bindings only |
| [SeedFinding/mc_noise_java](https://github.com/SeedFinding/mc_noise_java), mc_terrain_java, mc_biome_java | Java | MIT | 2023-06, dead | ≤1.17 terrain | old KaptainWutax libraries, no 1.18 noise router |
| Cuberite, Feather, Valence | C++/Rust | Apache/MIT | — | — | no vanilla 1.18 terrain: Cuberite has its own generators; Feather and Valence ship none |

### Coverage (✓ implemented, ~ partial, ✗ absent)

| Stage | Pumpkin | worldgen-rs | BCore | SteelMC | mcrs | deepslate | cubiomes |
|---|---|---|---|---|---|---|---|
| Density functions / noise router | ✓ codegen to static Rust | ✓ IR + Cranelift JIT | ✓ | ✓ build-time transpiler | ✓ | ✓ interpreter | climate only |
| Biome source (multi-noise, end, nether) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (B-tree tables) |
| Aquifer | ✓ (2.6k lines) | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| Ore veins | ✓ | ✓ | ✓ | ✓ | ? | ✓ | ✗ |
| Surface rules | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (MaterialSystem) | ✗ |
| Carvers | ✓ cave/canyon | ✓ cave/canyon | ✓ | ✗ | ✓ | ✗ | ✗ |
| Features + placement | ✓ ~60 feature types, trees | ✓ 56 feature files, trees | ✓ | ✗ | ✓ | ✗ | ✗ |
| Structures (jigsaw + hardcoded pieces) | ✓ all major | ✓ all major | ~ (per parity report) | ✓ starts + pieces | ✓ | placement only | positions only |
| Blending (old-chunk) | ✓ | ✓ | ? | ✓ (beardifier) | ? | ✗ | ✗ |
| Lighting | server | ✗ | ✓ | server | ✓ | ✗ | ✗ |

### Parity evidence

| Project | What is checked against vanilla |
|---|---|
| Pumpkin [code] | `proto_chunk_test.rs` compares NOISE and SURFACE stage blocks with **vanilla chunk dumps** in `assets/tests/*.chunk` (overworld, nether, end; seeds 0 and 13579; badlands and frozen-ocean chunks), all without blending or beardifier. `density_function_tests.json` and `multi_noise_biome_source_test.json` are vanilla-extracted. `parity_fingerprint_test.rs` is only a regression hash. Features and structures are not dump-tested. |
| BCore [readme] | Strongest claim. A real-server oracle compares whole chunks (block states, quart biomes, structure refs, light) over "69 requests across 11 histories" with zero diffs, but it says "worldgen parity is incomplete". The docs read as heavily machine-written. Verify before trusting. |
| worldgen-rs [code] | `tests/*_parity.rs` check the JIT against its own interpreter bit-for-bit, **not against vanilla**. The README only claims output "matches vanilla". |
| SteelMC [readme] | Mirrors vanilla structure; downloads the server jar at build time. No dump tests seen. |
| deepslate [code] | Unit tests per DF/noise. In practice it is checked by the many users of misode.github.io previews. |
| cubiomes [readme] | Biomes are checked against the game; widely used for seed finding. |

### Structure notes (top three Rust)

- **Pumpkin.** Datapack JSON comes from the Pumpkin [Extractor](https://github.com/Pumpkin-MC/Extractor)
  Fabric mod (`assets/*.json`). `tools/pumpkin-codegen` turns it into `pumpkin-data`, so the
  noise router, noise params, features, carvers and material rules are compiled in. The router
  has two layers: `proto_noise_router` (per seed) and `chunk_noise_router` (per chunk, with
  cell/flat caches). The repo's AGENTS.md requires vanilla names and vanilla method shapes.
  The server also accepts Bedrock clients: Java states are mapped to Bedrock through GeyserMC
  `blocks.nbt` (MIT, `assets/bedrock/`). This is a working "Java gen → Bedrock wire" pipeline,
  but it is GPL.
- **worldgen-rs.** `src/datapack` loads a dir or zip layered over a bundled vanilla datapack, so
  it is runtime data-driven. `worldgen/backend/{ir,jit}` holds a DF IR with analysis passes and
  Cranelift codegen. `worldgen/{biome,density,surface,carver,feature,placement,structure,generator,scheduler}`
  mirror vanilla's packages. The bundled `multi_noise_biome_source_parameter_list/overworld.json`
  is the *expanded* parameter table (t/h/c/e/d/w ranges), not the 1-line preset. It needs
  nightly (`portable_simd`, const traits). Lighting is not implemented.
- **SteelMC.** `build/density/transpiler` turns DF JSON into Rust at build time, the same idea
  as Pumpkin's codegen. Clean `noise/` module (aquifer, ore_veinifier, blended/simplex/perlin).
  No carvers or features in the worldgen crate.

## 2. Java data sources

| Source | What | Notes |
|---|---|---|
| Server jar `data/minecraft/worldgen/**` | `density_function` (62), `noise`, `noise_settings`, `biome`, `configured_feature`, `placed_feature` (298), `carver`, `structure`, `structure_set`, `template_pool`, `processor_list`, `world_preset`, `multi_noise_biome_source_parameter_list`, `flat_level_generator_preset` + `tags/worldgen` | Since 1.19.3 these ship as plain JSON in the jar. The download is a bundler: the real jar is at `META-INF/versions/<v>/`. The `.nbt` structure templates are at `data/minecraft/structure/`. |
| Data generator `--reports` (`java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports`) | `blocks.json` (states/ids), registries, **`biome_parameters/minecraft/overworld.json`** | The jar's `multi_noise_biome_source_parameter_list/overworld.json` is only `{"preset":"minecraft:overworld"}`. The real ~7.6k-entry table is built in code (`OverworldBiomeBuilder`) and is only exported by this report (e.g. [Arcensoth/mcdata](https://github.com/Arcensoth/mcdata/tree/master/generated/reports/biome_parameters/minecraft)). |
| [misode/mcmeta](https://github.com/misode/mcmeta) | Branches `data`, `data-json`, `summary`, `registries`, `diff`, … tagged per version; currently 26.4-snapshot-3 | Easiest per-version fetch. No repo license: the content is Mojang's. |
| [Pumpkin-MC/Extractor](https://github.com/Pumpkin-MC/Extractor) | Fabric mod; dumps runtime data, including flattened noise routers and test fixtures | GPL. Can be run to produce oracle dumps. |
| Decompiled source (official Mojang mappings) | Everything in §3 "hardcoded" | Reference only. Don't redistribute. |

**Hardcoded in Java, not in JSON:** noise algorithms (Improved/Perlin/Simplex/Blended/Normal), RNG
and seed derivation, aquifer logic, ore-vein constants (`OreVeinifier`), biome parameter table
(report only), blending, the `NoiseChunk` cell cache, surface extras (badlands clay bands,
eroded-badlands pillars, frozen-ocean icebergs), every feature/carver/placement-modifier
*type's* algorithm, non-jigsaw structure pieces (stronghold, fortress, mansion, monument,
mineshaft), and the decoration step order (`GenerationStep.Decoration`, 11 steps).

**Licensing of data:** Mojang JSON is EULA content, not open source. Pumpkin and SteelMC download
the jar at build time instead of vendoring it. Do the same: write a `tools/` fetcher pinned by
version and SHA, and commit only generated Rust or a cache in `.gitignore`.

## 3. Bedrock data sources

[Mojang/bedrock-samples](https://github.com/Mojang/bedrock-samples) (`main`, 1.26.50.4,
custom Mojang license), checked via the GitHub tree API:

| Path | Present for vanilla? | Contents |
|---|---|---|
| `behavior_pack/biomes/*.biome.json` | **yes, 89 files** | `minecraft:climate` (temperature, downfall, snow_accumulation), `minecraft:surface_builder` (`overworld`/`mesa`/`frozen_ocean`/`swamp`/`capped`/`the_end` + top/mid/foundation/sea materials, `sea_floor_depth`), `minecraft:mountain_parameters` (steep-slope material, `top_slide`), `minecraft:surface_material_adjustments`, `minecraft:tags`, `minecraft:village_type`, and for old biomes legacy `overworld_generation_rules` (hills/mutate/climate weights) + `multinoise_generation_rules` (target_* + weight) + `overworld_height` (`noise_type`) |
| `behavior_pack/features/`, `behavior_pack/feature_rules/` | **no**. The directories don't exist and no commits ever touched those paths | — |
| structures, noise, density functions, carvers, aquifers, ore veins | **no** | — |
| `metadata/json_schemas/server/biome/<ver>/` | schemas only | Component schemas, e.g. `NoiseDescriptor.json` (1.26.30), `SurfaceBuilderType.json` |
| `metadata/doc_modules/{biomes,features}.json` | docs only | Feature *types* for add-ons (`ore_feature`, `tree_feature`, `scatter_feature`, `geode_feature`, `cave_carver_feature`, …), the `placement_pass` field, `minecraft:replace_biomes` (custom biome replaces a vanilla one at a noise-selected amount) |
| `metadata/vanilladata_modules/mojang-biomes.json` | ids | Biome id list |

**Hardcoded in the Bedrock exe:** the noise router, the 1.18 multi-noise biome placement (vanilla
1.18+ biomes have no `multinoise_generation_rules`; the existing ones are pre-1.18 legacy and the
`target_*` values are 0/placeholder), aquifers, ore veins, carvers, every vanilla feature and its
placement, all structures, and the overworld surface rule logic beyond per-biome materials. The
biome JSON is a partial parameter source for surfaces (materials and the steep-slope flag). The
rules that decide *where* those materials go are code. The Molang/feature-rule pipeline in the
docs is the **add-on** path and does not describe how vanilla decorates.

Other Bedrock data: Java→Bedrock block and biome mapping from
[GeyserMC/mappings](https://github.com/GeyserMC/mappings) (`blocks.nbt`, `biomes.json`, MIT).
State palettes come from [CloudburstMC/Data](https://github.com/CloudburstMC/Data) (BDS-mined).

## 4. Java ↔ Bedrock differences relevant to a port

| Topic | Java | Bedrock | Source |
|---|---|---|---|
| Seed width | 64-bit always | 32-bit until **beta 1.18.20.21**, then 64-bit | [wiki: Seed](https://minecraft.wiki/w/Seed_(level_generation)) |
| Terrain + biomes | — | "All … 64-bit seeds generate the same terrain and biomes in both Java and Bedrock" | same wiki page |
| Structures, features, carver caves, mob spawns | — | "will generate differently" | same wiki page |
| Noise RNG | Xoroshiro128++ (`seed ^ 0x6A09E667F3BCC909` → stafford13 mix). Positional forks keyed by `md5(id)`; `legacy_random_source` (LCG) for nether/end noise settings | Must be identical for terrain to match, so assume a byte-exact port of the same | deepslate `XoroshiroRandom.ts`, Pumpkin `pumpkin_util::random` [code] |
| Feature/decoration RNG | `WorldgenRandom(Xoroshiro)`: `setDecorationSeed(seed,x,z)` (64-bit population seed) then `setFeatureSeed(pop, index, step)` | **MT19937**. Chunk seed `(x*m1 + z*m2) ^ worldSeed` in **32-bit**, then per feature-rule `hash_combine(chunkSeed, fnv(rule identifier))`. Two independent MT streams (positions vs placement) | featurelab `random/decorationseed.go` (RE of 1.26.50, MIT) [code]; Pumpkin `proto_chunk.rs:1136-1151` [code] |
| Carver RNG | `LegacyRandomSource` (Java LCG), `setLargeFeatureSeed` | MT-based, different carver set ("carver caves differ") | Pumpkin `carver/mod.rs:371`; wiki |
| General RNG | `java.util.Random` LCG | MT19937 (mt19937ar init, lazy table) | [Earthcomputer/bedrockified](https://github.com/Earthcomputer/bedrockified) `BedrockRandom.java` (MIT, 1.13-era) |
| Structure placement | `random_spread`/`concentric_rings` from `structure_set` JSON, LCG + salt | Different spacing/salts and stronghold rings; positions differ for the same seed | bedrockified `BedrockStrongholdStructure.java`; wiki |
| Decoration order | 11 `GenerationStep.Decoration` steps × biome feature lists (feature index is global, sorted across biomes) | Placement passes: `pregeneration`, `first`, `before/after_{underground,surface,sky}`, `{underground,surface,sky}_pass`, `final`; order inside a pass is per rule | `doc_modules/features.json`; Java `FeatureSorter` |
| Surface | One `surface_rule` tree in `noise_settings/overworld.json` | Per-biome `surface_builder` + `mountain_parameters` + `surface_material_adjustments`. Visually equivalent. **Block-level parity unverified** | biome JSON above |
| Biome storage | 4×4×4 quarts, 3D | per-block 3D palettes per subchunk (Bedrock stores finer). Java quarts must be expanded | Bedrock chunk format |
| Biome set/ids | `minecraft:windswept_hills` etc. | legacy names (`extreme_hills`, `mesa`, `roofed_forest`…), numeric ids. Bedrock-only `legacy_frozen_ocean`; `sulfur_caves` present in 1.26.50 samples | Geyser `biomes.json`; bedrock-samples |
| Heights | -64..320 overworld | same | — |

Not verified here, so test against BDS: whether Bedrock's aquifer and ore-vein streams are
bit-identical (the wiki's "same terrain" claim suggests yes), and whether Bedrock 1.26.x tracks
Java's newest density-function tweaks on the same release.

## 5. Data plan for sapling

1. **Noise/biomes/aquifer/veins:** generate from Java datapack JSON for a pinned Java version
   whose content matches the target Bedrock release. Fetch the server jar via a `tools/` script
   (version + SHA) and run `--reports` for `biome_parameters`. Codegen Rust (Pumpkin/SteelMC
   style), or load at runtime (worldgen-rs style). Oracle: BDS chunk dumps (see the molang BDS
   oracle memory) plus Java chunk dumps for cross-checks.
2. **Surface:** begin with Java's `surface_rule`. Cross-check per-biome materials against
   bedrock-samples `biomes/*.json`. Diff against BDS before shipping.
3. **Carvers, features, structures:** Java JSON gives the shapes and configs (trees, ores,
   jigsaw pools). RNG and ordering must be Bedrock's (MT19937, 32-bit chunk seed, pass order),
   so the decoration *driver* is Bedrock-specific even when feature bodies are ported from Java.
   featurelab (MIT, Go) is the best reference for that driver.
4. **IDs:** map Java block states and biomes to Bedrock with GeyserMC mappings (MIT).
   Vendor the mapping files and keep their license.
