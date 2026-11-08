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
  second and a half in; once 30 with 19 left, unexplained). An effect is first counted the tick
  after it is given (bds-r1fx.txt, 2026-11-05: speed 30 cleared a second in told 581, speed 2 a
  second in 21, instant health gone a tick after its `Add`); sapling the same (`effects.rs` `since`).
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
- A hurt player put in creative and given instant health is told its healed health at once
  (bds-v2drip.txt, 2026-11-05); sapling told it only on going back to survival, fixed
  (`vitals/stand.rs` tells a changed health while unhurt; test in thirteenth2x.rs).
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

## More effects (`fx`)

`research/captures/bds-fx.txt`, the actor in survival:

- Slowness (2) writes `minecraft:movement` as speed does, with the modifier `MovementSlowdown`
  (`7107de5e-7ce8-4030-940e-514c1f160890`) of -0.15 a level: 0.085 at the first, 0.04 at the fourth.
- A fist's hit on a cow cost it 3.99 under strength's second level (5) and 0.3 under weakness
  (18): the hit times 1.3 a level plus (1.3^levels - 1) / 0.3, and times 0.8 a level plus
  (0.8^levels - 1) / 0.4. A mob's hit scales the same (bds-a3fx3.txt; mob-kinds.md).
- A fall of 8 blocks cost 5 bare, 4 under jump boost (8), 1 under its fourth level and nothing
  under slow falling (27).
- Under water breathing (13) twenty seconds under water cost nothing.
- Haste (3), mining fatigue (4), invisibility (14), blindness (15), night vision (16) and
  levitation (24) sent `MobEffect` and nothing else; the actor fell from where levitation left it.
- Health boost (21) writes `minecraft:health` with a higher most: 28 at its second level, the
  modifier `HealthBoost` (`5d6f0ba2-1186-46ac-b896-c61c5cee99cc`) of 8, operation 0, operand 1.

## Armour and a late viewer (`fx3`)

Drilled 2026-10-29 (research/captures/bds-fx3.txt):

- Instant damage took 6 from the actor in a whole suit of iron, as bare: armour stops none of it, as
  sapling (`DamageCause::is_stopped_by_armour`).
- A viewer that came after the watcher was given an effect got no `MobEffect` for it; the watcher's
  `AddPlayer` carried `VisibleMobEffects` 16 (jump boost, `id << 1`). Sapling's carried 0
  (sapling-lateeffects.txt, re-drilled 2026-11-03); fixed (`vitals::effects::announced`,
  `visibility.rs`), test `a_viewer_coming_later_finds_a_players_effects_in_its_announcement`.
- A mob (2026-11-03, `bds_command_probe lateeffects`, research/captures/bds-lateeffects.txt): the
  rejoined actor's `AddEntity` of a cow under speed and slowness-without-particles carried
  `VisibleMobEffects` 2, speed alone. Sapling's carried 0, then 260 (slowness shown: `/effect`'s
  hide flag was dropped for mobs). Fixed: the announcement keeps the mob's effects
  (`creatures/afflictions.rs`, `mobs::reannounce_item`), and `/effect` hides a mob's particles
  (`ShardCtx::give_entity_effect_shown`); test `a_viewer_coming_later_finds_a_mobs_effects_in_its_announcement`.
  Re-drilled on sapling 2026-11-03 (sapling-lateeffects.txt): the cow's `AddEntity` carried 2 as
  BDS's. BDS sent a mob's `MobEffect` before its `VisibleMobEffects` and no `VisibleMobEffects`
  for slowness without particles (nothing shown changed); sapling sent it first and again.
  Fixed (`creatures/afflictions/told.rs`), test
  `a_mobs_effect_is_told_before_what_it_shows_and_a_hidden_one_shows_nothing`.

## Invisibility (groups BF and BG)

Drilled 2026-11-08 (`bds_command_probe bf1give`, `bf1potion`, `bf1near`, `bf1armour`, `bf1target`;
research/captures/bds-bf1*.txt, sapling-bf1*.txt after the port):

- A mob gets no `MobEffect` for invisibility, by `/effect`, splash or lingering potion. At the tick-end
  sync its `Flags` gain `INVISIBLE` (bit 5) and lose `CAN_SHOW_NAMETAG` (bit 14, set on a named mob),
  in one SetEntityData before its `VisibleMobEffects` (28 with particles, none for hidden ones); the
  tick after it is heard `disappeared` at its feet plus half its shown height (sheep and cow 0.65,
  zombie, skeleton and villager 0.95, chicken 0.4, calf 0.325), in the same sync order. Running out,
  `invisibility 0` and `clear` undo the flags at the sync and play `reappeared` the tick after. A raise
  while it lasts tells nothing; dying invisible plays nothing. Armour and held items are told as
  before (equipment packets unchanged, a late viewer gets them too); a late viewer's `AddEntity`
  carries the invisible flags and `VisibleMobEffects`, with no sound.
