# BDS feature rules (static RE + oracle, crates/sapling-worldgen/src/feature)

Companion to [worldgen-bds.md](worldgen-bds.md). Feature rules run in `decorationPostProcessChunk` after lava lakes.

## Data
- Rules and features are the vanilla pack JSON: `behavior_packs/vanilla_<ver>/__brarchive/{feature_rules,features}
  .brarchive` (256-byte entries: length byte + name, u32 offset/size at +248) plus `definitions/` (old 1.13 files).
  The 1.18+ ores (`*_lower`/`*_upper`, tuff, copper) exist only in the packs; the old unsplit rules there have
  iterations 0. `scripts/features_to_rust.py` transcribes them into `src/feature/data.rs` (the BDS install is not in
  the repo, and rule and feature data is static); biome filters are evaluated there against each biome's tags, as
  BDS does when it attaches rules to biomes at load. Rules whose feature type is not ported are left out (each rule
  has its own generators, so leaving one out shifts no other).
- Rule store (loader 144c03360): `unordered_map<pass, unordered_map<HashedString id, rule>>`, filled from the pack
  stack newest first (brarchive entry order), then `definitions/` by name; the first rule of an identifier wins.
  Each biome's decoration list is the rules whose filter passes, in the inner MSVC map's iteration order (bucket =
  FNV-1 64 of the full identifier & mask, 8 buckets, x8 growth). The script simulates the map; alphabetical order
  loses 0.5% of feature cells (overlapping stone blobs).

## Decoration of one chunk
- `decorateWorldGenPostProcess` 14821f400: per pass (`first_pass` .. `final_pass`), `BiomeDecorationSystem::
  decorate` 14580bb90 over the chunk's biome list (1438c8450: sub-chunk biome palettes bottom up, first seen
  first), `decorateBiome` 14580b170 per biome over its list, skipping other passes.
- Per rule: seed = boost `hash_combine(chunkSeed, (u32)FNV1_64(identifier))` with the large-feature chunk seed;
  MT1 draws the scatter (chance gate, iterations, one position per iteration, index counting down, axes in
  `coordinate_eval_order`, origin = chunk min corner, y 0); MT2, seeded alike, is handed to every placement.
- With more than one biome in the chunk, a position places only if its block's biome is the one being decorated;
  positions outside y -64..319 have no biome and are skipped without drawing (oracle: diamond and redstone rules
  scatter below the world; 73-81% to 99% precision). A one-biome chunk places every position.
- `ore_feature`: featurelab's port (Java legacy vein in f32, the Mth sine table, sphere pruning, cell-centre test,
  x/z/y loop); `may_replace` by block type; discard roll per matching rule when 0 < chance < 1, then the 6-face
  air test.

## Cross-chunk scheme
BDS decorates a chunk once its neighbours are generated and lets its features write into the 3x3 chunks around it;
writes further out are dropped (oracle: allowing two chunks is no better). Which neighbour decorates first depends
on load order, so cells two chunks' features both claim, and draws that depend on blocks a neighbour's features
changed, are not reproducible (the `-run2` worlds mask some of it).

sapling decorates each source chunk alone, over its 3x3 neighbours' terrain before features (cached loaded
columns), and logs each write with the block it replaced. A column replays the logs of the 9 source chunks around
it, x then z, applying a write only where the replaced block is still there, so the first source to claim a cell
keeps it. A column depends only on the seed, not on generation order. Seeding a source's view with its
neighbours' writes (several orders tried) changes nothing measurable. Cost: per column one decoration (cached
for the neighbours) plus the loaded columns of a 5x5 area, amortised to one per column when generating an area.
