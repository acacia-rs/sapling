# Waterlogging and per-block liquid reactions, from BDS (bdsre)

Read 2026-10-06 from `acacia/.testserver/bds-molang/bedrock_server.exe` (build stamp `6ab54bee`, the same exe as
`water.md`; the old `bds/` path is gone, so bdsre was run with `BDS_EXE` pointing there). No `name` labels were added.
Per-block data: `crates/sapling-data/data/liquid-reactions.tsv` (name, reaction, can_contain). Read `water.md` first for the flow rule.

## Data layout (verified)

`DetectionRule` (LeviLamina `shared_types/v1_26_0/block/DetectionRule.h`, 5 bytes) is copied into
`Block::mDirectData.mWaterDetectionRule` = Block+0xb8:

| Block+ | field | values |
|---|---|---|
| 0xb8 | `canContainLiquid` | 0/1. The script error "Block type cannot be waterlogged." tests exactly this byte (147ea6fd9) |
| 0xb9 | `stopsFlowDirections` (face mask) | 0 for every vanilla block in code; `blocked_faces` in `water.md` |
| 0xba | `onLiquidTouches` | Broken 0, Popped 1, Blocking 2, Noreaction 3 |
| 0xbb | liquid type | 0 = water for every rule |
| 0xbc | clip against collider | 0 for every rule |

Where it comes from:

- `Block` constructor (142ce8bf0, second overload 142ce8e40) zeroes +0xb5..0xbc, then writes `+0xba = 2`. So a
  block without a liquid-detection component is **Blocking, cannot contain**. This is the default for everything
  not in the TSV (stone, dirt, glass, logs, planks, double slabs, sea lantern, beacon...).
- `BlockComponentDirectData::_finalizeInit` (145825650) copies the 5 bytes from the permutation's
  `BlockLiquidDetectionComponent`, else the BlockType's (`BlockType+0x30` storage), else leaves the default.
- Vanilla blocks get the component in the vanilla registration (14849c7e0, ~1.6 MB): each block that has one
  builds a `BlockLiquidDetectionDescription` (vtable 14a762ec0) on the stack with a 5-byte `operator new` and
  `mov dword [p], imm32; mov byte [p+4], imm8`, then calls slot 4 (141a5ef10, `initializeComponentFromCode`)
  with `BlockType+0x30`. 705 such sites in that function plus 4 helpers.

## How the TSV was made

Scratch scripts (not kept in the repo): every reference to vtable 14a762ec0 → the rule bytes stored just before it;
the block it belongs to = the nearest preceding reference to a block-name `HashedString` global in the
registration function. Names: the id initialiser stores FNV-1 64 of each name (`movabs rax, h; mov [rip+g], rax`),
matched against FNV-1 of every name in `acacia-world/data/blocks.bin`; all 82 hashes not in that list are tags
(`minecraft:is_*_destructible`, `stone_pick_diggable`, ...), so no unknown block can shadow a rule. Base blocks passed
in r8/r9 (stairs/slab base material) were excluded from the nearest-name search.

Fixes applied by hand after reading the code:

- `sea_lantern` collected 3 rules: they belong to `mangrove_roots`, whose registration (1487a62e0, legacy id 0x2e1,
  two experiment branches) passes no name at the call site. `sea_lantern` keeps the default.
- `beacon` (1487a5160, id 0x8a) and the `element_N` loop (1487adcb0) register without a name and add no rule.
- Helpers with their own rule (0x20001 = contain, Blocking): 1487b62b0 (applied after each of the 8 lightning
  rods), 1487cdeb0 / 1487cf650 / 1487d0c80 (registerBlock for the 8 copper bars / chains / lanterns).
- `air` (1419c15c0, BlockTypeRegistry setup of air/unknown/placeholder): contain 1, Noreaction.
- `underwater_torch`: reaction byte = 1 + EducationOptions flag (1403932f0): Popped normally, Blocking with education.

Confidence: the rule bytes are exact. The block association is by proximity; it is **high** where a whole class
agrees (all 65 `_stairs`, 69 `_slab`, 32 `_wall`, 22 `_door`, 39 signs, 21 `_trapdoor`, 15 `_button`,
17 `_pressure_plate`, 14 `_fence`, 12 `_fence_gate`, 13 `_leaves`, 34 panes, 16 candles, 13 shelves: contain +
Blocking; 18 `_carpet`, 30 corals/fans: contain + Popped; 10 `_sapling`: Popped) and **medium** for single blocks
(median distance from name to rule ~600 bytes, 99th percentile ~2.4 KB; only `leaf_litter` sits further, 2.6 KB).
Surprising rows worth an in-game check: `cactus`, `cake`, `bed`, `sea_pickle`, `kelp`/`seagrass` (contain + Popped),
`snow_layer`/`powder_snow`/`chorus_flower` (Broken), `straw_bed` (contain + Broken).

Summary of the TSV (736 names): contain + Blocking 541 · contain + Popped 72 · Popped 73 · contain + Noreaction 31 ·
Noreaction 15 (water, lava, flowing_*, cave/weeping/twisting vines, nether_sprouts, pale_hanging_moss, chemistry
tables) · Broken 3 · contain + Broken 1.

## Suggested labels

