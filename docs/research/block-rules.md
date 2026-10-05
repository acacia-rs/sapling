# sapling block rules vs vanilla (2026-10-05)

Tags: **BDS** = bdsre decompile of `acacia/.testserver/bds/bedrock_server.exe` (label/VA given; `?` labels are ported
hints, the constants were read from the pseudo-C). **BOX** = collision boxes per state in
`acacia-server/crates/acacia-world/data/blocks.bin` (property names and values are the registry's own). **REC** =
recorded against real BDS in the acacia repo. **JAVA** = Java 26.3 decomp, `research/java-truth/decomp/src/net/minecraft/world/level/block/<Class>.java`,
mapped to Bedrock states (Java-derived). **PM** / **DF** / **NK** = PocketMine-MP / dragonfly / PowerNukkit source as
remembered, not re-fetched: treat as medium confidence. No recorded BDS capture covers block placement states, door
sounds or tool wear; the cheapest hard check for every PM/DF/NK/JAVA line is one bot run that places the block and
reads the state back.

`quarter` below is sapling's `Click::quarter()`: 0 looks south, 1 west, 2 north, 3 east. Face order: Down 0, Up 1,
North 2, South 3, West 4, East 5.

## 1. facing.rs

| Rule | Verdict | Correct rule / evidence |
|---|---|---|
| `pillar_axis` from the clicked face's axis | CONFIRMED | JAVA `RotatedPillarBlock.getStateForPlacement` (clicked face axis). Also right for `iron_chain`, `hay_block`, `bone_block`, basalt, deepslate. |
| `weirdo_direction` `["2","1","3","0"][quarter]` | CONFIRMED | 0 east, 1 west, 2 south, 3 north = the direction the placer looks (the stairs rise that way). REC `acacia-bot/examples/course.rs:189-190` (Java `facing=east` = `weirdo_direction=0`, same course walked on BDS); JAVA `StairBlock` (`getHorizontalDirection()`). |
| `upside_down_bit` (stairs) / `minecraft:vertical_half` (slabs): Down face = top, Up face = bottom, side = click height > 0.5 | CONFIRMED | JAVA `StairBlock`, `SlabBlock.getStateForPlacement`. Height is the click's y inside the cell. |
| `upside_down_bit` on trapdoors, same rule | CONFIRMED | JAVA `TrapDoorBlock`; PM `Trapdoor::place`. |
| trapdoor `direction` `["3","0","2","1"][quarter]` | CONFIRMED | BOX: open trapdoor `direction=0` stands on the west edge, 1 east edge, 2 north edge, 3 south edge, i.e. 0 = Java east, 1 west, 2 south, 3 north; Java/PM facing = opposite of the look. (Java uses the clicked side face when a side is clicked; PM/Bedrock always the look. Keep.) |
| `torch_facing_direction` `[top,top,south,north,east,west][face]` | CONFIRMED for the five real faces | The value names the side the supporting block is on (Geyser: Java `wall_torch[facing=east]` = `west`). WRONG for `Face::Down`: a torch cannot hang from a ceiling; vanilla refuses the place (see place.rs, support check). Applies to `torch`, `soul_torch`, `redstone_torch`, `unlit_redstone_torch`, `copper_torch`, coloured torches. |
| ladder / wall sign `facing_direction` = clicked face index | CONFIRMED for faces 2-5 | BOX: `ladder[facing_direction=2]` sits on the south edge (hangs on the block to the south, clicked north face), 3 north edge, 4 east edge, 5 west edge. REC `course.rs:191-192` (`facing=west` = 4). WRONG for Up/Down clicks: sapling writes 0/1, states with no box; vanilla refuses a ladder there, and a sign item clicked on a top face places `standing_sign` instead (different block). |
| button `facing_direction` = clicked face index 0-5 | CONFIRMED | 0 under a ceiling, 1 on a floor, 2-5 on that face (Geyser `face=floor` = 1, `ceiling` = 0; PM `Button::place` facing = face). |
| `lever_direction` | CONFIRMED | Wall: the clicked face's name. Floor/ceiling: `up_`/`down_` + `north_south` when the placer looks north or south, `east_west` otherwise. JAVA `FaceAttachedHorizontalDirectionalBlock.getStateForPlacement` + Geyser lever mapping; PM `Lever::place`. |
| `ground_sign_direction` = round((yaw+180)*16/360) mod 16 | CONFIRMED | JAVA `StandingSignBlock` / `RotationSegment.convertToSegment`; PM `FloorSign::place`. Also right for `standing_banner` (same property, same rule: JAVA `BannerBlock`). |
| `quarter()` and `sixteenth()` use `round()` | WRONG (edge) | Vanilla is `floor(x + 0.5)` (JAVA `Direction.fromYRot`, `RotationSegment`). `round()` goes away from zero at negative halves: yaw -45 gives quarter 3 (east) where vanilla gives 0 (south). Use `(yaw / 90.0 + 0.5).floor()`. |
| `minecraft:cardinal_direction` `[north,east,south,west][quarter]` (front faces the placer) | CONFIRMED for furnace, lit furnace, blast furnace, smoker, chest, trapped chest, ender chest, stonecutter_block, lectern, carved/lit pumpkin, end_portal_frame, repeaters, comparators | JAVA `AbstractFurnaceBlock`, `ChestBlock`, `DiodeBlock`... (`getHorizontalDirection().getOpposite()`); PM same. |
| same rule applied to doors | WRONG | BOX: closed `wooden_door` `cardinal_direction=south` is the west-edge slab (= Java `facing=east`), `west` = north edge (Java south), `north` = east edge (Java west), `east` = south edge (Java north): the Bedrock value is the Java facing turned 90 degrees clockwise (PM `BlockStateDeserializerHelper::decodeDoor`: "a door facing east is actually facing north"). A placed door faces the way the placer looks and sits on the near edge (JAVA `DoorBlock`, PM `Door::place`). Correct table: `["west","north","east","south"][quarter]`. Sapling's table puts the leaf on a side edge. |
| same rule applied to anvils | WRONG | BOX: `anvil` north/south is long in z, east/west long in x. Vanilla turns it across the look: Java `AnvilBlock` `facing = look.getClockWise()`, PM `Anvil::place` `rotateY(look, true)`. Table: `["west","north","east","south"][quarter]` (same as doors). Also `chipped_anvil`, `damaged_anvil`. |
| same rule applied to fence gates | WRONG (minor) | Gate `cardinal_direction` = the look direction, not its opposite: `["south","west","north","east"][quarter]` (JAVA `FenceGateBlock`, PM `FenceGate::place`). Closed it looks the same; it decides which way it swings. `in_wall_bit` = 1 when a wall block is beside it on the gate's line (JAVA `isWall` check on the two sides; BOX has the state). |
| `facing_direction` of dispenser, dropper `["2","5","3","4"][quarter]`, up/down past 45 degrees | CONFIRMED (horizontal) | Front faces the placer, 2 north 3 south 4 west 5 east (JAVA `DispenserBlock`: `getNearestLookingDirection().getOpposite()`; NK `BlockDispenser.place`). Vertical: NK/old Bedrock decide by position (placer within 1 block horizontally and eye more than 2 above the block = up, eye below the block = down), Java by look; sapling's pitch rule is the Java one. Unverified on BDS. |
| same for `piston`, `sticky_piston` | WRONG (medium confidence) | Bedrock pistons store the horizontal value reversed: `facing_direction` 2 = head points south, 3 north, 4 east, 5 west (NK `BlockPistonBase.getBlockFace`: horizontal values are `getOpposite()`; `place` stores the placer's look direction). Head still faces the placer, so the stored value is the look direction: `["3","4","2","5"][quarter]`; 0/1 as now. Verify with one placed piston before relying on it. |
| observer | MISSING | Its property is `minecraft:facing_direction` with string values `down,up,north,south,west,east` (registry), so the `facing_direction` arm never matches. Rule: the observing face looks away from the placer = the look direction (JAVA `ObserverBlock`: `getNearestLookingDirection().getOpposite().getOpposite()`): `["south","west","north","east"][quarter]`, `down` when looking down, `up` when looking up. |

### Missing orientations (all keep the item's default state today)

| Block (properties) | Rule to add | Evidence |
|---|---|---|
| doors: `door_hinge_bit` | 0 = hinge left (BOX: Bedrock `south`+hinge 0 opens onto the north edge = Java `hinge=left`). Set 1 when the cell to the door's left (counter-clockwise of the look) holds the same door, or when the right neighbour is solid and the left is not. Write the same direction, hinge and open bit into both halves. | PM `Door::place`; JAVA `DoorBlock.getHinge` (adds a click-position tie-break) |
| `bed`: `direction` 0-3, `head_piece_bit`, `occupied_bit` | `direction` = `quarter` (0 south, 1 west, 2 north, 3 east = the look). Foot (`head_piece_bit=0`) in the clicked cell, head (`=1`) one cell further along the look; both cells must give way and both need a non-air block under them. Breaking either half removes the other; one bed item drops. | JAVA `BedBlock.getStateForPlacement`/`setPlacedBy`; PM `Bed::place` |
| `hopper`: `facing_direction` 0,2-5, `toggle_bit` | Spout points into the clicked block: `facing_direction` = opposite of the clicked face; Up or Down face = 0. | JAVA `HopperBlock`; PM `Hopper::place` |
| `barrel` (`facing_direction` 0-5), `end_rod`, `lightning_rod` | barrel: faces the placer incl. up/down (JAVA `BarrelBlock`: nearest look direction, opposite). end_rod / lightning_rod: = clicked face (JAVA `RodBlock`). Bedrock end_rod swaps the two vertical-pair values on some versions; check one placement. | JAVA |
| glazed terracotta `facing_direction` 2-5 | Faces the placer: `["2","5","3","4"][quarter]`. | JAVA `GlazedTerracottaBlock`; PM |
| `loom`, `beehive`, `bee_nest` etc. (`direction` 0-3) | Legacy numbering 0 south, 1 west, 2 north, 3 east; front faces the placer: `[2,3,0,1][quarter]`. | JAVA (`getOpposite()`); PM legacy horizontal |
| skulls (`facing_direction`), wall banner (`facing_direction`), shulker boxes (no property) | Skull: floor = 1 with the rotation in the block entity (`Rotation` float / `Rot` byte); wall = clicked face 2-5. Wall banner = clicked face (separate block `wall_banner`). Shulker box: facing lives in block entity `facing` byte = clicked face. | PM `Skull`, `WallBanner`, `ShulkerBox` |
| rails (`rail_direction` 0-9) | 0 north-south, 1 east-west, 2-5 ascending east/west/north/south, 6-9 curves. Placed: 0 when looking north/south else 1, then reshaped to join neighbouring rails. | PM `BaseRail`; JAVA `RailBlock` |
| campfire, grindstone (`attachment`,`direction`), bell, `amethyst_cluster` (`minecraft:block_face` = clicked face name), lantern (`hanging` = clicked Down face) | Listed for completeness; not checked. | registry property names (BOX) |

## 2. doors.rs

Sound packet for all rows below: `LevelSoundEvent`, position = block centre (x+0.5, y+0.5, z+0.5), extra data = the
block's runtime id (the id sapling sends in UpdateBlock), entity type empty, not baby, not global. Ids are the enum
order of `acacia-proto` `SoundType` (`generated/types/part_02.rs`): PowerOn 73, PowerOff 74, PressurePlateClickOff 448,
PressurePlateClickOn 449, ButtonClickOff 450, ButtonClickOn 451, DoorOpen 452, DoorClose 453, TrapdoorOpen 454,
TrapdoorClose 455, FenceGateOpen 456, FenceGateClose 457. 451 and 73 were matched against the constants in the BDS
functions below, which fixes the numbering.

| Rule | Verdict | Correct rule / evidence |
|---|---|---|
| Button stays in 20 ticks | WRONG for wooden | BDS `ButtonBlock::buttonPressed?` 141a32720: `addToTickingQueue(.., delay)` with delay `0x14` (20), or `0x1e` (30) when the block's flag at `this+0x378` is set. 30 for every wooden button (`wooden_button`, spruce/birch/jungle/acacia/dark_oak/mangrove/cherry/bamboo/pale_oak/crimson/warped `_button`), 20 for `stone_button` and `polished_blackstone_button` (JAVA `Blocks.woodenButton` 30 / `stoneButton` 20). |
| A pressed button ignores another press | CONFIRMED | JAVA `ButtonBlock.useWithoutItem` (`POWERED` = consume); PM `Button::onInteract`. |
| Button sound | MISSING | Press: `ButtonClickOn` (451). BDS 141a32720 calls the level-sound broadcast with `0x1c3`, pos+0.5, block id read from `Block+0x114`; `PowerOn` (0x49) only on the old-base-game-version branch. Release: `ButtonClickOff` (450) (enum pair; release path not decompiled). |
| Lever toggles `open_bit` | CONFIRMED | BDS `LeverBlock::toggle?` 142fe3770 flips the bit (`old ^ 1`). |
| Lever sound | MISSING | BDS 142fe3770: sound id = `old_open_bit ^ 0x1c3`: turning on = `ButtonClickOn` (451), turning off = `ButtonClickOff` (450), pos+0.5. Older-version branches use `PowerOn`/`PowerOff` (73/74) or a level event `0xdac` with data 600/500; not needed for a current client. |
| Doors, trapdoors, fence gates toggle `open_bit` by hand; door toggles both cells | CONFIRMED | REC `acacia-bot/src/pathfind/execute.rs:51-64` (bot clicks a door on BDS and waits for `open_bit` to flip), `pathfind/terrain.rs:139-144`; BOX has `open_bit` on every half. |
| Iron door / iron trapdoor do not open by hand; copper ones do | CONFIRMED | JAVA `BlockSetType.IRON` (`canOpenByHand=false`), `COPPER` (true). `name.contains("iron_")` is exact for today's names. |
| Door / trapdoor / gate sound | MISSING | `DoorOpen`/`DoorClose` (452/453), `TrapdoorOpen`/`TrapdoorClose` (454/455), `FenceGateOpen`/`FenceGateClose` (456/457), extra data = block runtime id so the client picks the wood/iron/copper/bamboo sound. DF `session/world.go` (`sound.DoorOpen{Block}` etc.); enum names in acacia-proto. Not decompiled; older servers sent LevelEvent `SoundDoor` (1003), which still plays a generic sound. |
| Fence gate swings away from the user | MISSING | On opening, if the user looks opposite to the gate's `minecraft:cardinal_direction`, set it to the user's look direction first. JAVA `FenceGateBlock.useWithoutItem`; PM `FenceGate::onInteract`. |
| Pressure plates | MISSING | One property, `redstone_signal` 0-15 (registry): 0 up, 15 down for wooden/stone/polished_blackstone. Sensor box x,z 0.125-0.875, y 0-0.25 (JAVA `BasePressurePlateBlock.TOUCH_AABB`). Wooden: any entity (items, arrows); stone, polished blackstone: living entities only. `light_weighted_pressure_plate`: signal = min(entities, 15); `heavy_weighted_`: ceil(entities/10), max 15. Re-check every 20 ticks while down (weighted 10) and rise when empty (JAVA `getPressedTime`; BDS `BasePressurePlateBlock` subscribes to queued ticks). Sounds `PressurePlateClickOn` 449 / `PressurePlateClickOff` 448. |
| Bed second cell | MISSING | See section 1 (bed). |

## 3. updates.rs

| Rule | Verdict | Correct rule / evidence |
|---|---|---|
| FALLS: sand, red_sand, gravel, `*concrete_powder`, dragon_egg | CONFIRMED | JAVA `FallingBlock` subclasses (`ColoredFallingBlock`, `ConcretePowderBlock`, `DragonEggBlock`). `suspicious_gravel` already matches the `gravel` ending. |
| `snow_layer` in STANDS (pops) | WRONG | On Bedrock snow layers fall: BDS registers `TopSnowBlock` for `BlockQueuedTickEvent` exactly like `FallingBlock` (labels `subscribe<TopSnowBlock,...>` 14a8f0320) and it implements the FallingBlock virtual `getDustParticleName` (`TopSnowBlock::getDustParticleName?` 1471fda10, a ported `?` label); PM `SnowLayer` is `Fallable` too. Move `snow_layer` to FALLS. |
| Other falling blocks | MISSING | `anvil`, `chipped_anvil`, `damaged_anvil`, `suspicious_sand` (`:sand` does not match it; it and `suspicious_gravel` break without a drop on landing), `scaffolding` (when its `stability` runs out), `pointed_dripstone` stalactites. JAVA `AnvilBlock`, `BrushableBlock`, `ScaffoldingBlock`, `PointedDripstoneBlock`. |
| A block falls when the cell below is air, liquid or replaceable | CONFIRMED | JAVA `FallingBlock.isFree` (air, fire, liquid, replaceable). Uses `gives_way`, so section 4's list applies. |
| Falls at once to the lowest free cell | WRONG (known TODO) | BDS removes the block and spawns `minecraft:falling_block` (`FallingBlockActor`, ticked by `FallingBlockNormalTickSystem` 1446c4fd0); the entity carries the block's runtime id in its `Variant` metadata. Java numbers, likely the same: starts 2 ticks after the update, gravity 0.04 per tick, drag 0.98. Landing on a cell that cannot be replaced and is not a full top (torch, slab bottom is fine, rail, etc.): the block drops as an item instead. Concrete powder touching water (placed or landed) becomes the matching `*_concrete`. |
| STANDS: doors, carpets, rails, saplings, short/tall grass, ferns, deadbush, listed flowers, pressure plates, standing signs, redstone wire, repeaters, comparators, crops, reeds, cactus, mushrooms, cake, flower pot | CONFIRMED | JAVA `canSurvive` of each class. Flower pots need support on Bedrock only (PM `FlowerPot::canBePlacedAt`). Crops need `farmland` specifically; saplings/flowers need dirt-like ground; these pop too when the block below changes to something else, not only to a non-solid. |
| STANDS misses | MISSING | Same rule for: `lever` on a floor, floor buttons (see next row), `standing_banner`, `*_standing_sign` is covered, `lily_of_the_valley`, `wither_rose`, `torchflower`, `sunflower`, `lilac`, `rose_bush`, `peony`, `pitcher_plant`, `*_eyeblossom`, `pink_petals`, `wildflowers`, `sweet_berry_bush`, `nether_wart`, `crimson_roots`, `warped_roots`, `*_fungus`, `nether_sprouts`, `bamboo`, `bamboo_sapling` (covered), `mangrove_propagule`, `melon_stem`, `pumpkin_stem`, `torchflower_crop`, `pitcher_crop`, `kelp`, `seagrass`, `sea_pickle`, `waterlily`, `*candle`, `tripwire`, `lantern` (`hanging=0`), `bell` (floor), `leaf_litter`, `short_dry_grass`, `tall_dry_grass`, `bush`, `firefly_bush`, `azalea`, `big_dripleaf`, `small_dripleaf_block`, `moss_carpet` (covered by `carpet`), `chorus_plant`, `chorus_flower`. |
| HANGS: ladder, button, wall sign by `facing_direction` = face index | CONFIRMED for the four side faces and the ceiling | BOX (ladder), section 1. |
| Floor button never pops | WRONG | `depended` tests `face == Face::Up` before HANGS, so a button with `facing_direction=1` standing on the removed block stays. Check HANGS with `facing_direction=1` in the Up arm as well. |
| Torch: pops when its `torch_facing_direction` block goes | CONFIRMED | Section 1 mapping; JAVA `WallTorchBlock.canSurvive`, `TorchBlock.canSurvive`. |
| HANGS misses | MISSING | `lever` (`lever_direction`: `north/south/west/east` = the face it is on, `up_*` floor, `down_*` ceiling), `wall_banner` (`facing_direction`), `tripwire_hook` (`direction` 0 south 1 west 2 north 3 east = the way it points), `frame`/`glow_frame` (`facing_direction`), `*_hanging_sign`, `lantern` with `hanging=1` and `bell` (ceiling) need the block above, `cocoa`, `vine`, `glow_lichen`, `sculk_vein`, amethyst buds/cluster (`minecraft:block_face`), wall coral fans, `pointed_dripstone`. Trapdoors, skulls, end rods and beds need no support. |
| Popped blocks drop their item (`Breaker::UNAIDED`) | CONFIRMED | Hand-break loot is right for a pop. Loot-table exceptions that must come from `loot`: `cake` nothing, `short_grass`/`fern`/`tall_grass` seeds by chance only, `deadbush` sticks, `vine`/`glow_lichen` nothing, a door or bed one item for both cells. |
| Support = `is_solid()` of the gone cell | WRONG (approximation) | Vanilla tests the face: torches, rails, doors etc. stand on any sturdy top face (top slabs, upside-down stairs, and on Bedrock fence/wall tops, glass, hoppers) and not on full transparent blocks such as leaves on Java. No recorded list; keep `is_solid` until a bot sweep records what BDS accepts. |
| Water washes away torches, rails, redstone, plants | MISSING here | JAVA `FlowingFluid.canHoldFluid`/`beforeDestroyingBlock`: non-solid, non-waterloggable blocks are broken with drops when water flows in. Belongs to `water::disturb`. |

## 4. place.rs

| Rule | Verdict | Correct rule / evidence |
|---|---|---|
| REPLACED: short_grass, tall_grass, fern, large_fern, deadbush, vine, fire, seagrass, plus air and liquids | CONFIRMED | JAVA `Blocks.java` `.replaceable()` on AIR, WATER, LAVA, SHORT_GRASS, FERN, TALL_GRASS, LARGE_FERN, DEAD_BUSH, VINE, FIRE, SEAGRASS. Replacing one half of `tall_grass`/`large_fern` must clear the other half (`doors::remove_other_half` works: they have `upper_block_bit`). |
| `snow_layer` at any height | WRONG (Java-derived) | Only a single layer is replaced: `height=0` (JAVA `SnowLayerBlock.canBeReplaced`: `layers == 1`; a snow item adds a layer instead, up to 8). Bedrock `height` 0-7 = layers - 1. |
| Replaceables not listed | MISSING | `soul_fire` (`:fire` does not match it), `structure_void`, `light_block_0`..`light_block_15`, `bubble_column`, `crimson_roots`, `warped_roots`, `nether_sprouts`, `glow_lichen`, `resin_clump`, `leaf_litter`, `hanging_roots`, `short_dry_grass`, `tall_dry_grass`, `bush`. JAVA `Blocks.java` `.replaceable()` (Bedrock names from the registry; each worth one bot check). |
| Clicking a replaceable block places into its cell | CONFIRMED | JAVA `BlockPlaceContext` (`replaceClicked`). |
| No support check when placing | MISSING | BDS refuses a place whose block could not survive there (`mayPlace`): torch under a ceiling or on air, ladder/wall sign on a top or bottom face, door or rail without a solid top below, plants on the wrong ground. Reuse the section 3 lists: if the placed block is in STANDS/HANGS/torch and its support cell is not solid, resync and do not place. Today such a block is placed and stays forever. |
| Slab onto the same slab | MISSING | Clicking the open half of a slab with the same slab item makes `<name>_double_slab` (e.g. `smooth_stone_double_slab`, keep `minecraft:vertical_half`) in that cell instead of placing in the next cell. JAVA `SlabBlock.canBeReplaced` + `getStateForPlacement`. |
| Entity check: only players, only for solid blocks | CONFIRMED (partial) | JAVA `BlockItem.canPlace` (`isUnobstructed` against the block's collision shape). Vanilla also refuses over any other living entity/boat/minecart; items and XP orbs do not block. Box 0.6 x 1.8 matches the player. |
| Survival place takes one from the hand; windows resent after a failed place | CONFIRMED | REC `acacia-server/docs/research/live-action-tests.md` fix 7 (BDS resyncs a block click made with an item that differs from its copy). |
| Two-cell blocks need the cell above free | CONFIRMED | JAVA `DoorBlock.getStateForPlacement`, `DoublePlantBlock`. Bed needs the cell ahead instead (section 1). |
| Items whose block has another name | MISSING (unverified) | Door, sign, bed, skull, banner, cake, hopper, repeater, comparator, flower pot, cauldron, brewing stand, reeds, crops are items on Bedrock, not block items: their `ItemV4.block_runtime_id` is 0, so `claimed_block`/`held_block` return `None` and nothing is placed. Needs an item-to-block table (`oak_sign` -> `standing_sign`/`wall_sign` by face, `wooden_door` item -> `wooden_door` block, `wheat_seeds` -> `wheat`, ...). Check against `sapling-data` ItemRegistry before building. |

## 5. mining.rs and inventory/wear.rs

| Rule | Verdict | Correct rule / evidence |
|---|---|---|
| Break ticks = ceil(hardness x 30 / speed), x 100 when the tool cannot harvest; Efficiency adds level^2 + 1 | CONFIRMED | REC: same formula as `acacia-bot/src/interact/break_time.rs`, whose breaks real BDS accepts (`docs/research/live-action-tests.md`). |
| Using that time as the minimum a client must wait | WRONG (risk) | break_time.rs line 8-10 says its sword/shears speeds are Java's and "never faster than Bedrock": they are an upper bound chosen for a bot. As a server-side floor they can reject a vanilla client that breaks faster (shears on wool/leaves, sword on cobweb/bamboo/plants). `LEEWAY` 3 does not cover it. Loosen the check for sword and shears (accept from 1 tick) until Bedrock's own speeds are recorded. |
| `BlockStartBreak` data = 65535 / ticks; `BlockStopBreak` on abort | CONFIRMED | PM `SurvivalBlockBreakHandler` (`65535 / breakTime`), DF `session/handler_player_auth_input.go`. |
| Wear 1 per block, sword 2, none for blocks of hardness 0, survival only | CONFIRMED | JAVA `Item.mineBlock` (`Tool.damagePerBlock`: 1, swords 2; only when destroy speed != 0); PM `TieredTool`/`Sword::onDestroyBlock`. BDS `ItemStackBase::hurtAndBreak?` 141a56c80 returns early for amount < 1 and for a player with the instabuild ability (creative). Adventure mode wears too. |
| Tool gone when `Damage + uses >= max` | WRONG | BDS 141a56c80: the item breaks only when `maxDamage < newDamage` (`if (iVar8 < iVar16)`), otherwise `Damage = newDamage`. A tool survives at `Damage == max` and breaks on the next use: wooden 60 uses, not 59. Use `worn > lasts`. |
| On break | MISSING | Stack count - 1 (BDS 141a56c80 sets `count - 1`), and `LevelSoundEvent` `Break` (5) at the player (DF `sound.ItemBreak`). |
| Durabilities: gold 32, wood 59, stone 131, copper 190, iron 250, diamond 1561, netherite 2031, shears 238 | CONFIRMED | `acacia-world/src/registry/mining.rs:94-121` (DF `item/tool.go`), used by the bot's repair costs against real BDS (`workstation/repair.rs` tests: iron pickaxe 250). With the `>` rule these give the 33/60/132/251/1562/2032 uses the Bedrock wiki lists. |
| `Damage` kept as an Int tag on the stack | CONFIRMED | REC `acacia-bot/src/survival/tools.rs:46-54` reads `Damage` (Int, sometimes Short) from BDS stacks; `docs/research/vanilla-actions-2026-10-02.md:281` (rod gets Damage 1 and a new stack id). Keep the rest of the tag; a changed stack needs a new stack network id. |
| Unbreaking | MISSING | BDS 141a56c80: level = enchant id `0x11` (17, Unbreaking); the wear is applied when `level < 1` or `random.nextInt(100) < item.getDamageChance(level)`; the whole amount is applied or skipped in one roll (not per point). `getDamageChance` (virtual, default 100): tools `100 / (level + 1)` in integer maths = 50, 33, 25 for I-III; armour `60 + 40 / (level + 1)` (formula from the Bedrock wiki / PM `Durable::getUnbreakingDamageReduction`, not decompiled). |
| `Unbreakable` tag | MISSING | BDS 141a56c80 looks up an 11-character tag (`Unbreakable`) on the stack's user data and skips all wear when it is set (byte 1). |
| Other wear | MISSING (known TODO) | Hitting an entity: sword 1, other tools 2; hoe tilling, shovel paths, axe stripping, flint and steel, shears on sheep: 1 each (JAVA item classes; PM same). |

## Corrections, by priority

1. Door `minecraft:cardinal_direction`: `["west","north","east","south"][quarter]` (today the leaf lands on a side edge); add `door_hinge_bit` (BOX).
2. Tool break test `worn > lasts`, not `>=` (BDS 141a56c80); add Unbreaking roll `nextInt(100) < 100/(level+1)` and the `Unbreakable` tag.
3. Refuse placement without support (torch on ceiling/air, ladder and wall sign on top/bottom faces, door/rail/plant without ground); today they place and never pop.
4. `snow_layer`: falls, does not pop (BDS TopSnowBlock is a falling block); replaceable only at `height=0`.
5. Wooden buttons 30 ticks (BDS 141a32720); floor buttons must pop with their block (`depended` Up arm).
6. Sounds: `LevelSoundEvent` ButtonClickOn/Off 451/450 (buttons and levers, BDS), DoorOpen/Close 452/453, TrapdoorOpen/Close 454/455, FenceGateOpen/Close 456/457, extra data = block runtime id.
7. Anvil `cardinal_direction` `["west","north","east","south"][quarter]`; fence gate = look direction `["south","west","north","east"][quarter]` plus swing-away on open.
8. Observer: handle `minecraft:facing_direction` (string values, = look direction). Piston/sticky piston horizontal `["3","4","2","5"][quarter]` after one bot check.
9. Mining floor too strict for sword/shears: accept early breaks there.
10. `quarter()`/`sixteenth()`: `floor(x + 0.5)`, not `round()`.
11. Add falling: anvils, `suspicious_sand`; replaceables: `soul_fire`, `light_block_*`, `structure_void`, nether roots/sprouts, `glow_lichen`, `leaf_litter`, `hanging_roots`, dry grass, `bush`.
12. Add orientations: bed (two cells), hopper, glazed terracotta, barrel, loom/beehive, end rod, skull/banner/shulker, rails; lever/wall banner/tripwire hook/item frame support; pressure plates; double slabs; item-to-block table.
13. Falling-block entity instead of the instant drop (known TODO); concrete powder hardening in water.

Open checks for one bot session on BDS (place, read the state back): piston facing numbers, dispenser/piston up-down rule
(position vs pitch), trapdoor placed on a side face, end rod values, snow-layer replace height, lever sound ids on the wire,
door sound packet, which blocks count as support.
