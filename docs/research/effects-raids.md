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
- With its whole frame (42 blocks) it takes one hostile (family `monster`) in water or rain within its hunting box (below)
  as its `Target` (BlockEntityData) and bites it from the next pulse on: 4 magic (HurtAnimation 14) every 40
  ticks to death, `conduit.attack` heard at the conduit's corner a tick after each [cap bds-conduit2.txt].
  Sapling bit at once and silently; fixed.
- A zombie standing in rain 7 off was never taken in 10 s [cap bds-conduit2.txt]. Re-drilled
  2026-11-04 after rain came to read the roofs (sapling-conduit2.txt: sapling still bit it), then
  `conduitrain` (research/captures/bds-conduitrain.txt, examples/shared/fifth3_drill.rs): a fresh
  conduit with its whole frame at midnight in rain left a held zombie 7 off alone for 14 s, then
  took a zombie in the two-deep pool 5 off and bit it to death in 5, and the next pool zombie the
  same; a dead target does not block the next. Neither rain nor water was the reason: `conduitspill`
  showed the first zombie's cell held the conduit cube's spill, `liquid_depth` 4 on both servers,
  and a zombie walled in glass there in set water of depth 4 was not taken either, while
  `conduitdepth` took one 4 up and 5 north in set water of depth 4, 3, 2 and 1 (`/setblock` water is
  not ticked on BDS, so a set depth stays). It was reach: `conduitreach` left a zombie in a
  two-deep well level with the conduit 6 north alone, and `conduitspill` bit one in the spill 3
  north once, then let it go as the flow carried it north. Fitted: the mob's box must meet the
  conduit's block grown by 5 every way, to take it and to keep it (taken with its box 4.2 off, not
  5.2: 5 is the round number between; Java's 8 from the block's cell). Sapling took them all within
  8; fixed (`conduits/power.rs` `is_hunted_near`), test
  `a_whole_conduit_leaves_a_monster_in_water_past_its_hunting_box_alone`; re-drilled
  (sapling-conduitrain.txt, sapling-conduitreach.txt): as BDS, zombie for zombie. Rain alone wets a
  mob for the hunt (2026-11-04, `e1conduit`, bds-e1conduit.txt): a held zombie in rain on glass 4.5
  south, and one on glass over the frame, were each taken a second after they came and bitten 4 every
  2 s to death. Sapling's run on the open world (sapling-e1conduit.txt): the same, hit for hit. Left:
  sapling tells `Target` -1 after the kill (BDS sent nothing) and its `conduit.attack` comes in the
  bite's tick (BDS's a tick after). Fixed 2026-11-04 (twelfth wave): the -1 after a dead prey is
  kept untold and the sound comes a tick after the bite (`blocks/conduits/power.rs`), test
  `a_conduits_bite_is_heard_a_tick_late_and_its_dead_preys_target_goes_untold`; re-drilled
  (sapling-e1conduit.txt): each death's sound a tick after it and no -1, as BDS's. Players still take its
  power in rain [cap bds-conduit2.txt]. Captures research/captures/bds-conduit{rain,water,depth,
  spill,reach}.txt and their sapling twins; scenes in examples/shared/fifth3_drill.rs.
- `/setblock ... conduit` gives it its entity (`Active` 0, `Target` -1), shown as `BlockEntityData`
  [cap bds-conduit2.txt]. Sapling gave none, so the drill's conduit never woke there
  (research/captures/sapling-conduit2.txt, first run); fixed (`blocks/conduits.rs` `furnish`).
  Re-run, sapling gives the power and bites the zombie in the water to death in 5 bites as BDS; it
  hears `conduit.attack` in the bite's tick, not the next, and bites the rain zombie too.

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
  wave once all are dead [bds-raid.txt]. A wave's spot: the actor in the village bounds grown by 16
  farthest (across) from the middle (callback 142a4de30), then up to 10 tries at `rand*16+32` from it at a
  random angle, folded to the side away from the middle, then a spawn search about that point
  [`_createRaid` lambda_9 case 1, 0x142a50c10]. BDS's waves came 40-50 from the actor in raidlost and
  raidhero's first wave, its four raiders a few blocks apart. Sapling (waves.rs): the farthest player
  within 32 of the middle stands for the bounds; it was 32 from the middle [Java] until
  sapling-raid.txt showed it arriving at once.
