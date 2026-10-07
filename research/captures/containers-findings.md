# sapling vs BDS: containers capture

Sources: `bds-containers.txt` (BDS 1.26.52, ground truth) and `sapling-containers.txt`, same probe
(`crates/sapling/examples/bds_container_probe.rs`). `B:n` / `S:n` = line n of the BDS / sapling capture; ticks =
ms / 50. Sapling y = BDS y + 124 (feet B:1 `[0,-60,0]`, S:1 `[0,64,0]`).

Timing caveat (probe): `act()` runs the bot's action to completion before reading packets, so everything a
server sends during open/put/take/break is printed together at the start of the following settle. Those
timestamps are print times. Only intervals inside a settle (hopper moves, orb flight, sounds after a close) are
real ticks.

## Per step

**1. Hopper into an open chest.**
- BDS: open = ContainerOpen (window 2, Container), InventoryContent of the chest, BlockEvent ChangeState 1,
  InventoryContent of Inventory/Armor/Ui/Offhand, then the chest's content twice more (B:7-15). `chest.open`
  LevelSoundEvent at the block centre, extra_data -1132117234, about 1 tick later (B:16). After the hopper's
  setblock, the diamonds arrive one per InventorySlot (window 2, slot 0, count 1..10). Each slot change is sent
  twice in the same ms (B:19-39). First move ~8 ticks after the command; then every 394-406 ms = **8 ticks**
  ((10148-6548)/9 = 400 ms). Lid repeat at B:31/B:42 (80 ticks apart). Close: ContainerClose + BlockEvent 0 in one
  batch (B:44-45), `chest.closed` **~5 ticks later** (B:46, +264 ms).
- Sapling: ContainerOpen, chest content once, BlockEvent 1 (S:6-8). No sound. **No InventorySlot at all**: the
  source chest was empty because `/replaceitem` does not exist (D1). Lid repeats (S:11-12), close + BlockEvent 0
  (S:14-15), no sound.

**2. Furnace slots (survival).**
- `/give stone 4`: BDS sends 4 InventoryContent and a `pop` sound at the player's eye ~1 tick later (B:49-53).
  Sapling sends the 4 InventoryContent only (S:18-21).
- Open: BDS sends ContainerOpen (Furnace), the furnace's content, the 4 player windows, and the furnace again
  (B:56-62). Sapling sends ContainerOpen and the content (S:24-25).
- Stone into the result (slot 2): **accepted on both**. BDS: InventorySlot window 3 slot 2 to the requester,
  then the response Ok, Hotbar slot 0 `item_stack_id: None`, FurnaceOutput slot 2 count 4 (B:64-65). Sapling: the
  response alone, Hotbar slot 0 `item_stack_id: Some(0)` (S:27).
- Second `/give`: BDS sends 4 windows, the open furnace's content (still stone x4 in slot 2) and `pop`
  (B:67-72). Sapling sends the 4 windows (S:29-32).
- Stone into the fuel (slot 1): refused on both. BDS: one ItemStackResponse with **status 55**, no containers,
  nothing else (B:75). Sapling: **status Error (1)**, then 4 InventoryContent and the furnace's content
  (S:34-39).

**3. Experience.**
- `/replaceitem ... slot.container 2 iron_ingot 3`: on BDS the furnace's result is now iron_ingot x3 (B:81). The
  stone placed there in step 2 is overwritten and gone. On sapling nothing happened: slot 2 still holds the
  4 stone (S:45).
- Take: BDS: InventorySlot slot 2 empty, response FurnaceOutput emptied (`item_stack_id: None`), the
  ingots land in **HotbarAndInventory slot 1** (B:88-89), because slot 0 holds the second give's 4 stone. Sapling
  moved the 4 *stone* onto that same stone: slot 0, count 8, `Main(0)` (S:46-47). The different landing slot is
  entirely D1.
