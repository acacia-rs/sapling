# Villagers and wandering traders

What sapling copies of BDS's villagers (`villager_v2`) and wandering traders, and from where. Built in
`crates/sapling-shard/src/creatures/villagers*` and `crates/sapling-data/src/trades*`.

## Evidence

- The vanilla behaviour packs (`entities/villager_v2.json`, `wandering_trader.json`, `trader_llama.json`,
  `trading/economy_trades/*.json`), read through `crates/sapling-data/data/generate/trades.py`.
- `bds_command_probe n1restock|n1drop|n1recruit|n1golems|n1golems20|n1fear`
  (`examples/shared/ninth1_drill.rs`, 2026-11-04), BDS and sapling captures in `research/captures/`.
- Drills on BDS 26.x, `bds_command_probe trade` and `job` (`examples/shared/trade_drill.rs`):
  `research/captures/bds-trade.txt` and `bds-job.txt` (first form of the drills: 64 coal, no village),
  `bds-trade2.txt` and `bds-job2.txt` (the drills as committed: 128 coal, a bed and a bell first).
- bdsre: `MerchantRecipeList` load at 0x1416404a0 and `createTag` at 0x141641710 (the NBT keys below),
  the actor load at 0x140d26a70 (`TradeTier`, `TradeExperience`, `persistingOffers`,
  `persistingRiches`), `CraftHandlerTrade::_handleTrade2` at 0x148b6cc40 (the trade request).
- acacia's `docs/research/workstations.md` "Trades" and its 2026-10-02 vanilla capture: the client's
  request shapes, which acacia-bot's `Bot::trade` copies.

## Trade tables

Some packs ship tables compiled (`\x7fMCB`). Read by decoding every compiled file to its last byte and
comparing vanilla_1.21.40's wandering trader with vanilla_1.21.20's JSON:

```
"\x7fMCB" 4 bytes, i32, u8-length string "tiers"
u8 tiers; per tier: u8 groups; per group: i32 num_to_select (-1 all), u8 trades, trades; i32 total_exp_required
trade: u8 wants, each a u8 count of alternatives and the alternatives; the same for gives;
       i32 trader_exp, i32 max_uses, u8 reward_exp, i32 weight
alternative: i32 min, i32 max, u8-length name, u8 0, i32 2, i32 0, u8 0, f32 price_multiplier
```

Names keep old forms (`minecraft:fish`, `dye:0`, `bucket:2`); the script renames them and turns
`set_potion` into a data value (a tipped arrow's is its potion's plus one). Biome `filters`
(`is_mark_variant`) become the types that may offer an item.

## Professions

`Variant`: 0 unskilled, 1 farmer (composter), 2 fisherman (barrel), 3 shepherd (loom), 4 fletcher
(fletching table), 5 librarian (lectern), 6 cartographer (cartography table), 7 cleric (brewing stand),
8 armorer (blast furnace), 9 weaponsmith (grindstone), 10 toolsmith (smithing table), 11 butcher
(smoker), 12 leatherworker (cauldron), 13 mason (stonecutter), 14 nitwit. `MarkVariant`: 0 plains, 1
desert or mesa, 2 jungle, 3 savanna, 4 cold and not ocean or frozen, 5 swamp, 6 taiga (the pack's
filters in order, the last that holds wins). `SkinID` 0 to 5. A summoned villager is one of the 13 or a
nitwit, each as likely; `spawn_from_village` 90% unskilled, 10% nitwit; `become_<group>` that one.

bds-job2.txt: with a bed and a bell placed first, an unskilled villager took a blast furnace placed 4
blocks away 1.8 s later: `SetEntityData` of `Variant` 8, `TradingPlayerEid` 0, `MaxTradeTier` 4.
bds-job.txt: without the bed and bell it took nothing in 90 s. Neither showed the villager dropping its
profession in the 20 s after its furnace was broken, nor taking the lectern placed meanwhile.
bds-n1drop.txt (`bds_command_probe n1drop`, 2026-11-04): two summoned armorers at their blast
furnaces, one traded once; both furnaces broken; neither `Variant` changed in 90 s. Sapling dropped an
untraded villager's profession at its next look (the Java rule); it now keeps it.

