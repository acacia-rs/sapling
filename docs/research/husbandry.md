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
  a grown one's.
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

## Unverified

- Love lasts 600 ticks, the rest after breeding 6000; a mob looks 8 blocks for a mate and they breed
  after 60 ticks within 3; a birth leaves 1 to 7 experience (the Java game's numbers).
- A horse and a donkey have a mule (not in the packs; BDS's code).
- A tamed mob more than 12 blocks behind teleports 2 to 3 blocks from its owner (the Java game's).
- A failed ride makes a mount 5 calmer (the Java game's; BDS's `attemptTemperMod`, default 5 in
  `MountTamingComponent`'s set-up 141de92d0).
- A lead pulls past 4 blocks, drags past 6 and snaps past 10; a holder not found again for 100
  ticks lets go; a knot sits a quarter block up its fence, and a knot holding nothing goes.
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
  defaults (soft 4, hard 6, max 10) that sapling uses; the exact snap distance between 6.6 and 15.5
  was not pinned.

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
  mount despawn as it does.
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

Riding (the vehicles work reads `Raised::saddled`/`chested`), cats' and parrots' own behaviours
(sitting on chests and shoulders, gifts; parrots below), turtles' scutes, breeding of villagers,
foxes' trust (`entity_born` gives `trusting_fox`: `minecraft:trust` and a `defend_trusted_target`
within 25; BDS keeps `TrustedPlayersAmount`/`TrustedPlayer<n>`, strings at 141171c40 and
141172080): each needs goals sapling's mobs do not have (sitting on blocks, gift loot, trusted
targets). Bees and axolotls in buckets are ported (creatures/bees.rs, creatures/scooping.rs).

## Parrots on shoulders (`parrots`, not built)

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
false) is C++; when it tries again after a dismount was not found. Sapling's seats (riding/, the
vehicles work) take no player as a vehicle.

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
- Herds from natural spawning (above); cats', parrots' and foxes' own goals (Left out).
- The exact distance a lead snaps at, between 6.6 and 15.5.
