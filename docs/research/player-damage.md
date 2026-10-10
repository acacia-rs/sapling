# A player harmed, killed and risen

What BDS sends, from one run of `bds_command_probe hurt` (2026-10-10, research/captures/bds-hurt.txt): a
survival player (the bot, runtime id 1) and a watcher; breath and blocks from `smother`
(research/captures/bds-smother.txt). No decompile. Sapling builds it in `sapling-shard/src/vitals.rs` and
`vitals/death.rs`; where it differs is under "Sapling" below.

## Harm

- Each harm is told at once as `EntityEvent` `HurtAnimation` to the player and to those who see it, and the
  health follows as `UpdateAttributes` `minecraft:health` a tick later.
- The event's `data` is the cause: -1 for `/damage` without one, 2 `entity_attack`, 5 `fall`, 6 `fire`,
  7 burning on, 8 `lava`, 9 `drowning`. A fall's event carries `fire_at_position`, the player's eyes.
- `/damage @s 1` twice a second apart took 1 each; two within the 10 tick guard were not tried.
- A fall (the bot's own falling, after `/tp` up): 6 blocks cost 3, 12 cost 8, 40 killed, with the sounds
  `fall.big` and `fall` at the feet.
- Fire: 1 on stepping in (cause 6) and the burning flag, then 1 every 10 ticks (cause 7) while in it; once
  out, 1 every 20 ticks for 8 seconds more, then the flag clears.
- Lava: 1 as fire (cause 6), then 4 every 10 ticks (cause 8).

## Breath and blocks

- With water at its eyes a player's `Air` (`SetEntityData`, told every tick) falls 1 a tick from 300. At
  0 it goes on to -20, the player is hurt 2 (cause 9) and `Air` is 0 again: 2 every 20 ticks.
- Out of the water `Air` rises 4 a tick to 300.
- A block set over the head alone hurts nothing: BDS lays the player down (`BoundingboxHeight` 0.6 and a
  flag in `FlagsExtended`) until the block is gone.
- Both of its blocks filled with stone: 2 (cause 4) from about the 8th tick, then every 10 ticks, while
  `Air` falls as under water.

## Mending

- With hunger at 20 a hurt player gains 1 health every 80 ticks. Each gain adds 6 to
  `minecraft:player.exhaustion`; every 4 of exhaustion takes 1 from `player.saturation` (5 at first), and
  with saturation at 0 from `player.hunger`. All four are told by `UpdateAttributes`.

From `bds_command_probe hunger` (research/captures/bds-hunger.txt):

- The mending runs while hunger is 18 or more: hurt 14, a player mended six times and stopped at hunger 16.
- Exhaustion is spent 4 a tick, in the tick it is there: a mending's 6 leave 2, the next leaves 4 and the
  tick after 0. `/damage` adds no exhaustion.
- The hunger effect at amplifier 255 adds 1.28 of exhaustion a tick.

From `bds_command_probe tire` (research/captures/bds-tire.txt), the bot holding keys:

- A jump adds 0.05 of exhaustion, a jump while sprinting 0.2.
- Sprinting adds 0.1 a block run, counted only in ticks that end on the ground: between two sprinting
  jumps only the landing's step is added.
- Walking adds none.

From `bds_command_probe toil` (research/captures/bds-toil.txt) and `bite`:

- A block of dirt broken by hand adds 0.005. A block of stone broken by hand, which leaves
  nothing, adds none.
- A hit that lands on a cow adds 0.1, and so does one on an armour stand.
- Being hurt adds none: not by `/damage`, not by a zombie.

## Armour

From `bds_command_probe armour` (research/captures/bds-armour.txt), `/damage` on a player dressed
by `/replaceitem entity @s slot.armor.<head|chest|legs|feet>`:

- 5 as an entity's attack took 5 bare, 4.88 in a leather chestplate, 4.3 in an iron one, 2.5 in
  all of iron, 1.25 in all of diamond and 1.2 in all of netherite; 2 in the netherite took 0.432.
- That is the Java game's rule with its points and toughness: the hit is cut by
  `clamp(points - hit / (2 + toughness / 4), points / 5, 20) / 25`.
- Stopped the same way: `/damage` with no cause, `fire`, `lava`. Not stopped: `fall`, `drowning`.
- `/damage @s 2 suffocation` did nothing at all to the dressed player.
- No hit by `/damage` changed an armour slot: nothing wore.

## Starving

- With hunger at 0 a player is hurt 1 (cause 16) every 80 ticks, first 80 ticks after the hunger ran out,
  down to 10 health on the drilled server (easy), and no further.
- Between 1 and 17 of hunger nothing mends and nothing hurts.
- The saturation effect at amplifier 9 for a tick gave 10 hunger.

## Eating

From `bds_command_probe eat` (research/captures/bds-eat.txt), the bot holding use on an apple:

- The click in the air sets the using flag (`Flags` bit 4) for the player and its viewers, and sends the
  player every attribute.
- The client's `EntityEvent` `EatingItem` comes back to it and goes to its viewers.
- 32 server ticks after the click BDS ends the use itself: `CompletedUsingItem` with the item's network id
  and `Eat`, the inventory, hunger and saturation, the flag cleared, and the sound `burp` 0.1 under the
  eyes.
- An apple gave 4 hunger and 2.4 saturation; the saturation is kept up to the hunger.
- With a full hunger the bot does not try, so what BDS answers then is not seen.

## Death

- To the player, in one tick: `InventoryContent` for the inventory, armour, cursor and off hand (several
  times over), `EntityEvent` `DeathAnimation`, `DeathInfo` with the cause and the player's name, `Text`
  translating the same (`death.attack.lava`, `death.attack.generic` for `/kill`,
  `death.fell.accident.generic`), `SetEntityData` with `PlayerLastDeathPosition`, health 0, and `Respawn`
  with state 0 at the player's eyes. Then `SetEntityData` shrinking the box to 0.2.
- To a watcher: the death animation, the same `Text`, health 0, the shrunken box.
- What it held lies as item entities from the next tick (5 stone: one `AddItemEntity`); 20 ticks after the
  death come its experience orbs (3 for 3 levels) and the sound `disappeared`.
- Rising, once the client answers: `Respawn` state 0 at y 32767, `Respawn` state 1 at the world's spawn,
  `EntityEvent` `Respawn`, `SetHealth` 20, the sound `reappeared`, and `UpdateAttributes` with health 20,
  hunger 20, exhaustion 0, saturation 5, level 0, experience 0. A watcher gets the event, an empty
  `MobEquipment`, the attributes and the box again.

## Not drilled

Armour's wear, enchantments and what it does against burning on, effects, hunger from swimming, from a hit inside a mob's guard or from a block that breaks at once, each food's values and side effects, starving on normal and hard, the void, whether a player out of breath in a
block is hurt as one under water, another
player's hit (a zombie's: mob-attacks.md), `keepInventory`, a bed's or anchor's spawn, creative and spectator.