## Trading

- A use of the trader: BDS sends `UpdateTrade` alone (no `ContainerOpen`): window type Trading, size 0,
  `trade_tier` and `villager_unique_id` zigzag, `entity_unique_id` the player's runtime id as is (1),
  `display_name` `entity.villager.<profession>` (`armor`, `weapon`, `tool`, `leather`, `mason` for those
  five) or `entity.wandering_trader.name`, `new_trading_ui` and `economic_trades` true. The trader's
  `TradingPlayerEid` becomes the player's unique id and back to 0 at the close; it says `haggle`.
- The offer list holds every tier's offers from the first rolling, locked ones with their `tier`. Each
  recipe: `buyA`, `buyB`, `sell` (`Block` for a block item, `Count`, `Damage`, `Name`, `WasPickedUp` 0,
  `tag`: in name order, as BDS's compounds keep their keys, bds-s3books.txt; a wanted item without a
  data value has `Damage` 32767, any), `buyCountA`, `buyCountB`, `demand`,
  `maxUses`, `netId`, `priceMultiplierA`, `priceMultiplierB`, `rewardExp` (byte), `tier`, `traderExp`,
  `uses`; beside `Recipes`, `TierExpRequirements` (`[{"0": 0}, {"1": 10}, ...]`).
- A trade: `CraftRecipe` naming the `netId`, `Consume` of `buyCountA` from `Trade2Ingredient1` (slot 4)
  and of `buyCountB` from slot 5, `Take` of the result from the created output. BDS refuses a sold-out
  offer (`uses >= maxUses`, `maxUses` not negative). Each trade: `TradeExperience` raised by `traderExp`
  in a `SetEntityData` (with `TradeTier` at the threshold, 10 for apprentice: five coal trades),
  `haggle.yes`, one to three orbs at the trader. No second `UpdateTrade` while the screen stays open,
  a level-up included.
- `notifyTrade` (0x141baa8a0): `uses` += 1; `TradeExperience` grows only below `MaxTradeTier`; the
  orbs are 3 + nextInt(4) points a trade and 5 more on a level-up, at the trader, only for a
  `rewardExp` offer. A level-up sets a 10-tick timer; the economy's `newServerAiStep` (0x141ba8d80)
  counts it down only while no player trades, then gives the levelling player +1 standing and the
  villager regeneration for 200 ticks (bds-trade2.txt: `VisibleMobEffects` ~4 ticks after the close,
  10 s).
- Prices (`MerchantRecipe::calculateDemandPrices`, 0x14163f4d0): only `buyA` moves.
  `buyCountA = count − hero + trunc(pm × discount) + max(0, floor(demand × count × pm))`, from 1 to a
  stack; `hero = max(1, floor(count × (0.3 + 0.0625 × amplifier)))` with Hero of the Village (effect 29);
  `discount` the cured discounts (`economy_trade_table` `cured_discount` [-25,-20], `nearby_cured_discount`
  -20, capped by their `max_*`), which sapling lacks; a drilled cure gave none on BDS either (`g8cured`, Zombie villagers below).