- Orbs (BDS only): two AddEntity `xp_orb` (value 1 each, 3 x 0.7 = 2.1 -> 2) at feet + 1.62, i.e. the eye,
  thrown (velocity y 0.25 / 0.38, random yaw), and SoundOrb at the spawn point (B:90-92). Then UpdateAttributes
  `player.experience` 1/7, listed twice in one packet, tick 780 (B:93). That is orb 8, taken in its spawn tick
  with no move. Orb 7 gets a full SetEntityData, one delta and a motion (B:94-96), and is taken at tick 782:
  SoundOrb at its new position, xp 2/7 (B:98-99). Each RemoveEntity comes **one tick after** its pickup (B:97,
  B:100). Sapling: no orbs. Stone is worth 0.1 each (`xp.rs`), 0.4 rolled down to 0, so none was expected.
- `/summon xp_orb`: BDS throws it (velocity (0.14, 0.25, -0.05), B:104), sends a full SetEntityData, and toggles
  MOVING (B:105-108). It lands at +7 ticks (B:124), slides, starts flying to the player ~35 ticks after the spawn
  (B:134), and is taken at ~64 ticks: SoundOrb at the orb, xp 3/7, RemoveEntity next tick (B:164-166). Sapling:
  AddEntity with velocity 0, a `spawn` LevelSoundEvent (S:51-52). It falls straight down to the floor (S:53-59)
  and **never moves or is taken** in the 8 s that follow (D2).

**4. Shulker boxes.**
- Survival, "filled": BDS drops the box item with `Items: [diamond x5, Slot 4]` (B:412), velocity y 0.2, plus
  UpdateAttributes `player.exhaustion` 0.005 (B:413) and the full SetEntityData (B:414). Sapling drops the box
  with `Items: []` (S:70): the box was empty (D1). Sapling's item was picked up on landing (S:84-89). BDS's
  bounced out of reach (B:417-436).
- Survival, empty: both drop the box with `Items` a list of tag 0, no items (B:678, S:94). BDS: TakeItemEntity
  (B:692), RemoveEntity a tick later (B:693), and no InventoryContent/InventorySlot in the next 2.5 s. Sapling's
  item lay out of reach (S:94-115).
- Creative, "filled": BDS **drops the box with its diamonds** (B:698). Sapling drops nothing (S:118-119), but its
  box was empty (D1). `slots.rs:145-149` does drop a filled box in creative, so the capture cannot test it.
- Creative, empty: no drop on either (B:713-716, S:121-122). BDS's B:714-725 are the creative-filled item, which
  had been resting on this box, falling once it was gone.
- Breaking (survival): BDS sends a BlockStartBreak / ParticlePunchBlock / BlockBreakSpeed / BlockStopBreak cycle
  for every input tick (~59 cycles, B:174-411 and B:438-677), then ParticleDestroy. Sapling sends one
  ParticlePunchBlockNorth, BlockStartBreak (data 1092, as BDS) and ParticleDestroy (S:67-69, S:91-93).

## Differences, most important first

**D1. `/replaceitem` (and `/xp`) do not exist in sapling. Three of the four drills tested nothing.** 2026-10-07:
`/replaceitem block` added, answering as BDS in all ten cases of `bds_command_probe replaceitem`
(`bds-replaceitem.txt`, `sapling-replaceitem.txt`); `/xp` still missing; rerun this drill.
`crates/sapling/src/vanilla/` registers setblock, fill, give, clear, gamemode, summon, kill and tp only. No crate
has `replaceitem`. The probe does not print CommandOutput, so the failures were silent. Steps that silently did
nothing:
- Step 1: the source chest had no diamonds, so there was no hopper traffic (S:9-15 vs B:19-39). The hopper rate
  is untested on sapling.
- Step 3: the ingots were never placed. The stone stayed in the result (S:45 vs B:81) and stacked onto slot 0,
  hence `Main(0)` and no orbs (S:47 vs B:89-100).
- Step 4: both "filled" boxes were empty (S:70 vs B:412; S:118 vs B:698). Filled-box drops are untested on
  either game mode.
- Setup `xp -100L @s`: a no-op, harmless on a fresh player.

