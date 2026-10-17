# Bedrock buckets and the rest of lava, from BDS (bdsre)

Read 2026-10-06 from `acacia/.testserver/bds-molang/bedrock_server.exe` (the `bds/` copy is gone; this is the same
build: water.md's VAs, e.g. `_isLiquidBlocking` 148b5ee50, match byte for byte) with `py -3 bdsre.py c|dis|win`.
Entry point: the string `minecraft:water_evaporation_bucket_emitter` → `_emptyBucket`, its callers and the
`BucketItem` vtable; types and offsets from LeviLamina headers (`BucketItem.h`, `Block.h`, `Material.h`,
`IConstBlockSource.h`, `Actor.h`). No `name` labels were added; suggested labels below.
Block-name globals were resolved from their static initialisers (`movabs hash; mov [G], rax; lea r9, 'name'`).

| VA | Suggested label | What |
|---|---|---|
| 147a96a40 | `BucketItem::_useOn` | fill / empty on a clicked block (vtable ptr at 14a9096d0); folded with mob bucketing |
| 147a9a120 | `BucketItem::_emptyBucket` | places the contents; nether evaporation |
| 147a9ae40 | `BucketItem::_broadcastBucketEmptySound` | |
| 147a9cf20 | `BucketItem::_broadcastBucketFillSound` | by material of the taken liquid |
| 147a95f00 | `BucketItem::_replaceWithEmptyBucket` | |
| 147a99ce0 | `BucketItem::_takePowderSnow` | fill fallback when the cell is not a liquid source |
| 147a9b070 | `BucketItem::$dispense` | dispenser (not covered) |
| 147a95cd0 | `BucketItem::$getMaxStackSize` | 16 empty, else 1 |
| 147a95cf0 | `BucketItem::$isLiquidClipItem?` | true for fill types 0,1,6..11 (empty, milk, water, lava, powder snow); false for fish/axolotl/tadpole/sulfur |
| 141a86cd0 | `Item::_helpChangeInventoryItemInPlace` | replace-or-add-or-drop |
| 140d5d940 | `Actor::isCreative?` | game type 1, or 5 (default) with world default 1 |
| 1430afe60 | `GameMode::useItemOn` (existing label) | ClickBlock handling |
| 148b5eab0 | `LiquidBlock::_triggerFizzEffect` | lava fizz sound + smoke |
| 148b5ee50 | `LiquidBlock::_isLiquidBlocking` | lava branch: tail call `BlockType::isLavaBlocking` |
| 141a28450 | `BlockType::isLavaBlocking` | default implementation (vt slot 25 = +0xC8) |
| 148b5cd10 | `LiquidBlock::randomTick` | lava sets fire |

Globals: 14ccdaf88 water, 14ccdaf58 flowing_water, 14ccdafe8 lava, 14ccdafb8 flowing_lava, 14ccdb6e0 powder_snow,
14ccdb740 portal, 14ccdb0d8 bubble_column, 14ccdb870 snow_layer, 14ccdfb78 invisible_bedrock, 14ccf9950 air
(inferred, as in water.md). Fill-type → contents table 14a90c1c8 (index fill-2). `mFillType` = `this+0x218`:
0 empty, 1 milk, 2-5 fish, 8 water, 10 lava, 11 powder snow, 12 axolotl, 13 tadpole, 14 sulfur cube.
Offsets: `Block+0x71` cached `mIsSolid`; `+0xb8` `mCanContainLiquid`; `+0xba` `mOnLiquidTouches`
(liquid_reaction); `Material+2` mLiquid, `+3` mBlocksMotion, `+5` mSolid, `+6` mSuperHot; `BlockType+0x158`
`mThickness`. BlockSource vt: +0x10 getBlock, +0x20 getExtraBlock (layer 1), +0x28 getLiquidBlock, +0x60
getMaterial, +0x100 setBlock (layer 0), +0x108 setExtraBlock (layer 1), +0x110 removeBlock.

## Sounds and events (verified ids; acacia-proto names)

| When | Packet | Id | acacia-proto |
|---|---|---|---|
| fill water | LevelSoundEvent | 90 | `SoundType::BucketFillWater` |
| fill lava | LevelSoundEvent | 91 | `SoundType::BucketFillLava` |
| empty water | LevelSoundEvent | 92 | `SoundType::BucketEmptyWater` |
| empty lava | LevelSoundEvent | 93 | `SoundType::BucketEmptyLava` |
| (fish 213 `BucketEmptyFish`, powder snow 330/331, sulfur cube 608 `BucketEmptyLandAnimal`) | | | |
| water bucket in an ultrawarm dimension | LevelSoundEvent | 27 | `SoundType::Fizz` |
| ... and | spawn particle effect | `minecraft:water_evaporation_bucket_emitter` | SpawnParticleEffect packet |
| lava fizz (solidify, lava washing a block away) | LevelEvent | 1004, data = pitch*1000 | `LevelEventEvent::SoundFizz` |
| snow layer cleared under an emptied bucket | LevelEvent | 2001, data = block runtime id | `LevelEventEvent::ParticleDestroy` |

Bucket sounds (`broadcastSoundEvent(region, id, pos, data -1, no actor id, baby false, global false)`) are played at
the **player's feet** (`getAttachPos(ActorLocation::Feet)`), not at the block. The evaporation Fizz is at the block
centre (`pos + 0.5`); the particle effect at `pos` (integer corner, inferred from the float conversion).
The empty sound reads the material **after** placement: layer 1 if not air, else layer 0. In the nether nothing is
placed, so no BucketEmptyWater is sent there, only Fizz + particle. The strings `bucket.fill.water` etc. live only
in the LevelSoundEvent name table (14346a2b0); `ParticleFizzEffect` (2029) is not used by these paths.

Fizz helper 148b5eab0 (only on the public BlockSource, `region+0x1d`): LevelEvent 1004 at `pos + 0.5`, data
`((rand - rand) * 0.8 + 2.6) * 1000`; then 8x a legacy particle, `ParticleType 5` (Smoke) at
`(x + rand, y + 1.2, z + rand)` via Level vt+0x7e8 (slot not decoded: likely `addParticle`, which a server
may not send; if anything goes out it is `LevelEventEvent::AddParticleSmoke` 16389).

## How the client asks (Q5)

The bucket arrives as an item use **ClickBlock** (`InventoryTransaction` UseItem, or PlayerAuthInput's item
interaction) handled by `GameMode::useItemOn(item, pos, face, clickPos, clientBlock, ...)`. Buckets that clip
liquids (above) make the client's ray stop on liquid cells, so `pos` can be a water/lava cell. Server checks, in order:
1. `clientBlock` (if given) must equal the server's layer-0 block or `getLiquidBlock(pos)` (Block+0x108 compare);
   `invisible_bedrock` at `pos` fails.
2. Empty bucket only: fail if neither layer 0 nor the liquid block is a liquid material, the layer-0 block has an
   empty shape, it is not powder snow and not `canContainLiquid`.
3. Unless sneaking (and some item tags), `block.use()` runs first (cauldron etc.); if it handled the click, stop.
4. `Item::useOn` → `BucketItem::_useOn(instance, player, pos, face)`. In creative the item is copied first and
   restored only if `_useOn` failed.
sapling: hook this into `Request::click` (sapling-shard/src/blocks.rs), which today treats a non-block hand as
`interact`. ClickAir (`BucketItem::use`) was not decompiled; nothing suggests it places liquid.

## Pseudo-code (verified on BDS unless marked)

```
use_on(item, player, pos, face):                                       # 147a96a40
    if item.fill == EMPTY: return fill(item, player, pos)
    if item.fill == MILK: return FAIL                                  # milk is use()/drink
    clicked = block0(pos)
    t = pos
    if !(item.fill == WATER and clicked.canContainLiquid): t = pos + face   # waterlog only with water
    # (fish buckets keep t = pos when clicked is water/flowing_water)
    if block0(t.down) is snow_layer:
        level_event(2001 ParticleDestroy, t.down, runtime_id(snow)); remove_block(t.down); t = t.down
    contents = default_state(WATER -> minecraft:water, LAVA -> minecraft:lava)   # liquid_depth 0
    if !empty_bucket(contents, t, player, face): return FAIL
    empty_sound(player, t)                                             # 92 / 93 by material at t
    player.useItem(item, PourBucket(9), consume = true)                # CompletedUsingItem method 9
    if player is Player and !creative: replace_with_empty_bucket(item, player)
    return SUCCESS

empty_bucket(contents, t, placer, face):                               # 147a9a120
    old = block0(t); extra = block1(t)
    if !extra.material.liquid:                                         # a liquid in layer 1 skips these checks
        if old.material.solid and !(fill == WATER and old.canContainLiquid): return false
        if old.material == Portal or extra is not air: return false
    if !may_build(placer, t - face, face): return false                # BlockSource vt+0x1e8 permissions
    if dimension.ultraWarm and contents is minecraft:water:            # Dimension+0x1a6, inferred as in water.md
        sound(27 Fizz, t + 0.5); particle_effect("minecraft:water_evaporation_bucket_emitter", t)
        return true                                                    # bucket is still emptied
    if contents is lava or old is air or contents is flowing_lava or !old.canContainLiquid:
        if old is not air and old.material.solid: return false
        if old.liquid_reaction == Popped(1): drop_loot(old)            # as if broken, chance 1.0
        set(layer 0, t, contents, flags 3)                             # anything else non-solid: replaced, no drop
    else:
        set(layer 1, t, contents, flags 3)                             # waterlogged: clicked block stays
    schedule(t, contents is water ? 5 : lava ? (ultraWarm ? 10 : 30))  # BlockTickingQueue::add, immediately
    return true

fill(item, player, pos):                                               # empty-bucket branch of 147a96a40
    l = liq(pos)                                                       # layer 1 if liquid, else layer 0
    if l.material not in {Water, Lava} or l.liquid_depth != 0:
        return take_powder_snow(item, player, pos) ? SUCCESS : FAIL    # only flowing liquid: FAIL
    keep = l is water and (is_src_water(pos.up) or block0(pos.up).isSolid)
           and all(is_src_water(n) or block_at(n).isSolid for n in [down, -z, +z, -x, +x])
    if !keep:                                                          # Bedrock: no air pocket in the open sea
        if block1(pos) is air: set(layer 0, pos, air, flags 3)
        else:                  set(layer 1, pos, air, flags 3)         # takes water out of a waterlogged block
        if block0(pos) is bubble_column: set(layer 0, pos, air, flags 3)
    fill_sound(player, l.material)                                     # 90 / 91, at the player's feet
    if creative or !player: return SUCCESS                             # item unchanged
    item.count -= 1
    new = l is water ? water_bucket : lava_bucket                      # ItemStack names 14ccc95f0 / 14ccc9620 (inferred)
    if item.count == 0: item = new                                     # in place, same slot
    elif !player.add(new): player.drop(new, randomly = false)          # ItemAcquisitionMethod Filled(9)
    return SUCCESS

replace_with_empty_bucket(item, player):                               # 147a95f00
    if item.count == 0: item = minecraft:bucket
    else: player.add(bucket)                                           # return ignored (filled buckets stack to 1)
```

`keep` neighbours: `is_src_water(n)` = `liq(n)` is water with depth 0; `isSolid` is `Block+0x71` of `liq(n)`
(which is layer 0 for non-liquid cells). Lava sources are always removed.

## Lava: what blocks it, what it does on entry (Q6)

`blocking(q, from, face)` when `liq(from)` is lava (148b5ee50, `dis`): returns `block0(q).type.isLavaBlocking()`
and nothing else: no `liquid_reaction`, no `blocked_faces`, no `canFlowInto`.

```
isLavaBlocking(type):                                                  # 141a28450, default
    if type.thickness > 0: return false
    return type.material.blocksMotion or type.material.solid
```

Overrides (`dis`): 239 block vtables use the default, 18 return false, 14 return true, 1 forwards (141a827e0 →
1403932f0). LeviLamina declares the override on: AbstractCandle, Azalea, BambooStalk, BaseRail, BigDripleaf,
Campfire, ChemistryTable, ChorusFlower, ChorusPlant, CoralFan, CoralPlant, EndPortal, ItemFrame, Ladder, Multiface,
Portal, PowderSnow, Scaffolding, SmallDripleaf, TopSnow, UnderwaterTorch, WallFoliage, Waterlily. Which of
those say true was not mapped (no class names on the vtables).

On entry (`spread_to`, water.md): lava never overwrites lava; any non-air non-blocking `block0` gets
`fizz(n)` and is replaced by lava with **no drops** (the Popped/Noreaction branches are water-only), layer 1 is
not used. Entering a water cell then solidifies via `neighbor_changed` / `onPlace` (water.md).

## Lava fire (Q7, short)

`randomTick` 148b5cd10, lava material only: needs game rule #3 (doFireTick) and, when the rule list is long
enough, rule #40 (unidentified); a BlockSource predicate on `pos` must be false (unidentified). If `pos.up` is air or
`LavaFlammable::Always` (`Block+0xa6`): up to 10 tries, pick `pos + (rand3-1, rand3-1, rand3-1)`; if that block is
`LavaFlammable::Always`, set its first air neighbour (+x, -x, +y, -y, +z, -z) to the block 14ccdb710 (fire,
inferred) with flags 3 and stop. Differs from Java (which climbs upward).

## Answers

1. Fill takes from the clicked cell itself (`pos`, no face offset), via `getLiquidBlock`: so layer-1 water of a
   waterlogged block is taken (layer 1 set to air, block stays). Sources only (`liquid_depth 0`); flowing → fails.
   Water enclosed by sources/solid blocks (up + 5 others) is not removed. Powder snow: fallback; fish/milk: ignored.
2. Empty: the clicked block when it can contain liquid and the bucket is water (→ layer 1), else `pos + face`.
   A non-solid block there is replaced (Popped drops loot, others vanish); solid → fail; layer-1 non-liquid → fail.
   Nether water: Fizz 27 + `water_evaporation_bucket_emitter`, nothing placed, bucket still emptied.
   A tick is queued at once: 5 water, 30 lava (10 ultrawarm). Snow layer under the target is broken first.
3. Survival fill: stack -1, filled bucket replaces in place if the stack is gone, else added, else dropped.
   Empty: becomes `minecraft:bucket`. Creative: item never changes.
4. Table above.
5. ItemUse ClickBlock; see "How the client asks".
6. `isLavaBlocking` only; replaced with fizz, no drops.

## Unverified / inferred

- `isLiquidClipItem` slot identity (147a95cf0) inferred from behaviour; `Player::useItem` in creative assumed not to
  consume (moot: creative skips the replace); `Player::add` failure for the empty bucket is ignored (not decompiled).
- Which classes' `isLavaBlocking` return true; `mThickness` values per block; per-material `blocksMotion`/`solid`.
- Level vt+0x7e8 (8 smoke particles) and whether a server sends them.
- Game events posted (14cc8a91c on fill, 14cc8a9fc on empty; likely fluid_pickup / fluid_place) not named.
- `_emptyBucket` also fizzes if contents is `flowing_water` into a `mSuperHot` material: not reached by buckets.
- `BucketItem::use` (ClickAir), dispenser, cauldrons, fish/axolotl/tadpole/sulfur, powder snow, milk: not covered.
- Java cross-check not done: `research/java-truth` was not found on disk.