- Restock (`resupplyTrades`, 0x141bb0e90, run as `minecraft:trade_resupply` is added): per offer
  `demand = max(0, demand + 2 × uses − maxUses)`, `uses = 0`. `behavior.work` (`active_time` 250,
  `goal_cooldown` 200, `can_work_in_rain` false; the farmer's `work_composter` too) fires
  `minecraft:resupply_trades` on each arrival, which adds `trade_resupply_component_group`; only the
  schedule events (gather, home, bed, wander, play) remove it, so it restocks once a work period, at
  the first arrival, used offers or not [pack]. bds-n1restock.txt: an armorer traded once after its
  first arrival showed `uses` 1 at every reopening for 118 s. Sapling restocked at the first arrival
  with a used offer; now at the first arrival.
- The draw (`_generateTrades`, 0x141bac370): a group with `0 < num < size` draws `num` times; the n-th
  draw rolls nextInt(sum of the weights from place n on), walks from place n, and swaps the trade it
  lands on into place n; offers come in draw order. The biome filter applies after the draw, per
  alternative: a drawn trade none of whose alternatives the type may offer makes no offer. Each item's
  alternative is nextInt(alternatives); wants before gives.
- `random_dye` (0x14752e880): `colour[perm[nextInt(16)]]`, perm
  `[15,7,8,0,3,1,14,11,10,2,6,12,4,5,13,9]` over the dye colours black 0 to white 15 (white `F0F0F0`); a
  second dye when nextFloat() < 0.3, a third when another < 0.2; channels averaged as f32 (c/255 summed,
  × 1/n, clamped) and truncated back to bytes; `customColor` ARGB with alpha 255.
- Wandering trader: its table's one tier; two `trader_llama` appear at its feet half a block up ~50 ms
  after it, with a `plop`.

## Kept

The actor compound keeps `Variant`, `MarkVariant`, `SkinID`, `TradeTier`, `TradeExperience` and
`Offers` (`Recipes` without `netId`, `TierExpRequirements`).

## Schedules

- A villager's `minecraft:scheduler` picks its goals by `hourly_clock_time`, the time of day
  [pack `villager_v2.json`]. `work_schedule` (also the farmer's, fisher's and librarian's groups): work
  0–8000, gather 8000–10000, work 10000–11000, home 11000–12000, bed 12000–24000. `basic_schedule`
  (unskilled): wander in place of work. `jobless_schedule` (nitwit): bed 0–2000, wander 2000–13000,
  home 13000–14000, bed after. `child_schedule`: play 0–11000, home 11000–12000, bed after.
- Gathering walks to the village's middle (`behavior.mingle`, `mingle_distance` 2, speed 0.5); bed time
  walks to its bed and sleeps on it (`behavior.sleep`, speed 0.6) [pack]. Speeds are `minecraft:movement`
  0.5 times the goal's multiplier.
- Home adds no goal (`home_schedule_villager` is `{}`; `move_towards_dwelling_restriction` is a base goal
  for one outside its village): a villager strolls. Corrected 2026-11-03: sapling walked it to its bed
  from 11000, so all three were by the beds 5 s after `time set 11400`; BDS's came one at a time and all
  three only past 12000 [bds-village.txt].

## Beds

- At its job looks a villager without a bed claims the nearest free bed head in the job site's reach,
  and gives it up once the bed goes [sapling; BDS claims beds as village POIs]. Babies claim one too:
  with 4 villagers and 3 beds BDS laid an unskilled adult, the armorer and the baby down
  [bds-village.txt].
- Lying down: SetEntityData box 1.0 by 0.3 and `FlagsExtended` bit 12 over bits 17 and 25; getting up
  sends its own box and the bits back. The first lay down the tick after 12000, the rest as they reached
  their beds; all rose the tick after 24000. The bed's `occupied_bit` stays 0 [bds-village.txt].

## Fear

- `behavior.avoid_mob_type`: a zombie, zombie villager, illager, vex or zoglin within 8 that it sees
  (glass hides, as for hunters) walks it away at 0.6 [pack; `ignore_visibility` unset]. Corrected
  2026-11-03: a zombie caged in glass 1 to 7 off moved none of BDS's villagers in 24 s; sapling's,
  which feared it unseen, walked out of 8 of it [bds-village.txt, sapling-village.txt].
- `behavior.hide` (priority 0, 30 s, a bed, speed 0.8) [pack]. A bell rung by hand or by power marks
  every actor with `minecraft:hide` in the bell's box grown 32 across and 12 up, then rests 300 ticks; a
  bell a villager rings in a raid marks none [BDS 0x141f1c400, its callers 141f1c130, 142ce0770,
  142cdf7c0 pass 1, `Village::tick` 142a23bc0 passes 0].
- Goals in order: trading customer, hide, flee, then the schedule [pack priorities: hide 0, avoid 4].

## Standing

- Each village keeps a standing per player, clamped to [-30, 10] [BDS `Village::modifyStanding`
  0x142a31380].
- A trade adds none; a trade's level-up adds 1 as it ends (Trading above) [BDS 0x141ba8d80;
  corrected 2026-11-04: 14194d9b0 is the legacy `villager` v1's `LegacyTradeableComponent`]. A player's hit on a villager takes 1, or 3 from a baby
  [BDS DwellerComponent hurt 141ba6be0]. A villager killed by a player takes 2 instead (below, "Shots");
  an iron golem 5 [BDS DwellerComponent death 141ba6460].
- A cure gives 5 to every player the village keeps a standing for, the curer only if it has one
  [BDS conversion 0x142445c80, read 2026-11-05: for a converting zombie villager (types 0x30b2c, 0x30b74)
  it finds the overworld village about the actor and calls `modifyStanding(+5)` over the whole
  `mPlayerStanding` map, Village+0x218, as `modifyStanding` 0x142a31380 reads it]. Sapling:
  `standing.rs` `cured`, from `conversion.rs` `turned`; test `a_cure_raises_every_kept_standing_by_five`.
- A new villager's first village gives every player it keeps a standing for the pack's
  `first_founding_reward` 5, once (`RewardPlayersOnFirstFounding`) [BDS DwellerComponent 0x141ba80b0, an
  inhabitant joining with the flag set]. Drilled 2026-11-05 (`y3found`, dumped with `records`): a hit's
  -1, then a villager summoned, left 4; with nobody hit first the village kept no player. Sapling:
  `standing.rs` `founded`, from `villagers.rs` `tick`, the flag kept in the actor compound
  (`storage.rs`); test `a_new_villagers_first_village_raises_every_kept_standing_by_five`.
- The cure's +5, drilled (`y3cure`, first scene): a hit's -1, a new villager (its +5), that villager
  killed by a zombie and its zombie villager cured kept 10, which takes the cure's 5 and the cured
  villager's own founding reward (−1 + 15, held at 10); sapling's cured villager is new and rewards too.
- Shots (`y3ranged`): a held villager shot twice to death left -3, the first arrow's -1 and the kill's
  -2: a shot counts as a hit, and a killing hit counts as the kill alone (the dweller's hurt handler,
  0x148487bd0, returns once health is under 1). Sapling counted -1 more for a killing hit, and nothing for
  shots; fixed (`standing.rs` `hit_delta`, `shot`, from `harm.rs` `wound_by`; tests
  `a_killing_hit_counts_as_the_kill_alone`, `an_arrow_on_a_villager_costs_its_archer_standing_as_a_hit`).
  The held golem 9 off took no target.
- Two village-wide timers step good standings down by one every 120000 ticks and bad ones up every
  12000, by as many steps as periods passed [BDS `Village::tick` 142a23bc0,
  `mGoodStandingDecayTimer`/`mBadStandingDecayTimer`].
- A village's iron golem takes the nearest player at -15 or lower [BDS `getClosestBadStandingPlayer`
  0x142a30c80 from the defend-village goal 148a38fd0; pack `behavior.defend_village_target`]. Only a golem
  of the village defends: one spawned with `minecraft:from_village` (its `village_created` group) took
  a player at its first hit on a grown villager; a `from_player` golem and a plain one never did; ten
  hits on a baby moved none [bds-golems.txt]. Sapling marks golems by that spawn event (`golems.rs`).
  Ten hits on a baby again (`y3baby`, 2026-11-05) left the player at -30 (dumped), and the held village
  golem 13 off still took no target in 30 s. The goal (0x148a38fd0) draws its chance, takes the nearest
  aggressor of the village's lists (0x142a30b10; the dweller's hurt handler adds any attacker of a
  villager or golem, 141ba6be0) and, when that is a player, swaps it for the nearest player at -15 or
  lower (0x142a30c80, no distance limit); why that took nobody here is not known. Sapling's golem takes
  a player at -15 by standing alone, so it would turn on the baby's hitter: left open.
  `ab3golem` (2026-11-05, bds-ab3golem.txt, sapling-ab3golem.txt): the same ten hits with the golem
  left free (no slowness). BDS's golem took slimes that spawned about the floor as targets, then none
  through and after the hits; so its slowness was not what kept it off. Past the player swap the goal
  filters and calls canAttack (14903c720) with a must_reach path test (14903f1c0); not traced. Sapling's
  golem took no target either, so the two agree on this scene.
  Traced 2026-11-05 (thirteenth3ad): `ab3golem` on the Linux BDS under gdb (`BDS_GDB`, breakpoints in
  `DefendVillageTargetGoal::canUse`, Linux 0x576c340; bds-ad3golem-gdb.txt, its hits in
  bds-ad3golem-gdb-hits.txt). Of 1063 calls the golem had its village every time and an aggressor
  (the player) in 796; the chance (+0x90) passed 55 times, the swap for a player at -15 or lower gave
  the player 25 times, the filters passed it each time, and canAttack (Linux 0xbdd60e0, called with
  no reach flags; its result is canUse's) refused it 23 times and took it twice. Both times the golem
  took the player as its target (`TargetEid` -4294967295 at 58.4 s and 86.2 s), between slimes it
  took. So BDS's golem does turn on the baby's hitter, seldom: canAttack's refusals, not the
  standing, keep it off. The earlier run's no target was these odds.
  One level deeper (thirteenth3ad, bds-ad3golem-gdb3-hits.txt): canAttack's refusal is its first
  test, the target goals' shared targetable check (goal vtable +0x60, Linux 0xbdd87a0, filter tail
  0xbdd83b0), never the reach path. It refuses a dead or removed target, an invulnerable player
  (ability 8: the creative watcher, every time) and then any target whose filter it fails; the
  defend goal has one filter, the pack's `is_family` mob or player with the default `max_dist` 16,
  tested as squared distance from the golem strictly under 16². All 27 defend calls on the actor
  failed that: the golem had wandered 30-42 off. So the hitter is taken only by a golem within 16.
  Sapling's has both tests (`hunting::seen` and `standing::outlaw`: mortal, within the golem's
  `sees` 16, at most 16 rather than under) and, in a rerun with temporary logging, took the actor at
  -15 from 11 off; the same rule.
