# sapling vs BDS

## Sweep of every bds_command_probe word (2026-10-24)

All 105 words that have a `bds-<word>.txt` were run on sapling master (`tools/remote-drills.sh`, 2x ticks) and
compared drill line by drill line with `tools/drill_vs_bds.py`: answers and chat exactly (decimals masked),
every other packet by count per bot. Rerun: `tools/remote-drills.sh bds_command_probe <dir> <words>`, then
`py -3 -I tools/drill_vs_bds.py --sweep research/captures <dir> <words>`.

The BDS captures come from a different world, so much of what differs is the setup, not sapling: BDS's bots
stand near y -19 (sapling's at 64), its ground is not all stone (a `/fill` there changes blocks sapling's
leaves alone), and its world has campfires and sculk near the drills (8437 `block.campfire.crackle`, 703 sculk
sounds). Read a word's differences with that in mind.

What sapling did otherwise, by how often:

| Gap | Size | State |
|---|---|---|
| `/fill` that changes nothing answered "No blocks filled"; BDS: `commands.fill.success ["0"]`, failed | 155 lines, most words | fixed |
| Other players' `MovePlayer`: BDS relays one a tick per moving input, sapling only when a player moved | 76k packets | open, likely harmless |
| Item and entity names in answers by title case: BDS `Raw Chicken`, `Flint and Steel`, `Potion of Slowness`, `Water Bottle`, a lying item by its item (`Rotten Flesh`, not `Item`) | ~40 lines | fixed but for lying items (`/kill` still says `Item`): `sapling-data/data/names.tsv` by `examples/bds_name_probe.rs` |
| Big fills as `UpdateBlock` each where BDS sends `UpdateSubchunkBlocks` | 27.8k extra UpdateBlock | open |
| Sounds BDS makes and sapling not: a player's `land` to others (248), `hit` (60), `fall` (43), `jump_to_block` (41), `born` (23), `armor.equip_iron` (22), `open`/`close` (34), underwater enter/exit (34), `bottle.empty` (44) | ~600 | open; `land` is the server's (acacia's bot sends no sounds), to everyone but the lander |
| Mobs never showed `MOVING` (bit 34), which BDS toggles by docs/research/moving-flag.md's rule for every actor: 9871 toggles to sapling's 62 | 12 mob words: 565 BDS, 6 before, 184 after (AI differs) | fixed: a mob's flags are one word (`sync::Told::flag_mob`/`set_flag`), so fire, fuses, charges and spells no longer clear each other's bits either |
| `BREATHING` (bit 35) toggles of mobs under water (342), `VisibleMobEffects`, and the whole-dictionary `SetEntityData` BDS repeats (D9) | ~15k | open |
| A summoned armour stand moves in its spawn tick on sapling, the tick after on BDS (bds-water-flags.txt 48061/48112), so its `MOVING` shows a tick early too; BDS's zombie moved within 1 ms, but in a batch of three ticks, so the rule for mobs is not settled | | open |
| `/effect` and `/damage` took players alone: none found where BDS's put slowness on a zombie (`effects3`, `mob-effects`, `wind-burst`, `stare`) | 30 lines | fixed (`ctx.remove_entity_effect`, `clear_entity_effects`, `hurt_entity`) |
| `/kill @e[type=!player]` found nothing where BDS killed items or mobs still there (`effects3`, `wind-burst`, `stare`); `stare`'s enderman is gone before its `/tp` and `/effect` | ~25 lines | open |

# Mechanisms capture, round 2

Sources: `bds-mechanisms.txt` (ground truth) and `sapling-mechanisms.txt`, same probe
(`crates/sapling/examples/bds_mechanism_probe.rs`, radius 8, `stand()` waits for cells). `B:n` / `S:n` = line n of
the BDS / sapling capture; ticks = watcher ms / 50. Sapling y = BDS y + 124. Known items from the brief are not
repeated (UpdateSubchunkBlocks, UpdateBlockSynced, falling-block MOVING/Health, plates, wet-stairs Rejected,
splash sounds, mob loot/death delay). Spawn sounds since 2026-10-07: at the head BDS computes (creatures.rs,
docs/research/spawn-sound.md).

Command Text (round 3, sapling run on testbox, 2026-10-07): all 23 lines agree with BDS's in kind, receiver,
order and JSON, after adding BDS's trailing newline. BDS's `/kill @e[type=chicken]` named three chickens to
sapling's one: its world had two more within range.

/gamerule (`bds_command_probe`, `bds-gamerule.txt` vs `sapling-gamerule.txt`, both on testbox, 2026-10-07): all 33
commands answered alike (CommandOutput, echo, GameRulesChanged) but for the starting values: BDS's fresh world had
`doMobSpawning = false` and `playersSleepingPercentage = 150` (source not found; the drill wipes the world), so its
`playerssleepingpercentage 150` changed nothing and sent no GameRulesChanged, as sapling does for no change.

/kill (`bds_command_probe kill`, `bds-kill.txt` vs `sapling-kill.txt`, 2026-10-07): DeathAnimation with the answer,
loot (BDS's tables) at the mob's feet + 0.075 in the same tick, RemoveEntity 21 ticks on; the armor stand goes at once
without either; none with `doMobLoot` off. The loot lands at once, lifted out of the floor (body.rs `lift_out`).
2026-10-24: the summoner's `EntityEvent EntitySpawn` (data `(legacy type id << 16) | 2`, to it and its viewers,
`creatures/summoned.rs`) and an item's whole metadata again after AddItemEntity (`mobs::item_shown`) now agree.

Plates (`bds_plate_probe`, `bds-plates.txt` vs `sapling-plates.txt`, 2026-10-07): walking on and off a written and
a hand-placed plate gives the same states, clicks, positions and data on both. Sapling pressed under a summoned
chicken in the summon's tick, BDS the tick after: fixed (entities.rs `bodies`). Left: redstone (BDS lights the lamp
beside the plate; sapling has none).

