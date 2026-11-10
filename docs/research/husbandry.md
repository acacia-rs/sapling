# Husbandry: breeding, babies, taming, leads and what animals give

What sapling's `creatures/husbandry` does and where each number comes from. No BDS traffic of any of
it was recorded; the packs and the binary are the evidence, and the rest is listed under "Unverified".

## Sources

- **The vanilla behaviour packs** (`acacia/.testserver/bds-blocks/behavior_packs`, read through
  `crates/sapling-data/data/generate/husbandry.py` into `husbandry.tsv`, sapling-data README
  "Husbandry"): `minecraft:breedable` (breed items, mates, `require_tame`, `require_full_health`),
  `minecraft:ageable` (duration 1200 s = 24000 ticks, each feed item's `growth` share, 0.1 where it
  names none), `minecraft:tameable` (items, `probability` 0.33 for wolves, cats and parrots),
  `minecraft:tamemount` (temper 0 to 100, a llama's 0 to 30, each food's `temper_mod`),
  `minecraft:healable` (a parrot's cookie heals 0: it poisons, left out), `minecraft:spawn_entity`
  (a hen's egg every 300 to 600 s, an armadillo's scute), `behavior.follow_owner` (start and stop
  distances: wolf and cat 10 and 2, parrot 5 and 1), `minecraft:sittable`,
  `behavior.owner_hurt_by_target` (the wolf), `minecraft:leashable`, `behavior.tempt` (items and
  speed), `minecraft:movement`, and the component groups the `entity_born`, `ageable_grow_up`,
  `on_tame`, `on_sheared` and `on_eat_block` events swap.
- **The packs' `minecraft:interact`**: shears on a sheep (`loot_tables/entities/sheep_shear.json`:
  1 to 3 wool of its `Color`, not a lamb, `hurt_item` 1, sound `shear`); a bucket on a cow and a
  grown goat (`milk`); a bowl on a mooshroom (`mooshroom_milking.json`: mushroom stew at
  `mark_variant` -1, else suspicious stew of that data; sound `milk_suspiciously`); a flower on a
  brown (`variant` 1) mooshroom sets `mark_variant` (poppy 0, cornflower 1, tulips 2, azure bluet 3,
  lily of the valley 4, dandelion 5, blue orchid 6, allium 7, oxeye daisy 8, wither rose 9,
  torchflower 10, open eyeblossom 11, closed 12; sound `eat`); shears on a mooshroom (5 mushrooms of
  its colour, becomes a cow); a brush on an armadillo (a scute, `hurt_item` 16). `behavior.eat_block`
  on a sheep: grass block to dirt, short or tall grass to air, success chance 0.02 a lamb's and 0.001
  a grown one's. BDS's lambs (bds-j3sheep.txt) ate only the block under them, never grass beside
  them, and the block went 36 ticks (`time_until_eat` 1.8 s) after `EatGrassAnimation`; sapling eats
  and regrows wool then too. 64 summoned in plains came `Color` 0 ×49, 15 ×7, 7 ×3, 8 ×3, 12 ×2:
  a spawned (not bred) sheep's colour is rolled by its biome's `*_color` event, in AddEntity.
- **BDS's binary** (`bdsre`, strings): actor NBT fields `InLove`, `LoveCause`, `BreedCooldown`,
  `OwnerNew`, `LeasherID`, `Temper`, `IsBaby`, `IsTamed`, `Sitting`, `Sheared`, `Saddled`,
  `Chested`, `Color`, `MarkVariant`, `CustomName`, `Persistent`, `definitions`; sound names
  `lead.leash`, `lead.unleash`, `lead.break`, `leashknot.place`, `leashknot.break`, `plop`,
  `mob.armadillo.brush`, `shear`, `milk`, `milk_suspiciously`.
- **Recorded AddEntity** (`sapling-data/data/Mobs.bin`): a baby is announced with the grown box and
  `Scale` 0.5; the box it collides with is the scaled one.

## What sapling keeps

`sapling_leveldb::Raising` reads and writes the fields above; a baby's ticks left to grow are kept as
a negative `Age`, as BDS keeps them (a calf born just before was kept as Int -23879;
research/captures/bds-t2ekeep-dump.txt). A kept `definitions` list is edited as each event's groups say. A name tag, a lead or an
owner sets `Persistent` and keeps the mob from despawning. Kinds the packs give no `minecraft:ageable`
(a baby zombie) stay babies.

## A click's swing (bds-b1click.txt, `b1click` in fourteenth1b_drill.rs)