- Prices take no standing: `economy_trade_table` reads only the cured, nearby-cured and hero discounts
  [BDS definition 141ba86e0].

## Golems

- `Village::tick` (0x142a23bc0) spawns an iron golem when the villagers holding a bed (the claimed
  POI map's bed slot) number at least max(20, villagers), at least three in four villagers worked within
  24000 ticks (`mVillagerWorkTimestamps`), the village's defenders number fewer than villagers / 10,
  and nextInt(700) is 0; `iron_golem` with `minecraft:from_village`, 10 tries at the centre plus
  nextInt(17) − 8 on x and z, y from +6 down to −6 (0x143905ed0), added as a defender. Cats: every 1200
  ticks, fewer than 5 cats within 48 and fewer than villagers / 4 brings one (`spawn_from_village`, or
  `spawn_midnight_cat` some nights); not built.
- bds-n1golems.txt: 20 beds and 10 working armorers spawned no golem in 150 s, as the rule has it.
  bds-n1golems20.txt: 20 armorers spawned none either (the beds 28 from the furnaces, likely past their
  claim reach); sapling's (sapling-n1golems20.txt) spawned none. A spawning village was not seen.
  Sapling's port (`golem_spawns.rs`) counts its own villages' villagers with a claimed bed and those
  that started work within a day, and draws 20 times every 20 ticks.
- A spawning village, `f3golem` (2026-11-04, bds-f3golem.txt, sapling-f3golem.txt): 20 beds, each with
  an armorer 2.5 off and its blast furnace 4 off, mob spawning on, golems counted every 6 s. BDS's first
  came 109 s after the count began (at 18.5, -0.5), its second 6 s later, then none to the end (176 s): two, its
  villagers / 10. Sapling's came 55 s and 85 s in, then none: the same cap, the waits within the
  1-in-700 draw's spread (one run each).
- A village golem is kept: its actor's `definitions` gain `+minecraft:village_created` (the pack's
  `from_village` group), which a load reads back (`golems.rs`).

