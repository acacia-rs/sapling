# Where plants and redstone stand, and shapes that reconnect (2026-10-06)

Tags as in block-rules.md. **BDS** = bdsre on `acacia/.testserver/bds-molang/bedrock_server.exe` (26.50); labels
added this session (`names.user.json`): `WallBlock::tryFixWallStates` 1425e4e60, `WallBlock::_desiredConnectionState`
1425e4430, `WallBlock::_shouldBePost` 1425e39c0, `WallBlock::onPlaceBase` 1425e4820, `SugarCaneBlock::mayPlace`
1471ee4d0, `::tick` 1471edc40, `::randomTick` 1471ee070, `CropBlock::mayPlaceOn` 1471463b0, `::canSurvive` 147146150,
`::neighborChanged` 147146030, `::randomTick` 1471459b0, `RedStoneWireBlock::canSurvive` 144bc31e0. Other VAs are
unlabelled. Names of block globals were read off their static initialisers (`'minecraft:...'` next to the store).
**REG** = Geyser `bedrock/block_palette.26_50.nbt` (same names as sapling's `blocks.bin`, which has
`minecraft:connection_*` and `minecraft:corner`). **GEY** = Geyser `mappings/blocks.nbt` (Java state order to Bedrock
state). **JAVA** = `javap -c` on the 26.3 server jar (classes named), tags from its `data/minecraft/tags/block`.

Vtable slot numbers used below (BDS, checked against SugarCane overrides): 66 `mayPlace(region,pos)`, 67
`mayPlace(region,pos,face)` (calls 66), 68 `mayPlaceOn(region,belowPos)`, 74 `neighborChanged`, 121 `canSurvive`.
The base `mayPlace` 141a27520 = in build height + cell replaceable + `mayPlaceOn(below)`; no light, no support face.
"Support(face, type)" = `BlockType::canProvideSupport(block, face, BlockSupportType)`, `Center`=0 `Edge`=1 `Any`=2;
blocks with a `minecraft:support` component answer from it instead.

## Part A: items, blocks, ground

### A1. Item -> block (all items are in sapling's `ItemRegistry.bin` and Geyser's item list; REG default state = all zero)

