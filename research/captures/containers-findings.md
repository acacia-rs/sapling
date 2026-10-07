# sapling vs BDS: containers capture (round 2)

Sources: `bds-containers.txt` (BDS 1.26.52, ground truth) and `sapling-containers.txt` (rerun 2026-10-07, after
`/replaceitem block`, chest sounds and status 55 went in), same probe (`crates/sapling/examples/bds_container_probe.rs`).
`B:n` / `S:n` = line n of the BDS / sapling capture; ticks = ms / 50. Sapling y = BDS y + 124.

Timing caveat (probe): `act()` runs the bot's action to completion before reading packets, so what a server sends
during open/put/take/break prints together at the next settle's start. Only intervals inside a settle (hopper moves,
orb flight, sounds after a close) are real ticks.

## Per step

**1. Hopper into an open chest.** Both: ContainerOpen, the chest's content, BlockEvent ChangeState 1, `chest.open`
at the block centre ~1 tick later (B:7-16 +72 ms, S:6-9 +48 ms). BDS also resends the 4 player windows and the
chest content twice more (B:10-15). Diamonds then arrive one InventorySlot per move, every 8 ticks on both: BDS
6548 -> 10148 (B:19-39), sapling 6538 -> 10136 (S:12-22), first move ~8 ticks after the hopper's setblock on both.
BDS sends every change **twice** (same ms); sapling once. Lid repeats (B:31/42, S:17/23). Close: ContainerClose +
BlockEvent 0 together, `chest.closed` ~5 ticks later on both (B:44-46 +264 ms, S:25-27 +249 ms). `chest.*`
extra_data differs: BDS -1132117234, sapling 741882976 (D9).

**2. Furnace slots (survival).** `/give stone 4`: BDS 4 windows + `pop` at the eye (B:49-53); sapling 4 windows
(S:30-33). Open: BDS ContainerOpen, content, 4 windows, content again (B:56-62); sapling ContainerOpen + content
(S:36-37). Stone into the result: Ok on both; BDS sends the requester InventorySlot slot 2 first (B:64) and answers
Hotbar 0 `item_stack_id: None` (B:65); sapling the response only, `Some(0)` (S:39). Second give: BDS adds the open
furnace's content and `pop` (B:67-72); sapling the 4 windows (S:41-44). Stone into the fuel: status 55, nothing else,
on both (B:75, S:46).

**3. Experience.** `/replaceitem` puts iron_ingot x3 in slot 2 on both (B:81, S:52; the step-2 stone is overwritten).
Take: ingots to HotbarAndInventory slot 1, `Main(1)` on both (B:89, S:54); BDS again sends InventorySlot first (B:88)
and `None` for the emptied FurnaceOutput, sapling `Some(0)`.
- Orbs, BDS (B:90-100): two value-1 orbs at the eye (feet + 1.62), thrown, random yaw, full metadata. Orb 8 is taken
  **in its spawn tick** (SoundOrb at the spawn point, xp 1/7, tick 780, no move), orb 7 after one move, **2 ticks
  later** (xp 2/7, tick 782). RemoveEntity of each **a tick after** its pickup (B:97, B:100). UpdateAttributes lists
  `player.experience` twice, `tick: 780`.