## Zombie villagers

- `f3convert` (bds-f3convert.txt, sapling-f3convert.txt, 2026-11-04): `/damage` of 100 by a zombie on an
  armorer at work, on hard, made it a `zombie_villager_v2` in the hit's tick, unheard, where it stood and
  thrown as the hit threw it, with `Variant` 8, `MaxTradeTier` 4, its `TradeTier`, `SkinId` and
  `MarkVariant` (`keep_level`), shown hurt (`HurtAnimation`, health 19 of 20). On normal 4 of 6 turned
  (the pack's half; easy turns none, not drilled). Ported (`villagers/conversion.rs` `infect`, from a zombie's or husk's
  killing hit, by command or in a fight; test
  `a_villager_a_zombie_kills_on_hard_becomes_a_zombie_villager_keeping_its_profession`). Sapling's drill:
  hard 1 of 1, normal 2 of 6. The residue (sapling's `AddEntity` carried the defaults and a same-tick
  `SetEntityData` the villager's values) is fixed for every mob (thirteenth2ar, bds-ar2look.txt,
  sapling-ar2look.txt): a SetEntityData kept after an announcement is folded into its `AddEntity`
  (`mobs/folded.rs`), and what a new mob's first tick tells that its announcement already shows is not
  sent (`entities/tell.rs`). BDS's baby `zombie_villager_v2` (`as_baby`) shows 0.5 and 0.98 by 1.96, a
  baby villager's look; sapling's shows it once AN's `baby_look` covers that kind. A plain-summoned
  zombie villager came with `Variant` 8 on BDS (its spawn rolls a profession), 0 on sapling: not followed.