## The round-1 fixes

| Fix | Now | Evidence |
|---|---|---|
| (1) unchanged writes not sent | agrees | No setup flood: sapling's first packet is the slab drill's punch at 13.4 s (S:8), every later drill found its cells loaded, 315 watcher UpdateBlocks in the whole run (was ~13k). The setup itself is not printed, so this is indirect. |
| (2) /setblock liquid: neighbours only | agrees | Bucket drill: `water[liquid_depth=3]` stays depth 3 with no packets on both (B:5315-5350, S:368-377). Lava column: setblock -> first fall 30 ticks on BDS (B:6434 216568 -> B:6441 218078), 31 on sapling (S:761 199894 -> S:763 201429); was 15. |
| (4) falls start before the tick's moves | agrees | Per-tick positions now match: support break T, AddEntity T+1, y = L+0.263 at T+2 on both (B:1127/1228/1236, S:12/17/19). Anvil: on_ground +12, block +13, RemoveEntity +16 ticks on both (B:1483-1692, S:50-87). Stack: one start per tick on both (BDS flushed two ticks in one batch at 88052-88054: entity 18 has two deltas, B:3058/3064). |
| (5) no move packet in the spawn tick | overshoots, see D2 | BDS *does* send the spawn tick's move, after the announcement, in the same batch: falling block AddEntity + SetEntityData + `MoveEntityDelta y -56.6192` (actor and watcher 36712, B:1228-1233); item AddItemEntity + SetEntityData + delta (actor 37257, B:1273-1276). Sapling now sends none (S:17 -> first delta S:19 a tick later). |
| (6) refused block's item where the entity was | partly | x/z now exact (-31.5, 2.5) on both. y and motion still differ, see D3. |

## The two moved bucket clicks

- **Lava bucket aimed into the ground** (face Down onto dirt): refused on both, no block change, no sound, nothing
  to the watcher. BDS resyncs 6 layer-0 cells only (target, above, below x2, target, two below; B:5746-5778) and
  sends no InventoryContent; sapling resyncs target, below, two below in layers 0 and 1 and sends InventoryContent
  (4 windows) (S:528-539). See D7.
