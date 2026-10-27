# bds-combat findings

Captures (drills in `crates/sapling/examples/shared/combat_drill.rs`, run with tools/remote-drill.sh against a
BDS copy on devbox, 2026-10-27): `bds-shield.txt`, `bds-totem.txt`, `bds-crit.txt`, `bds-sprint.txt`,
`bds-swing-wear.txt`, `bds-thorny.txt`, `bds-void.txt`, `bds-anvil.txt`. Decompile readings are from the
`bds-molang` exe with bdsre (addresses below); where a capture and a reading disagree the capture wins.

## Shield (`shield`; `Player::_hurt` 1402b5fe0, `isBlocking` 1402ab9e0, `isDamageBlocked` 1402abea0)

- Sneaking with a shield (off hand first, then main hand) sets FlagsExtended bit 8 (acacia-proto's
  `TRANSITION_BLOCKING`, one bit off) with bit 9 for about two ticks; lowering sets bit 9 alone for two ticks.
- `isBlocking` needs the shield up more than 4 ticks (`currentTick - mBlockingTick > 4`).
- Turned away: entity_attack, projectile and entity_explosion with a source entity whose horizontal offset has
  a positive dot product with the look (45° blocked, exactly 90° and behind not). Not turned away: fire,
  `/damage` without a source. The decompile also lists block explosions, Thorns and fireworks (not done).
- A block costs nothing: no flash, no health, no throw of the player. The shield wears `1 + floor(amount)`
  (4 -> +5, 2 -> +3, 10 -> +11; no Java >= 3 threshold).
- Packets, off hand: InventoryContent Offhand (new Damage), LevelSoundEvent `item.shield.block` at the eyes
  (entity_type "", unique -1), SetEntityData FlagsExtended with bits 10|11, then without, to the player and its
  viewers. A main-hand shield is not reshown until it goes down.
- A zombie whose hit was turned got SetEntityMotion ~0.556 across, 0.4 up: `Mob::knockback` {0.6, 0.6, cap
  0.4, keep 0.5} (1429f1a20), for entity_attack only.
- A vindicator's axe hit was turned (wear +19), then `break` (entity_type player) at feet + 0.745; its hits 1 s
  and 2 s later landed. Decompile: any attacker whose main hand has `minecraft:is_axe`, cooldown 100 ticks.
- After a mob's or an arrow's blocked hit BDS cleared the sneak flag and the client sneaked again (not done).
  A blocked arrow bounced back at about a tenth of its speed.

## Totem (`totem`; 1429e3e50)

Order: effects removed in id order; MobEffect Add regeneration (10) amp 1 900, absorption (22) amp 1 100, fire
resistance (12) amp 0 800; EntityEvent ConsumeTotem; LevelEvent SoundTotem at the eyes; InventoryContent Offhand
(emptied); HurtAnimation of the cause; health 1. Off hand first when both hands hold one; a main-hand totem sends
no inventory packet. Void and `/kill` skip it.

## Critical hits (`crit`; `Player::attack` 1402b84b0)

A falling hitter's hit costs x1.5 (fist 1 -> 1.5; 1.425 through a zombie's armour). Animate CriticalHit (rid of
the struck, data 55.0) to everyone, after HurtAnimation; viewers hear `attack.critical` from the hitter's middle
instead of `attack.strong`. Conditions read: fall distance > 0, not on ground, not climbing, not in water, not
blind, not riding; sprinting does not stop it (it does in Java).

## Sprint and Knockback throw (`sprint`; `Mob::knockback` 1429e7130)

A standing player was thrown (0.4 across, 0.4 up) by a walking fist and (0.6, 0.32) by a sprinting one. Read:
a first throw {0.4, 0.4, cap 0.4, keep 0.5}, `y = min(y*keep + up*1.2, cap)`, then for `s = 1.15*Knockback +
(sprinting ? 1 : 0) > 0` a second {0.4s, 0.1s, cap 0.4, keep 0.5}. The hitter's sprint flag dropped at the hit
(client side; not copied).

## Weapon wear (`swing-wear`)

Wooden sword 57 -> 58 per hit, axe and pickaxe 57 -> 59; a stick nothing; creative nothing. Two hits in one
tick wore the sword twice though the second did not land. After every hit (fist too) BDS resent the hitter's
Inventory, Armor, Ui and Offhand windows. A stack that broke: LevelSoundEvent `break` at the hitter's eyes
(entity_type "", unique -1) to it and its viewers.

## Standing hazards (`thorny`, `void`, `anvil`)

- Cactus: 1 a hit (cause contact, 1) each time the guard let one in, while against it.
- Sweet berry bush (growth 3): 1 a hit (contact) while walking through, none standing still; read: growth > 0
  and `|dx|` or `|dz|` > 0.003 a tick.
- Powder snow: the bot did not sink in, nothing measured. Read: freeze +1/140 a tick inside, 1 every 40 ticks at
  full (not done).
- Void: 4 a hit (cause void, 12) at y -110 down to death, `death.attack.outOfWorld`. Read: below the bottom of
  the world less 16.
- Anvil: 6 blocks up cost 10, 16 killed (`death.attack.anvil`); HurtAnimation (cause 17) before LevelEvent
  SoundAnvilFall at the landing cell. Two a block past the first, at most 40, as Java. The helmet case died too,
  so what a helmet spares is unmeasured.
- Fly-into-wall needs elytra flight, which sapling lacks; read: `(speed before - after)*10 - 3`.