- A zombie villager keeps what it was (`Infected`) for its cure; `creatures/turning.rs` carries it back.
- Curing (pack `villager_converted` → `to_villager`; BDS 1424459c0, 14194f610): a golden apple used on
  one under weakness (used up out of creative) sets it shaking, takes the weakness, gives 300 s of
  strength and plays `remedy`; it becomes a `villager_v2` heard `unfect` after `value` + `range_min` +
  nextInt(`range_max` − `range_min`) seconds × 20 = 3600 to 5999 ticks, each tick one in 100 looking
  within 4 for beds and iron bars (at most 4), each taking a tick more off three times in ten. Ported
  (`villagers/conversion.rs` `cure`, `hasten_cures`; test
  `a_weakened_zombie_villager_given_a_golden_apple_shakes_and_is_cured`). The pack's `heal` is left out.
  `f3cure` (2026-11-04, bds-f3cure.txt): three held zombie villagers each given an apple shook, then a
  tick later `remedy` from their middle (feet + 0.95, of no entity); they became villagers 181.9 s,
  211.1 s and 178.4 s on, the last with a bed and iron bars beside it (under the 180 s floor: the
  assist), each `unfect` at its middle in the `AddEntity` tick. Not drilled on sapling (300 s scene).
  The cure's discounts, `g8cured` (2026-11-05, twelfth wave; bds-g8cured.txt, sapling-g8cured.txt): an
  armorer's offers read, it killed by a zombie on hard, its zombie villager cured by the actor beside a
  bed and iron bars (BDS 274 s, sapling 221 s), the cured villager's offers read again: on BDS every
  `buyCountA` was what it was before (iron leggings 7, diamond leggings 25, diamond helmet 23), and the
  neighbour 4 blocks off kept its prices too. No cured or nearby-cured discount showed, so sapling's
  cure giving none is the same. The price rule (0x14163f4d0, called per offer by 141bd2dd0) does add
  `pm × discount` from the economy component's cured and nearby fields, clamped by their maxima; what
  sets them was not traced.
  The "creeping" held villager of that drill (checked 2026-11-05, group 11) was not slowness 255
  failing: the slowness came a second after the summon, and sapling's armorer had set off for its blast
  furnace 0.35 s in (MOVING on, then off at 0.8 s; nothing after the slowness). BDS's summoned villagers
  first move 0.1 to 3.7 s in (64 villagers over bds-f3golem, n1golems, n1golems20, n1drop, n1restock,
  f3convert, g8cured, f3breed), sapling's mostly 0.04 to 3.4 s: the same, by chance on which side of the
  slowness line one fell.