- **Creative water bucket**: works on both. Same UpdateBlock (source at +1y), `bucket.empty.water` at the player,
  same flow cells and depths, first spread +5/+6 ticks (B:5788-5836, S:543-567). BDS sends one InventoryContent set
  (bucket kept, id 365) before the block (actor 171404); sapling sends none in creative (buckets.rs:53). Low.

## Remaining and new differences, most important first

**D1. Sapling's first two drills never ran (setup cells air).** Fixed: both run in the round-3 capture
(`sapling-mechanisms.txt` S:14). "sand, three cells onto dirt" and "gravel onto a
torch" (x -36, -34, chunk x -3) report `nothing to break ... minecraft:air` (S:3-6); BDS ran both (B:3-966). The
drill's `/setblock stone` + block at y 66/67 left air on the actor's client, while the same setup at x -32 worked.
Cause not in the capture: `Probe::setup` swallows CommandOutput and `stand()` checks only that cells are loaded.
Probably sapling (drill_server's world has no loader, so `World::set_block` should not refuse), possibly a send
race on the first tp. Fix the probe first: print setup's CommandOutput errors and check the drill's cells after
`setup` (fail the drill if they are not what was set); then rerun. These two drills are the only refused-landing
and on-dirt checks besides the slab.

**D2. Spawn tick's move not sent (fix 5 went too far).** Fixed (entities.rs `AfterAnnouncing`; round 3
S:14-17: AddEntity then the delta, same batch). BDS: announcement with the pre-step state, then the
tick's delta in the same batch (falling block -0.0792, item its first step). Sapling: entities.rs:83-86 drops the
move. Fix: keep the packets and send them after the announcement in the same tick (push the announcement to the
current viewers at spawn, or have `mobs::update` flush deltas of `Born == now` entities after announcing). BDS is right.

