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
  recipe: `buyA`, `buyB`, `sell` (`Name`, `Count`, `Damage`, `Block` for a block item, `WasPickedUp` 0;
  a wanted item without a data value has `Damage` 32767, any), `buyCountA`, `buyCountB`, `demand`,
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
  -20, capped by their `max_*`), which sapling lacks (no curing).
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
  [BDS DwellerComponent hurt 141ba6be0]. A villager killed by a player takes 2 more; an iron golem 5
  [BDS DwellerComponent death 141ba6460].
- Two village-wide timers step good standings down by one every 120000 ticks and bad ones up every
  12000, by as many steps as periods passed [BDS `Village::tick` 142a23bc0,
  `mGoodStandingDecayTimer`/`mBadStandingDecayTimer`].
- A village's iron golem takes the nearest player at -15 or lower [BDS `getClosestBadStandingPlayer`
  0x142a30c80 from the defend-village goal 148a38fd0; pack `behavior.defend_village_target`]. Only a golem
  of the village defends: one spawned with `minecraft:from_village` (its `village_created` group) took
  a player at its first hit on a grown villager; a `from_player` golem and a plain one never did; ten
  hits on a baby moved none [bds-golems.txt]. Sapling marks golems by that spawn event (`golems.rs`).
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
- A village golem is kept: its actor's `definitions` gain `+minecraft:village_created` (the pack's
  `from_village` group), which a load reads back (`golems.rs`).

## Unverified (sapling's choice where BDS was not seen)

- `enchant_with_levels` as the enchanting table's draw at that cost; a librarian's book any enchantment
  but the curses, Soul Speed, Swift Sneak and Wind Burst, its price
  `base + rand(base_random + level × per_level_random) + level × per_level`, doubled for a treasure, at
  most 64.
- Job sites: looked for within 10 blocks across and 4 up or down, every 60 to 100 ticks (the dweller's
  `update_interval_base` 60 and `_variant` 40, the interval counter at `DwellerComponent`+0x40,
  148487bd0), with no village needed (BDS's dweller POI query range not decompiled; bds-job.txt: no
  job without a bed or bell).
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

## Not drilled

- Gathering and the other schedule goals' walks beyond the home and bed counts above; standing's
  numbers (no packet shows them).
- Fear in the open: `n1fear` (bds-n1fear.txt, sapling-n1fear.txt, 2026-11-04) held a zombie still in
  the open by two villagers: BDS's both stayed within 8 of it for 15 s, one within 4 for 9 s; sapling's
  fled (0 or 1 within 8). Not fixed: as with the glass cage, BDS's `behavior.avoid_mob_type` did not
  fire for a still zombie; its `canUse` (`probability_per_strength`, the target's strength) was not
  found in the binary, and a hunting zombie was not drilled.
- The schedule's re-check delay (`max_delay_secs`, up to 10 s); here every tick, which matched the
  drilled sleep and wake ticks.
- A baby's hurt standing (-3) moved no BDS golem within 30 s, not explained.
- Standing from ranged kills, the +5 of 0x142445c80 (curing) and the founding reward; the aggressor a
  golem defends against. Standing is kept as BDS's `VILLAGE_<id>_PLAYERS` (`Players`, each `ID` long and
  `S` int; 0x142a2c140), the id sapling's village uuid and `ID` the low half of the player's uuid (sapling
  players keep no `ActorUniqueID`).

## Left out

Gossip (Bedrock has none: its discounts are the cured ones of `economy_trade_table`, and sapling has
no curing), villages as BDS's dweller-claimed POIs, mingling partners, babies growing up and villager breeding (husbandry's: `villagers::arm` with `minecraft:entity_born`
makes a baby's unskilled trader), zombie villagers and curing, explorer maps (their trades are never
offered), the llamas' leashes and their going with their trader, a villager picking up and sharing food,
farmers farming, cats spawning, the old trading screen.