## Unverified (sapling's choice where BDS was not seen)

- `enchant_with_levels` as the enchanting table's draw at that cost; a librarian's book any enchantment
  but Soul Speed, Swift Sneak and Wind Burst, curses included (bds-s3books.txt, twelve librarians'
  sixteen books: curse of vanishing I for 24; every other price in the formula's range), its price
  `base + rand(base_random + level × per_level_random) + level × per_level`, doubled for a treasure, at
  most 64.
- Job sites: looked for within 16 blocks across and 4 up or down, every 60 to 100 ticks (the dweller's
  `update_interval_base` 60 and `_variant` 40, the interval counter at `DwellerComponent`+0x40,
  148487bd0), with no village needed (BDS's dweller POI query range not decompiled; bds-job.txt: no
  job without a bed or bell). `f3poi` (2026-11-04, bds-f3poi.txt): an unskilled villager by a bell, held
  still with slowness 255, took no blast furnace 6, 10, 14, 18 or 26 off in 30 s, so the range was not
  read; a held villager seems to need to reach the site. `s3poi`, `s3poi2`, `s3poi3` (2026-11-05,
  bds-s3poi*.txt): a free unskilled villager 1 off a bell with a bed 3 off took a blast furnace 7, 9,
  11, 13 and 14 off the bell (twice) within 15 s, 15 off once of twice, and none 16, 20, 26 or 32 off
  in 60 s, so 16 across is a fit; the up and down reach stays a guess. BDS's `family=armorer`
  matched the armorer: a profession's component group adds its `type_family`. Sapling's selectors
  now ask the mob (`EntityBackend::entity_families`, `villagers/jobs.rs` `FAMILIES` from the pack:
  villager, the trade's group (peasant, blacksmith, artisan, priest or none), the profession, mob;
  a zombie villager its own, `zombie_villager`, zombie, undead, monster, mob); test
  `a_villager_that_takes_up_armoring_is_of_the_armorer_family`.
- The wandering trader leaves after 2400 or 3600 seconds (its `minecraft:timer`, taken as seconds) or
  once sold out, when no player is within 24 blocks; its timer is not kept by the actor compound.

Drilled: bds-village.txt, bds-golems.txt (`bds_command_probe village|golems`): sleeping (fixed: lying
shown, babies sleep), which golems defend (fixed). bds-golems2.txt (a restart) only counted the golems.

Re-run on sapling 2026-11-03 at TICK_SCALE 1, `SAPLING_DRILL_WORLD=lowflat` (research/captures/
sapling-village.txt, sapling-golems.txt): golems the same as BDS. Village: by night 2-3 villagers lay
within 3 of (-4, 9) where BDS had 1-2, and by morning sapling's had walked out of 8 of (0, 2) before the
zombie came, where one of BDS's stayed through it. Answers named `Villager V2` were fixed to
`Villager` (effects-raids.md). After the home and fear fixes (same day), villagers within 3 of the beds
per check: BDS `0 1 1 1 2 1 2 3 3 3 3 3 2 1 1 1 1 1`, sapling before `1 3 3 3 3 3 3 3 3 3 3 3 2 2 2 1 1 1`,
after `0 0 0 0 0 0 0 2 2 2 2 2 1 1 1 1 0 0`: no early rush to bed; BDS's single early sleeper and its third
bed-goer not matched (strolls are random on both).

## Breeding

Drilled 2026-11-05 (twelfth wave, group 11; `examples/shared/twelfth11_drill.rs`, two unskilled
villagers in the three-bed village; `research/captures/bds-g11*.txt`, `sapling-g11*.txt`):
- Food: `/replaceitem entity … slot.inventory` fills the pack's 8-slot inventory. Bred: both given 3
  bread in one of three runs (`g11love-2`; not `g11love`, `g11love-3`), one given 24 that it shared
  (`g11share`), 24 dropped at their feet (`g11breed-2`, `f3breed`; not `g11breed`, whose pickup came
  late). Not bred: one given 3 (two runs), both given 2, both given 3 with one bed left (`g11nobed`), none
  given anything over 300 s (`g11unfed`). In `g11breed-2` one villager took all the bread and took back
  each 3 it threw its mate, so the mate need not hold food (sapling's rule; not settled).
- The pairs that bred kept within 8 of each other and fell in love 19 to 29 s after they were fed;
  those that did not had strolled apart (`g11love-3`: 11 to 32 off from 41 s). Sapling: a willing one
  (3 bread, or 12 carrots, potatoes or beetroots) looks a tick in 1000 for a grown, awake mate within 8
  with a bed free within 32 (`villagers/love.rs`, `beds.rs` `one_free`).
- In love: `INLOVE` (flag bit 7) on both in one tick for 300 ticks; one walks to the other; both throw
  `LoveParticles` events (BDS 4 to 20 each in the 300 ticks, at random; sapling a tick in 30); then the baby comes at the middle
  of the walker's cell (`-4.5 -59.5 6.5` beside a walker at `-4.51, 6.05`), `INLOVE` off both, heard
  `born` at the baby's head with `is_baby_mob` (also for animals' bred babies, `bds-llamabreed.txt`).
  Each parent eats what made it willing and rests 6000 ticks (Java's; not drilled).
- Pickup (`villagers/pantry.rs`): a grown villager walks to food within 3 and takes the whole stack at
  once (`TakeItemEntity`; `g11breed-2` took 24 bread 0.45 s after the drop). Sharing (`share_items`:
  BDS threw 3 bread each 1.5 s, which the thrower mostly took back) is not built.
- Sapling drilled (`SAPLING_DRILL_WORLD=lowflat`): both given 3 bred in one of three runs
  (sapling-g11love-6.txt: `INLOVE` 20.2 s after the feed, 300 ticks, the baby heard `born`; -7, -8 none,
  their pair strolled apart), as BDS's one in three; 24 dropped was taken 0.2 s after the drop with no
  baby (sapling-g11breed-2.txt), as BDS's `g11breed`. A first try searched free beds only within the
  job reach (10) and never bred: the pairs strolled out of it.
- Left: the baby is announced at the grown box and scale 1 and loses its `BABY` flag bit the next tick,
  where BDS's comes at scale 0.5 (box 0.98 by 1.96); `summon villager … minecraft:entity_born` does the
  same (sapling-village.txt). `data/Mobs.bin` has no baby `villager_v2` recording, so `mob_look` has none,
  and `born` is heard at a grown head (1.62 up; BDS 0.83).
- Test: `two_villagers_with_three_bread_each_and_a_free_bed_have_a_baby_heard_born_300_ticks_after_love`.

## Not drilled and left out

See villagers-open.md.
