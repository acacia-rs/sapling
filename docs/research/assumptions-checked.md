# Assumptions checked against BDS (2026-10-24)

Six rules from NEXT.md §3 read in the decompile. Exe: `acacia/.testserver/bds-blocks/bedrock_server.exe`, image
6ab54bee, read with bdsre (`c`, `dis`, `win`, `vt`, `refs`, `callers`). Pack data: the vanilla behaviour packs'
`__brarchive` files, read with the `brarchive` reader in `crates/sapling-worldgen/scripts/bds_packs.py`.
Block materials: `crates/sapling-worldgen/scripts/data/block_types.tsv` (a live registry walk of this image;
column 6 is the material type).

| # | Rule | Verdict | Confidence |
|---|---|---|---|
| 1 | Mirrored shaped recipes | matches for vanilla; sapling ignores the per-recipe flag | high |
| 2 | Container close distance | unknown: no server distance close found | medium-low; drill needed |
| 3 | Orb pickup box | box matches; sapling's numbers are too wide | high on the box, medium on the numbers |
| 4 | Lava and non-collision blocks | differs | high |
| 5 | Items lava does not burn | close; differs on 2 items and a game rule | high |
| 6 | Burnable wooden blocks | matches except `stripped_bamboo_block` | high |

## 1. Mirrored shaped recipes

**sapling.** `crates/sapling-data/src/recipes.rs:146-161` `Recipe::is_laid_out` tries every offset `[false, true]`
mirrored. `lies_at` (127-139) mirrors with `width - 1 - across` and requires every cell outside the shape to be
empty. The shape is built at line 82, and the packet's `assume_symmetry` is not read.

**BDS.**
- `ShapedRecipe::matches(container, ctx)` **141b56750** (vtable 14a773010, which `Recipes::addShapedRecipe`'s
  lambda 141b6d510 stores):
  ```c
  cVar2 = fcn.141b575c0(this, grid, x, y, '\0');            // ShapedRecipe::matches(grid, xOffs, yOffs, xFlip)
  if (cVar2) return 1;
  if ((*(this + 0x100) == '\x01') && fcn.141b575c0(this, grid, x, y, '\x01')) return 1;
  ```
  The mirror is tried only when `+0x100` is set. That offset is `ShapedRecipe::mAssumeSymmetry` in the LeviLamina
  layout (Recipe ends at 0x100; `mWidth` 0x38 and `mHeight` 0x3c match the loop bounds).
- `matches(grid, x, y, flip)` **141b575c0**: the flipped index is `y*w + (w-1-x)`, the same as sapling's. It walks
  all 3x3 cells, so a cell outside the shape must be empty, as sapling has it.
- The JSON loader **141b44380** (at 141b46c75) defaults the flag to true and overrides it only by a bool
  `assume_symmetry`:
  ```
  mov byte ptr [rbp+0x320], 1 ; isBool? -> asBool(false) -> [rbp+0x320]
  ```
- No vanilla recipe sets the flag: across all 2061 `recipe_shaped` files in every vanilla pack, none has an
  `assume_symmetry` key. So every vanilla shaped recipe mirrors. The flag also travels in `CraftingData` for each
  shaped recipe (`acacia_proto` `ShapedRecipe.assume_symmetry`).
- Code-built shaped recipes go through 141b4d120, called from 141b40130, 141b410b0 and 141b40aa0. One of these
  calls passes 1; the other two pass a value from their caller, which I did not trace.

**Change.** Carry `assume_symmetry` from the `CraftingData` shaped recipe into `Shape` at `recipes.rs:82`, and try
`mirrored = true` only when it is set. For vanilla data this changes nothing, unless a code-built recipe ships
false; checking `CraftingData.bin` for false values would settle that. No drill is needed.

## 2. Distance at which BDS closes an open container screen

**sapling.** `crates/sapling-shard/src/containers/access.rs:36-38` closes when `blocks::reaches` fails. That is
`blocks.rs:85-91`: the eye to the block centre, at most `REACH` 7.0, or `CREATIVE_REACH` 13.0 in creative
(`blocks.rs:77-78`). The TODO is at `access.rs:3`.

**BDS.**
- `ContainerManagerModel::isValid(float pickRange)` is vtable +0x98. The doc's `ChestContainerManagerModel::isValid`
  142fbb3c0 tests `dist²(block centre, player pos) <= pickRange²`.