- The player (`bg2player`, ported in `vitals/effects/invisible.rs`): `MobEffect` to it and its viewers
  before the answer, then at the sync one SetEntityData of flags (`INVISIBLE` on, `CAN_SHOW_NAMETAG`
  off) and changed `VisibleMobEffects`, stamped with its input tick; `disappeared`/`reappeared` 0.9 over
  its feet the tick after. A raise tells nothing; sneaking keeps `INVISIBLE`; once seen again it keeps
  `CAN_SHOW_NAMETAG` for good. Every player's `VisibleMobEffects` waits for the sync. A splash hits both
  players in sync order. A late viewer's `AddPlayer` carries the flags. A player leaving is heard
  `disappeared` at feet + 0.9 after its `RemoveEntity` (roster.rs). Left: the thrown invisibility
  potion is heard `disappeared`/`reappeared` itself (REMAINING3).
- Late viewers (`bg2names`): a mob's `AddEntity` carries the flags it has now (BREATHING, MOVING, a
  charge, a name), not those it was made with (`mobs/live.rs`); a mob made this tick keeps its own.
- Hunting: a zombie (sight 35) took a bare invisible player only within 2.45 of its eyes (2.41 yes,
  2.57 no; 4, 9 and 15 off never) and an invisible villager 4 off never; in iron the reach grew with
  the pieces worn: four took it 15 off, two 9 off but not 15, one not 9 off. That is sight x 0.7 x
  max(0.1, pieces / 4). Ported (`creatures/afflictions/invisible.rs`, `hunting/prey.rs` `spotted`,
  at taking only: whether a hunter drops prey that turns invisible was not drilled). Tests
  `thirteenth1bf`. Sapling's re-drilled zombies took the invisible actor at 1.63 to 1.79 off, inside
  the rule; where BDS's did not take it, sapling's had strolled into reach (stroll noise).

## Not drilled

Every other effect (food, milk and potions: food-effects.md). The record's names and what a join is
told: the drill bot comes back with a new identity, so BDS gave it a new player (L).

## Sapling

The effects of `fx` are built, health boost with the attribute BDS wrote (its default and
default most stay 20). Those BDS only told of are only told of here,
so levitation lifts as far as the client does it; invisibility: "Invisibility" above.
Run against sapling the drill gave BDS's speeds, falls and strong hits (7.7 and 6.01 left of a
cow's 10); its bare and its weak hit missed a cow that had strolled off, and a test holds them.

Effects on mobs, run against sapling (research/captures/sapling-mob-effects2.txt against
bds-mob-effects2.txt): the same `MobEffect` and `VisibleMobEffects` (536 for slowness and fire
resistance). A slowed zombie standing in fire was hurt as in fire (cause 6) to death on BDS; on
sapling as burning (cause 7), seven times (mob fire harm, mob-attacks.md; not fixed here).

The four omen effects on mobs (research/captures/sapling-effects3.txt against bds-effects3.txt):
the same keys, `VisibleMobEffects` 576-582, wind burst, cobwebs, slimes and silverfish (their
counts are random). Both tell each new mob `VisibleMobEffects` 0 in its `AddEntity`: the earlier
"BDS did not" read bds-effects3.txt's cut-off lines; whole lines (bds-t2storm.txt, bds-h3keep.txt,
bds-h3herds.txt) hold it on BDS too.

A player dying under an omen effect (research/captures/sapling-player-omens.txt against
bds-player-omens.txt): the same wind burst, cobwebs and slimes, at the drill world's own floor;
sapling sends the burst's particle a tick before the push, BDS with it. Later sapling captures had
them together but in the death's tick; BDS's burst, webs and slimes all came the tick after the
death. Fixed 2026-11-05: a player's omens wait a tick (`vitals/omens.rs` `Due`).

Effects wait in storage with a player's items (`ActiveEffects`, `sapling-leveldb/src/player.rs`)
and are told to it as added when it is back; neither the record's names nor what BDS tells at a
join was measured, and absorption's cushion comes back empty.


`vitals/effects.rs`, `sapling::vanilla` `/effect`, plugins by `give_effect`, `remove_effect` and
`clear_effects`. The eight effects above, by these numbers, absorption and nausea (food-effects.md; nausea is only told), resistance and fire resistance. Different: `Remove` always holds the
ticks left. At a rise (bds-r1fx.txt, 2026-11-05) BDS told each effect `Remove`, by id, with the
ticks it had left at the death, after the Respawn and before SetHealth: they do not count down
while dead. sapling does the same (`effects::end_at_rise`, test
`effects_stand_still_while_dead_and_are_told_gone_by_id_at_the_rise`). Saturation feeds hunger alone, its level a
tick for every tick it lasts, as a stew's did on BDS (bds-foods2.txt: 6 ticks fed hunger 6 to 12,
saturation stayed 6); `/effect` gives it, like the instant ones, for one tick. Run against sapling
(`research/captures/sapling-effects.txt`) the drill gives the same packets, turns and answers.