Fix: add `/replaceitem block <pos> slot.container <slot> <item> [amount]` to `vanilla/blocks.rs`, through a
backend call that goes via `containers::set_slots` (so viewers get InventorySlot and hoppers/furnaces see it)
or writes the block entity when nothing holds the slots. Add `/xp` too. Add `CommandOutput` to the probe's
`shown!` list so an unknown command shows. Then re-run. BDS is the reference here; this is missing sapling
functionality, not a disputed behaviour.

**D2. `/summon xp_orb` makes a plain mob: no throw, a spawn sound, never followed or taken.**
`creatures.rs:73-89` (`spawn`, reached through `EntityBackend::spawn_entity`, `vanilla/players.rs:150-155`) gives
it `Physics::MOB`, motion 0, `spawn_sound` and a `Creature`. It never gets an `orbs::Orb`, so `orbs::tick`
ignores it. BDS: thrown, SoundOrb/xp/RemoveEntity on pickup (B:104-166). Fix: in `creatures::spawn`, send
`minecraft:xp_orb` to `orbs::spawn` with value 1 (B:104 `ExperienceValue 1`) and no spawn sound. Have
`orbs::spawn` return the entity so the API gets an id. BDS is clearly right.

**D3. Orb pickup timing: no 7-tick wait; a 2-tick gap per player instead; removal a tick later.**
BDS took orb 8 in the tick it appeared (B:90-93, no move packet) and orb 7 two ticks later (UpdateAttributes
tick 780 -> 782, B:93/B:99). `orbs.rs:36-38` `TOUCH_AFTER = 7` makes every orb wait 7 ticks, citing "one
recording". This recording contradicts it. The 2-tick spacing matches Java's per-player `takeXpDelay` (2).
Confirm in BDS with bdsre (`ExperienceOrb::playerTouch` / Player). BDS also sends RemoveEntity one tick after
SoundOrb + xp (B:97, B:100, B:166). Sapling removes in the same tick (`orbs.rs:147-150`). Fix: drop
`TOUCH_AFTER` and add a per-player "last orb taken" tick (2-tick gap). Defer the orb's `mobs::remove` by a tick.
The capture is clear about BDS; the cooldown's exact rule is not proven yet.