- Each raider of a wave gets its own spot: `_createRaid` lambda_10 (0x142a50270) calls the level
  spawner's `spawnMobGroup` (vtable slot 16, 0x1415ec6c0) with group `minecraft:raid_group_{n}` and
  doScatter set when the chunks about the spot are loaded (0x142a2a790). Per mob, up to 10 draws of a
  cell 0..8 - 4 off on x, z then y (0x14597bfc0, args 10, 4, 4); in its column the first floor from
  the drawn height down 4 then up 3 (y, y-1..y-4, y+1..y+3) that has a collision box, is not leaves
  (material 7) and has two box-free cells over it; the mob stands on that floor's corner (integer x, z),
  else on the wave's spot [cap bds-raidlost.txt, bds-raidaim.txt: corners (-25,-30), (-26,-30),
  (-28,-36), (-31,-30)]. Sapling had all on one cell's middle; fixed (`waves::scatter`), re-drilled
  (sapling-raidaim.txt: (24,-39), (26,-34), (26,-35), (28,-39)). Villagers ring the bell every ~3.5 s
  while a raid gathers [bds-raid.txt; not built].
- With two raiders or fewer left the bar reads `%raid.name - %raid.progress <n>` (0x14116bbf0:
  `NumRaiders <= 2`; bds-n1recruit.txt: `progress 1` at 0.2 with one recruit of 120 health left).
- A cleared wave shows `%raid.name - %raid.progress 0` at progress 0 for ~2 s (40 ticks) before the next
  wait, or before victory [bds-raid.txt].
- Won after the last wave: Hero of the Village (29) for 48000 ticks at amplifier 0 (raid level - 1),
  particles off, to the players who slew a raider; none when every raider was `/kill`ed, not the
  trigger nor a player in the village [bds-raid.txt, bds-raidhero.txt]. Lost when no villager is left
  within 64 [Java].
- Boss bar to players within 96: `raid.name` (no `%`), red, re-sent about every second, filling over the
  wait and emptying with the wave's health, then `%raid.name - %raid.victory` at progress 0 or
  `%raid.name - %raid.defeat` at the health left, for ~30 s; BDS sent no HideBar in the 62 s that
  followed, and sapling no longer hides it (`creatures/raids/bar.rs` `Ended`) [bds-raid.txt, bds-raidlost.txt; strings at 14116bbf0]. It is a BossEvent on the player's
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
  it has no `VILLAGE_<uuid>_INFO`. `SpawnX/Y/Z` are floats (`getFloat` 0x14284fcc0 into raid+0x38..0x40),
  the last wave's spot, 0 before the first; sapling keeps them since 2026-11-04 (`raids/kept.rs`,
  `Raid::spawn_at`, test `a_kept_raid_reads_back`). A kept raid's raiders rejoin it as they load; it is not judged lost
  while its middle is unloaded. BDS's kept raid showed its bar 129 ms after a rejoin, its raiders still
  there, and went on to defeat [bds-raidkeep.txt, bds-raidkeep2.txt; sapling's drill server keeps no
  world, so BDS only].
- A raider's `TargetEid` (2026-10-30 check): BDS sends the target's unique id (the player's
  -4294967295 in bds-raid.txt, bds-fights.txt). Sapling sends `NetId`, which is the unique id it
  announces too: every `AddEntity`/`AddPlayer`/`StartGame` carries unique id = runtime id
  (`sapling_data::add_entity`, `player.rs`, `join.rs`), so the client resolves it. Same; nothing to fix.

## Ominous bottle and trial omen