| Item | Block, placed state | Ground (BDS rule) | Evidence |
|---|---|---|---|
| `wheat_seeds`, `carrot`, `potato`, `beetroot_seeds`, `torchflower_seeds` | `wheat`, `carrots`, `potatoes`, `beetroot`, `torchflower_crop`; `growth=0` | below is `farmland` only | BDS `CropBlock::mayPlaceOn`: surface 0 (`"supports_crops"`) = list 0x14ccd9548 = [`minecraft:farmland`]. JAVA `CropBlock` `SUPPORTS_CROPS` = farmland |
| `pitcher_pod` | `pitcher_crop` `growth=0,upper_block_bit=0` (one cell) | farmland | same CropBlock rule (`PitcherCropBlock` subscribes like CropBlock); JAVA same tag |
| `melon_seeds`, `pumpkin_seeds` | `melon_stem` / `pumpkin_stem` `growth=0,facing_direction=0` | farmland | BDS StemBlock vtable 14A8BCBD0 slot 68 = 1465fdb10, reads the farmland list 0x14ccd9548. JAVA `supports_stem_crops` = farmland |
| `nether_wart` | `nether_wart` `age=0` | below == `soul_sand` | BDS NetherWart slot 68 1465ed3e0 compares with 0x14cce4210 (`minecraft:soul_sand`) |
| `sweet_berries` | `sweet_berry_bush` `growth=0` | vegetation list (below); not into water or lava | BDS 1471eebb0 ctor -> vtable 14A8EDDD0: slot 66 1465ec830 (refuses when the cell's liquid layer is material 5/6 water/lava, then base rule), slot 68 = BushBlock list 14715cec0 |
| `cocoa_beans` | `cocoa` `age=0`, `direction` 0 log south, 1 log west, 2 log north, 3 log east | side face of `jungle_log`/`jungle_wood`/stripped variants; `direction` points at the log = opposite of the clicked face | GEY Java `facing` N,S,W,E -> 2,0,1,3; JAVA `CocoaBlock` (`SUPPORTS_COCOA` = `#jungle_logs`, facing toward the log). Bedrock cocoa ctor inlined; not decompiled |
| `glow_berries` | `cave_vines` (Java sets a random age 0-24; Bedrock not checked) | block above is `cave_vines`, `cave_vines_body_with_berries`, `cave_vines_head_with_berries`, or gives Support(Down, Center); no liquid in the cell | BDS CaveVines vtable 14A9455D0 slot 121 148b21770 |
| `sugar_cane` | `reeds` `age=0` | see A2 | BDS |
| `redstone` | `redstone_wire` `redstone_signal=0` (keep 0: no simulation) | see A3 | BDS |
| `repeater`, `comparator` | `unpowered_repeater` / `unpowered_comparator`; `minecraft:cardinal_direction` = facing.rs generic arm `[north,east,south,west][quarter]` (faces the placer); `repeater_delay=0`, comparator bits 0 | see A3 | block-rules.md section 1 (JAVA `DiodeBlock` opposite of look); GEY Java facing N -> `north` |
| `string` | `trip_wire`, all bits 0 | none: `mayPlaceOn` and `canSurvive` both return true | BDS TripWire vtable 14A7A3390: slot 68 and 121 = 1400a2880 (`mov al,1; ret`) |

Vegetation list (BushBlock `mayPlaceOn` 14715cec0 and crop surface 1 `"supports_vegetation"`, list 0x14ccd3328, 11
entries): `dirt`, `grass_block`, `podzol`, `coarse_dirt`, `mycelium`, `dirt_with_roots`, `moss_block`,
`pale_moss_block`, `mud`, `muddy_mangrove_roots`, `farmland`.

Light: placement never asks it (base `mayPlace`). `CropBlock::canSurvive` adds "raw light >= a per-block minimum
(field +0x3a4, value not recovered) or the column sees the sky"; `CropBlock::neighborChanged` only re-checks
`mayPlaceOn`, and `CropBlock::randomTick` checks light only to grow. Ignore light. (JAVA `CropBlock.canSurvive` =
light >= 8 or sky, and Java does refuse a dark placement: a Java/Bedrock difference.)

### A2. Sugar cane (`reeds`) - BDS `SugarCaneBlock::mayPlace` 1471ee4d0 (`canSurvive` = slot 121 -> same function)

1. below is `reeds` -> may stand (no other check).
2. else below must carry block tag `dirt` and not be `farmland` (exclusion list 0x14ccd9548), or carry tag `sand`.
3. the cell and the cell above must hold no liquid (`getLiquidBlock(pos)`, `(pos+1)`: `Material.mLiquid`), so no
   waterlogged/underwater cane.
4. one of the 4 horizontal neighbours of the ground block (x, z-1; x+1; z+1; x-1, all at y-1) has a liquid-layer block
   tagged `water`, or is `frosted_ice`.

Tag strings: 0x14ccd9518 = `"dirt"`, 0x14ccd9560 = `"sand"`, 0x14ccd9590 = `"water"`, 0x14ccdb400 =
`minecraft:frosted_ice`. Which blocks carry `dirt`/`sand` is set in the registration function 14849c7e0 (11 `addTag`
sites for `dirt`) and was not enumerated; JAVA `supports_sugar_cane` = `#substrate_overworld` + `#sand` is the
closest list: grass_block, dirt, coarse_dirt, podzol, mycelium, rooted dirt, moss, mud, muddy roots, sand, red_sand,
suspicious_sand. One bot sweep settles the Bedrock list.

Pop: `SugarCaneBlock::tick` (queued) and `::randomTick` both call `canSurvive` and on failure
`destroyBlock(pos, drop=1)`. Removing the water does not notify the cane (the water touches the ground, not the cane),
so on BDS it goes at its next random tick or the next update of a direct neighbour. Sapling can pop at once.

### A3. Redstone ground

| Block | Placement / survival (BDS) | Evidence |
|---|---|---|
| `redstone_wire` | no water in the cell (liquid layer material 5); below gives Support(Up, Center), or below is `glowstone`, `redstone_lamp`, `lit_redstone_lamp` | `RedStoneWireBlock::canSurvive` 144bc31e0; `mayPlace` 144bc32e0 tail-calls it; globals 0x14cce1348 / 0x14ccd91e8 / 0x14ccd9218 |
| repeater, comparator | below gives Support(Up, Any) | DiodeBlock vtable 14A850A50 slot 121 144bc7ec0 (`vt+0x90(block, 1, 2)`) |
| rails | place: below Support(Up, Center); survive: below Support(Up, Edge) + ascending rule (B2) | BaseRail vtable 14A7D5590 slot 66 142cd6c80, slot 121 142cd5660 |
| `trip_wire` | anything (A1) | |

Which blocks give which support was not decompiled per block. Expected (unverified): Center = something under the
middle of the face (full cubes incl. glass, top/double slabs, upside-down stairs, fence and wall tops); Edge = a full
top face. Sapling only knows `is_solid()`: use "solid, or a top/double slab, or upside-down stairs" for
wire, diodes and rails until a sweep records BDS's answers.

### A4. What pops a plant when its ground changes

| Block | Trigger | BDS |
|---|---|---|
| crops, `pitcher_crop` | a neighbour changes and `mayPlaceOn(below)` fails (farmland turned to dirt, ground broken) -> queued tick 1 later -> pop with drops | `CropBlock::neighborChanged` 147146030 (`fcn.1415f50f0(..., delay 1)`) |
| stems, nether wart, sweet berry | same, via the shared `neighborChanged` 141a367a0 (`canSurvive` = `mayPlaceOn(below)`, BushBlock 1465ed380) | |
| reeds | A2; also when the reeds below go (rule 1 fails) | |
| cave vines | own `neighborChanged` 148b21870; support above gone | |
| cocoa | log gone (own `neighborChanged`, header) | |
| wire, diodes, rails | shared `neighborChanged` 142cd1ec0 -> `canSurvive` | |

Sapling's `settle` pops only when the cell under turns non-solid: crops on farmland -> dirt, reeds losing water, and
nether wart on non-soul-sand are missed. Rule to add: on a change at P, re-run the placement test of the block at
P+up (and of the cocoa beside P, the cave vines under P) and pop on failure.

### A5. Levers, vines, wall banners, beds (2026-10-27, `bds_command_probe holds`, research/captures/bds-holds.txt)

Each support was `/setblock`ed to air. A lever on a stone's side and a wall banner on one popped a tick later with their
item. A vine on a stone's side went a tick later with no item and the vine hanging under it the tick after: a vine
keeps the faces with a solid block beside or that the vine over it has (JAVA 1.12 `BlockVine.recheckGrownSides`), and
goes with none. A vine under a stone, its only face on air, went with the stone. A bed's half set to air took the
other half in the same packet, with that half's break particles and no item. Sapling does all of this in the change's own tick (`blocks/updates.rs`).

`/setblock bed` put the head on the named cell and the foot behind it, both with `Bed` entities (color 14), and
refused a second `/setblock` on the head; sapling's `/setblock` puts the one half.

## Part B: shapes that reconnect

### B1. States (REG)

| Blocks | Shape states | Who sets them |
|---|---|---|
| every fence (`oak_fence` ... `nether_brick_fence`), `glass_pane` + stained panes, `iron_bars`, `trip_wire` | `minecraft:connection_{north,east,south,west}` 0/1 (trip_wire also `attached_bit`, `suspended_bit`, ...) | the server: data-driven trait `minecraft:connection` (`enabled_states: ["minecraft:cardinal_connections"]`, parser 1439ce480), applied by `BlockConnectionComponent::handleConnectionUpdate` on place and neighbour change. `FenceBlock::getConnectionsFromState` reads the states. |
| every stairs | `minecraft:corner` none/inner_left/inner_right/outer_left/outer_right, plus `weirdo_direction`, `upside_down_bit` | the server: placement trait `minecraft:corner_and_cardinal_direction` with `blocks_to_corner_with` = `q.any_tag('minecraft:cornerable_stairs')` (`StairBlock::_addHardCodedBlockComponents` lambda 146619710) |
| every wall (`cobblestone_wall` ... `sulfur_wall`; no `wall_block_type` any more) | `wall_connection_type_{north,east,south,west}` none/short/tall, `wall_post_bit` | the server: `WallBlock::tryFixWallStates` |
| fence gates | `in_wall_bit`, `minecraft:cardinal_direction`, `open_bit` | the server |
| rails | `rail` `rail_direction` 0-9; `golden_rail`, `detector_rail`, `activator_rail` `rail_direction` 0-5 + `rail_data_bit` | the server |

So the premise "fences/stairs/panes have no connection states" is false for 26.x: sapling must compute all of them,
or the client shows lone posts and straight stairs.

Value mapping (GEY): Java `north=true` <-> `minecraft:connection_north=1`; Java stairs `shape` <-> `minecraft:corner`
same names; Java gate `facing` N/S/W/E <-> same `cardinal_direction`; Java rail shape ordinal <-> `rail_direction`
(0 NS, 1 EW, 2 asc east, 3 asc west, 4 asc north, 5 asc south, 6 SE, 7 SW, 8 NW, 9 NE).

### B2. Who connects to whom (fences, panes, walls)

BDS mechanism (`_desiredConnectionState` 1425e4430, `ConnectionID.h`): each connecting block has a ConnectionID:
Fence 0 (wooden fences), NetherBrickFence 1, Wall 2, ThinFence 3 (panes, bars), TripWire 4. A neighbour accepts
block X from direction d when its connection rule component (permutation first, then block type) has a non-empty
accept mask containing X's id and enables the opposite of d. `accepts_connections_from`: `none` / `only_fences` /
`all`. Fence gates, stairs, tripwire hooks and six more classes rewrite the rule per permutation (callers of
1425ea120; gate = 1425e9c30, tripwire hook 14257cbe0 sets mask TripWire and one direction). Where vanilla assigns
the default rule to ordinary blocks was not found: use the Java lists.

| Block | Connects toward a neighbour that is | JAVA |
|---|---|---|
| fence | same family (any wooden fence with any wooden fence; nether brick only with nether brick), a fence gate whose axis is across the line, or a block with a sturdy face toward it that is not an exception | `FenceBlock.connectsTo`/`isSameFence` (`FENCES`, `WOODEN_FENCES`), `FenceGateBlock.connectsToDirection`, `Block.isExceptionForConnection` (leaves, barrier, carved pumpkin, jack o'lantern, melon, pumpkin, shulker boxes) |
| pane, iron bars | another pane/bars, a wall, or a sturdy face (not an exception) | `IronBarsBlock.attachsTo` (`WALLS`) |
| wall | wall, pane/bars, a gate across the line, or a sturdy face (not an exception) | `WallBlock.connectsTo` |

### B3. Walls (BDS, exact)

When: `WallBlock::onPlaceBase` (place event) and the queued tick (`tick` -> `tryFixWallStates(region,pos,3)`), which
`neighborChanged` schedules (`_tryAddToTickingQueue`). Server only (returns at once on a client level).

```
tryFixWallStates(pos):
  for d in [north, east, south, west]:              # table 14a7a9270: dir 2,5,3,4 -> state
      side[d] = desired(pos, d)
  post = shouldBePost(pos, sides)
  setBlock(pos, wall with side[*], wall_post_bit=post) if changed

desired(pos, d):
  n = block at pos+d
  if not accepts(n, Wall, from=opposite(d)): return none        # B2
  arm = ARM[d] + pos            # cell-relative boxes, y 0..1:
                                # north x .3125-.6875 z 0-.25; south z .75-1; east x .75-1 z .3125-.6875; west x 0-.25
  o = outline (selection box, vt slot 8 getOutline) of the block at pos+up, its minY -0.01 and maxY +0.01
  return tall if o intersects arm (strict <) else short

shouldBePost(pos, n,e,s,w):
  above = block at pos+up
  if above is a wall with wall_post_bit=1: return 1
  ns = n != none and n == s and e == none and w == none       # straight north-south line
  ew = e != none and e == w and n == none and s == none
  o = outline of above, minY -0.01, maxY +0.01
  if o intersects POST (x .25-.75, y 0-1, z .25-.75, at pos):  # 0x14ccd2808, init 14261bfb1
      if ns: return n == short
      if ew: return e == short
      return 1
  return not (ns or ew)
```

Differences from JAVA `WallBlock` (`shouldRaisePost`: `WALL_POST_OVERRIDE` tag, `isCovered` = full cover of a test
shape): Bedrock asks overlap with the selection box, not a tag or full cover: anything whose box touches the centre square
(a torch, a lantern, a full block) raises a post on a straight short wall, and any block reaching an arm's outer
quarter makes that arm tall.

Fence gate `in_wall_bit` (JAVA `FenceGateBlock.getStateForPlacement`/`updateShape`, not decompiled on BDS): 1 when the
neighbour on either side along the gate's line (gate facing north/south: west and east cells; east/west: north and
south cells) is a wall (`WALLS`). Recompute on neighbour change.

### B4. Stairs corner (JAVA `StairBlock.getStairsShape`, values mapped 1:1 by GEY)

`facing` = Java facing of the stair (`weirdo_direction` 0 east, 1 west, 2 south, 3 north); "stairs" = any block with
`upside_down_bit` + `weirdo_direction` (Bedrock tag `cornerable_stairs`), same `upside_down_bit`.

```
front = block at pos + facing
if stairs(front) and front.half == half and axis(front.facing) != axis(facing)
   and canTake(pos, opposite(front.facing)):
     return front.facing == ccw(facing) ? outer_left : outer_right
back = block at pos - facing
if stairs(back) and back.half == half and axis(back.facing) != axis(facing) and canTake(pos, back.facing):
     return back.facing == ccw(facing) ? inner_left : inner_right
return none
canTake(pos, d): n = block at pos+d; !stairs(n) or n.facing != facing or n.half != half
```
Run for the placed stair and its four horizontal neighbours after any change. `ccw` of north is west.

### B5. Rails (JAVA `RailState`, `BaseRailBlock`, `RailBlock`; BDS confirms support rules only)

Shapes: NS 0, EW 1, AE 2, AW 3, AN 4, AS 5, SE 6, SW 7, NW 8, NE 9. `straight` = golden/detector/activator (0-5 only).

```
connections(shape) at p:  NS {N,S}  EW {W,E}  AE {W, E+up}  AW {W+up, E}  AN {N+up, S}  AS {N, S+up}
                          SE {E,S}  SW {W,S}  NW {W,N}  NE {E,N}
isRail(q): block at q is any rail
railAt(q): first of q, q+up, q-down holding a rail -> RailState(q', shape) else null
hasConn(R, q): some c in R.conns with c.x == q.x and c.z == q.z       # y ignored
removeSoft(R): drop each c in R.conns where railAt(c) is null or !hasConn(railAt(c), R.pos); else c := that rail's pos
canConnectTo(R, other): hasConn(R, other.pos) or |R.conns| != 2
hasNeighborRail(q): r = railAt(q); r != null and (removeSoft(r); canConnectTo(r, self))

initial state (getStateForPlacement): EW when the placer looks east/west, else NS.
place(self, powered=false, always=true, default=current shape):     # onPlace -> updateState -> updateDir
  n,s,w,e = hasNeighborRail(pos+N/S/W/E)
  shape = null
  if (n||s) && !(w||e): NS ;  if (w||e) && !(n||s): EW
  if !straight:
     s&&e&&!n&&!w: SE ; s&&w&&!n&&!e: SW ; n&&w&&!s&&!e: NW ; n&&e&&!s&&!w: NE
  if shape == null:
     if (n||s)&&(w||e): default ; elif n||s: NS ; elif w||e: EW
     if !straight:                                   # unpowered order; last match wins
        if n&&w: NW ; if n&&e: NE ; if s&&w: SW ; if s&&e: SE
        (powered: SE, SW, NE, NW in that order)
  if shape == NS: if isRail(N+up): AN ; if isRail(S+up): AS
  if shape == EW: if isRail(E+up): AE ; if isRail(W+up): AW
  if shape == null: default
  set self shape; self.conns = connections(shape)
  if always or state changed:
     write it; for c in self.conns: r = railAt(c); if r: removeSoft(r); if canConnectTo(r, self): connectTo(r, self)

connectTo(R, other):                # neighbour re-shapes toward the new rail
  R.conns += other.pos
  n,s,w,e = hasConn(R, R.pos+N/S/W/E)
  shape = null; if n||s: NS ; if w||e: EW
  if !R.straight: s&&e&&!n&&!w: SE ; s&&w&&!n&&!e: SW ; n&&w&&!s&&!e: NW ; n&&e&&!s&&!w: NE
  if shape == NS: N+up rail -> AN ; S+up rail -> AS
  if shape == EW: E+up rail -> AE ; W+up rail -> AW
  if shape == null: NS
  write R with shape            # setBlockAndUpdate; does not recurse into place()
```

Removal: neighbours keep their shape (JAVA `affectNeighborsAfterRemoval` only sends updates: `pos+up` when the
removed rail sloped, `pos` and `pos-down` for straight rails). Plain-rail redstone switching (`RailBlock.updateState`
re-runs `place(.., always=false)` when 3 rails surround it) needs power: skip.

Pop (BDS `canSurvive` 142cd5660, = JAVA `shouldBeRemoved`): below lacks Support(Up, Edge), or shape AE and the block at
x+1 (same y) lacks it, AW x-1, AN z-1, AS z+1. Placement needs only Support(Up, Center) below (142cd6c80).
`rail_data_bit` stays 0 (no redstone; detector rails would set it while a minecart sits on them).

## Open checks (one bot session on BDS: place, read back)

1. Which blocks carry tags `dirt`/`sand` (sugar cane on podzol, mycelium, moss, mud, suspicious sand?).
2. Default connection rules: fence beside leaves, glass, stairs back, slab, chest; pane beside wall; wall beside fence.
3. Rails/wire/repeater on a fence top, glass, top slab, hopper (Center vs Edge vs Any).
4. Cave vines placed state (`growing_plant_age` random?) and whether glow berries go under a slab.
5. Stairs corner recomputed when a neighbour changes later (trait update), not only at placement.