| VA | Suggested label |
|---|---|
| 142ce8bf0, 142ce8e40 | `Block::Block` (default rule) |
| 142ceef00 | `Block::_finalizeComponents?` (calls the direct-data finaliser once, sets +0xcc bit) |
| 145825650 | `BlockComponentDirectData::_finalizeInit` |
| 145870010 | `BlockComponentStorage::tryGetComponent<BlockLiquidDetectionComponent>` (type id global 14cd647e0) |
| 14a762ec0 | `BlockLiquidDetectionDescription::vftable` |
| 144c330a0 / 144c330b0 | `BlockLiquidDetectionDescription::getName` / `::initializeComponent` |
| 141a5ef10 | `BlockComponentDescription::initializeComponentFromCode(BlockComponentStorage&)` (shared slot) |
| 14849c7e0 | `VanillaBlockTypes::registerBlocks?` |
| 1425f41e0 | `VanillaBlockTypeIds` static init |
| 1409f59e0 | `LevelChunk::setBlock` |
| 1409f67c0 | `LevelChunk::setExtraBlock` (writes layer 1) |
| 142ce8a30 | `Block::canKeepAsExtraBlock?` (`keeps` below) |
| 1415f7260 | `BlockSource::setExtraBlock` |
| 14185bbc0 | `SubChunk::_setBlock` (assert string) |
| 1487b62b0 | lightning-rod liquid-detection helper |

## Placement rule (verified, `LevelChunk::setBlock` 1409f59e0)

`N` = block being written to layer 0, `O` = old layer 0, `E` = old layer 1. Runs on every layer-0 write.

```
keeps(N, C):                                     # 142ce8a30: may N sit on top of C kept in layer 1?
    if N.type != C.type:
        if C has <snow-related component 14cc3e308>: return 144c45b40(...) && N.type is TopSnow
        if N.type.isPreservingMediumWhenPlaced(C.type): return true          # BlockType vt+0x1c8
    return C.material.liquid && !N.material.liquid && N != air
           && C.material.type == Water && C.liquid_depth == 0               # water SOURCE only, never lava

set_layer0(N):
    if (!N.canContainLiquid && !(N.type.properties & TopSnow))                 # BlockType+0x130 bit 3
       or (O.type.canHaveExtraData() && !N.type.canHaveExtraData() && E has no <component>):   # vt+0x2c8
        E := air                                        # e.g. stone placed into water: water gone
    elif keeps(N, O):  E := O                           # (a) placed into a water source: source -> layer 1
    elif N is air:                                      # (b) breaking / clearing layer 0
        if E != air:
            if E is minecraft:water and O.stopsFlowDirections == 0x3f: E := flowing_water[same states]
            N := E; E := air                            # layer-1 water drops into layer 0
    elif !keeps(N, E): E := air                         # replacing a waterlogged block: keep E only if N can
    layer0 := N
```

So for sapling:

- (a) Placing a `can_contain` block into a water **source** (`liquid_depth` 0, in layer 0 or already in layer 1)
  keeps `minecraft:water[liquid_depth=0]` in layer 1. Into **flowing** water (depth 1..15) the water is
  discarded; it may come back by flowing (rule (c)). Lava is never kept. A non-`can_contain` block always clears
  layer 1 (top snow is the exception: it keeps plants under it, "snowlogging").
- Reaction does not matter for (a): a chest (contain + Blocking) placed into a source is waterlogged even though
  flowing water can never enter an empty chest.
- (b) Breaking (writing air over) a waterlogged block moves the layer-1 water into layer 0 unchanged (still a source
  if it was one) and empties layer 1. `air` itself is contain + Noreaction, which is why this branch is reached.
  Vanilla never sets the face mask, so the `flowing_water` conversion does not fire for vanilla blocks; the
  moved water is plain `minecraft:water` and setBlock itself queues no liquid tick (see Unverified).
- (c) Flowing water entering a cell (`water.md` `spread_to`, 148b5e580): Blocking → stopped; Popped → loot dropped,
  water replaces the block in layer 0; Broken → replaced, no drop; Noreaction → block stays, water written with
  `BlockSource::setExtraBlock` (layer 1). `can_contain` is **not** consulted on this path and
  `BlockSource::setExtraBlock` (1415f7260) only checks that the *water* may be an extra block (`BlockType+0x164`
  bit 7), so the Noreaction blocks with can_contain 0 (cave/weeping/twisting vines, nether sprouts, pale hanging
  moss) also end up holding water in layer 1. Layer 1 is then also checked by `spread_to` for Popped.

## Unverified

- Which BlockTypes override `isPreservingMediumWhenPlaced` (vt+0x1c8; base 140076010 returns false) and
  `canHaveExtraData` (vt+0x2c8). They can keep non-source water (or other media) in layer 1 on placement
  (likely the underwater plants and bubble column). Not enumerated.
- `BlockSource::setBlock` → `LevelChunk::setBlock` was inferred from the call graph (1409f59e0 is the chunk's layer-0
  writer and has the SubChunk write); the BlockSource wrapper itself was not decompiled, nor whether the player
  place/break paths add their own liquid handling (e.g. scheduling a tick on the freed water after a break).
- Bucket: emptying a water bucket into a `can_contain` block was not traced (`BucketItem`); expected to write a
  source into layer 1 via `setExtraBlock`.
- Snowlogging details: the component behind 14cc3e308 and 144c45b40 (a state that must equal 4) were not decoded.
- Per-row block association for single blocks is by code proximity (see Confidence); no in-game check was made.
- Experiment-gated rules: `mangrove_roots` has two branches (same rule); other experiment branches were not
  separated, so an experimental block's rule may differ from release only if both branches differ (none seen).
- Java 26.3 cross-check of the TSV was not done (the `research/java-truth` path was not found).