- I scanned the whole `.text` for slot +0x98 calls with `xmm1` set in the 10 instructions before (script in the
  session scratchpad, `slot98.py`). That found 27 sites. Only `broadcastChanges` **142fbad70** passes a range, and
  it passes `FLT_MAX`:
  ```c
  cVar7 = (**(*param_1 + 0x98))(param_1, 0x7f7fffff);   // isValid(FLT_MAX): existence and type only
  ```
  None of the other sites is a container model.
- `ServerPlayer::initiateContainerClose` **140756fb0** has one caller, **142fc5210**. That function is a callback
  that **142fc5600** subscribes on the container (`(*0x14a59a850)(container, ..., {vtable 14a7e9f40, 142fc5210,
  this})`): BDS closes the screen when the container goes away (the block is broken).
- `doDeleteContainerManager` (1407589e0 and its lambda 14a70e0a0): its callers are the disconnect path
  140983800, a container-list sweep 141036730, 1410266a0, and an ECS path 14104d640. None of them computes a
  distance.
- So the server has no distance close that I could find. Whether the vanilla client closes the screen itself
  past some range is not visible here.

**Confidence.** Medium-low. The scan misses a range set more than 10 instructions before the call, or passed
through a non-`rax` dispatch.

**Drill.** Open a chest on BDS, then `/tp` the player 8, 16 and 64 blocks away without breaking it, and record
`ContainerClose` from each side. Repeat with the chest broken; the breaking case should close (142fc5210).

**Change, if the drill shows no close.** Drop the distance close: `close_out_of_reach` and its call. Keep the
close when the block actor is removed or changes type.

## 3. Experience-orb pickup

**sapling.** `crates/sapling-shard/src/orbs.rs`:
- Lines 35-37 set `TOUCH_ACROSS` 1.55, `TOUCH_BELOW` -1.0 and `TOUCH_ABOVE` 2.3, measured from the feet.
- `touching` (136-143) is run from the orb's own tick (150), before the follow step.
- `SIZE` 0.25 at line 26 is marked unverified.
- `TAKE_EVERY` is 2 ticks (40).
- No age gate on a touch.

**BDS.**
- `Player::aiStep` **1402ad060** (the Player vtable slot after `pushActors`) ends with a call to **1402ac4c0**:
  ```c
  if (!level->isClientSide() && health(this) > 0 && this->canInteractWithOtherEntitiesInGame()) {   // vt +0x120
      pickupArea(this, &aabb);                                     // 1402ac2c0
      region->fetchEntities(&list, this, aabb, 0, 0);              // BlockSource vt +0xf8
      for (Actor* a : list) if (this->vt[0x6e8](a)) a->playerTouch(*this);   // vt +0x130 = playerTouch
  }
  ```
- **1402ac2c0**: when the player is not riding, the area is the player's AABB (`+0x220`) grown by **1 in x and z
  and 0.5 in y**:
  ```c
  fVar7 = box[5] + 1.0;  fVar8 = box[4] + 0.5;  fVar6 = box[3] + 1.0;  fVar4 = box[1] + -0.5; ...
  ```
  When riding a vehicle whose `+0x269` flag is clear, the area is the union of the player's and the vehicle's
  boxes grown by 1, 0, 1.
- `ExperienceOrb::playerTouch` **142f9e210** returns early in these cases: the level is client side; the player
  is not alive (vt +0x170); or `+0x3c4` is non-zero and a level lookup of it succeeds (not identified). Otherwise
  it applies Mending, gives the experience and removes the orb (vt +0x58). There is no age check, and no
  per-player delay is visible.
- `ExperienceOrb::normalTick` **142f9d480** keeps the follow-path take that sapling already has: `|gap|/8 <= 0.225`
  across, then `playerTouch` (`orbs.rs:163`).
- Slot names come from counting LeviLamina virtuals; slot 38 = `playerTouch` checks against the labelled
  function. The `+0x6e8` name (`isActorRelevant`) is a hint.

**Where sapling differs.**
- The touch runs from the player's tick in BDS, so its order within a tick and its once-per-player behaviour
  differ.
