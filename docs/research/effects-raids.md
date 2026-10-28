# Effects and raids

Fatal poison, conduit power, darkness, the omens and Hero of the Village; the warden's darkness,
the conduit's power and hunt; raids; ominous trial spawners; the hero's discount. Sources per rule:
`[BDS <va>]` read with bdsre from BDS 1.26.52 (`.testserver/bds-blocks`), `[pack]` BDS's own
definitions, `[cap]` a capture in `research/captures`, `[Java]` the Java game's code where nothing
Bedrock was found.

## Effects

| effect | id | key | source |
|---|---|---|---|
| fatal_poison | 25 | `potion.poison` | [BDS 141c4ed80] effect registration |
| conduit_power | 26 | `potion.conduitPower` | same |
| bad_omen | 28 | `effect.badOmen` | same |
| village_hero | 29 | `effect.villageHero` | same |
| darkness | 30 | `effect.darkness` | same |
| trial_omen | 31 | `effect.trial_omen` | same |
| raid_omen | 36 | `effect.raid_omen` | same |

- Fatal poison hurts as poison (every 25 ticks >> level) but goes past 1 health and kills [cap bds-fx2.txt].
- Conduit power keeps air topped up as water breathing does [cap bds-fx2.txt], and digs as haste of its
  level [cap bds-dig.txt: conduit power 0 cracks at 629, haste 0 the same].
- Darkness is never shown with particles, whatever was asked [cap bds-fx2.txt].
- Undead refuse fatal poison as they refuse poison [Java `MobEffectUtil`; not drilled].
- Dolphin's Grace is not an effect on Bedrock: BDS has `DolphinBoostSystem` (`IsNearDolphinsFlagComponent`,
  `PredictDolphinBoost`), a client-predicted swim boost, and no effect id for it [BDS strings]. Not built.

## Warden's darkness

- Every 121 ticks, darkness 260 ticks, amplifier 0, no particles, to each survival or adventure player
  within 20 of the warden [cap bds-warden-findings.md; pack `minecraft:mob_effect` effect_range 20,
  effect_time 13, cooldown_time 6].

## Conduit

- Every 40 ticks an active conduit gives conduit power (260 ticks, ambient) to players in water or rain
  within `frame / 7 * 16` blocks [Java `ConduitBlockEntity.applyEffects`].
- With its whole frame (42 blocks) it bites one hostile (family `monster`) in water or rain within 8 for 4
  magic damage, keeping it as `Target` while it stays in reach [Java `updateDestroyTarget`].

## Raids

- Bad omen in a village turns to raid omen for 30 s at its level, where the player stood kept
  [pack player.json `minecraft:gain_raid_omen`; BDS string `RaidOmenPosition`]; as raid omen runs out the
  raid starts there [pack `minecraft:trigger_raid`].
- A village is a bell within 48, else a bed within 48, with a villager within 64 of it [Java; BDS villages
  are dweller-claimed POIs, not built].
- Waves by difficulty: `0x07050300 >> difficulty * 8`, peaceful 0, easy 3, normal 5, hard 7
  [BDS 142a29ac0 `Village::_createRaid`]. No bonus wave was found.
- Wave n is `definitions/spawn_groups/raid_group_<n>.json` [pack]: 4 pillagers; 3 pillagers 2 vindicators;
  3 pillagers 1 ravager; 3 witches 3 pillagers; 4 vindicators 1 evoker 2 ravagers (one ridden by a pillager);
  5 pillagers 2 vindicators 1 evoker; 6 vindicators 1 witch 2 evokers 2 ridden ravagers. Raiders spawn
  with `minecraft:spawn_for_raid` (ravagers `..._with_pillager_rider` / `..._with_evoker_rider`).
- 300 ticks before each wave; a wave spawns 32 out at a random angle on the surface; the next wave once
  all are dead [Java `Raid`].
- Won after the last wave: Hero of the Village for 48000 ticks at raid level - 1 to the triggering
  player(s) and players within 96 [Java]. Lost when no villager is left within 64 [Java].
- Boss bar to players within 96: `%raid.name`, filling over the wait and emptying with the wave's health,
  then `%raid.name - %raid.victory` / `- %raid.defeat`, red, removed 600 ticks after [BDS strings at
  14116bbf0; Java `updateBossbar`]. It is a BossEvent on the player's own runtime id.
- A second omen in a running raid raises its level, at most 5 [Java].

## Ominous bottle and trial omen

- An ominous bottle gives bad omen 120000 ticks at its data value as amplifier; drunk, no bottle left [Java].
- A trial spawner waiting or active that scans an omened player in range turns ominous: bad omen becomes
  trial omen for 18000 ticks a level, LevelEvent 3616 at the spawner, its mobs removed, the ominous config
  [docs/research/spawners.md "Ominous"; Java `TrialSpawner.tryDetectPlayers`].

## Hero of the Village

- A villager (not a wandering trader) asks `max(1, floor((0.3 + 0.0625 * amplifier) * count))` less of
  the first ware [Java `Villager.updateSpecialPrices`].

## Not drilled

- Every [Java] rule above: village detection, wave spot and timing, victory and defeat, the boss bar's
  progress, hero reach and length, ominous bottle length, hero discount, conduit range and bite.
- Fatal poison refused by undead.
- Raiders seeking the village, raid captains and banners, celebrating, joining passing raiders; saved raids.
- Bells lighting raiders (Bedrock has no glowing).
- Ominous item spawners from an active ominous trial spawner.
- Villager gossip, reputation, schedules and sleeping.