Each click on a mob that BDS takes (wheat to a cow, shears to a sheep, a bucket to a cow) sends the clicker and
its viewers `Animate` `SwingArm` with the source `interact`, in the tick of the click; a click not taken (a stick)
sends none. The client's own swing is relayed to the viewers with the source it gave. Sapling: `actions.rs`
`swing`, `movement.rs` `relay_swings`; tests `a_click_a_mob_takes_swings_the_clickers_arm_for_it_and_its_viewers`,
players.rs.

## Unverified

- Love lasts 600 ticks, the rest after breeding 6000; a mob looks 8 blocks for a mate and they breed
  after 60 ticks within 3; a birth leaves 1 to 7 experience (the Java game's numbers).
- A tamed mob more than 12 blocks behind teleports 2 to 3 blocks from its owner (the Java game's).
- A failed ride makes a mount 5 calmer (the Java game's; BDS's `attemptTemperMod`, default 5 in
  `MountTamingComponent`'s set-up 141de92d0).
- A lead pulls past 4 blocks, drags past 6 and snaps past 11 (see "A lead" below); a holder not
  found again for 100 ticks lets go; a knot sits a quarter block up its fence, and a knot holding
  nothing goes.
- A wolf defending its owner looks 16 blocks, bites every 20 ticks within 0.8 past its box, and its
  bite throws nothing.
- Creative players keep the name tag they use; buckets and bowls still fill.
- Sheep the shard spawns are white; a baby's coat is not its parents'.

## Drilled 2026-11-05 (`n13herd`, bds-n13herd.txt vs sapling-n13herd.txt)

- Same: a sheep offered an ink sac, lapis, cocoa beans or bone meal shows nothing; red dye sets
  `Color` 14; a creative player keeps the dye, wheat and lead (no `InventorySlot`); a lead shows
  `LEASHED` and `LeadHolderEid`, and `lead.leash` from the sheep's middle.
- Fixed: `lead.leash` is told as nobody's (`entity_type` "", unique id -1); a summoned mooshroom is
  announced with `MarkVariant` -1 (it was 0 until raised, shown.rs).
- Differs, not fixed: holding wheat near the sheep set flag bit 6 (tempted) on BDS, cleared when the
  lead was held; sapling shows no such flag. BDS's feed sent `EatingItem` (data 22085632), then
  `INLOVE`, then `LoveParticles` (21) every 800 ms while in love (bds-g11love-2.txt: at random gaps
  for cows); sapling sends `InLoveParticles` (26) once. BDS sent no `OwnerEid` with the lead;
  sapling sends -1. BDS's `lead.leash` came a tick after the metadata.
- A stored mob in love does not show its hearts until fed again.
- Tempt (`ah3tempt`, bds-ah3tempt.txt): cows 5, 9, 11 and 14 off the actor holding wheat all came at
  once at 0.2108 a tick. Each kept walking 2.27 off and stopped walking 2.22 off, coming to rest 1.81
  to 1.97 off. Sapling followed only within 10 and stopped walking under 2.5. Fixed: within 16 (the far
  bound not reached), stopping under 2.25 (`husbandry/breeding.rs`). Re-drilled, sapling's cows came to
  rest within 0.001 of BDS's. Test `a_cow_10_6_off_follows_wheat_and_stops_about_2_off`.

## Drilled (2026-11-04, `bds_command_probe n3ride`, `n3foals`)

- **A wild mount lets its rider stay 60 ticks** (research/captures/bds-n3ride.txt): three fresh
  horses each showed `TameSuccess` or `TameFail` 2.9 s after the rider's link, the failed one
  throwing the rider off then (`SetEntityLink` type 0, `rider_initiated`, and a
  `CorrectPlayerMovePrediction`). `MountTamingSystem::tick` 14848cba0 → 14848d3c0 counts the ticks
  a player rides it, resets the count when nobody does, and on the 60th rolls the temper as the
  Java game does. Two of the three were tamed by their first ride: a new mount's temper is rolled
  in its range (148f38bb0), not 0. Sapling: `riding/breaking.rs`, `owners::first_temper`; test
  `a_wild_horse_throws_its_rider_off_until_its_temper_lets_a_ride_tame_it`. Re-drilled
  (sapling-n3ride.txt): `TameFail` 2.95 s after each link, as BDS's 2.9.
- **A foal's strides are its parents' blended** (bds-n3foals.txt, six pairs of tamed horses fed
  golden carrots): each of a foal's health, movement and jump strength lay within the parents' mean
  ± half of (their gap + 0.3 of the kind's range), and its health was not whole (22.289 from 24 and
  23). That is Java's `createOffspringAttribute` (mean + spread × (three fractions' mean − 0.5),
  mirrored at the range's ends); the six foals' draws came out −0.26 to 0.22. The C++ was not found
  (no `blend_attributes` user decompiled). Sapling: `sapling_data::blend_strides`,
  `creatures/strides.rs` `blend`.
- **A lead** (bds-n3ride.txt, a cow led by a player teleported 3 blocks further each 3 s): it held
  6.6 blocks off and the cow walked up to 0.5 to 3.5 of the holder; it snapped (the lead dropped
  where the cow stood) the tick after the holder was put 15.5 off. Consistent with the pack's
  defaults (soft 4, hard 6, max 10); the exact snap distance between 6.6 and 15.5 was not pinned.
- **Leads and knots** (bds-uuknot.txt, `bds_command_probe uuknot`, a slowed cow at x -6.5 and its
  holder teleported from 1 block off to 7-15 off): the lead held at 11 blocks (the cow was pulled
  along) and snapped at 11.99, so sapling snaps past 11. A snap is heard `lead.break` as no one's
  from the holder's middle (feet + 0.9); the lead drops at the cow. A knot's feet are 0.25 above
  its fence's floor, `leashknot.place` from its middle and `leashknot.break` from its feet, both
  as no one's. The cow tied to a knot dying took the knot away in the same tick. A click on the
  knot handed the cow back to the player (`LeadHolderEid` the player's, `lead.leash` from the
  player's middle, no lead dropped) and the knot went the tick after. Sapling does all of these
  (`creatures/husbandry/knots.rs`, `leads.rs`); a kept knot from a world folder still waits 100
  ticks for its mobs. What a hit on a knot does was not drilled.

## Herds (ported from the packs)

A spawn rule's `herd` list (`event`, `event_skip_count`, `initial_event`, `initial_event_count`)
is now in spawn-rules.tsv's `herds=` field; `_spawnMobCluster` 1415ed230 picks one herd at random
(`nextInt(herds)`) and fcn 1415ed0d0 fires `initial_event` on members below `initial_event_count`
and `event` on those at or past `event_skip_count`. Sapling (`spawning/herds.rs`) spawns with the
events that make a mob (babies of fox, axolotl and polar bear herds, patrol captains and followers),
so a fox herd is two grown then babies, and fires the others on the spawned mob
(`creatures::fire_herd_event`). Natural spawning is not drillable live (random spot and time), so
the events were drilled by summoning with them (2026-11-05, `h3herds`, bds-h3herds.txt against
sapling-h3herds.txt):

- Looks (`creatures/looks.rs`): a horse's `entity_spawned` rolls one of 7 coats (`Variant`) and 5
  markings (`MarkVariant`); `make_<coat>` sets the coat (BDS's `make_black` came 4, 0). A tropical
  fish's rolls `Variant` 0-1, `MarkVariant` 0-5, `Color` and `Color2` 0-14; `become_<name>` sets all
  four from the pack's group (clownfish 0, 0, 1, 0; red snapper 1, 3, 14, 0). A summon's event
  replaces `entity_spawned`; a herd's comes after it, so a herd's horses share a coat and keep their
  markings. `Color2` is saved (sapling-leveldb `Raising`) and announced. Sapling matched BDS on the
  four fired looks; the rolled ones are random on both sides.
- Riders (`creatures/riders.rs`): a zombie horse's `spawn_adult_with_rider` gets a zombie
  (`spawn_as_rider`), a camel husk's `spawn_with_rider` a husk in front and a parched (`ranged_mode`)
  behind; a drowned's `as_rider` makes a zombie nautilus half a block up and rides it (new seat
  layouts in `riding/seats.rs`: camel husk as the camel, the nautilus (0, 0.925, -0.2) for the
  drowned family). Each rider is announced already in its seat, as BDS's were. Riders of a natural
  mount despawn as it does. A rider a mount's event made gets no first data (only its flags follow its
  AddEntity), where the mount, and a mount a rider's event made, get one (bds-h3herds.txt, bds-ar2look.txt;
  `entities::skip_first`).
- Coats: a cat's `entity_spawned` rolls one of its 11 coats, 15 weight each (`looks.rs`; BDS's summoned
  cat came 6, bds-ar2look.txt). A wild llama shows 0 slots per strength, a tamed one 3 (bds-ar2look.txt,
  bds-mountdeath.txt).