- The numbers. The test is an AABB overlap between the grown player box (0.6 x 1.8 grown to 2.6 across and
  [-0.5, 2.3] in y) and the orb's own box. For an orb box `w x h` with its position at the box's bottom centre,
  the orb is touched when `|dx| < 1.3 + w/2` and `-0.5 - h < dy < 2.3`. With `w = h = 0.25` that is 1.425 and
  (-0.75, 2.3). sapling has 1.55 and [-1.0, 2.3]. `drops.rs:197-201` already uses this form for items.
- Unverified: the orb's box size, and whether `fetchEntities` overlap is strict. The box is in the `xp_orb`
  entity JSON in `entities.brarchive`; it was not read here.

**Change.** Replace `touching` with the same overlap as `drops::reaches`:
`across = 0.3 + 1.0 + SIZE / 2.0`, and `dy` in `(-0.5 - SIZE, 1.8 + 0.5)`. Better still, run the touch from the
player's side, once per player tick, over orbs and items together. Read `xp_orb`'s `minecraft:collision_box` to
fix `SIZE`.

**Drill.** Only for strict versus inclusive overlap, and the 2-tick spacing (the `+0x3c4` lookup).

## 4. Lava and blocks without collision

**sapling.** `crates/sapling-shard/src/blocks/liquid.rs:79`: `(Cell::Block(_, solid), Liquid::Lava) => solid`,
where `solid` means the state has a collision box (`liquid.rs:96`, `acacia-world` `is_solid`). Lava also stops at
`is_solid` in `flow.rs:35`. Whatever is not stopped is burnt by `harden::fizz` (`flow.rs:102`).

**BDS.**
- `LiquidBlock::_isLiquidBlocking` 148b5ee50: lava returns `block.type.isLavaBlocking()` (vt +0xc8), and nothing
  else (buckets-lava.md).
- The default is `BlockType::isLavaBlocking` **141a28450** (dis):
  ```
  if (type->mThickness[+0x158] > 0) return false;
  return material[+3] /*blocksMotion*/ || material[+5] /*solid*/;
  ```
- The overrides were read off each type's vtable slot 0xc8 (worldgen-bds-liquids.md, "Lava", 2026-10-11).
  - **Return true**, so lava is stopped: bamboo, every candle and candle cake, campfire and soul campfire,
    ladder, scaffolding, glow lichen, sculk vein (Multiface), frame and glow frame, portal, end portal, resin
    clump. That is 46 types.
  - **Return false**, so lava burns them: the four rails, azalea and flowering azalea, big and small dripleaf,
    coral plants and fans (alive and dead), chorus plant and flower, powder snow, snow layers, lily pad, shelf
    mushrooms (WallFoliage). That is 48 types.
  - **Everything else uses the default.** It reads the material, not the collision box: torches, webs, levers,
    tripwire, hooks and redstone wire burn (their material neither blocks motion nor is solid).
- So sapling is wrong in both directions:
  - Lava burns ladders, scaffolding, candles, campfires, frames, lichen, bamboo and portals in sapling; BDS keeps
    lava out of them.
  - sapling leaves alone the rails, dripleaf, azaleas, coral, chorus, powder snow, snow layers and lily pads that
    have a collision box; BDS burns them.
- The worldgen crate already ports the rule: `crates/sapling-worldgen/src/feature/liquid/flow.rs:57-71`
  `lava_blocking`. Its base case uses `is_solid()` (collision) as a stand-in for the material. Material +3 is in
  `src/block/material_table.rs` (`OPEN_MATERIAL`).
- Not read: which types have `mThickness > 0`.

**Change.** Give the shard the worldgen rule. Move `lava_blocking` and its two name lists to `sapling-data`, or
generate a per-name table from `block_types.tsv` columns 2 (vtable), 7 and 8 (material +3/+5) plus the slot-0xc8
override target. Then use it in `liquid.rs:79` and `flow.rs:35` instead of `state.is_solid()`. No drill is needed.
A one-tick spot check of a rail, a ladder and a torch in a lava stream would confirm it.

## 5. Items lava does not burn

**sapling.** `crates/sapling-shard/src/drops.rs:55-59`: `ancient_debris`, or a name that starts with `netherite_`
and does not end in `_template`. It is used at `drops.rs:166-170`.

**BDS.**
- The flag is `Item::mFireResistant`: bit 0x20 of `Item+0x152`. The offset is from the LeviLamina layout and
  checks out, because `mBlockType` lands on +0x178 and the burn modifier on +0x1b0, as furnaces.md has them.
