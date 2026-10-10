# A player harmed, killed and risen

What BDS sends, from one run of `bds_command_probe hurt` (2026-10-10, research/captures/bds-hurt.txt): a
survival player (the bot, runtime id 1) and a watcher. No decompile. Nothing of this is built in sapling:
players there have no health.

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

Armour, effects, hunger from running or mining, starving, drowning itself, the void, a mob's or another
player's hit, `keepInventory`, a bed's or anchor's spawn, creative and spectator.
