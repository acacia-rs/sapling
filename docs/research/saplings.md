# Sapling growth (BDS, bds-blocks build)

Source: bdsre static decompile of `.testserver/bds-blocks/bedrock_server.exe`. Labels persisted with
`bdsre.py name`. [D] = decompiled, [I] = inferred.

## Functions

| VA | Label | Meaning |
|---|---|---|
| 148b3c7a0 | SaplingBlock::SaplingBlock | ctor(name, id, single?, singleWithBeehive?, quadruple?); stores the three optional feature names at +0x378/+0x3a0/+0x3c8; subscribes randomTick [D] |
| 14a94b1a0 | (vtable) | SaplingBlock vtable; slot 61 = onFertilized [D] |
| 148b3cda0 | SaplingBlock::randomTick | BlockRandomTickEvent handler [D] |
| 148b3ced0 | SaplingBlock::onFertilized | bone meal [D] |
| 148b3d630 | SaplingBlock::_tryAdvanceAgeBit | age_bit 0 -> set 1, return true; age_bit 1 -> return false [D] |
| 148b3d0d0 | SaplingBlock::_getQuadrupleSaplingOffset | optional<BlockPos> 2x2 offset [D] |
| 148b3d460 | SaplingBlock::_growTree | picks quad / beehive / single feature [D] |
| 148b3d900 | SaplingBlock::_placeTreeFeature | clear to air, place, restore on failure [D] |
| 148b3dd10 | SaplingBlock::_setSaplingCells | setBlock on 1 or 4 cells, flags 4 [D] |
| 148b57d80 | SaplingBlock::_isBeehiveFlower | flower predicate for the beehive check [D] |
| 1415f7870 | BlockSource::getRawBrightness | max(sky - skyDarken, block light); 0 if block light-dampening 15 [D] |
| 1416007e0 | BlockSource::fetchBlocks | blocks in a box matching a predicate [I name, D shape] |
| 141cd3970 | ActorGameTypeComponent::tryGetGameType | -1 if no component [D] |
| 148b655d0 | MangrovePropaguleBlock::randomTick | [D] |
| 148b65df0 | MangrovePropaguleBlock::_isHanging | [I from header] |
| 148b65f70 | MangrovePropaguleBlock::_growTree | [D] |
| 148b66820 | MangrovePropaguleBlock::onFertilized | [D] |
| 14580fc40 | AzaleaBlock::onFertilized | [D] |
| 14849c7e0 | (block registration) | calls the SaplingBlock ctor per species [D] |

## Species table (ctor args at 14849c7e0) [D]

| ctor call | block id | single | single with beehive | quadruple |
|---|---|---|---|---|
| 1485bdfb5 | 6 (oak) | random_oak_tree_from_sapling_feature | random_oak_tree_with_beehive_from_sapling_feature | - |
| 1485bed23 | 0x438 spruce | select_standing_spruce_tree_feature | - | mega_spruce_tree_feature |
| 1485bfa6d | 0x439 birch | birch_tree_feature | birch_tree_with_beehive_feature | - |
| 1485c07c3 | 0x43a jungle | select_undecorated_jungle_tree_feature | - | mega_jungle_tree_feature |
| 1485c1496 | 0x43b acacia | savanna_tree_feature | - | - |
| 1485c217d | 0x43c dark oak | - | - | select_roofed_tree_feature |
| 14857b15a | 0x322 cherry | cherry_tree_feature | cherry_tree_with_beehive_feature | - |
| 148600c63 | 0x4ed pale oak | - | - | pale_oak_tree_feature |
| 148629c79 | 0x576 ? | select_poplar_tree_feature | - | - |

All names carry the `minecraft:` prefix. Slot assignment checked from the has_value bytes for oak,
spruce, jungle, birch, dark oak, pale oak [D]; cherry and acacia by string order only [I]. Block id
0x576 is an unnamed sapling (poplar, probably education/experimental) [I]. Species by id: mapping of
0x438.. to spruce..dark oak follows the feature names [I].

Feature JSON (`definitions/features`) [D]:
- random_oak_tree_from_sapling: weighted 9 select_standing_oak_tree_feature : 1 fancy_oak_tree_feature.
- random_oak_tree_with_beehive_from_sapling: weighted 9 oak_tree_with_beehive_feature : 1
  fancy_oak_tree_with_beehive_feature.
- birch_tree_with_beehive: aggregate first_failure [birch_tree_feature, beehive_search_feature].
- select_standing_spruce / select_undecorated_jungle / select_roofed: aggregate first_success
  [optional_*_with_vines_feature, *_tree_feature].
- cherry_tree_feature, cherry_tree_with_beehive_feature, pale_oak_tree_feature have no JSON there
  (code-registered or in a brarchive) [I].
No biome check anywhere in the sapling path [D].

## Pseudo-code

