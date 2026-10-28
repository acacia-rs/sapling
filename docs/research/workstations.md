# Workstations: enchanting table, anvil, grindstone, smithing table

What BDS does at the four workstation screens, read from the BDS exe with bdsre
(`acacia/.testserver/bds-molang/bedrock_server.exe`, PE stamp 6ab54bee; addresses are that
build's). Where sapling copies it: `sapling_data::enchant::{kinds, table, anvil, grind}`,
`sapling_data::gear`, `sapling_data::smithed`; the shard's side in
`crates/sapling-shard/src/inventory/forge.rs` and `crates/sapling-shard/src/workstations*`.
Tags: **BDSRE** read in the exe; **inf** inferred from the code around it; **wiki** not read.

## Requests (vanilla client, from acacia's captures)

acacia `docs/research/workstations.md` and `vanilla-actions-2026-10-02.md` hold the recorded
requests. UI slots: anvil input 1, material 2; enchanting input 14, lapis 15; grindstone 16, 17;
smithing input 51, material 52, template 53; created output 50.

| Screen | Craft action | Then |
|---|---|---|
| Enchanting | `CraftRecipe{option net id, 1}` | ResultsDeprecated, Consume 14×1, Place 50→14, Consume 15×(index+1) |
| Anvil | `CraftRecipeOptional{0, filtered_string_index}`; name in `custom_names`, cause AnvilText | Consume 2×units, Consume 1×count, Place 50→… |
| Grindstone | `CraftGrindstoneRequest{input's stack net id, 1, cost}` | ResultsDeprecated, Consume 16 (17), Place 50→… |
| Smithing | `CraftRecipe{recipe net id, 1}` | ResultsDeprecated, Consume 53, 51, 52, Place 50→… |

The server works out every result itself (`_screenSimCraft` for the anvil, a craft handler per
screen for the others); a request whose consumes do not match is refused.

## Enchanting table

- **Bookshelves** (`EnchantUtils::findBookshelves` 141b658a0, BDSRE): for each direction
  (dx, dz) in {-1,0,1}² but (0,0) the gap column (dx, y and y+1, dz) must both be clear (block
  flag `+0x163 & 0x40`, taken as air); then shelves count at (2dx, 2dz) on both layers, and off a
  diagonal also at (2dx, dz) and (dx, 2dz). Only `minecraft:bookshelf` (not chiseled). At most 15.
  Unlike Java, a shelf off a corner looks through the corner cell, not the side.
- **RNG**: one MT19937 `Random` seeded with the player's enchant seed for all three offers;
  `nextInt(n) = genrand % n`, `nextFloat = genrand · 2⁻³²`.
- **Gate**: a non-null stack (`141a549d0`: air, or count and aux all zero, is null), count > 0,
  enchantability > 0 and no `ench` list on the item; else no offers.
- **Costs** (141b63d40): `base = nextInt(8) + 1 + b/2 + nextInt(b+1)`;
  `[max(base/3, 1), base·2/3 + 1, max(base, 2b)]`. No zeroing of low offers (Java has it).
- **Per offer, in order**: select enchantments, then the name (`nextInt(2)+3` words, each
  `nextInt(55)` from BDS's word table at 14a7733a0, the Java list cut at "stale").
- **Select** (`EnchantUtils::selectEnchantments` 141b62f70):
  - `a, b = nextInt(e/4+1)` twice; `L = max(1, (int)((nextFloat·0.3 − 0.15 + 1)·(cost+a+b+1) + 0.5))`
    in f32 (Java uses two floats).
  - Pool: every non-treasure enchantment whose primary **or secondary** slots fit, at the
    highest level whose `minCost ≤ L`. maxCost is never read.
  - Weights from frequency: 30→10, 10→5, 3→2, 1→1.
  - First pick by weight, removed from the pool, kept if it goes with what is already in the same
    activation list (`ItemEnchants::canEnchant` 141af34d0). Then
    `while nextInt(50) ≤ L && picks < initial pool size { pick; L = L·4/5 + 1 }`. Clashing picks
    are not pruned beforehand and still use a turn. Books keep every pick (Java drops one).
- **Taking offer i** (`CraftHandlerEnchant::_handleCraftAction` 1446e15a0): survival needs
  `cost ≤ level` and `i+1` lapis; takes `i+1` lapis and **i+1 levels** (not the cost). Creative
  skips both checks and pays nothing. A book becomes `enchanted_book`.
- **Option net ids**: one server-wide counter, pre-incremented, skipping 0.
- **Seed**: player NBT `EnchantmentSeed` (Int). A new player gets `nextInt()` (31-bit, never
  negative); after each enchant `seed = Level random nextInt()`.

### Enchantments (`initEnchants` 141b5be40, BDSRE)

Frequency / primary slots / secondary slots / minCost(l) / max level. Slot bits as in
`sapling_data::gear::slot`.

| id | name | freq | slots | also | minCost | max |
|---|---|---|---|---|---|---|
| 0 | protection | 30 | armour | | 1+11(l−1) | 4 |
| 1 | fire protection | 10 | armour | | 10+8(l−1) | 4 |
| 2 | feather falling | 10 | feet | | 5+6(l−1) | 4 |
| 3 | blast protection | 3 | armour | | 5+8(l−1) | 4 |
| 4 | projectile protection | 10 | armour | | 3+6(l−1) | 4 |
| 5 | thorns | 1 | chest | head, legs, feet | 10+20(l−1) | 3 |
| 6 | respiration | 3 | head | | 10l | 3 |
| 7 | depth strider | 3 | feet | | 10l | 3 |
| 8 | aqua affinity | 3 | head | | 1 | 1 |
| 9 | sharpness | 30 | sword, spear | axe | 1+11(l−1) | 5 |
| 10, 11 | smite, bane | 10 | sword, mace, spear | axe | 5+8(l−1) | 5 |
| 12 | knockback | 10 | sword, spear | | 5+20(l−1) | 2 |
| 13 | fire aspect | 3 | sword, mace, spear | | 10+20(l−1) | 2 |
| 14 | looting | 3 | sword, spear | | 9l+6 | 3 |
| 15 | efficiency | 30 | digging | shears | 1+10(l−1) | 5 |
| 16 | silk touch | 1 | digging | shears | 15 | 1 |
| 17 | unbreaking | 10 | 0x811E7F | 0x72E1C0 | 5+8(l−1) | 3 |
| 18 | fortune | 3 | digging | | 9l+6 | 3 |
| 19–22 | power, punch, flame, infinity | 30, 3, 3, 1 | bow | | 1+10(l−1), 12+20(l−1), 20, 20 | 5, 2, 1, 1 |
| 23, 24 | luck of the sea, lure | 3 | fishing rod | | 9l+6 | 3 |
| 25 | frost walker (treasure) | 3 | feet | | 10l | 2 |
| 26 | mending (treasure) | 3 | as unbreaking | | 25l | 1 |
| 27 | binding curse | 1 | 0x4400F | | 25 | 1 |
| 28 | vanishing curse | 1 | all | | 25 | 1 |
| 29–32 | impaling, riptide, loyalty, channeling | 10, 3, 30, 1 | trident | | 8l−7, 7l+10, 7l+5, 25 | 5, 3, 3, 1 |
| 33–35 | multishot, piercing, quick charge | 3, 30, 10 | crossbow | | 20, 1+10(l−1), 12+20(l−1) | 1, 4, 3 |
| 36, 37 | soul speed, swift sneak (treasure) | 1 | feet, legs | | 10l | 3 |
| 38 | wind burst (treasure) | 3 | mace | | 9l+6 | 3 |
| 39 | density | 10 | mace | | 8l−3 | 5 |
| 40 | breach | 3 | mace | | 9l+6 | 4 |
| 41 | lunge | 10 | spear | | 8l−3 | 3 |

Clashes (141b5b290): protection 0, 1, 3, 4; damage 9, 10, 11, 39, 40; silk touch with fortune;
depth strider with frost walker; infinity with mending; riptide with loyalty and with
channeling; multishot with piercing.

### Enchantability

Tools by tier (BDSRE): wood 15, stone 5, copper 13, iron 14, diamond 10, gold 22, netherite 15.
Armour: leather 15, chain 12, copper 8, iron 9, gold 25, diamond 10, turtle 9, netherite 15.
Carrot/fungus on a stick, trident, fishing rod: 1. Shears, flint and steel, shield, brush,
compass, elytra: 0 (never offered).

## Anvil (`Anvil_Cartography_computeResult_lambda` 148b7fd20, BDSRE)

- **Penalty**: input `RepairCost` + material `RepairCost`.
- **Material repair** (material is the item's repair item, item damageable): unit =
  `min(damage, max/4)`, none if < 1; each unit up to the material count: `damage −= unit`, +1.
  Consumes the units used.
- **Same item** (same id, damageable): `damage = max(0, d1 + d2 − max − max·12/100)`; +2 if
  that mends. Then enchantments merge. `minecraft:glow_stick` never combines.
- **Enchantments** (`Anvil_applyEnchantsCost_delta` 141b65460): cost = worth(after) −
  worth(before), worth = Σ multiplier·level; multiplier by frequency 30→1, 10→2, 3→4, 1→8,
  halved (at least 1) off an enchanted book. Only gains cost; a clash costs nothing (Java +1).
  A gain on an input stack of more than one costs 40.
- **Merge one** (141af34d0): must fit primary or secondary slots; if the item has it below max,
  equal levels go up one, else the higher; at max nothing; a clash is skipped.
- **Rename**: a new non-empty name +1; an empty name on a named item clears it, +1.
- **Total** = penalty + work. Rename only (work 1): `min(total, 39)`. Otherwise out of creative
  a total over 39 is "Too Expensive". Survival needs total ≤ level; creative pays nothing.
- **New RepairCost** = `2·max(input, material) + 1`, unless rename only.
- **Wear** (`Anvil_damageOnUse` 14580f450): on taking the result, out of creative,
  `nextFloat < 0.12`: anvil → chipped_anvil → damaged_anvil → gone, direction kept. Level event
  1021 (anvil use) on every use, creative too; 1020 (anvil break) when it goes.
- A filled or empty map with a material runs the cartography recipes instead (not in sapling).

## Grindstone (`CraftHandlerGrindstone::_handleCraftAction` 1446e1fc0, BDSRE)

- The action's recipe net id must be the stack net id of slot 16 or 17; its cost must equal the
  result's `RepairCost`. Each input count ≤ 1; consumes all of each.
- Two items: same id and damageable; the tag of the first one that has one;
  `damage = max(0, d1 + d2 − max − max/20)`.
- One item: stripped; nothing if unchanged. A trimmed item alone in the first slot gives
  nothing (quirk).
- Strip: all enchantments off, curses (27, 28) back (deduplicated across both inputs),
  `RepairCost` = [0, 1, 3] by curses kept. An enchanted book with no curse left becomes a book.
- Experience: n = ⌈Σ minCost(level) of non-curses / 2⌉, `n + nextInt(n)`, as orbs at the player.

## Smithing table (`CraftHandlerSmithingTable::_handleCraftAction` 1446e4f30, BDSRE)

- Recipe by net id; base and addition must be there; consumes one of each slot.
- Transform: the recipe's result with the base's whole tag.
- Trim: the base with `Trim{Material, Pattern}`; pattern from the template, material from the
  addition (`TrimData` from the client's start game).

## One enchantment table (2026-10-28)

`sapling_data::enchant::KINDS` (kinds.rs) is the only table: `/enchant`'s word and name key next to
`initEnchants`' rarity, costs and slot bits. `/enchant` checks what goes on an item by those bits too
(`slots_of`: a book all, a skull or head the cosmetic-head bit, else `gear::gear`), so it now takes what BDS's
secondary slots allow (Silk Touch on shears, the blade enchantments on a spear, curses on heads). Queries:
`enchant::level(tag, Enchant::X)`, the shard's `enchants::level_of(stack, Enchant::X)`, a plugin item's
`Item::enchantment(id)`; `Enchant::ALL`, `Enchant::from_id`, `Enchant::kind`.

## Banners (BDSRE)

- Placing (`BannerBlockActor` 141f0cad0, called from 142a817f0): the item's `Patterns`, at most 6 of them, go
  into the block entity; with no item tag the base colour is the aux value (`& 0xf`). sapling also keeps the
  item's `Type` (1 ominous).
- Load 141f0c460 and save 141f0c0f0 read and write `Base`, `Type`, `Patterns` (`{Color, Pattern}`).
- Loom (`CraftHandlerLoom` 1446e4100): a banner with more than five patterns is refused (1446e43b5), so six
  is the most.
- Breaking (sapling, inf): the banner item gets `{Patterns, Type}` back from the entity (nothing for a plain
  one), by hand, by losing its support and in a blast, where it is not merged with other banners.
  `loot::Tags` carries it, since a plugin's `Item` has no tag.

## Brewing

A hopper fills a bottle slot only when it is empty (`canPushInItem` 141f219d0, docs/research/brewing.md).

## Not drilled

- The bookshelf gap flag (taken as air only; may include replaceable blocks).
- Bow, crossbow, book and mace enchantability (1, 1, 1, 15 from the wiki).
- Whether an offer with no enchantments is sent (sapling sends all three).
- The offer name's trailing space (sapling trims it).
- The activation list (worn / held / itself) each enchantment is sent in.
- Whether a rename to an item's default name counts (sapling compares the custom name only).
- Whether a 1.26 client still sends `AnvilDamagePacket`. BDS's handler (1409aa040) rolls the wear
  again for an anvil within 8.7 of the player (14 in creative); sapling does the same out of
  creative. A creative player's packet got no answer at all (research/captures/bds-packets.txt).
- Which of the request's two name lists is the filtered one.
- `/enchant` by the slot bits: spear, shears and head cases (only sword, stone, book were captured).
- A broken banner's item tag (`{Patterns, Type}`, none for a plain one), a placed banner keeping the
  item's `Type`, and a banner in a blast not merging.
- The loom's limit of six patterns and the block's cap of six on placing (read, not played).
- A hopper refusing a bottle onto a filled bottle slot of a brewing stand.
- No `bds_*` drill was run against BDS for any of this; the numbers are from the exe only.
