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
  `Interact LeaveVehicle` or was bucked off.
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
- Detector rail: on while a cart is on it, released 20 ticks after.
- TNT minecart: power 3, fuse 80; flint and steel primes; a fall > 3.5 bursts it at once; a blast primes it on
  10 + rand(20).
- Pull-in (BDSRE 14241fde0, from the push collision): a mob narrower than 1.375, not riding, with a free seat.

## Unverified

- The links' `immediate`/`rider_initiated` for a non-click mount; type 2 for a second rider.
- The add link's order against `SetEntityData` for later viewers: sapling sends seats and links after AddEntity.
- That BDS opens a ridden chest boat's slots on the inventory key (`Interact OpenInventory`).
- A boat's or minecart's stored `Pos`: feet here.
- A placed minecart's height on its rail (box bottom on the rail block's floor here; BDS spawns at y + 0.5 then snaps).
- Detector rail timing (Java's), hopper minecart pick-up range, the activator lock of a hopper minecart (not done).
- What raises a boat's underwater counter (not done); boat fall breaking (none found).
- Horse and camel steering speeds and the jump charge (sapling: walk at movement speed, no charged jump).
- The horse screen (`ContainerOpen` type Horse + `UpdateEquip`): not done; the strider and pig boost by stick use.
