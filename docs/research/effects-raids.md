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

- Bad omen in a village turns to raid omen for 30 s the next tick, at amplifier 0 whatever the bad omen's
  (so a level 1 raid), particles on, where the player stood kept [bds-raid.txt; pack player.json
  `minecraft:gain_raid_omen`; BDS string `RaidOmenPosition`]; as raid omen runs out the raid starts there
  [pack `minecraft:trigger_raid`]. Bad omen taken outside a village stays bad omen.
- A village is a bell within 48, else a bed within 48, with a villager within 64 of it [Java; BDS villages
  are dweller-claimed POIs, not built]. A bell or bed set by `/setblock` or a plugin gets its block
  entity, which this reads; worldgen's beds and bells get none (structure-block-entities.md).
- Waves by difficulty: `0x07050300 >> difficulty * 8`, peaceful 0, easy 3, normal 5, hard 7
  [BDS 142a29ac0 `Village::_createRaid`]. No bonus wave was found.
- Wave n is `definitions/spawn_groups/raid_group_<n>.json` [pack]: 4 pillagers; 3 pillagers 2 vindicators;
  3 pillagers 1 ravager; 3 witches 3 pillagers; 4 vindicators 1 evoker 2 ravagers (one ridden by a pillager);
  5 pillagers 2 vindicators 1 evoker; 6 vindicators 1 witch 2 evokers 2 ridden ravagers. Raiders spawn
  with `minecraft:spawn_for_raid` (ravagers `..._with_pillager_rider` / `..._with_evoker_rider`).
- 300 ticks before each wave (the bar fills over them), the first wave ~15 s after the start; the next
  wave once all are dead [bds-raid.txt]. A wave comes 32 out at a random angle on the surface [Java];
  BDS's horns were 48-70 from the beds and bell, its search 10 tries at `rand*16+32` from an actor,
  turned away from the village middle [`_createRaid` lambda_9 0x142a50c10], not copied. Villagers ring
  the bell every ~3.5 s while a raid gathers [bds-raid.txt; not built].
- A cleared wave shows `%raid.name - %raid.progress 0` at progress 0 for ~2 s (40 ticks) before the next
  wait, or before victory [bds-raid.txt].
- Won after the last wave: Hero of the Village (29) for 48000 ticks at amplifier 0 (raid level - 1),
  particles off, to the players who slew a raider; none when every raider was `/kill`ed, not the
  trigger nor a player in the village [bds-raid.txt, bds-raidhero.txt]. Lost when no villager is left
  within 64 [Java].
- Boss bar to players within 96: `raid.name` (no `%`), red, re-sent about every second, filling over the
  wait and emptying with the wave's health, then `%raid.name - %raid.victory` at progress 0 or
  `%raid.name - %raid.defeat` at the health left, for ~30 s; BDS sent no HideBar, sapling hides it after
  600 ticks [bds-raid.txt, bds-raidlost.txt; strings at 14116bbf0]. It is a BossEvent on the player's
  own id.
- A second omen in a running raid raises its level, at most 5 [Java].
- Raiders walk to the village's middle until within 2 when they have no prey [pack `spawn_for_raid`
  groups' `behavior.move_to_village`]. They hunt as their pack's `nearest_attackable_target` names:
  vindicators and evokers grown villagers, wandering traders, iron and snow golems and players;
  pillagers and ravagers also; witches golems [pack]. Pillagers shoot, ravagers bite as melee, evokers
  cast fangs that harm mobs too (`hunting.rs`, `shooting.rs`, `spells.rs`).
- On defeat the living raiders celebrate ~30 s: hops and `celebrate` sounds every 2-5 s each
  [bds-raidlost.txt; pack `celebrate` group, `minecraft:behavior.celebrate`].
- BDS raids have no captains: `_createRaid` (0x142a50270) spawns with `spawn_for_raid` only;
  `spawn_as_illager_captain` is sent only by structure spawns (141f82140) and patrols (1483599e0).
  A captain wears the ominous banner (`set_banner_details` type 1, damage 15) in its chest slot, drop
  chance 1, and drops `pillager_captain.json`'s ominous bottle (amplifier 0–4) when a player killed it;
  no bad omen on kill [pack].
- A raid is kept as `VILLAGE_<uuid>_RAID`: `State`, `Status`, `GroupNum`, `NumGroups`, `GameTick`, `Ticks`,
  `SpawnX/Y/Z`, `Raiders` (unique ids), `NumRaiders`, `TotalMaxHealth`, `SpawnFails` [BDS
  `VillageManager::loadAllVillages` lambda 0x142a52630]; sapling adds `CenterX/Y/Z` and `OmenLevel`, as
  it has no `VILLAGE_<uuid>_INFO`. A kept raid's raiders rejoin it as they load; it is not judged lost
  while its middle is unloaded. BDS's kept raid showed its bar 129 ms after a rejoin, its raiders still
  there, and went on to defeat [bds-raidkeep.txt, bds-raidkeep2.txt; sapling's drill server keeps no
  world, so BDS only].

## Ominous bottle and trial omen

- An ominous bottle gives bad omen 120000 ticks at its data value as amplifier, particles off; drunk, no
  bottle left [bds-ominous.txt].
- A trial spawner waiting or active that scans an omened player in range turns ominous: bad omen becomes
  trial omen for 18000 ticks a level, LevelEvent 3616 at the spawner, its mobs removed, the ominous config
  [docs/research/spawners.md "Ominous"; Java `TrialSpawner.tryDetectPlayers`].

## Hero of the Village

- A villager (not a wandering trader) asks `max(1, floor((0.3 + 0.0625 * amplifier) * count))` less of
  the first ware [Java `Villager.updateSpecialPrices`].

Drilled: bds-raid.txt, bds-raidhero.txt, bds-raidlost.txt, bds-raidkeep.txt, bds-raidkeep2.txt,
bds-ominous.txt (`bds_command_probe raid|raidhero|raidlost|raidkeep|raidkeep2|ominous`). Fixed from them:
raid omen amplifier 0, the bar's `raid.name`, the 40-tick cleared bar, the defeat bar's health, heroes
only by slaying, the bottle's particles, `/setblock` bells and beds keeping their entity.

## Not drilled

- Every [Java] rule above: village detection, the wave's spot, hero discount (the trade after the win
  missed its villager), conduit range and bite.
- Fatal poison refused by undead.
- Raiders seeking the village and captains (BDS's drilled raids had none, no banners). The village uuid
  sapling keys a raid by is its own. Sapling's own raid drills ran LAGGED at TICK_SCALE 2; the shard
  tests in `tests/integration/raids.rs` carry the rules.
- Joining passing raiders (`RaidRecruitSystem` 0x141959650): not built.
- Bells lighting raiders (Bedrock has no glowing).
- Ominous item spawners from an active ominous trial spawner.
- Villager schedules, sleeping and standing: see villagers.md.