- It is set in two places:
  - Item registration **141fc2880**, 15 `or byte ptr [reg+0x152], 0x20` sites, after each `registerItem` call.
    The names were read from each item's HashedString global initializer:
    > netherite_sword, netherite_shovel, netherite_pickaxe, netherite_axe, netherite_hoe, netherite_ingot,
    > netherite_scrap, netherite_helmet, netherite_chestplate, netherite_leggings, netherite_boots,
    > netherite_horse_armor, netherite_nautilus_armor, netherite_block, ancient_debris
  - The data-driven component `minecraft:fire_resistant`, applied by **143030b50**
    (`flag = (flag & ~0x20) | comp.value << 5`, registered from `ComponentItemDataLoader::_parseLegacyComponents`'
    lambda_109). In the vanilla packs only `netherite_spear.json` carries it (`{"value":true}`, in
    vanilla_1.21.130 and 1.26.0).
- So 16 items in all. `netherite_upgrade_smithing_template` is not one; sapling already excludes it.
- `ItemActor::isFireImmune` **14598af40**:
  ```c
  if (item && (item->[+0x152] & 0x20)) return 1;
  if (actorFlag(this, 0x32 /*FireImmune*/)) return 1;
  return !gameRules[40].bool;      // only when the rule list has >= 41 entries, else 0
  ```
  Rule 40 is `AllowDestructiveObjects` in LeviLamina's `GameRuleId.h`. That is a hint: the enum order may differ
  in this build. When the rule is false, no dropped item burns.

**Where sapling differs.** sapling's name test accepts any `netherite_*`, which happens to include
`netherite_spear`. So the flagged list matches today's items, but only by prefix. sapling has no FireImmune actor
flag and no `AllowDestructiveObjects` rule.

**Change.** Replace the prefix test with the explicit list of 16 above, so that a new `netherite_*` item does not
silently become fireproof. If sapling has or gets the game rule, also return true when `allowdestructiveobjects`
is false. Drill: only to confirm rule 40's name (set it false, drop a stick in lava).

## 6. Burnable wooden blocks

**sapling.** `crates/sapling-shard/src/furnaces/fuel.rs`:
- `is_wood` (48-53): the tags `planks`, `logs_that_burn` and `wooden_slabs`; or a species name plus a
  `WOODEN_PARTS` suffix (16); or the `WOODEN` names (18-22).
- The nether exclusions are `FIREPROOF` name substrings (13, 44-46). Buttons are decided by name (60).

**BDS.**
- `getItemBurnDuration` 1457f2bb0 (furnaces.md) reads the block type's material: `Material::mType == 2` is Wood.
  Ladder is special-cased, and a 48-name nether list comes first.
- The Wood set comes from `block_types.tsv` column 6 (264 types). My comparison script is in the session
  scratchpad (`woodcmp.py`); it runs sapling's `is_wood` over every BDS type name.
  - **BDS Wood, not wood in sapling:**
    - The nether list (crimson/warped, chorus, mushroom blocks, creaking heart, bee nest, beehive): fuel 0 on
      both sides.
    - Door, sign, hanging-sign and banner blocks: their items are decided by the item factor (`item_first`,
      `fuel.rs:84`).
    - Double slabs: they have no item.
    - `camera`, `chalkboard` (Education).
    - **`stripped_bamboo_block`**: the one survival item that differs. BDS gives 1.5 (300 ticks in a furnace);
      sapling gives 0, because it is not in `logs_that_burn`, and `bamboo` + `_block` does not match the
      `stripped_` prefix.
  - **sapling wood, not BDS Wood:** `ladder` only, which BDS's rule e also gives 1.5. Same result.
  - **sapling `FIREPROOF` substring hits outside BDS Wood** (crimson/warped fungus, nylium, roots,
    `warped_wart_block`): 0 in BDS as well (no material rule, item factor 0). Same result.

**Change.** Minimal: add `"stripped_bamboo_block"` to `WOODEN` (`fuel.rs:18-22`). Exact: generate the Wood-material
name set from `block_types.tsv` column 6 into `sapling-data`, and test `material == Wood` in place of `is_wood`.
The tag and suffix heuristics can then go. The edu-only `camera` and `chalkboard` would then burn too, as on BDS.
No drill is needed. Whether door and sign items have a null `mBlockType`, which is why their own factor wins, is
furnaces.md's claim and was not re-checked here.
