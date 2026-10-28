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
a negative `Age` (only BDS's legacy upgraders read `Age`; where today's BDS keeps a baby's progress was
not found). A kept `definitions` list is edited as each event's groups say. A name tag, a lead or an
owner sets `Persistent` and keeps the mob from despawning. Kinds the packs give no `minecraft:ageable`
(a baby zombie) stay babies.

## Unverified

- Love lasts 600 ticks, the rest after breeding 6000; a mob looks 8 blocks for a mate and they breed
  after 60 ticks within 3; a birth leaves 1 to 7 experience (the Java game's numbers).
- A horse and a donkey have a mule (not in the packs; BDS's code).
- The hearts shown on falling in love are `InLoveParticles` (26), not `LoveParticles` (21).
- A tamed mob more than 12 blocks behind teleports 2 to 3 blocks from its owner (the Java game's).
- A failed ride makes a mount 5 calmer (the Java game's).
- A lead pulls past 4 blocks, drags past 6 and snaps past 10; a holder not found again for 100
  ticks lets go; a knot sits a quarter block up its fence, and a knot holding nothing goes.
- A wolf defending its owner looks 16 blocks, bites every 20 ticks within 0.8 past its box, and its
  bite throws nothing.
- Only `<colour>_dye` dyes a sheep (not ink sacs, lapis, cocoa beans or bone meal).
- Creative players keep the food, dye, lead and name tag they use; buckets and bowls still fill.
- A new mooshroom's `MarkVariant` is -1; sheep the shard spawns are white; a baby's coat is not its parents'.
- A stored mob in love does not show its hearts until fed again.

## Left out

Riding (the vehicles work reads `Raised::saddled`/`chested` and calls `owners::tame_by_riding`),
cats' and parrots' own behaviours (sitting on chests and shoulders, gifts), turtles' eggs and
scutes, breeding of villagers, foxes' trust, bees, axolotls in buckets, natural herds of babies.

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