- Orbs, sapling (S:55-77): two orbs at the eye, thrown (velocities in BDS's range), yaw 0, short metadata (D5). They
  fly 6 ticks (S:57-71), then **both** are taken **in the same tick** 32737, ~6 ticks after spawning (S:72-75), and
  removed in that tick (S:76-77). UpdateAttributes lists `player.experience` **four** times, `tick: 0`.
- `/summon xp_orb` (B:104-166 vs S:81-89): unchanged from round 1. BDS throws it, it follows the player from ~35
  ticks and is taken at ~64 (SoundOrb, xp 3/7, RemoveEntity next tick). Sapling makes a plain mob: velocity 0,
  `spawn` sound, falls straight down, never followed or taken (D2).

**4. Shulker boxes.**
- Survival, filled: both drop the box with `Items: [diamond x5, Slot 4]` (B:412, S:100), velocity y 0.2. BDS's
  item compounds also carry `WasPickedUp: 0b` and order `Count, Damage, Name, Slot`; sapling's `Name, Count, Damage,
  Slot` (D10). Neither picked up (both landed out of reach).
- Survival, empty: both drop with `Items` = an empty list of tag 0 (B:678, S:125). BDS's was picked up (B:692-693),
  sapling's lay out of reach: random spill.
- Creative, filled: **both drop it with its diamonds** (B:698, S:150). Sapling's landed within reach and was picked
  up (S:162-168); BDS's came to rest on the empty box next to it and was not.
- Creative, empty: no drop on either (B:713-716, S:170-171).
- Breaking: unchanged from round 1 (D8).

## Differences, most important first

**D1. `/replaceitem` missing.** Fixed 2026-10-07: every drill now ran; hopper, furnace ingots, filled boxes and
creative drops all exercised (above). `/xp` (setup `xp -100L @s`) is still absent; harmless on a fresh player.
The probe still prints no CommandOutput: add it to `shown!` so a missing command shows.

**D2. `/summon xp_orb` makes a plain mob.** Fixed 2026-10-07 (`creatures.rs` `spawn_entity` -> `orbs::throw`, worth
1, no spawn sound). S:81-89 vs B:104-166. `creatures.rs:73-89` (via
`vanilla/players.rs` Summon) gives it `Physics::MOB`, motion 0, `spawn_sound` and no `orbs::Orb`, so it is never
followed or taken. Fix: in `creatures::spawn`, hand `minecraft:xp_orb` to `orbs::spawn(world, layer, feet, 1)`
(value 1, B:104), no spawn sound; make `orbs::spawn` return the entity for the API's id. BDS is clearly right.

**D3. Orb pickup timing.** Fixed 2026-10-07 (`orbs.rs` `TAKE_EVERY`: no wait for age, one orb per player per 2
ticks, gone the tick after; the 2-tick gap is the recording's, not decompiled). BDS: taken as soon as touched, even in the spawn tick, at most one orb per player
every 2 ticks (B:93 tick 780, B:99 tick 782), RemoveEntity a tick after the pickup. Sapling: `orbs.rs:38`
`TOUCH_AFTER = 7` holds both orbs ~6-7 ticks, then takes both in one tick and removes them in it (S:72-77,
`orbs.rs:149`). Fix: drop `TOUCH_AFTER`; give players a last-orb-taken tick and let an orb be taken only 2+ ticks
after the previous one (Java's `takeXpDelay`; confirm `ExperienceOrb::playerTouch` with bdsre); defer the orb's
`mobs::remove` one tick after the SoundOrb + attributes. BDS is right on this recording; the exact cooldown rule
is the one thing to confirm.

**D4. Fuel refusal.** Fixed 2026-10-07 (`requests.rs` `CANNOT_HOLD`): status 55 and no resync on both (B:75, S:46).

**D5. Orb announcement and xp attributes.** Fixed 2026-10-07 (orb from the recorded entry; progress named twice
with the server tick); the full SetEntityData after AddEntity and the random yaw are still not sent. Orb AddEntity: sapling's metadata is Flags, Variant, Color, Nametag,
Scale, MarkVariant, BoundingboxWidth/Height, ExperienceValue (S:55); BDS's is the full recorded set (Health,
OwnerEid, Air, ChargeAmount, LeadHolderEid, InteractiveTag, ...; B:90), followed by a full SetEntityData and MOVING
toggles (B:94, B:105-108), and a random yaw (B:90-91). `sapling_data::add_experience_orb` (sapling-data
`entity.rs:156-161`) builds from `plain(...)`; the summon path already uses the recorded orb (S:81 has BDS's keys).
Fix: start from `RECORDED["minecraft:xp_orb"].grown` and set `ExperienceValue`; random yaw is optional. Xp
UpdateAttributes: sapling lists `player.experience` four times with `tick: 0` (S:73, S:75); BDS twice with the
server tick (B:93). `experience_attributes` (sapling-data `player.rs:21-33`) keeps every PROGRESS entry of the
template: keep the first two (or dedupe the template to BDS's), and fill `tick`. Low-medium.

**D6. Item pickup packets.** RemoveEntity a tick after TakeItemEntity since 2026-10-07 (`entities::leave`, shared
with orbs); the inventory update is kept, as the fix said. Sapling: 4 InventoryContent, TakeItemEntity, RemoveEntity, one tick (S:163-168;
`drops.rs:197-200`). BDS: TakeItemEntity, RemoveEntity one tick later (B:692-693), no InventoryContent or
InventorySlot in the next 2.5 s. Fix: defer `mobs::remove` one tick (share D3's deferral). Leave the inventory update
until a recording with all packet kinds shows how BDS tells the client (it must, somehow). Medium-low.

**D7. Requester's InventorySlot and emptied ids.** Fixed 2026-10-07 (the requester gets the slot before its
answer; an emptied slot has no stack id). BDS sends the changed container slot's InventorySlot to the
player whose request changed it, before the response (B:64, B:88); `containers/slots.rs:98` skips that player
(`knows`). BDS answers an emptied slot `item_stack_id: None` (B:65, B:89); sapling `Some(0)` (S:39, S:54;
`inventory/requests.rs:143`). Fix: `item_stack_id: held.map(|held| held.id)`; drop the `knows` filter. Low.

**D8. Survival break traffic and exhaustion.** BDS: a BlockStartBreak / ParticlePunchBlock / BlockBreakSpeed /
BlockStopBreak cycle per input tick (~59, B:174-411), UpdateAttributes `player.exhaustion` +0.005 per block
(B:413, B:679). Sapling: one punch, start, destroy (S:97-99, S:122-124), no hunger. The cycles follow the bot
sending ContinueBreak every tick (acacia-bot `interact/breaking.rs:87-105`); check against a real client's
inputs before copying. Exhaustion belongs to a hunger feature. Low.

**D9. Window resends, `pop`, chest sound data.** BDS on every container open resends the 4 player windows and
the container's content twice more (B:10-15, B:58-62, B:82-86); a `/give` with a screen open resends that
container and plays `pop` at the eye ~1 tick later (B:53, B:71-72). Sapling: neither (`containers/screens.rs`
`show_screen`; `vanilla/players.rs` Give). `chest.open/closed` extra_data: BDS -1132117234, sapling 741882976
(S:9, S:27): both should be the chest block's network id, so the two chests' states differ (setblock default
facing?) or BDS sends something else; compare with the chest's `UpdateBlock` id. Fix: add the `pop` (cheap);
resends optional; settle the extra_data from the registry. Low.

**D10. Shulker item NBT.** BDS's per-item compound: `Count, Damage, Name, Slot, WasPickedUp: 0b` (B:412, B:698);
sapling's: `Name, Count, Damage, Slot` (S:100, S:150). Order should not matter to the client; `WasPickedUp`
might (item stacking/comparison). Fix: write `WasPickedUp: 0b` in sapling-world's `with_container_items`. Low.

**D11. Hopper duplicates and lid phase.** BDS sends every hopper slot change twice (B:19-39); don't copy. Lid
repeat: sapling on `Tick % 80` (`screens.rs:146`), BDS ~80 ticks after the open in this one sample (B:9 -> B:31);
sapling came 77 ticks after (S:8 -> S:17) by chance. Check `ChestBlockActor::tick` with bdsre before changing.
Low.

## Probe and drill artefacts

- Times of act() packets are print times (the 238 break LevelEvents at one ms, B:174-411).
- RemoveEntity of ids never announced (B:40-41, B:73) and B:4 UpdateAttributes `movement` come from BDS's own
  world/setup.
- Which dropped box gets picked up follows the random spill: survival-empty on BDS, creative-filled on sapling.
- Sapling's orbs flew 6 ticks before pickup partly because of `TOUCH_AFTER` (D3), not the throw.

## Matching now

Hopper: one item per 8 ticks, first ~8 ticks after placement. Chest: lid events, `chest.open` +1 tick,
`chest.closed` +5 ticks, at the block centre. Furnace: anything into the result, status 55 for the fuel, ingots to
`Main(1)`, two value-1 orbs at the eye with BDS-range throws, xp progress-only attributes 1/7 then 2/7. Shulker
boxes: filled and empty drops in survival with BDS's `Items` (empty = list of tag 0), filled drops in creative,
empty none in creative; item velocity y 0.2 and the MOVING flag a tick later.
