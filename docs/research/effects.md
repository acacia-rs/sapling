# Effects

What BDS does with `/effect` on a survival player, from `bds_command_probe effects`
(`research/captures/bds-effects.txt`), and from `hunger` and `hurt` for the hunger and saturation
effects.

## Told

- Given: `MobEffect` `Add` to the player and its viewers, with the effect's id, the amplifier,
  `particles` (false with `hideParticles`) and the ticks it lasts: seconds x 20, or 1 for instant
  health, instant damage and saturation.
- Given again while it is on: `Update` with what stays on. A weaker one changes nothing (speed 1
  with 20 ticks left, given speed 0 for 3 seconds: still 1, 20 ticks).
- Over: `Remove` with duration 0. Cleared: `Remove` with the ticks that were left (562 of 600 a
  second and a half in; once 30 with 19 left, unexplained).
- Sometimes `SetEntityData` `VisibleMobEffects` beside it.
- Ids: speed 1, instant health 6, instant damage 7, regeneration 10, hunger 17, poison 19,
  wither 20, saturation 23.

## Done

An effect that works by turns takes one in each tick whose ticks left divide by its interval,
the first tick included: regeneration for 120 ticks turned with 100 and 50 left, for 60 at the
third level with 60, 48, 36, 24 and 12.

- Instant health: `4 x 2^amplifier` of health, a tick after the command.
- Instant damage: `6 x 2^amplifier`, as a hit of cause 14.
- Regeneration: 1 health every `50 >> amplifier` ticks.
- Poison: 1 every `25 >> amplifier` ticks, cause 14, down to 1 health and no further.
- Wither: 1 every `40 >> amplifier` ticks, cause 15.
- Hunger: `0.005 x (amplifier + 1)` of exhaustion a tick (1.28 at 255).
- Saturation: amplifier 9 gave 10 hunger in its one tick.
- Speed: `UpdateAttributes` `minecraft:movement` at `0.1 x (1 + 0.2 x (amplifier + 1))` with the
  modifier `MovementSpeed` (id `91aeaa56-376b-4498-935b-2f7f68070635`, operation 2, operand 2) of
  that amount; plain 0.1 again when it is over.
- The hunger's mending goes on under poison.
- A player in creative is given poison, told of it, and not hurt.
- Resistance (`more`, research/captures/bds-more.txt): a hit of 10 took 8 at the first level and
  2 at the fourth, as an entity's attack, magic, a fall or no cause alike: a fifth off a level.
- Fire resistance: `/damage` as `fire`, `lava`, `fire_tick` or `magma` answers
  `commands.damage.failed`; an entity's attack takes its whole.
- Names and ids: `%potion.resistance` 11, `%potion.fireResistance` 12; nausea is 9.

## The command

- `commands.effect.success` with `%potion.<name>`, the amplifier, the player and the seconds (0
  for the instant ones); names: `moveSpeed`, `heal`, `harm`, `regeneration`, `hunger`, `poison`,
  `wither`, `saturation`.
- No seconds: 30. No amplifier: 0.
- 0 seconds takes the effect off: `commands.effect.success.removed` with the name and the player,
  said also of speed a second after it had run out.
- `clear`: `commands.effect.success.removed.all`, or `commands.effect.failure.notActive.all`
  where there was none, each with the player.
- An amplifier over 255: `commands.generic.num.tooBig` with it and 255. An unknown effect is a
  syntax error at its word.

## Not drilled

Every other effect (food, milk and potions: food-effects.md), what armour does against instant damage,
effects on mobs, a death under an effect, what a late viewer is told.

## Sapling

`vitals/effects.rs`, `sapling::vanilla` `/effect`, plugins by `give_effect`, `remove_effect` and
`clear_effects`. The eight effects above, by these numbers, absorption and nausea (food-effects.md; nausea is only told), resistance and fire resistance. Different: `Remove` always holds the
ticks left; no `VisibleMobEffects`; saturation gives twice its hunger in saturation, the Java
game's rule; effects end unsaid when the player rises. Run against sapling
(`research/captures/sapling-effects.txt`) the drill gives the same packets, turns and answers.