```
randomTick(ev):                                   // [D] 148b3cda0
  if region.getBlock(pos).type != this: return
  if getRawBrightness(pos.above, dim.skyDarken, true) < 9: return   // raw >= 9
  if ev.random.nextInt(7) != 0: return            // 1/7
  if _tryAdvanceAgeBit(region, pos): return       // first success only sets age_bit
  off = _getQuadrupleSaplingOffset(region, pos)
  _growTree(region, pos, ev.random, single, beehive, quad, off)

_tryAdvanceAgeBit(region, pos):                   // [D] 148b3d630
  b = region.getBlock(pos)
  if b[age_bit] != 0: return false
  region.setBlock(pos, b.with(age_bit, 1), flags=1)
  return true

onFertilized(region, pos, actor, type):           // [D] 148b3ced0
  rnd = region.getILevel().getRandom()            // level random, not the tick random
  if single is none and beehive is none:          // dark oak, pale oak
    off = quadOffset(pos)
    if quad is set and off is none: return false  // bone meal not used
  else off = quadOffset(pos)
  creative = actor has game type and (gt == 1 or (gt == 5 /*default*/ and level default == 1))
  if type == Rapid or creative:                   // Rapid = FertilizerType 1
    _growTree(..., rnd, off); return true         // no chance, no age_bit
  if rnd.nextFloat() < 0.45 and !_tryAdvanceAgeBit(region, pos):
    _growTree(..., rnd, quadOffset(pos))
  return true                                     // consumed even on a miss

_getQuadrupleSaplingOffset(pos):                  // [D] same BlockType (state ignored)
  for off in [(0,0), (0,-1), (-1,0), (-1,-1)]:    // x,z; this scan order
    c = pos + off
    if c, c+x, c+z, c+x+z are all this type: return off
  return none

_growTree(region, pos, rnd, single, beehive, quad, off):   // [D] 148b3d460
  if <gate A false> and <ILevel gate B true>: return true  // see Unverified
  if quad set and off set: return _place(pos, off, quad, rnd)
  if beehive set and rnd.nextFloat() < 0.05:
    if any block in box (x-2,y,z-2)..(x+2,y,z+2) is _isBeehiveFlower:   // 5x1x5 at sapling y
      return _place(pos, none, beehive, rnd)
  if single set: return _place(pos, none, single, rnd)
  return false

_place(region, pos, off, name, rnd):              // [D] 148b3d900
  f = level.featureRegistry.lookup(name); if none: return false
  saved = region.getBlock(pos)
  _setSaplingCells(pos, off, air)                 // flags 4; 4 cells if off else pos only
  origin = off ? pos + off : pos                  // NW (min x, min z) corner of the 2x2
  ok = f.place(ctx{region, origin, rnd})
  if !ok: _setSaplingCells(pos, off, saved)        // all 4 cells get pos's block state
  return ok
```

Flowers accepted by `_isBeehiveFlower` (name-hash compare) [D, names resolved by hash of the
VanillaBlockTypeIds initialiser]: cherry_leaves, pink_petals, blue_orchid, allium, azure_bluet,
oxeye_daisy, cornflower, lily_of_the_valley, dandelion, wither_rose, torchflower, open_eyeblossom,
closed_eyeblossom, sunflower, rose_bush, pitcher_plant, chorus_flower, spore_blossom, a list global
(0x14ccd2590, likely poppy/tulips) and three ids that misresolved (0x14ccdb198 "portal",
0x14ccd2e78 "petrified_oak_double_slab", 0x14ccd2e18 unresolved; likely wildflowers/peony/lilac) [I].

### Mangrove propagule (own class, no age_bit) [D]
```
randomTick: if block != this: return
  if !_isHanging: if tick.nextInt(7) == 0: _growTree(region, pos, tick.random)   // no light check
  else if propagule_stage != 4: setBlock(stage + 1, flags 3)
onFertilized: if !canBeFertilized(above): false
  not hanging: Rapid or creative -> grow; else nextFloat < 0.45 -> grow; true
  hanging: advances stage (not read further)
_growTree: f1 = nextFloat, f2 = nextFloat
  name = f1 >= 0.85 ? (f2 < 0.01 ? mangrove_tree_with_beenest : mangrove_tree)
                    : (f2 < 0.01 ? tall_mangrove_tree_with_beenest : tall_mangrove_tree)   // *_feature
  setBlock(pos, air, 3); ok = place at pos; if !ok: setBlock(pos, saved, 3)
```

### Azalea / flowering azalea [D for AzaleaBlock]
onFertilized: Rapid or creative -> grow; else level random nextFloat < 0.45 -> grow. No age_bit, no
random-tick growth found. Places `minecraft:azalea_tree_feature` at pos; the sapling path's
clear-to-air step is absent. Flowering azalea uses the same class [I].

## Answers
1. Random tick: raw brightness at pos+1 >= 9, then nextInt(7)==0, then age_bit two-step: first
   success sets age_bit=1 (setBlock flags 1), next success grows. [D]
2. Bone meal: survival 0.45 per click (level random), then the same age_bit step, so ~2 successes.
   Creative (or FertilizerType::Rapid) grows at once, skipping chance and age_bit. Always returns
   true except quad-only species without a 2x2. [D]
3. See species table. Beehive variant: 5% (nextFloat < 0.05) and a flower in the 5x1x5 at the
   sapling's y; else the plain feature. Oak 9:1 fancy is a weighted_random_feature in JSON. [D]
4. 2x2: spruce, jungle (optional), dark oak, pale oak (required). Order (0,0),(0,-1),(-1,0),(-1,-1);
   origin = min corner; all four cleared to air (flags 4) before placing; failure restores all four
   with the original block at pos. A failed single restores pos. [D]
5. Yes, sapling cell(s) set to air before placing. No dirt write in the sapling code; any dirt under
   the trunk comes from the tree feature itself. [D for absence in sapling code]

## Unverified
- `_growTree` opening gate: Level+0xc0 vt 0xc30 false and ILevel vt 0x9e8 true -> return true
  without growing; likely a client-side / authority check. [I]
- setBlock flag meanings (1 for age_bit, 4 for clearing/restoring, 3 in mangrove). [I]
- The tree feature writes directly to the BlockSource (no transaction seen in the context ctor
  1438edb00), so a feature failing mid-way may leave partial writes; only the sapling cells are
  restored. [I]
- age_bit global is 0x14cd240a0, identified as age_bit only by usage. [I]
- IRandom slots: vt+8 = nextInt(n), vt+0x28 = nextFloat. [I]