- An ominous bottle gives bad omen 120000 ticks at its data value as amplifier, without particles;
  drunk, no bottle left, `apply_effect.bad_omen` (the player's) and `ominous_bottle.end_use` heard
  [cap bds-fx3.txt, bds-trialomen.txt, bds-ominous.txt]. Sapling showed particles and the empty-bottle sound; fixed.
- A trial spawner waiting or active that scans an omened player in range turns ominous: bad omen becomes
  trial omen for 18000 ticks a level, LevelEvent 3616 at the spawner, its mobs removed, the ominous config
  [docs/research/spawners.md "Ominous"; Java `TrialSpawner.tryDetectPlayers`].

## Omen sounds and particles (group BD, 2026-11-08)

- Bad omen never shows particles, however given: `/effect @s bad_omen 30 0` (no `true`) came as a
  MobEffect with particles false and no VisibleMobEffects, on a player and on a sheep; raid omen and trial
  omen by `/effect` show them (VisibleMobEffects 72, 62; both on a sheep 8008) [cap bds-bd2event.txt].
  Sapling showed bad omen's; fixed (`vitals/effects.rs` `has_particles`, `creatures/afflictions.rs`).
- Each omen taking hold is heard: `apply_effect.bad_omen`, `.raid_omen`, `.trial_omen`, with the
  entity's type and unique id, by `/effect`, by `gain_raid_omen` (event or a village) and by a bottle;
  not as one is raised again (bad omen 1 over 0 was silent). A player's is 0.2 under its eyes and 0.2
  ahead, as the bottle's; a mob's 0.85 of its shown height up and 0.9 of its width ahead along its yaw
  (sheep, cow and calf, pig, chicken), a zombie's 0.9 up, a villager's at its feet [cap bds-bd2event.txt,
  bds-bd2mouth.txt; also in bds-raid.txt, bds-raidhero.txt, bds-raidlost.txt]. Sapling played only the
  bottle's; ported (`vitals::effects::omen_sound`, `give` / `give_unheard`, `afflictions/told.rs`
  `omen_heard`). Other kinds go by the general rule, undrilled. BDS sends it after the command's answer
  with the tick's VisibleMobEffects; sapling right after the MobEffect.

## Hero of the Village

- A villager (not a wandering trader) asks `max(1, floor((0.3 + 0.0625 * amplifier) * count))` less of
  the first ware [Java `Villager.updateSpecialPrices`].

Drilled: bds-raid.txt, bds-raidhero.txt, bds-raidlost.txt, bds-raidkeep.txt, bds-raidkeep2.txt,
bds-ominous.txt (`bds_command_probe raid|raidhero|raidlost|raidkeep|raidkeep2|ominous`). Fixed from them:
raid omen amplifier 0, the bar's `raid.name`, the 40-tick cleared bar, the defeat bar's health, heroes
only by slaying, the bottle's particles, `/setblock` bells and beds keeping their entity.

## Not drilled

- Every [Java] rule above: village detection (the trade after the win missed its villager; the hero
  discount is decompiled, villagers.md "Trading").