## Sapling

- Built: health for survival and adventure players; falls, fire, lava, drowning and smothering; `/damage`,
  `/kill` and the plugin calls `health`, `hurt_player`, `kill_player`; death with what was held and
  experience dropped (not under `keepInventory`), the death screen, rising at the world's spawn.
- A fall costs each block begun past three, as a mob's (12 blocks: 9, where BDS took 8 from the bot).
- Fire and lava in one tick are told as one health (16), where BDS told 19 and then 16.
- Run against sapling (research/captures/sapling-hurt.txt, sapling-smother.txt) the drills show the same
  flashes, causes and rhythm; the health differs where BDS mends (those runs were before sapling mended)
  or `/effect` heals, which sapling lacks.
- Glass and leaves smother nothing (glass drilled; leaves taken as glass).
- Mending and starving are built to the numbers above (`vitals/hunger.rs`), and so is what a jump and
  a sprint cost (`vitals.rs`), a block broken with what harvests it and a hit that takes health
  from a mob (an armour stand has none here). Swimming costs nothing yet.
- Armour stops what it stopped on BDS, by the same rule (`creatures/health.rs` `shielded`,
  `sapling-data/src/armour.rs`); it does not wear, and burning on goes through it as in the Java
  game, which was not drilled.
- Eating is built (`vitals/eating.rs`, `sapling-data/src/food.rs`): the bot ate an apple and a steak on
  sapling in the same 32 ticks and packets (research/captures/sapling-eat.txt). Only the apple's values are
  drilled; the others are the Java game's. A stew leaves no bowl, and no food does more than feed.
- Not built: drinks, effects, the shrunken box and `PlayerLastDeathPosition`, lying down under a
  block, a bed's spawn, `doImmediateRespawn`, health kept across a rejoin.
