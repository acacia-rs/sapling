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

Armour, effects, hunger from running or mining, starving, the void, whether a player out of breath in a
block is hurt as one under water, a mob's or another
player's hit, `keepInventory`, a bed's or anchor's spawn, creative and spectator.

## Sapling

- Built: health for survival and adventure players; falls, fire, lava, drowning and smothering; `/damage`,
  `/kill` and the plugin calls `health`, `hurt_player`, `kill_player`; death with what was held and
  experience dropped (not under `keepInventory`), the death screen, rising at the world's spawn.
- A fall costs each block begun past three, as a mob's (12 blocks: 9, where BDS took 8 from the bot).
- Fire and lava in one tick are told as one health (16), where BDS told 19 and then 16.
- Not built: mending and hunger, the shrunken box and `PlayerLastDeathPosition`, lying down under a
  block, a bed's spawn, `doImmediateRespawn`, health kept across a rejoin.
