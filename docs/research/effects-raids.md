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
- Undead take fatal poison: `/effect` of it on a zombified piglin answered success, set
  `VisibleMobEffects` and cost it a point at once and every 25 ticks to death, as a cow [cap
  bds-fx3.txt]. They refuse plain poison (a lingering poison cloud left one alone, bds-clouds2.txt).
  Sapling refused both; fixed.
- Dolphin's Grace is not an effect on Bedrock: BDS has `DolphinBoostSystem` (`IsNearDolphinsFlagComponent`,
  `PredictDolphinBoost`), a client-predicted swim boost, and no effect id for it [BDS strings]. Not built.

## Warden's darkness

- Every 121 ticks, darkness 260 ticks, amplifier 0, no particles, to each survival or adventure player
  within 20 of the warden [cap bds-warden-findings.md; pack `minecraft:mob_effect` effect_range 20,
  effect_time 13, cooldown_time 6].

## Conduit

- Every 40 ticks an active conduit gives conduit power (260 ticks, particles, not ambient) to players in
  water or rain whose block is less than `frame / 7 * 16` off along each axis: with the ring of 16, 31
  along x and (23, 2, 23) on the diagonal were reached, 32 along x was not [cap bds-conduit2.txt]. Sapling
  had the Java game's sphere and ambient; fixed.
- With its whole frame (42 blocks) it takes one hostile (family `monster`) in water or rain within 8 as
  its `Target` (BlockEntityData) and bites it from the next pulse on: 4 magic (HurtAnimation 14) every 40
  ticks to death, `conduit.attack` heard at the conduit's corner a tick after each [cap bds-conduit2.txt].
  Sapling bit at once and silently; fixed.
- A zombie standing in rain 7 off was never taken in 10 s [cap bds-conduit2.txt]; sapling takes it (not
  understood).
- `/setblock ... conduit` gives it its entity (`Active` 0, `Target` -1), shown as `BlockEntityData`
  [cap bds-conduit2.txt]. Sapling gave none, so the drill's conduit never woke there
  (research/captures/sapling-conduit2.txt, first run); fixed (`blocks/conduits.rs` `furnish`).
  Re-run, sapling gives the power and bites the zombie in the water to death in 5 bites as BDS; it
  hears `conduit.attack` in the bite's tick, not the next, and bites the rain zombie too.

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
- Raiders walk to the village's middle until within 2 when they have no prey [pack `spawn_for_raid`
  groups' `behavior.move_to_village`]. They hunt as their pack's `nearest_attackable_target` names:
  vindicators and evokers grown villagers, wandering traders, iron and snow golems and players;
  pillagers and ravagers also; witches golems [pack]. Pillagers shoot, ravagers bite as melee, evokers
  cast fangs that harm mobs too (`hunting.rs`, `shooting.rs`, `spells.rs`).
- On defeat the living raiders celebrate 600 ticks: hops and `celebrate` sounds [pack `celebrate`
  group, `minecraft:behavior.celebrate`].
- BDS raids have no captains: `_createRaid` (0x142a50270) spawns with `spawn_for_raid` only;
  `spawn_as_illager_captain` is sent only by structure spawns (141f82140) and patrols (1483599e0).
  A captain wears the ominous banner (`set_banner_details` type 1, damage 15) in its chest slot, drop
  chance 1, and drops `pillager_captain.json`'s ominous bottle (amplifier 0–4) when a player killed it;
  no bad omen on kill [pack].
- A raid is kept as `VILLAGE_<uuid>_RAID`: `State`, `Status`, `GroupNum`, `NumGroups`, `GameTick`, `Ticks`,
  `SpawnX/Y/Z`, `Raiders` (unique ids), `NumRaiders`, `TotalMaxHealth`, `SpawnFails` [BDS
  `VillageManager::loadAllVillages` lambda 0x142a52630]; sapling adds `CenterX/Y/Z` and `OmenLevel`, as
  it has no `VILLAGE_<uuid>_INFO`. A kept raid's raiders rejoin it as they load; it is not judged lost
  while its middle is unloaded.

## Ominous bottle and trial omen

- An ominous bottle gives bad omen 120000 ticks at its data value as amplifier, without particles;
  drunk, no bottle left, `apply_effect.bad_omen` (the player's) and `ominous_bottle.end_use` heard
  [cap bds-fx3.txt, bds-trialomen.txt]. Sapling showed particles and the empty-bottle sound; fixed.
- A trial spawner waiting or active that scans an omened player in range turns ominous: bad omen becomes
  trial omen for 18000 ticks a level, LevelEvent 3616 at the spawner, its mobs removed, the ominous config
  [docs/research/spawners.md "Ominous"; Java `TrialSpawner.tryDetectPlayers`].

## Hero of the Village

- A villager (not a wandering trader) asks `max(1, floor((0.3 + 0.0625 * amplifier) * count))` less of
  the first ware [Java `Villager.updateSpecialPrices`].

## Not drilled

- Every [Java] rule above: village detection, wave spot and timing, victory and defeat, the boss bar's
  progress, hero reach and length, hero discount.
- Raiders seeking the village, captains and banners, celebrating, saved raids: built from the pack and
  bdsre, never compared with a live BDS raid. The village uuid sapling keys a raid by is its own.
- Joining passing raiders (`RaidRecruitSystem` 0x141959650): not built.
- Ominous item spawners from an active ominous trial spawner: a `/setblock` trial spawner has an empty
  block entity and never woke for an omened player 4.5 off in 30 s (bds-trialomen.txt); it needs a trial
  chamber's spawner.

Drilled the same: a bell rung with a slowed pillager 4 off sent nothing for the pillager (bds-fx3.txt),
as sapling (Bedrock has no glowing).
- Villager schedules, sleeping and standing: see villagers.md.