- Captains (BDS's drilled raids had none, no banners). The village uuid sapling keys a raid by is its own.
- Re-run on sapling 2026-11-03 at TICK_SCALE 1 with `SAPLING_DRILL_WORLD=lowflat` (research/captures/
  sapling-raid.txt, sapling-raidlost.txt; the default drill world's raised ground at y 60-63 had put
  every wave on it, 120 above the village). Same now: the waves' kinds, the bar, the omen. Fixed:
  the wave's spot (above), `/effect ... saturation` answers (vanilla/effects.rs: an instant effect's
  seconds are taken as ticks, saturation 1000 answered 50 and lasted 1000 ticks), villagers named
  `Villager` in answers (sapling said `Villager V2`). BDS's pillagers shot the survival actor (19 hurts
  in raidlost) and two stayed within 16 of the village 30 s after its fall; sapling's took only
  villagers, came within 16 at once and walked off to celebrate. Explained 2026-11-03: geometry, not
  the hunt. BDS's wave came from the east, where the actor at (0.5, 0.5) was nearer than the villagers;
  sapling's from the west, where the villagers were, and they chased a fleeing villager out of the
  village. `raidaim` (bds-raidaim.txt) held the wave still: 10 from a villager (14.6 from the actor)
  all four took the villager; moved 12 from the actor (15.5 from it) all four took the actor in the same
  tick (pack `reselect_targets`; sapling kept its first prey: fixed, the pillager's `reselects` trait
  in mob-traits.tsv); after the villagers' kill they kept the actor, stayed within 16 and none
  celebrated, as sapling's goal order has it (a hunter with prey does not celebrate). Re-drilled on
  sapling after the fix (2026-11-03): the swap to the actor matches, but sapling's slowed pillagers
  then shot the actor from 12 off; BDS's neither loaded nor shot from there (they did from 10 off of
  the villager, with the pack's `ranged_attack` `attack_radius` 8). Settled 2026-11-03 by
  `pillrange` (bds-pillrange.txt, positions captured): a lone slowed pillager held 12, 11, 10, 10, 9
  and 9 east of the survival actor (BDS's `tp` centres a whole x: `tp 10` put it at 10.5) loaded its
  crossbow at each, held it charged and never shot; held 8 and 7 off it shot every 46 ticks. So
  `attack_radius` 8 is the rule: `RangedAttackGoal::tick` (0x149fe7ec0) shoots only while the
  squared distance is at most the radius squared, and charges whenever it sees its prey. raidaim's
  shots "from 10 off" the villager were at villagers that had strolled for 10 s before their
  slowness (their moves are not in VILLAGE_SHOWN). Sapling shot from any range; fixed (the trait
  `fires=8`, `shooting.rs` holds a loaded shot until the prey is that near). Re-drilled
  (sapling-raidaim.txt, sapling-pillrange.txt): no shot from 12 at the actor; after the villagers'
  kill all four kept the actor, stayed within 16 for 21 s and none celebrated, as BDS's. One
  difference left: at the move BDS's four swapped to the actor in the same tick; sapling's let the
  villager go then took the actor over the next 0.45 s (their next looks). Drilled and fixed 2026-11-04
  (`pillswitch`, examples/shared/last_drill.rs; research/captures/bds-pillswitch.txt against
  sapling-pillswitch.txt): four pillagers held 6 east of a held villager took it; the survival actor
  put 3 east of them was taken by all four in the very tick it came; sent 30 off, they let go in that
  tick (one swapped straight to the villager) and took the villager again 1 to 9 ticks later; with
  the villager dead and the actor back, they took it 0 to 9 ticks after it came. So a pillager with prey
  reselects every tick, and one without looks every few ticks, as sapling's `LOOKS` has it. Sapling's
  reselected only at its looks; now every tick while it has prey (`hunting::pick`), and one that lost
  its prey takes none in that tick; test `a_pillager_swaps_to_the_nearer_prey_the_tick_it_is_there`.
  Re-drilled: three of four swapped in the arrival tick, let go together, retook the villager 7 to 9
  ticks later, took the returning actor 0 to 9 ticks after. Left, geometry rather than the rule:
  sapling's first pillager, which strolled before its slowness, kept the villager, and the three swapped
  back to the villager 1.7 s after each arrival, when arrows had thrown the actor 2.9 east (BDS's 1.96,
  no swap back); BDS's pillagers' places were not captured. Settled 2026-11-04 (`pillswitch2`, the
  same lines with moves and throws shown; bds-pillswitch2.txt against sapling-pillswitch2.txt): the
  places were alike (on both the first pillager strolled before its slowness, to 5.7,-1.5 on BDS and
  4.1,1.6 on sapling; the other three stayed at x 10.5). The knockback differed twice over. BDS
  threw the resistance V actor once a volley (6 throws, each with a `HurtAnimation`, cause 3) though
  three bolts hit within 2 ticks, and threw it 0.4 across along the line from the pillager to it,
  not the bolt's (from 3 west and 1 north: `(0.379, 0.4, 0.126)`); its villager the same. Sapling
  threw on every bolt (2 or 3 a volley, no `HurtAnimation`: resistance V took the whole hit, so no
  guard began) along the bolt. Fixed: a hit resistance takes whole still lands outside the guard,
  losing nothing (`Health::hurt`), a bolt in the guard throws nothing (`struck::hit_from` asks
  `Health::lands`), and a shot throws a player away from its archer (`hits.rs` `archer_feet`); test
  `a_pillagers_bolt_throws_its_prey_away_from_the_pillager`. Re-drilled: one throw and hurt a volley,
  6 in all, and the three no longer swap back to the villager while the actor is there.
- raidhero on sapling 2026-11-03 (sapling-raidhero.txt): `/damage … entity CmdActor` killed every
  raider but made no hero, left no experience, counted 1 and answered `commands.damage.failed [""]`
  with no raider left. BDS: 11 orbs, Hero of the Village after the win, success counts 7, 5 and 4,
  `noTargetMatch`. Fixed: a player-dealt `/damage` (`ctx.hurt_entity_by`) kills as the player's hit,
  `/damage` counts each one hurt and answers `noTargetMatch` to none. BDS re-sends the raid bar once a
  second as a ShowBar; sapling sent each tick's change (918 BossEvents against 100): now once a second.
  Re-drilled after the fix (sapling-raidhero.txt): hero added, orbs dropped, counts 4, 5, 4 on the
  raiders (BDS's first 7 took 3 wild slimes too), `noTargetMatch`, 124 BossEvents.
- LAGGED at 1x (2026-11-03, raidaim under `perf record` on testbox at load ~49): the debug drill_server
  fell behind once at the setup fills (+8 s), once at +17 s, then 8 to 64 ticks over 5 s from the wave's
  spawn (+79 s). The profile is flat; on top are debug-build costs (SipHash of `ChunkPos` per block
  read, ub precondition checks, memmove) and the pillagers' routing (`paths::route`, `holds`,
  `is_standable`, `landing`, each block read through `World::block`'s hashed chunk map). Not cut: no
  single sapling hot spot; a per-route chunk cache in `paths` is the lever if it matters.
- Joining passing raiders: `RaidRecruitSystem` (0x141959650) runs for a hostile dweller (role 2 at
  `DwellerComponent`+0x58) of a village with a raid, at its dweller update (+0x40 at 0); every living
  mob with `minecraft:can_join_raid` (pillager, vindicator, evoker, ravager, witch) in its box grown 16
  on each axis, not queued and not in the raid, gets `minecraft:spawn_for_raid` and is queued.
  `CanJoinRaidQueueSystem` (0x148493280) adds a queued mob to the raid of the village it stands in
  (`NumRaiders` + 1, `TotalMaxHealth` + its max health). Built (`raiders::recruits`, every 20 ticks
  while a wave fights, the raid's 96 reach as its village); test `a_pillager_by_a_raider_joins_its_raid`.
  Drilled 2026-11-04 (`n1recruit`, examples/shared/ninth1_drill.rs): a pillager summoned 2 from a wave
  pillager, the wave then killed; BDS's bar stayed on `%raid.name - %raid.progress 1` at 0.2 (24 of 120)
  and no wave came until the newcomer died; sapling's the same (sapling-n1recruit.txt).
- Hero discount: decompiled, as sapling's (villagers.md "Trading").
- Ominous item spawners from an active ominous trial spawner: a `/setblock` trial spawner has an empty
  block entity and never woke for an omened player 4.5 off in 30 s (bds-trialomen.txt); it needs a trial
  chamber's spawner. Tried again 2026-11-04 with an egg-woken spawner mid-wave (bds-t3trial.txt): Trial
  Omen did not turn it ominous in 30 s on BDS either. Drilled 2026-11-04 (`u1omen`, bds-u1omen.txt) with
  the omen given before the egg: a `/setblock` spawner waiting for its creative actor turned ominous the
  tick the actor turned survival, showing 3616 with 0 then 3615 with 1 at its corner, and
  `ominous=1,trial_spawner_state=1` then `=2` in one UpdateSubchunkBlocks. Sapling sent 3616 with 1 at the
  middle; fixed (`spawners/trial/ominous.rs`, test `bad_omen_turns_to_trial_omen_and_the_spawner_ominous`).
  A spawner set `["ominous"=true]` woke with 3615 alone. The active ominous spawners then made three
  `minecraft:ominous_item_spawner`s, the first 5.5 over the actor's feet 4 ticks after it woke, the next
  160 ticks on: `ominous_item_spawner.spawn_item_begin` as it came,
  `about_to_spawn_item` 27 ticks on, and at 62 ticks its item (an arrow thrown down at 1.29, a lingering
  potion dropped at 0.5) with `spawn_item` and the spawner gone. Ported 2026-11-05
  (`spawners/trial/ominous_items.rs`); BDS's rule and what is left are in spawners.md, "Ominous item spawners".

Drilled the same: a bell rung with a slowed pillager 4 off sent nothing for the pillager (bds-fx3.txt),
as sapling (Bedrock has no glowing).
- Villager schedules, sleeping and standing: see villagers.md.