- Riders' equipment: each rider rolls its rider group's `minecraft:equipment` table
  (`riders::GEAR`): the zombie and the husk `zombie_rider_equipment` (an iron spear), the parched
  `skeleton_gear` (a bow), the drowned rider `drowned_rider_equipment` (a trident, no shell;
  drowned.rs), as BDS's held them. The tables' `enchant_random_gear` (BDS's zombie's spear came
  enchanted) and armour pools (BDS's parched wore a helmet) are not rolled.
- A horse, donkey, mule, zombie horse or skeleton horse summoned with no event rolls its pack's
  `entity_spawned` foal weight (9 of 45, 20 of 100) and comes a foal by `entity_born` (BDS: 2 of 4
  plain horse summons; `birth::summoned`). The other kinds' foal weights (cows 5 of 100 and the like,
  zombies' babies) are not rolled.
- The husk's `Variant` 2 and the parched's 4, as BDS announces every one (`genes::FIXED`).
- Links: BDS sent no seating link (`type` 1 or 2) `immediate` in any capture; sapling's links told
  to later viewers were, and are not now. Left: BDS announced each rider before its mount, with the
  links in the mount's `AddEntity`, where sapling sends the mount first and the links after (see
  vehicles.md "Unverified", the send order line of leftovers.md).