**D4. Furnace fuel refusal: status 55, no resync.** Fixed 2026-10-07 (`requests.rs` `CANNOT_HOLD`: a request
only a container's refusal stopped). B:75 vs S:34-39. `requests.rs:127-130` answers every
failure `Status::Error`. `requests.rs:178-182` then resends all four windows and the container. BDS sends only
the response. Fix: have `apply` return why it failed. When `Inventory::is_refused` (`inventory.rs:200-202`)
stopped it, answer `Status::Unknown(55)` (by position in `ItemStackNetResult` this is probably
`CannotPlaceItem`; from memory, confirm with bdsre). Keep Error elsewhere and skip `send_all`/`resend` for this
refusal at least. Whether BDS resyncs after other failures is not recorded. BDS is clearly right for this case.

**D5. Chest open/close sounds missing.** Fixed 2026-10-07 (`lid.rs`, 1 and 5 ticks late). BDS: `chest.open` ~1 tick after the open (B:16), `chest.closed` ~5
ticks after the close's BlockEvent 0 (B:46). Both are LevelSoundEvent at the block centre, extra_data = the
chest's block id, entity_unique_id -1. Sapling: none. `containers/lid.rs:4-5` TODO says the names were not
found; they are these. Fix: `lid.rs:34-41` `sounds()` adds chest/trapped_chest `["chest.open", "chest.closed"]`,
with the close sound delayed ~5 ticks (lid swing). Check the other sounds' delays the same way. BDS is right.

**D6. Item pickup packets.** Sapling: 4 InventoryContent, TakeItemEntity and RemoveEntity, all in one tick
(S:84-89; `drops.rs:197-200`, `inventory::change` sends the windows). BDS: TakeItemEntity, RemoveEntity one tick
later (B:692-693), and no inventory packet of a printed kind for 2.5 s. Fix the removal tick (as D3). Do not
drop the inventory update on this evidence: BDS must tell the client somehow, maybe through a kind the probe
does not print. Re-record with all kinds before changing it. Medium-low.

**D7. Requester's InventorySlot and emptied-slot ids.** BDS sends the container slot's InventorySlot to the
player whose request changed it, before the response (B:64, B:88). `slots.rs:98` skips that player (`knows`).
BDS answers an emptied slot with `item_stack_id: None` (B:65 Hotbar, B:89 FurnaceOutput). Sapling sends
`Some(0)` (S:27, S:47; `requests.rs:138`). Fix: `item_stack_id: held.map(|h| h.id)`. Optionally drop the
`knows` filter. Low.

**D8. Window resends on open and on /give.** On every container open BDS also sends the player's 4 windows and
the container's content twice more (B:10-15, B:58-62, B:82-86). A `/give` with a screen open resends that
container (B:71) and plays `pop` at the eye ~1 tick later (B:53, B:72). Sapling: one content on open
(`screens.rs:73-84`), no pop or container on give (`vanilla/players.rs:75-89`). The pop is worth adding.
Copying the redundant resends is optional. Low.

**D9. Lid repeat phase.** BDS re-sends ChangeState 1 ~80 ticks after the open, then every 80 (B:9 4624 ->
B:31 8598 -> B:42 12597). Sapling repeats on `Tick % 80 == 0` (`screens.rs:146`): the first came 64 ticks after
the open (S:8 -> S:11). One BDS sample suggests a per-chest counter from the open. Check
`ChestBlockActor::tick` with bdsre before changing. Low.

**D10. Survival break traffic and exhaustion.** BDS sends a Start/Punch/BreakSpeed/Stop cycle per input tick
(B:174-411) and UpdateAttributes `player.exhaustion` +0.005 per broken block (B:413, B:679). Sapling sends one
start and no exhaustion: there is no hunger in sapling (no match for `exhaustion` in crates/). The cycles follow
the bot sending ContinueBreak every tick (acacia-bot `interact/breaking.rs:87-105`). Check what BDS does for a
real client's input before copying. Exhaustion belongs to a hunger feature. Low.

**D11. Orb and item metadata (D9 of the mechanisms findings).** After AddEntity, BDS sends the orb's full
SetEntityData and toggles MOVING (B:94, B:105, B:108, B:129, B:136). Furnace orbs get a random yaw (B:90-91,
101.25 / 73.125). Sapling sends neither. Low.

**D12. Hopper duplicates.** BDS sends every slot change twice (B:19-39). This is probably two listeners firing;
don't copy it. The rate itself, 8 ticks, matches `hoppers.rs:24` `MOVE_TICKS`. A first move ~8 ticks after
placement is consistent with sapling's pull-on-first-tick, push-after-cooldown (`hoppers.rs:180-190`). Re-check
once D1 lets the drill run.

## Probe and drill artefacts (not server differences)

- `Main(1)` vs `Main(0)` for the ingots, the missing orbs in step 3, the hopper's silence and the empty "filled"
  boxes all come from D1.
- Timestamps of act() packets are print times (see the top). For example, all 238 break LevelEvents at 64898.
- RemoveEntity of ids never announced in the capture (B:40-41, B:73): BDS entities outside the probe's view, or
  left over from setup. B:4 UpdateAttributes `movement` is setup.
- Which shulker item gets picked up depends on the random spill direction (B:417-436 out of reach, S:83-88 in
  reach).
- The probe doesn't print CommandOutput, so failed commands are invisible. Fix it with D1.

## Matching

ContainerOpen fields and window numbering (2, 3, 4). Lid BlockEvent 1/0 at open/close, in the close's batch,
repeated every 80 ticks. The furnace accepts anything into its result (B:65, S:27, `furnaces.rs:119-124`).
Empty-box item tag `Items` = list of tag 0 (B:678, S:94). Survival-break item velocity y 0.2 and MOVING a tick
later. No creative drop of an empty box. From source only, since D1 kept them from running: hopper rate 8
ticks, furnace orbs at feet + EYE_HEIGHT (`furnaces/xp.rs:106`) split as BDS ([1, 1] for 2 points), filled
shulker boxes dropping in creative (`slots.rs:145-149`).
