# Villagers and wandering traders

What sapling copies of BDS's villagers (`villager_v2`) and wandering traders, and from where. Built in
`crates/sapling-shard/src/creatures/villagers*` and `crates/sapling-data/src/trades*`.

## Evidence

- The vanilla behaviour packs (`entities/villager_v2.json`, `wandering_trader.json`, `trader_llama.json`,
  `trading/economy_trades/*.json`), read through `crates/sapling-data/data/generate/trades.py`.
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
- Wandering trader: its table's one tier; two `trader_llama` appear at its feet half a block up ~50 ms
  after it, with a `plop`.

## Kept

The actor compound keeps `Variant`, `MarkVariant`, `SkinID`, `TradeTier`, `TradeExperience` and
`Offers` (`Recipes` without `netId`, `TierExpRequirements`).

## Unverified (sapling's choice where BDS was not seen)

- Prices: `buyCount = count + max(0, floor(count × demand × priceMultiplier))`, from 1 to a stack; demand
  at a restock moves by `uses − (maxUses − uses)` (the Java game's rules).
- The orbs' worth: 3 to 6 points a trade (Java); BDS showed one to three orbs.
- The draw of a tier's offers: by weight without putting back, alternatives each as likely;
  `enchant_with_levels` as the enchanting table's draw at that cost; a librarian's book any enchantment
  but the curses, Soul Speed, Swift Sneak and Wind Burst, its price
  `base + rand(base_random + level × per_level_random) + level × per_level`, doubled for a treasure, at
  most 64; `random_dye` one of the 16 dye colours.
- Job sites: looked for within 10 blocks across and 4 up or down, every 60 to 100 ticks (the dweller's
  update interval), with no village needed; a villager loses its profession when its site goes and it
  never traded (BDS was not seen doing so); restocking on reaching its site in working hours (0–8000 and
  10000–11000), at most once each, so twice a day.
- The wandering trader leaves after 2400 or 3600 seconds (its `minecraft:timer`, taken as seconds) or
  once sold out, when no player is within 24 blocks; its timer is not kept by the actor compound.

## Left out

Gossip and reputation (and so discounts for curing and for Hero of the Village, whose effect the shard
cannot give), raids, villages (beds, bells, meeting points), sleeping and schedules other than work,
babies growing up and villager breeding (husbandry's: `villagers::arm` with `minecraft:entity_born`
makes a baby's unskilled trader), zombie villagers and curing, explorer maps (their trades are never
offered), the llamas' leashes and their going with their trader, a villager picking up and sharing food,
farmers farming, iron golems, the old trading screen.