**D3. Refused falling block's item: 0.49 too low and motionless.** Fixed: spill motion by round 3, and
2026-10-07 the item's box starts at the falling block's wire y (drops.rs `let_go`; round 3 was still 0.125 low). BDS AddItemEntity `(-31.5, -58.885, 2.5)`
= item bottom at the falling block's *wire* y (-59.01; item wire = bottom + 0.125), velocity `(-0.051, 0.2,
-0.031)` (gravel: `(0.008, 0.2, 0.050)`), then it hops and settles on the slab (B:1273-1366). Sapling `(-31.5,
64.625, 2.5)` = bottom at the entity's bottom 64.5, velocity 0, never moves (S:42). Fix: flight.rs:103 pass
`wire(at)` instead of `at`; drops.rs:104-107 `let_go` gives spill's motion (`±SPILL_ACROSS, SPILL_UP`). BDS is right.

**D4. Waterlogged stairs leak through their full face (round-1 item 23).** Fixed in `2048a45`. Water bucket on east-facing
stairs: BDS spreads to (-2,1) and (-2,3) only (B:5377, +5 ticks), never east; sapling to (-1,64,2) east first
(S:391, +5 ticks) and on into the pit. Fix as before: flow.rs `slope`/`spread_to` (flow.rs:67-104) skip sides whose
face of the layer-1 holder is full; confirm in `LiquidBlock::_canSpreadTo`. BDS is right.

**D5. Lava column after the first hardening runs late (new, medium-low confidence).** 2026-10-07: partly
explained, partly not matchable. Sapling dropped a second queued tick for a cell already queued; BDS keeps both
(liquid.rs `Flows` now does, apart from `neighborChanged`'s same-tick check). The step-4 winner between the falling
L10 and the sideways L1 follows BDS's per-chunk heap order and its unseeded lava rolls, so the three BDS runs
disagree among themselves; mask those cells rather than copy one (docs/research/tick-queue.md). Steps 1-3 match (+30, +60,
+90/93 ticks, (18,64,-3) -> stone). Step 4: BDS +120 ticks, y65 side cells (17,-3),(19,-3),(18,-4) as 2704000175
(B:6491); sapling +131, (17,-3),(18,-4),(18,-2) as 2396626060 and only (19,-3) as 2704000175 (S:786-789). Last
fizz/harden: BDS +149 (B:6498-6505), sapling +164 (S:801-811). Elsewhere sapling holds 30-tick lava and 5-tick water
steps, so not tick drift. Likely the depth/tick rule for flowing lava fed from above beside a falling column
(flow.rs `fed`, 38-55). Needs a dedicated drill with tick numbers before fixing.

**D6. Hand-placed sand: BDS disagrees with itself.** Settled 2026-10-07: two more BDS runs on testbox
(`bds-run-a.txt`, `bds-run-b.txt`) both fall 2 ticks after the place, as this run did; sapling now waits 2
(falling.rs `Change::Placed`). Round 1: falls in the placement batch. This run: place +
sound at 93642, UpdateBlockSynced + AddEntity at actor 93765/93770, ~2.5 ticks later (B:3304-3313). Sapling fell
in the placement batch (S:276-279).

**D7. Refused bucket: resync and inventory.** Fixed 2026-10-07 (buckets.rs `use_bucket`: refusal resends clicked,
above, target, below target; a use resends the inventory in creative too). As round-1 item 16: sapling sends InventoryContent on a refusal
(buckets.rs:53-55 runs whatever `worked` is) and resyncs `pos, face neighbour, below it` in both layers
(buckets.rs:51); BDS sends no inventory packets and 6 layer-0 cells. Fix: only `inventory::change` when the bucket
was used; match BDS's cell list. Low.

**D8. Refused landing order and tick.** Fixed 2026-10-07 (flight.rs: particle on L, entity and item on L+1). BDS: ParticleDestroy on L (B:1265), RemoveEntity then AddItemEntity on L+1
(B:1268/1273, actor and watcher). Sapling: particle, AddItemEntity, RemoveEntity all on L (S:41-43; flight.rs:100-105).
Low.

**D9. Item entity metadata.** 2026-10-07: MOVING now follows BDS for items and falling blocks (sync.rs
`Told::moving`: set a tick after a tick that began in the air, cleared two after landing). 2026-10-24: the full
SetEntityData after AddItemEntity is sent (`mobs::item_shown`); in the kill drill sapling's loot still sends none of
BDS's two Flags-only updates (MOVING set, then cleared). BDS follows AddItemEntity with a full SetEntityData (Flags + Health ...), Flags |=
MOVING the next tick, cleared on landing (B:1274, 1277, 1315). Sapling sends none. Same family as the known
falling-block gap. Low.

**D10. Cosmetic.** 2026-10-07: anvil sound and break particles now go first. The fizz "a tick early" is BDS sending
block changes in a later UpdateSubchunkBlocks batch while events go at once (same tick); not copied. Fizz: BDS sends `SoundFizz` a tick before the hardened block (209500 ->
209548, 221023 -> 221062, 224012 -> 224058); sapling same batch (harden.rs:56). Sapling writes a cell twice in one tick
(lava then cobblestone, S:803-804, 809-810). Anvil: BDS `SoundAnvilFall` before the on_ground delta (B:1670-1672),
sapling after it (flight.rs:113-119); landing motion `-0.0` vs `0.0`. Block break: BDS ParticleDestroy before the
UpdateBlock, sapling after (blocks.rs:259-265).

## Matching now

Falling blocks (slab, anvil, concrete powder into water incl. -0.4527 motion and landing at -60.51/63.49, snow
layer, stack, hand-placed flight): same deltas, ticks and blocks. Buckets: fill/empty sounds at the feet, layer-1
stairs water, lava pit, snow layer (particle at the corner, 4-way flow), grass (no seeds on either this time),
adventure refusal. Lava: east wall 30-tick steps, both walls, water/lava fizz cells. Waterlogging: stairs into a
source, wet-stairs break (3 s, restored), stone into a source, stairs into flowing water (spread then recede, +5
ticks). /summon, /kill: AddEntity/RemoveEntity as before (rest known).