- Members a herd always includes: BDS parses `permute_type` `guaranteed_count` (0x141d3f910), but no
  vanilla spawn rule sets it, so there is nothing to build.

## Left out

Riding (the vehicles work reads `Raised::saddled`/`chested`), parrots' own behaviours (shoulders,
below), breeding of villagers, a cat's sitting on blocks, a fox's `defend_trusted_target` (within 25)
and saving its trust (BDS keeps `TrustedPlayersAmount`/`TrustedPlayer<n>`, strings at 141171c40 and
141172080). Bees and axolotls in buckets are ported (creatures/bees.rs, creatures/scooping.rs).

## Cats and foxes (thirteenth2ar; `ar2fox`, `ar2sneak`, `ar2cat` in thirteenth2ar_drill.rs)

- Foxes flee players (`behavior.avoid_mob_type`), ported in `husbandry/avoiding.rs`. BDS's foxes 3 and 3.6
  blocks from a standing survival player fled at once. Foxes 4, 5.8 and 8.7 off did not. That fits the
  fox's box grown 3 each way meeting the player's box. Each ran to a spot further from the player at
  0.43 a tick within 7 and 0.2 beyond (1.5 times and once its pace), 2 to 10 blocks, and stopped near a
  cell's middle. `FlagsExtended` bit 22 (`1 << 22`) was on while it ran. Next to a sneaking player
  nothing fled (bds-ar2sneak.txt). When the player stood up, the fox 2.8 off fled on that tick. Foxes born
  by `entity_born` fled as well: they trust only their breeders. Sapling's bred fox now trusts the
  players who fed its parents (Java's rule; not drilled; not saved). In sapling's first run
  (sapling-ar2sneak.txt) a fox ran straight past the player to a spot that was only further away. Its
  spot must now lie on the far side and out of the box. The born foxes fled on their first tick on both.
- A cod click that puts a tamed cat in love leaves it sitting. Each cod click while it is in love sat it
  down or stood it up, 5 of 5 in both av2cat runs (bds-av2cat.txt, 2026-11-08); the earlier "kept
  sitting" reading of bds-ar2cat.txt had only the love-starting click. `clicks.rs` toggles it again.
- A cat tamed by feeding sits: BDS sent `TAMED` and `SITTING` in one packet (bds-ar2cat.txt). A parrot
  did not sit (bds-parrots.txt). The wolf is undrilled. Ported in `owners.rs` for cats only.
- Sleeping with its owner (`pet_sleep_with_owner`), ported in `husbandry/bedtime.rs`. The tick the owner
  lay down, the stood-up cat set off from 8 blocks away at 0.28 a tick (1.2 times its pace). It stopped
  on the foot of the bed, 0.3 from the cell's middle at the bed's top. About 6 ticks later it showed
  `RESTING` (bit 23). When the night was skipped it lost `RESTING` on the owner's wake tick and was put at
  (-2, 7), 4.7 blocks off, an unexplained spot. It gave no gift in that run (the gift comes 7 times in 10).
  Sapling's cat (sapling-ar2cat.txt) set off on the same tick, lay at (-2.32, 2.55), got up on the wake
  tick and gave its gift then. It is not moved first.
  Re-drilled 2026-11-08 (group AV, `av2cat`, bds-av2cat.txt, same bed and owner spots): put at exactly
  (-2.0, -26, 7.0) again on the wake tick, flags first, then a gift (`AddItemEntity` at its new feet +
  0.285). Read under gdb (thirteenth2ay `ay2gdb`, bds-ay2gdb.txt; Linux ELF `DropItemForGoal` vtable
  0xeee83c8: start 0x5774b40, findTargetBlock 0x57754a0, isValidTarget 0x57757a0, tick 0x5773eb0):
  around the block of the owner's position (eyes, (-3.5, -24.38, 3.5) on waking) it scans as
  `MoveToBlockGoal` (layers from one under, rings out to `search_range` 5, height 2) for a solid block
  with air over it, keeping the first whose squared block distance is strictly nearest `offering_distance`
  squared (+0xac = 25): (-2, -27, 7). A cat further than `minimum_teleport_distance` (+0xb0 = 2, held
  unsquared but compared with the squared distance) from its corner is put there plus `teleport_offset`
  (0, 1, 0) (0xb2dacf0). Ported (husbandry/offering.rs, bedtime.rs `wake`; test
  `a_cat_waking_with_its_owner_in_the_morning_goes_to_its_offering_spot`).
- Sitting on blocks (`ocelot_sit_on_block`): in 40 s the stood-up cat went around the lit furnace, beside
  it on three sides, and never got on top. It never went near the chest or the bed. In av2cat a standing
  tamed cat had a chest alone for 80 s and a bed alone for 80 s: it walked over the chest once and never
  sat on either. Sapling's cats do not sit on blocks either: same as far as drilled.
  Drill trap: extra feeds toggle a cat in love, so a following click may sit it (av2cat's first run).

Turtle scutes (bds-ak1scute.txt, ported): a baby turtle grown up (at its tenth seagrass there) drops its
`minecraft:ageable` `drop_items` (`turtle_shell_piece`, named `turtle_scute` now), one, from its feet +
0.125 tossed 0.2 up, before the grown scale's SetEntityData in that tick, the scute's own data after it
(husbandry.tsv `sheds`, husbandry/products.rs `shed`). Sapling announces new entities at the tick's end,
so the shed item is shown at once to viewers within range (mobs/at_once.rs), as an allay's throw is.

Allays handed items (bds-ak1allay.txt, ported in husbandry/allays.rs): a click with anything but a lead,
not sneaking, on an allay with an empty hand gives it one of the stack (MobEquipment, the hand keeps the
rest, same tick); the next tick `item_given` and its flags with `ACTION` and `OwnerEid`. A click with
something on one that holds something does nothing; it dropped its item when killed (not ported).
`ambient.tame` (bds-aq3allay.txt, ported `allays::check_ambient`): BDS's `AmbientSoundServerSystem`
re-reads an allay's `ambient` event names when its counter passes its period, then restarts it; read
under gdb (bds-aq3exit, Linux ELF 0x6d05e1f) the allay's period was 50, so it checks 50 ticks after it
appears and every 51 after. Holding something it sounds `ambient.tame`, else `ambient`, told as a
SetEntityData with that name alone; in all five drills it switched 152 ticks after appearing. The
period comes from a random draw at init (Windows 0x145e927d0); every allay drilled drew 50 (pigs 60).

Allays collecting and giving back (thirteenth3ap 2026-11-07, bds-ap3give.txt and bds-ap3note.txt; ported
in husbandry/allays/collecting.rs and giving.rs, tests integration/thirteenth3ap.rs). The pack's
priorities: pickup_items 2 (max_dist and search_height 32, goal_radius 2.2, speed 6, same items as in
hand), give to a note block 3, give to its owner 4 (both run speed 8), stay near the note block 5 (from
16 off until 4), follow_owner 6, random_hover 9; inventory one slot.
- The tick after matching dirt lay 9 off it flew at it (push 6 x 0.0098 on the hover's ramp, about 0.43
  a tick) and took it once its move left it within 2.2: `TakeItemEntity` to the allay, its hand
  unchanged. It left cobblestone alone and took every matching item lying near before giving.
- It then flew at its owner's eyes (8 x, about 0.58 a tick) and, before its move within 3 of them,
  threw one item a tick: shown 0.6654 over its feet, moving along the line to the eyes a block up at 0.2
  across and 0.3 up (decompiled 0x5745670). The player took it 6 to 9 ticks later. After each throw it
  took nothing up for 3 s (`pickup_item_delay`).
- `item_thrown` came on some throws only (5 of 14 in the last note run, 1 of 9 in the first): the throw
  plays it when the level's `currentTick` (Level vtable 0x280) is a multiple of 7 and then a roll is
  under 0.9 (0x5745670); the sounded throws in bds-aq3allay.txt fall 98, 231 and 350 ticks apart.
  Sapling reads its level's age for it, and shows the item, then the sound, then the item's data.
- A note block played within 16 (its vibration listener) drew the dirt instead of the owner, thrown at
  the block's centre a block up from within 3; 3 to 3.7 s on it took the same dirt up again and threw
  it back, over and over, until about 30 s after the note (the goal's `+0x54` seconds, 0x5743700; the
  first throw to the owner came 34.3 s after it, sapling's 33.8 s). Played again, the next dirt went
  there again. Within 16 it did not fly to the block with nothing to give.
- Sapling's flight is straight, with no path: aimed at an item itself it sank to the floor and stuck on
  the note block's side, so it aims at the middle of the item's cell (BDS's took items from 0.6 to 0.8
  over the floor). Its random hover ignores slowness, so the drills put it back before handing it dirt.
  The last runs (research/captures/bds-ap3give.txt, bds-ap3note.txt, sapling-ap3give.txt,
  sapling-ap3note.txt) match in order: take, throw, retake 3.3 to 3.5 s on, give to the owner after the note.
- Following its owner (thirteenth3aq, bds-aq3allay.txt): left 30 blocks off, BDS's flew at the owner
  at about 0.56 a tick and hovered once about 3 to 4 off; sapling's `follow` flies at the owner's eyes
  from over 16 off until within 4 (giving.rs).
- What it carries is kept with the world (bds-aq3allay.txt's world): `ChestItems` slot 0 (`Count`,
  `Name`, `WasPickedUp 1`); sapling writes it on each take and throw and reads it back on load
  (allays.rs `keep_carried`, `adopt`).
- Its owner is kept too (thirteenth3as, `as3allay` then `as3allay2` on the kept world;
  research/captures/bds-as3allay-dump.txt): `OwnerNew` is the giver's unique id with `IsTamed` 0, and
  after a restart the allay was announced with that `OwnerEid` and followed the owner. Sapling keeps it
  as a tamed mob's (`Raised` owner, allays.rs `hand`; test
  `an_allay_s_owner_is_kept_with_the_world_and_shown_when_it_is_loaded`). After the restart BDS's
  allay took dropped dirt but threw none to its owner: none in 11 s (as3allay2), none in 40 s (av2allay2,
  2026-11-08). Its `OwnerEid` stayed the actor's first-launch unique id (-4294967295) while the rejoined
  actor came back as -8589934589 (the dropped dirt's owner), so no player in the world is its owner. Sapling
  keeps a player's unique id in its player data, so its allay still delivers. Settled 2026-11-08
  (thirteenth2ay `ay2ident`, bds-ay2ident.txt): within one launch each rejoin got a new unique id
  (-4294967295, then -4294967274, -4294967273), lost the diamonds it held (`clear` found none) and its
  position, and BDS wrote no player record to the world: the probe's offline login (no xuid, empty
  self-signed id) is a new player each time. The ownerless allay is a probe artifact; sapling's allay
  delivering to a returning player is as BDS for a recognized one.

## Parrots on shoulders (built 2026-11-07, group AS: husbandry/shoulders.rs)

`bds-as3parrot.txt` (thirteenth3as_drill.rs): a parrot summoned with `on_tame` came tamed and owned by
no one, and mounted any player, owner or not: about 100 ticks after it appeared it flew to the nearest
and got on some 28 ticks later. A second shoulder took (-0.4, -0.2, -0.1); BDS told the first parrot's
link (type 1) again, then the new one's (type 2), not the first one's seat. A fall threw both off, the
newest first (type 0, not immediate, no relinking), once it reached 1.59 to 2.11 blocks; the head going
under water threw them a tick after its air began to drop; damage did not. They came back 7 to 13 s
later. Sapling: `find_mount` at 100 ticks, 16 and 2 (measured to the player's eyes, where it flies),
off at a 2-block fall or eyes in water, the 100 ticks again after (tests in thirteenth3as.rs).
Re-drilled (research/captures/sapling-as3parrot.txt): the mount, the second shoulder's links and the fall's
links match; sapling's second parrot was tamed by the fifth seed (BDS's the eighth), so the three
clicks after sat it down and it did not come back. `on_tame` now tames a parrot (owners.rs).
Earlier, `bds-parrots.txt`:

`bds-parrots.txt` (survival actor, a parrot tamed with seeds on the fifth try): the parrot, tamed and

`bds-parrots.txt` (survival actor, a parrot tamed with seeds on the fifth try): the parrot, tamed and
standing, sat on the actor 2.3 s after it was last stood up by a click: `SetEntityData` with
`RiderSeatPosition` (0.4, -0.2, -0.1) and `RiderRotationLocked` 1, then `SetEntityLink` type 1 (rider,
`rider_initiated`, not immediate) with the player's unique id as the ridden, then `mount` at the player's
eyes. Its `RIDING` flag (bit 2) was never set; only its `MOVING` bit (34) went on and off, and no
`MoveEntity` was sent while it sat. A jump, sneaking and `/damage @s 1 entity_attack` left it there; a
`tp ~ ~4 ~` took it off about 9 ticks into the fall (`SetEntityLink` type 0, not immediate), likely
Java's `fallDistance > 0.5`. It did not sit on the player again in the 18 s after. The actor left and
came back as a new player (see bundles.md "Open"): the parrot, still in the world, sat on the newcomer
as it was shown (no delay), and its death sent the link's removal `immediate` after `RemoveEntity`.
The pack's `find_mount` (`start_delay` 100, `within_radius` 16, `mount_distance` 2, `target_needed`
false) is C++.

## A tamed wolf's guard (2026-11-08, `g14wolf`, bds-g14wolf.txt)

A wolf tamed by bones (bit 24 `SITTING` and 26 while it begged, both off once fed and the bones gone)
took a zombie for its foe the tick after the zombie hurt its owner, not when the zombie only took the
owner for its target. Standing 2.4 off on the ground it leapt at once: motion told 0.4 toward the
zombie and 0.4 up (`behavior.leap_at_target` `yd` 0.4), then fell as a thrown mob. Its bite (swing,
the zombie's hurt, `attack` from its middle a tick on) came 0.75 off centre to centre and threw the
zombie 0.4 across and up as any hit does; the zombie turned on it and threw it back the same tick.
BDS told both throws' 0.4 at once, as sapling now does (hunting/prey.rs `hit`; mob-kinds.md
"Mob knockback").
Bites came 58 to 94 ticks apart, each after both were thrown apart: no pace can be fitted from them.
Ported in husbandry/guarding.rs; test `a_tamed_wolf_leaps_at_what_its_owner_hits_and_its_bite_throws_it`.

`x2wolf` (fourteenth2x, bds-x2wolf.txt): six wolves fed four rounds, five tamed (each feed after taming flips
`SITTING`); the two left standing (ids 19, 20, tamed in round two) bit a held, warded, healing zombie: 13 bites, 20 ticks apart at the least (19.9 to
20.1), 31 to 35 when the zombie had thrown the wolf back; feet to feet 0.61 to 1.421 (dz 1.41 at the widest),
inside `melee_attack`'s reach `(width × 2)² + foe width` = 1.428 and past the hunters' per-axis 1.4 box. Sapling's
guard now bites by that reach (guarding.rs `reaches`), its pace 20 as before.

`af2wolf` (fourteenth2af, bds-af2wolf.txt): four wolves tamed and left standing east of a three-high stone wall,
the survival owner and a held zombie west of it. The zombie hurt the owner at 38.6 s; all four wolves set off
without sight of it (1.4 to 12 s later), along the wall to its ends at z ±6 and round, and bit it: `owner_hurt_by_target` needs no sight (goal+0x32 `must_see` off by default), as
sapling's guard, which takes no sight either. sapling-af2wolf's wolves took it up too but pressed on the
wall at x 3.3, walking straight: the guard's chase now walks as a hunter does, by a way round where the
floor bars it (guarding.rs `chase` through `hunting::approach`; test fourteenth2af
`a_tamed_wolf_walks_round_a_wall_to_what_its_owner_hits`).

## Babies following their parents (2026-11-09, `af2baby`, bds-af2baby.txt)

`behavior.follow_parent` is BDS's `FollowParentGoal` (Linux vtable 0xeee5ba8; canUse 0x57420e0,
canContinueToUse 0x57425a0, tick 0x57427d0), Java's rule: a baby (flag 11) takes the nearest grown,
living one of its own `ActorTypeComponent` among the entities its box grown by 8, 4, 8 fetches, if that one
is 3 or more off (squared 9 at 0x1098858); it keeps following while 3 to 16 off (256 at 0x1098344),
its path to that one found again every 10 ticks, at its movement times `speed_multiplier`.
On BDS a baby cow, pig and sheep 7 east of a held grown one walked straight to it at 0.163 a tick
(movement 0.25 × 1.1, the same track for all three) and stopped 2.79 off. A piglet seemed to take up
a pig 9.72 off along x and a lamb 9.75 off did not; fourteenth2ai settled it (below): the search is
the goal's box, 8 across, and the piglet had strolled into it. A cow's follow and stroll share priority
6 (its `cow_baby` group's over its own 5), so a calf on a stroll walked it to its end, past and away
from its mother, and followed only after (`strolls_on` in mob-traits.tsv; also mooshroom and rabbit).
A plain `/summon` is a baby 1 time in 20 (the first run's "mother" pig was one): summon parents with
`spawn_adult`. Sapling: husbandry/parents.rs; test
`a_baby_cow_walks_to_a_grown_one_and_stops_three_off`. sapling-af2baby: all three stopped 2.8 to 3.0
off their mother's first spot as BDS's; after that, both servers' babies stroll and follow by chance.

`ai2kin` (fourteenth2ai, bds-ai2kin.txt, gdb on canUse's fetch 0x5742296 and its take 0x574256f in
research/captures/bds-ai2kin-gdb.txt): the box is the baby's own grown by 8, 4, 8 (constants 0x13e9250,
0x13e5d50, 0x1098808, 0x1098494), and BlockSource::fetchEntities (slot 29, 0xc8db0f0, per chunk
0xc9e0ce0) keeps the entities whose box meets it strictly. Lambs and piglets 9, 9.75 and 11 east of a
held grown one took none while they stood; all four takes came at 8.2 to 8.75 off, once a stroll had
brought them in (8 + the halves of both boxes, 8.675 across). So bds-af2baby's piglet strolled in, and
its lamb at 9.75 was out of reach. parents.rs `SEARCH` is back to 8, 4 (strict); test fourteenth2ai
`a_piglet_nine_off_does_not_take_up_a_grown_pig_and_one_seven_off_does`.

## Group K (2026-11-08, fourteenth2k_drill.rs)

- `k2babies` (bds-k2babies.txt): of 100 plain summons each, 4 cows, 6 pigs, 6 chickens, 3 sheep, 3
  mooshrooms and 24 cats were babies: the packs' `entity_spawned` 5% and 25%. Baby pigs, cows and
  chickens rolled no voice (property index 1 left 0 in all 16); baby cats did. Sapling: birth.rs
  `BABIES`, properties.rs `rolled`.
- `k2mounts` (bds-k2mounts.txt): a tamed horse and donkey fed golden carrots had a mule at their
  feet with 3 orbs, as sapling's breeding.rs `MIXED`. A tamed donkey's chest click set `CHESTED`
  with `armor.equip_generic` (riding/packs.rs; mob-life.md "Mount inventories"); a second chest
  click rode it, a saddle saddled it, a sneaking click opened it, and its death dropped the chest
  and the saddle.

## Not drilled

- `property_inheritance` (which parent a baby's climate variant comes from): bdsre finds no
  `property_inheritance` string or hash user to read. `u1inherit` (bds-u1inherit.txt, 2026-11-04,
  an ice plains world with cows summoned in the End and teleported over) gave no pair: the End's
  cows came out cold too, as sapling's, and no calf came in 15 s (the pens' slowness 255 kept the
  loving pairs apart; the bot also feeds only within 2.8 of its eyes and in sight, so it fed one
  of eight). Sapling's `/execute ... in overworld run tp` of a mob answered success and left it in
  the End: fixed (2026-11-05, `entities::teleport_into`, `ShardCtx::move_entity_into`; BDS took the
  mob off the old dimension's screens and announced it at its new place, under a new runtime id
  and the same unique id; sapling kept both until 2026-11-05, now gives the next runtime id
  (mobs/rehomed.rs); test `a_mob_sent_into_another_dimension_leaves_one_screen_for_the_other`). `w1inherit`
  (bds-w1inherit.txt, sapling-w1inherit.txt, the flat plains world): two cows summoned in the
  Nether came warm, two in the End cold, the overworld's temperate, on both. Each pair was fed
  before the pens closed. BDS made three calves in 25 s: warm x cold gave cold (the pair is known
  by when the parents' love particles stopped), then a cold and a temperate one. Sapling made
  four: warm, temperate, temperate, cold. Every calf took one parent's climate on both sides,
  which fits sapling's random pick of a parent; three calves cannot show the odds.
- Herds from natural spawning (above); parrots' goals and a cat's sitting on blocks (Left out).
- The exact distance a lead snaps at, between 11 and 11.99 (bds-uuknot.txt).
