# BDS riding, boats and minecarts: ground truth for sapling (2026-10-25)

Tags: **BDSRE** (bdsre on BDS 1.26.52, `acacia/.testserver/bds-blocks/bedrock_server.exe`; VAs as given), **ED** (the
entity JSONs in BDS's own `behavior_packs/vanilla_1.26.*/__brarchive/entities.brarchive`; the newest pack wins),
**REC** (this repo's `research/captures/bds-ride.txt`, `bds-mounts.txt`, `bds-boats.txt`, recorded by the `sapling`
example `bds_ride_probe`), **CAP** (acacia's `docs/research/riding-fishing-elytra.md`, a Windows 1.26.52 client on BDS),
**LL** (LeviLamina headers), **LANG** (`resource_packs/vanilla/texts/en_US.lang`). Items under "Unverified" at the end
were not found in any of these; sapling does what is listed there.

## Who moves a vehicle

- The predicted flag is `VehicleInputIntentComponent` byte 3 (BDSRE):
  - `SetIsClientPredictedBoatSystem` 1483c6100: a boat or chest boat is client-predicted whenever a player drives it.
  - `SetIsClientPredictedHorseSystem` 1483c6d00: horse, donkey, mule, skeleton and zombie horse only when the driver is a
    player and the horse is TAMED (flag 28) and SADDLED (flag 8).
  - Everything else is server-simulated from the rider's `PlayerAuthInput` move vector and rotation: pig, strider,
    minecarts, camel, llama, untamed or unsaddled horses (CAP: a ridden pig's input has no `ClientPredictedVehicle`).
- A predicted vehicle's driver sends `ClientPredictedVehicle`, `vehicle_rotation` and `predicted_vehicle`, with the
  vehicle's position as the input's. BDS re-simulates the vehicle and corrects (CAP). Sapling takes the driver's
  position (riding/drive.rs; TODO there).
- `MoveActorAbsolute`, `PlayerInput` and `PassengerJump` from a rider: a vanilla client sends none (CAP); ignored.

## Links and seats

- `SetEntityLink {ridden, rider, type, immediate, rider_initiated, angular_velocity}`; type 0 removes, 1 is the
  driver's seat, 2 any other (LL). REC (bds-mounts.txt): a click mount sends `LevelSoundEvent mount`, the rider's
  seat `SetEntityData` and the link with `immediate` false, `rider_initiated` true.
- Remove link (BDSRE 1457bd1f0): `immediate` = the passenger is being destroyed; `rider_initiated` = it left by
  `Interact LeaveVehicle` or was bucked off. Sapling sends `immediate` when a rider is forgotten (killed or
  removed): `riding.rs` `forget`.
- Mobs shown with a mount (REC bds-h3herds.txt): the driver's `AddEntity` (seat items in its metadata), its
  equipment and its link, then the mount's `AddEntity` with the riders' links inside, the link again, then the
  mount's data. A passenger keeps its own place. Sapling: `mobs/linked.rs` (order, dressing) filled by
  `riding/announced.rs`; a player rider's seat still goes out as its own `SetEntityData`.
- A rider's data: `RiderSeatPosition` (seat in the vehicle frame plus the rider's height), `RiderRotationLocked`,
  `RiderMaxRotation` (the seat's `lock_rider_rotation`), `RiderSeatRotationOffset` (`rotate_rider_by`; the boat's −90).
- Rider height above its seat (REC): player +1.22001 (its wire position is at the eyes), zombie −0.5, cow 0.
  Wire position above the box's bottom (REC): boat +0.375, minecart +0.35.
- Seats (ED, `minecraft:rideable`; `[x, y, z]` in the vehicle frame; seat i is the i-th rider's):

| kind | seats | who | notes |
|---|---|---|---|
| boat | 1 rider [0,−0.2,0]; 2 riders [0.2,−0.2,0], [−0.6,−0.2,0]; turn −90, lock 90 | any | pull-in, width < 1.375; bamboo raft y 0.1 |
| chest_boat | [0.2,−0.2,0] lock 90 | any | pull-in; 27 slots, sneak-click opens |
| minecart | [0,−0.2,0] | any | pull-in; activator rail ejects |
| pig, saddled | [0,0.63,0] | player | `action.interact.ride.horse` |
| strider, saddled | [0,1.7,−0.2] | player | sneak skips |
| horse / skeleton / zombie horse | [0,1.1,−0.2] | player (+skeleton, zombie / +zombie) | `action.interact.mount` |
| donkey / mule | [0,0.925,−0.2] / [0,0.975,−0.2] | player | |
| llama, trader_llama | [0,1.17,−0.3] | player | not steerable |
| camel | [0,1.905,0.5], 2nd rider [0,1.905,−0.5] | player | pull-in |

- Mounting: an `ItemUseOnEntity` Interact with nothing that the entity's `minecraft:interact` takes in hand (CAP);
  eye to vehicle ≤ 6.7. `canAddPassenger_impl` 14241f7a0: not already riding, alive, not itself, a free seat, family
  allowed. Pigs and striders take players only saddled; a saddle is spent outside creative (ED `use_item`).
- Dismount: `Interact LeaveVehicle` naming the current vehicle (sent for sneak too; BDSRE 1457db120). BDS took the
  client's exit spot (CAP). When the server picks one, `ExitVehicle::search24_modern` 142cb6b60 (riding/exit.rs).

## Steering (ED)

- Pig: walks the rider's way while it holds a carrot on a stick; movement 0.25; the stick boosts ×1.35 for 3 s and
  costs 2 durability. Strider: warped fungus on a stick, ×1.45 mount speed, movement 0.16, lava 0.32; boost 16 s,
  cost 1. Horse jump strength 0.4–1.0 (donkey/mule 0.5); camel movement 0.09.

## Persistence

- The vehicle saves `LinksTag`: compounds `{entityID: Long (rider's UniqueID), linkID: Int}`; BDSRE loader
  140d4d820 makes `linkID` 0 the driver, anything else a passenger. A player saves `RideID` (BDSRE 1402af220).

## /ride (BDSRE setup 143afcd60, execute 143b01370)

- Permission level 1. Defaults: `teleport_rider`, `until_full`. start_riding: no target → `generic.noTargetMatch`;
  more than one ride → `startRiding.tooManyRides`; `teleport_ride` with several riders → `tooManyRiders`; not rideable →
  `notRideable`; full → `rideFull`; `if_group_fits` short of seats → `groupDoesntFit`; riders left over when it filled →
  `rideFullRidersLeft`; else `success`/`failure` with the names. REC has the answers of every overload.

## Boats

- Placing (BDSRE 142a853c0): only on the UP face with air above; at the clicked block plus the click position;
  yaw = player yaw + 90; refused where an entity is in the boat's box. Variant = wood: oak 0, spruce 1, birch 2,
  jungle 3, acacia 4, dark_oak 5, mangrove 6, bamboo 7 (raft), cherry 8, pale_oak 9.
- Integrity is the synched `Health` 40. A hit: `HurtDirection` flips, `HurtTime` 10, Health −= 10·damage; a creative
  hitter breaks it at once with no drop; else it breaks at ≤ 0. +1 a tick while 1 ≤ Health < 40.
- Breaking drops `<wood>_boat` (bamboo: `bamboo_raft`, chest: `<wood>_chest_boat`) under `doEntityDrops`; a chest
  boat spills its slots first.
- Unoccupied boats are simulated by the server with acacia-physics' `boat_tick` (friction 0.9 in water, buoyancy).
- Driven boats underwater 25 ticks throw their riders out (BDSRE 14250c800).

## Minecarts

- Kinds (ED): `minecart` (rideable), `chest_minecart` (27 slots, window `MinecartChest`), `hopper_minecart` (5,
  `MinecartHopper`), `tnt_minecart`, `command_block_minecart`. No furnace minecart (BDSRE type table 0x14a76dc48).
- Placing (BDSRE 141a6a620): only on a rail; refused if an entity is in the rail's cell; spawned at the cell's
  middle.
- Integrity as boats, Health −= 8·damage; mends +1 below 20. Drops the cart's own item.

### Rail physics (BDSRE 1418f9a60 and the systems it registers)

1. vy −= 0.04. On a rail (in the cart's cell or the one below) drag: 0.997 ridden, 0.96 empty, container carts
   0.98 + (15 − signal)·0.001.
2. Slopes: rising east vx −= 0.0078125, west vx +=, north vz +=, south vz −=.
3. The motion is turned along the rail (sign kept), |v| ≤ 2.0.
4. A rider holding forward (`move_vector.z` > 0) at |v|² < 0.01 pushes by 0.1 along its yaw and cancels the brake.
5. An unpowered powered rail halves |v|, or stops a cart with |v|² < 0.0009.
6. The move a tick is v·(0.75 with a rider), each axis capped at 0.4.
7. Going down gains |v| += (y_old − y_new)·0.05.
8. A powered rail adds 0.06 along v when |v| > 0.01; at rest on a straight rail it kicks 0.02 away from a solid block
   at either end.
9. Off a rail: each axis capped at 0.4, ×0.5 on the ground.

- Pushing (ED `legacy_minecart`): strength 0.1·min(1, 1/d); the cart takes half, the pusher a quarter.
- Activator rail powered: a minecart throws out its riders, a TNT minecart primes, a hopper minecart locks.
- Detector rail: pressed by a cart in its middle, checked again every 20 ticks from the press (group I below).
- TNT minecart: power 3, fuse 80; flint and steel primes; a fall > 3.5 bursts it at once; a blast primes it on
  10 + rand(20).
- Any other vehicle in a blast is hurt by the mob rule (drilled 2026-11-05, research/captures/bds-c1blast.txt,
  `c1blast`: a minecart and a boat at 6.0 to 7.8 off a TNT): harm = (i²+i)/2·7·8+1, with
  i = (1 − (distance − width/2)/8)·seen, from the TNT's feet + 0.06. A minecart loses harm×8 of its
  integrity, a boat harm×10 (truncated); HurtDirection flips as for a hit and it breaks at ≤ 0, its item
  and RemoveEntity the tick after. The fit is within 3 in total; the TNT had hopped ±0.2. sapling does
  the same (`vehicles.rs` `blasted`, `creatures/blast.rs` `harm_by`).
- Pull-in (BDSRE 14241fde0, from the push collision): a mob narrower than 1.375, not riding, with a free seat.

## Drilled 2026-11-04 (research/captures/bds-t2eboats.txt)

- A chest boat's rider's inventory key opened its slots as ContainerOpen `Container` (not `ChestBoat`)
  at the boat's cell, with the boat's id. Sapling sent `ChestBoat`; fixed (`vehicles/cargo.rs`
  `open_seated`, test `a_chest_boat_opens_to_its_riders_inventory_key_as_a_plain_container`).
- TNT 2, 4 and 6 blocks from three boats and three minecarts broke all six on both, each dropping its item.

## Unverified

- The links' `immediate`/`rider_initiated` for a non-click mount; type 2 for a second rider.
- A player rider for later viewers, drilled 2026-11-05 (`m3rideview`, thirteenth3m_drill.rs,
  research/captures/bds-m3rideview.txt; a watcher sent 3000 off and back to the actor in a boat): its
  `AddPlayer` carries the seat (`RiderSeatPosition` 0, 1.02, 0 and the rotation lock) and is followed
  at once by its `SetEntityLink`, then the boat's `AddEntity` with the link in `links`, nothing after it.
  Sapling sent the boat, then the seat's `SetEntityData`, then the link; now as BDS (`riding/announced.rs`
  files a player rider's seat and link as a mob's, `visibility.rs` `spawn_packets`; test
  `a_late_viewer_gets_a_player_rider_seated_then_its_link_then_the_boat`). BDS's ride start sets
  `FlagsExtended` bits 8 and 9 (acacia's shield `RAISED` and `MOVING`), told to the rider and its
  viewers; the dismount tells bit 9 alone, which stays, a later `AddPlayer` too. Sapling does so
  (`vitals/shield.rs` `tell_ride`, `shown_bits`, also in `visibility.rs` and `movement/crawl.rs`; test
  `a_rider_is_shown_with_its_shield_bits_and_the_moving_one_stays_after`). `/ride` onto a saddled pig
  failed on both.
- Detector rail timing, hopper minecart pick-up range: first drilled 2026-11-05 (`e2rails`,
  thirteenth2e_drill.rs); the activator lock is real: a hopper minecart on a powered activator rail took
  nothing, and its item some 3 ticks after the power went (fixed, `vehicles/cargo.rs` `take_up`). The
  rest is settled below ("Drilled 2026-11-05, group I").

## Drilled 2026-11-05, group I (`bds_command_probe i2rails`, `i2reach`, `i2reach2`; thirteenth2i_drill.rs)

- **Detector rail** (bds-i2rails.txt; `DetectorRailBlock` vtable 14A93CB20): `entityInside` of an
  unpressed rail runs `checkPressed` (148a1e7b0), which looks for minecarts in the cell's box x and z
  0.2 to 0.8, y 0 to 0.8; it presses (15) or lets go, and while pressed queues a tick 20 on whose
  `tick` (148a1ee00) checks again. A summoned cart pressed its rail a tick after its AddEntity; one
  standing 26 s and killed let go 520 ticks after the press, the 26th check; one rolled over the rail
  let go 20 ticks after its press. Sapling pressed at once and let go 20 ticks after it last saw a
  cart; fixed (`vehicles/detectors.rs`, tests `carts_drilled.rs`); re-drilled the same to a tick.
- **Killed carts drop their item**: `/kill` of a minecart left a minecart item on BDS, none on
  sapling; fixed (`creatures/killing.rs`).
- **Cart height**: summoned at y -26 a cart was shown at -25.65 (its box's bottom on the rail's
  floor) and from its first tick at -25.5, its box's middle at the rail's y + 0.5. Sapling kept it at
  -25.65; fixed (`vehicles/rails.rs` `RIDE`, test
  `a_minecart_rides_its_rail_with_its_middle_half_a_block_up`).
- **Hopper minecart reach** (`HopperSystem` 148489a30, `Hopper::_tryAddItemsFromPos` 1429cdfe0): a
  cart that took an item waits 4 ticks, a wait the cart's cell changing ends; it pulls from a box a
  block wide around its box's middle, from the middle up a block, then the block above that, then
  from its own box a quarter wider in x and z. BDS took items on ledges with their feet up to -23.625
  over a cart on y -26 (middle -25.5) and none from -23.5 up; items 0.55 to 0.95 east on the floor.
  Sapling's reach was its box, and its middle 0.15 low; fixed (`vehicles/cargo.rs` `reaches`).
  Floor items slide after `/tp`, so the east edge is only bounded (taken at 1.30 and 1.45, sliding).
- BDS took a reached item a tick after the `/tp`, sapling in the `/tp`'s tick (its commands run
  before the carts' tick); fixed 2026-11-05: a moved item waits out the tick (`vehicles/cargo.rs`
  `MovedBy`, test `a_hopper_minecart_takes_an_item_moved_into_its_reach_a_tick_after_the_move`).
- What raises a boat's underwater counter (not done): in bds-uuboat.txt a pig's and the actor's
  boats under water poured over them floated up and kept their riders 5 and 8 s, so the counter
  was never reached; a held-down boat and a paddling rider are still to drill.
- Boat fall breaking: none, as sapling (bds-uuboat.txt: boats summoned 20 and 6 blocks up landed
  whole, no item).
- A summoned boat on the ground (bds-uuboat.txt) is shown 0.375 up (-26 → -25.625) and shifted in
  x and z: (4.5, 4.5) → (4.7, 4.3), (2, 2) → (2.7, 2.7), (6.3, 2.8) stayed, (9.5, 4.5) → (9.7, 4.7);
  boats summoned in the air or in water stayed where summoned. Rule fitted (thirteenth2ao,
  bds-ao2boat.txt, 30 boats at x and z fractions .35 to .65, heights -26.5 to -20): the summon y is
  the boat's wire point, its box 0.375 lower, then pushed out of blocks as a summoned mob (see
  water-movement.md "Summoned into blocks"); vehicles.rs `summon`, body/spawn_push.rs. Sapling now
  matches 28 of 30; open: (-0.5, 4.5) and a lone (4.5, 4.5) went to z 4.3 where the fit gives 4.7.
  Traced, still open (thirteenth2ar): at z 4.5 the box's two overlaps are 0.2 each, and their f32 values
  (0.20000005 below, 0.19999981 above) send every z 4.5 the same way, so they cannot explain why x 9.5 went
  to 4.7 and x 4.5 and -0.5 to 4.3. The tie seems settled by something that depends on x; no drill was run.
  Drilled under gdb 2026-11-08 (thirteenth2ay `ay2gdb`, bds-ay2gdb.txt): (4.5, 4.5) → (4.7, 4.3), (9.5, 4.5)
  → (9.7, 4.7), (-0.5, 4.5) → (-0.3, 4.3) again, already so in the AddEntity. The Linux ELF's
  `MoveTowardsClosestSpaceSystemImpl` (vtable 0xefd8a70, per entity 0x74a90a0, a velocity push) never ran
  for these boats, and `ActorMoveSystemImpl::updateDepenetration` (adapter vtable 0xef8f6b8) is a later
  tick's: the shift is made on the spawn path before the AddEntity, not found. Found 2026-11-08
  (thirteenth2ba, watchpoints): the actor's initial move depenetrates over the touched block boxes last
  first, f32 ties deciding the z side; see water-movement.md "Summoned into blocks". Done.
- Horse and camel steering speeds and the jump charge (sapling: walk at movement speed, no charged jump).
  Drilled 2026-11-05 (group AI, `ai1ride`, bds-ai1ride.txt): a tamed, saddled horse is client-predicted;
  BDS took the bot's walk and 10-tick charged jump with one vehicle correction at the mount, as sapling
  takes the client's moves, so its steering is the client's. A `camel_saddled` camel seated no one on
  BDS (its `rideable` is in `camel_adult`, which that event does not add); sapling seated the bot,
  fixed (`riding/inventory.rs` `unrideable`). A grown camel (ak1camel, saddled by `/replaceitem`, walked
  south 3 s) is steered server-side on BDS (its rider gets its MoveEntityDelta) at 0.198 blocks a tick;
  sapling's walked it at 0.0175 (rein 0.09 through the legs' squared stride), now 0.303 to match
  (riding/mounts.rs `CAMEL_SPEED`). The 10-tick charge after `!stop` dashed on neither.
- Underwater counter, drilled 2026-11-05 (group AI, `ai1boat`): a pig in a boat in a pool under a
  stone roof floated up to the roof (boat -26.08 on both) and stayed seated 12 s on BDS, never thrown
  out; with its head in the roof it held its breath (Air 1 a tick) and was hurt every 10 ticks. Sapling
  harmed no seated mob; fixed (`creatures.rs`: a seated mob gets its harm tick at its seat).
- Single summons on the ground (bds-ai1boat.txt): (4.5, 4.5) → (4.7, 4.3), (2, 2) → (2.7, 2.7),
  (9.5, 4.5) → (9.7, 4.7), (3.5, -2.5) → (3.7, -2.3), (8, -3) → (8.7, -2.3); (6.3, 2.8), (12.25, 0.75),
  (10.9, 5.9) stayed. Each shift puts an edge of the 1.4 box on a block line; rule above.
- The horse screen: done without a capture, see mob-life.md "Mount inventories".

## Carts on carts (drilled 2026-11-05, `bds_command_probe l2carts`, thirteenth2l_drill.rs)

- **The slope that stayed** (bds-i2rails.txt): BDS's cart "at rest on a slope" sat on a rail that
  was no slope. A `/setblock` slope with `/fill`ed rails beside it was turned flat as a placed rail
  joins (Java `RailState`); sapling kept the set shape. Fixed: a commanded rail joins its neighbours
  (`blocks/commanded.rs`, `blocks/rails.rs` `join_placed`); the cart stays.
- **Against a block**: a cart at 0.4 a tick along a flat rail stopped at 12.51 against a stone at 13,
  motion zeroed; sapling's rolled through and climbed it. Fixed (`vehicles/bumps.rs` `walled`).
- **Cart into cart**: one rolling at 0.4 into one at rest stopped at its side (centres 0.98 apart),
  motion zeroed; touching carts then push apart as Java's `applyEntityCollision` for two minecarts,
  each pair twice a tick: each keeps 0.2 of its motion, takes their mean and ±0.05·min(1, 1/d) along
  their line, then rail drag ×0.96: 0.0576 the first tick, 0.0599 while they touch. Matches to 1e-4
  (`vehicles/bumps.rs`, tests `carts_drilled.rs`). The common part of their motion grows 1.38× a
  tick while they touch, so any unevenness shows: sapling's summoned cart lost 0.0075 to its snap up
  onto the rail (the downhill rule read the raw old y); now measured from the old spot on the rail's
  line, as Java's `getPos` (`vehicles/rails.rs` `roll`).
- **Parting** (bds-p2carts.txt, carts 0.2, 0.5, 0.7 and 0.9 apart): carts reach a fifth of a block across
  for others to push (Java's `grow(0.2, 0, 0.2)`): 1.0 apart, 0.02 short of touching, they pushed 0.0576,
  0.05386, 0.05171. Overlapping by 0.125 or less (0.045 to 0.1056 parted, 0.165 did not), a settled cart is
  moved out of the other by the whole overlap after its move and stops: the 0.4-apart pair's 0.1056 step
  (0.98 - 0.8744) and the 0.2-apart pair's 0.12588 then 0.04941. A fresh cart is not moved but is in the
  way (`vehicles/bumps.rs` `Parting`, tests `carts_a_block_apart_push_each_other_from_a_fifth_of_a_block_off`,
  `carts_overlapping_a_little_are_parted_at_once_and_stop`). A cart rolling into one at rest stops at its side
  only when it goes from out of reach straight to touching (BDS's from 1.223 apart); in reach first, it is
  pushed.
- A new cart is first pushed the tick after its AddEntity on BDS (both carts held that tick), in that
  tick on sapling; fixed 2026-11-05 (`vehicles/bumps.rs` push skips carts not yet `Settled`, test
  `a_new_cart_is_pushed_only_from_the_tick_after_its_first`).
- **Up a slope into a block** (bds-p2carts.txt): stopped at 12.51 against stone at 13 and rolled back; the box
  is shoved as if on the slope's top, its bottom at the rail's y + 1, then its y read off the rail (Java
  `moveAlongTrack`; `vehicles/carts.rs`, test `a_cart_running_up_a_slope_into_a_block_stops_against_it_and_rolls_back`).
- **Stored `Pos`**: BDS kept a minecart on a rail at y -26 at -25.5, where it was shown
  (bds-l2carts-dump.txt), as sapling does (`actors.rs` `compound_now`, the wire's lift 0.35).

## Drilled (2026-11-04, `bds_command_probe n3stick`, `n3exit`; examples/shared/ninth3_drill.rs)

- **The carrot stick** (research/captures/bds-n3stick.txt, a saddled pig penned, then one falling
  from y 60): each use boosted it, heard as `boost` at the pig's head, and wore the stick 2 (21, 23,
  25); the third broke it (`break` at the rider's eyes) and left a fishing rod in the hand (the
  rider's interact text became `action.interact.fishing`). A pig falling through the air boosted
  too. Sapling wore the stick but made no sound and left an empty hand; fixed in
  `riding/mounts.rs` `boost` and `inventory::wear_hand_leaving`, test
  `a_worn_out_carrot_on_a_stick_boosts_once_more_and_leaves_a_fishing_rod`. Re-drilled
  (sapling-n3stick.txt): the three boosts heard at the pig's head as BDS's, the rod left, the
  falling pig boosted. Left: BDS showed the watcher a `MobEquipment` with each new wear, sapling
  does not; BDS heard the `break` 0.5 higher (the seated rider's eyes). Fixed 2026-11-04: a wear
  that leaves the stick in hand shows the rider's watchers its hand (`inventory/wear.rs`
  `wear_hand_leaving`; BDS's came 1 to 4 ticks after the boost, sapling's in its tick), test
  `a_carrot_on_a_stick_a_boost_wore_is_shown_to_the_rider_s_watchers`. The `break` height is BDS
  hearing it at a rider position older than the pig's bob (-23.65 against the MovePlayer's
  -24.15 at that tick in bds-n3stick.txt), not a rule: left.
- **Rearing** is cleared the tick the ridden horse lands (riding/rearing.rs `grounded`, test
  `a_ridden_horse_rears_from_its_jump_until_it_lands`), as research/captures/bds-horse-jump.txt had it.
- **Exit spots** (bds-n3exit.txt, a saddled pig with a block of twelve kinds on its north side, the
  rider let off by `/ride stop_riding`): with a still pig BDS tries north first. Water, fire,
  cobweb, a ripe berry bush, powder snow, a wither rose, and a magma floor all let it off north
  into that cell, as sapling does; stone and a hole in the floor sent it south, as sapling.
  Differences left (exit.rs TODO): next to lava and to a campfire BDS sent no correction at all
  (the client stayed put; in lava it caught fire); by a ladder it was put at (-0.94, -0.06) from the
  seat; and BDS's correction kept the rider 0.16 to 0.23 above the floor where sapling puts it on
  the floor. Off a harnessed happy ghast (slowed) BDS put the rider on top of it (`on_top_center`):
  see "Happy ghast" below.
  Re-drilled 2026-11-07 (group AK, `ak1exit`, eleven still pigs with blocks north and south; the
  ninth wave's "no correction" and ladder readings came from its negative-x cells, `{x}.5` being the
  wrong cell there): BDS let the rider off north into a ladder's cell for every facing, onto a
  campfire north (feet at its 0.4375 top), and into lava north, or south past stone with lava south,
  0.23 above the floor; with stone north and a campfire south it sent no correction. Fixed: a cell
  blocks only through boxes under the rider's middle (exit.rs `top`), so a ladder on the side no
  longer sends it south; re-drilled (sapling-ak1exit.txt) every spot matches but two (exit.rs TODO):
  by lava sapling's rider is on the floor, BDS's 0.23 up, and with stone north and a campfire south
  sapling steps it onto the campfire.
  Settled 2026-11-07 (group AQ, `aq3exit` under gdb on ExitVehicleSystemImpl, research/captures/
  bds-aq3exit.txt): BDS's exit search put the rider on the floor of the lava cell like any other
  (final eye -24.379 for lava north, lava south past stone, and nothing about), but its correction then
  kept the height the client was seated at (eye -24.14999), with lava's -0.02 fall and off the ground;
  where nothing was about it sent none. Sapling now corrects a rider let off into lava so
  (exit.rs, movement `push_into_lava`). The campfire case was the drill's pig: it strolled south
  before slowness took hold and the campfire set into it pushed it on, so BDS let the rider off north
  onto the campfire by the same rule sapling uses. Off a happy ghast see "Happy ghast" below.

## Happy ghast (drilled 2026-11-07, group AS: `as3ghast`, research/captures/bds-as3ghast.txt)

- Unharnessed it has no `rideable`: `/ride` answered `notRideable` (riding.rs `seats_left`).
- A click holding a harness: `MobArmorEquipment` with it in the body slot, `armor.equip_generic` at its
  middle (feet + 2), and `Flags` with `FlagsExtended` gaining bits 58, 59 and 61. Shears on one no one
  rides: the body slot emptied, the harness tossed 5.125 over its feet, `armor.unequip_generic`, and
  bit 59 stayed. Both in riding/harness.rs.
- Seats (`adult_harnessed` `rideable`): four at (0, 3.8, 1.7), (-1.7, 3.8, 0), (0, 3.8, -1.7),
  (1.7, 3.8, 0); a player's seat item is `RiderSeatPosition` (y 5.02001), `RiderMaxRotation` 181,
  `SeatThirdPersonCameraRadius` 8 and `SeatCameraRelaxDistanceSmoothing` 6. The first rider: seat,
  `mount`, `attach`, link, `AmbientSoundInterval` 30 (the n3exit run heard `attach` a tick later). A
  second: seat, `mount`, the driver's link again, its own (type 2); no one else's seat. Every seating on
  any vehicle tells all riders' links again (as bds-ride.txt's boat did); seats that did not move are not
  told (riding/told.rs).
- One getting off of two: its link 0, then the other's link again. The last: link 0, the rider corrected
  onto its top at its middle, feet 4.025 over its feet (4.0247 in bds-n3exit.txt, by `/ride`, and 4.0250
  here, by the client's leave; exit.rs `ON_TOP_LIFT`), `detach` and `AmbientSoundInterval` 5.
- Steering: the server moves it (no `ClientPredictedVehicle`): each tick 0.91 of its speed plus its
  `flying_speed` 0.016 the way its driver looks, pitch and all (`free_camera_controlled`, backwards at
  0.5). Looking straight up it rose 0.016, 0.03056, 0.04381 ... and coasted down at 0.91 after the keys
  let go; sapling's sapling-as3ghast.txt run matched every `MoveEntityDelta` to the last digit
  (harness.rs `flight`, FLYER physics).
- Ported 2026-11-08 (group AV, test file integration/thirteenth3as.rs): the `become_immobile` and
  `become_mobile` sensors (riding/stilled.rs: a live non-spectator player not riding a happy ghast within
  3 across and 1.5 up or down of feet + 4.5 stills it; none within 3.5 and 2 frees it). Stilled: `Flags`
  and `FlagsExtended` with bits 54, 56 and 57, property `can_move` 0, then `UpdateAttributes` with two
  `minecraft:movement` entries (0, or 0.016 freed), and it stops steering. A late viewer's `AddEntity`
  carries the harness and still bits (`ghast_flags`, creatures/shown.rs). The last rider's correction
  onto its top has delta 0 (movement `push_unmoving`), and `detach` with `AmbientSoundInterval` come a tick
  after the link. Shears: `MobArmorEquipment`, the harness item, the sound, then `FlagsExtended` alone.
- `vertical_movement_action`, traced 2026-11-08 (thirteenth2ay `ay2gdb`): `VerticalMovementActionSystem`'s
  per-entity tick (Linux ELF 0x730c230) adds the pack's `vertical_velocity` (0.5) to the vehicle's input
  intent y (+0x18) when the controlling passenger's `MoveInputComponent` flag word (+0x60) has bit 0x10.
  Under gdb (0x730c399) every one of 738 ticks read 0x100, through the bot's `Jumping`, `JumpDown`,
  `WantUp`, `JumpPressedRaw` and `Up` with and without a walk, so none of those inputs sets it. The only
  `or $0x10` into +0x60 in the ELF is 0x7472a7b, in a system function at 0x7472930 that sets it when byte
  +0x61 bit 0 is on, the move vector (+0x24, +0x28) is not zero, and a check at 0xbb60980 passes (an
  auto-jump-like path). Settled 2026-11-08 (thirteenth2ba, bds-ba2gdb-ghast.txt, bds-ba2ghast.txt): a
  watchpoint on +0x60 showed its writer is 0x73e72b0, which copies `PlayerAuthInput` input data into the
  component each input (input bit 6 `Jumping` → 0x10, bit 7 `AutoJumpingInWater` → 0x20, 16/17 `WantUp`
  and `WantDown` → 0x4/0x8, 8 `Sneaking` → 0x1, 20 `Sprinting` → 0x2, 24 `PersistSneak` → 0x80; both jump bits cleared
  when byte 2 bit 2 of its third argument's flags is set). The acacia bot never sent `Jumping` while
  riding anything but a horse (riding/tick.rs reports keys only); with a drill-only patch that does, the
  flags read 0x114 and the ghast rose. Rule: the pack's 0.5 is added to the driver's flight intent's y,
  the intent cut to length 1, times `flying_speed`: jump alone 0.008 more a tick after 0.91 of its speed,
  jump with a walk straight up 0.016 (as a walk alone). Ported: harness.rs `flight`; sapling-ba2ghast.txt
  matches BDS tick for tick except one-tick shifts at key changes (line pacing). acacia should report
  `Jumping`/`JumpDown`/`WantUp` while riding any vehicle (riding/tick.rs), else bots can't make a happy
  ghast rise.
- Left: stilled.rs
  takes the player's eyes at feet + 1.62 and ignores slowness modifiers.

## Fire, lava, TNT minecarts and hopper minecarts (drilled 2026-11-08, group F: `ffburn`, `fftnt`, `ffcart`)

- Burning (bds-ffburn.txt): a boat summoned into lava showed `Health` 0 in its summon tick and was gone with
  its item the next; a minecart or chest minecart resting on lava (it stayed on top), a minecart in fire and a
  boat or chest boat in fire showed 24, 8, −7 (boat 20, 1, −8) and broke into their item the tick after.
  `/damage 2 fire` took 20 off a boat and 16 off a cart, so fire hurts 2 a tick and lava a boat 4, through the
  ×10 / ×8 of a hit, mending between. Sapling: `vehicles/burning.rs`; its boat's third tick in fire takes 2
  where BDS's took 1 (same break tick). Decompiled: `Minecart` hurt 14598be20 takes damage ×8 and breaks at
  integrity ≤ 0.
- TNT minecart (bds-fftnt.txt; decompiled `MinecartTNT` hurt 14598f0a0, prime 14598f200): fire and lava
  prime it (`MinecartTntPrimeFuse` and `FuseLength` 79 in the tick it is hurt), `FuseLength` is sent every
  tick down to 0 and it bursts the tick after; flint and steel or a fire charge in a player's hand primes it
  with the pack's `ignite` and no event (the item is kept); `/damage` of fire or lava primes it at once,
  `fire_tick` 3 s later; a shot on fire primes it at fuse 0 (bursting 2 ticks after the hit); a creative
  player's plain arrow broke it (`Health` −41). The blast is the pack's power 3 whatever the speed (no scaling
  in hurt or tick). Sapling: fire, lava, fire charge and the shown fuse fixed (`carts.rs`, `burning.rs`);
  arrows still pass through vehicles, `/damage` does not reach them, and yaw on rails was not compared.
- Hopper minecart on a detector rail (bds-ffcart.txt): a comparator reading from the rail gave 3, 6, 9 for
  1, 2, 3 full stacks of 5 slots (`floor(fullness·14)+1`), each change seen at the rail's 20-tick recheck.
  Hoppers filling and emptying carts were not measured (the capture shows no slots) and the laden-drag rows
  all ran into the end stone; both open.
- `summon <vehicle> <name>` gives BDS's vehicle no name: `@e[name=..]` found none and `/kill` listed
  "Minecart"; sapling names it.
