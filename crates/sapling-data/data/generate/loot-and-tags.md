# Block loot and recipe item tags (ground truth for sapling)

Regenerate: `py -3 block-loot.py` and `py -3 item-tags.py` (a few seconds each, no arguments).
Inputs: `../java-truth/client-26.3.jar`, `../java-truth/quads.json.gz` (block-state domains), `bedrock-item-names.txt`,
`recipe-tags-used.txt`, `pmmp-item_tags-1.26.30.json`.

## block-loot.tsv — 880 rows, 546 blocks (verified: Java 26.3 loot JSON)

Columns: block, condition, item, min, max, chance, fortune, notes, bedrock_item, bedrock_item_known.
`block`/`item` are Java ids without namespace; `bedrock_item` is namespaced and present in `bedrock-item-names.txt`.
Omitted: the 704 blocks whose table is "itself, always, 1" (no row = drops itself). 1201 tables + 63 wall variants read.

- **Rows are additive**: every row whose condition holds drops independently (wheat `age=7` gives wheat AND seeds;
  carrots `default` 1 plus `age=7` 1..4 = 2..5). Exception: rows noted `else-branch` exclude the earlier alternative
  (gravel: flint 0.1, else gravel 0.9).
- condition: `default`, tool labels `silk_touch`, `no_silk_touch`, `shears`, `no_shears`, `shears_or_silk_touch`,
  `no_shears_no_silk_touch`; Java block-state literals `age=7`, `type=double`, `age!=3`, `a|b` value lists; `;` = and.
  `entity_breaker` = broken by an entity (snow, chorus_flower); `tool=#minecraft:cluster_max_harvestables`
  (amethyst_cluster) = pickaxe-class tool tag.
- count is uniform in min..max. `fortune`: `ore_drops` (count x (1 + max(0, rand(0..level+1) - 1))),
  `uniform_bonus:n` (count + rand(0..n*level)), `binomial:extra,p` (count = min + Binomial(extra + level, p);
  `max` = min + extra, i.e. level 0). `limit_count=a..b` in notes clamps after fortune.
- chance: level-0 chance; `chance_by_fortune=` in notes is the full per-level table (last entry repeats).
- Other notes: `count~binomial(n,p)` (stems), `count = number of true states among ...` (glow_lichen, sculk_vein),
  `copy_components:` / `copy_state:` (item keeps block-entity data), `loot_table_of=` (wall variant uses the base
  block's table, from `Blocks.wallVariant` in the decompiled source), `count=uniform(-6,2) before the clamp`.
- Explosion decay / survives_explosion ignored as requested.

## block-loot-nothing.txt — 134 blocks (verified: same source)

`block<TAB>reason`: blocks dropping nothing with a plain tool. Reasons: `silk_touch_only` 87, `shears_only` 7,
`shears_or_silk_touch_only` 4, `empty_loot_table` 14, `no_loot_table` 22 (matches the 22 `noLootTable()` in Blocks.java).

## item-tags.tsv — 238 rows, 17 tags

Columns: tag, item (Bedrock name, in `bedrock-item-names.txt`), source, in_bedrock_dump.
- source `java-tag:<file>.json`: Java tag expanded (nested tags followed), file = the tag naming the item directly.
  `bedrock-dump`: member only in the Bedrock dump. No row needed `bds-data` or `inferred` (see below).
- in_bedrock_dump: cross-check against pmmp/BedrockData `item_tags.json` (commit bdb44a48, "Updated from 1.26.30",
  a runtime dump of real BDS; copied here). `yes` (225 rows) = Bedrock-confirmed; `item_not_in_dump` (10 poplar
  rows) = item newer than the dump; `tag_not_in_dump` (3) = tag newer than the dump. No row is `no`: Java and Bedrock
  never disagreed on an item both know.
- Tag name mapping (inferred by name + identical member sets where the dump has the tag): egg->eggs,
  is_pickaxe->pickaxes, trimmable_armors->trimmable_armor, mushrooms_for_stew->mushrooms, others same name.

## What BDS itself holds (verified by extracting all 597 `.brarchive` files)

- No item-tag membership for any of these tags: item JSON `minecraft:tags` exist only for food and spears. Tags are
  compiled into `bedrock_server.exe` (not disassembled here).
- `loot_tables/blocks` exists only in `vanilla_1.26.50` (99 tables: wool/concrete slabs+stairs, red_shrub,
  shelf_mushroom). Sampled ones match the Java rows (double slab 2, shelf mushroom growth 1 -> 2).
- `smithing_table` is NOT an item tag: it is the `tag` (station) field of SmithingTransform/SmithingTrim recipes in
  `CraftingData-dump.txt` and `"tags":["smithing_table"]` in recipe JSON. No rows.

## Caveats

1. **mushrooms_for_stew** (new in 1.26.50, absent from the dump): BDS recipes listed `brown_mushroom` + `red_mushroom`
   until `vanilla_1.26.50/recipes/mushroom_stew.json` switched to the tag; Java's recipe uses `#mushrooms` the same
   way, which also holds `shelf_mushroom`. brown/red are verified, **shelf_mushroom is inferred**. sapling's
   `has_tag` currently lacks it.
2. **Poplar** (logs, wood, stripped, planks, slab): Java-only evidence, item postdates the dump. Likely right.
3. **decorated_pot_sherds includes `minecraft:brick`** on Bedrock (dump); Java's tag does not.
4. **trim_templates** has no Java tag: 18 `*_armor_trim_smithing_template` from the dump (netherite upgrade excluded).
5. Dump is 1.26.30, BDS here is 1.26.5x: membership could have changed since for old items (not seen so far).
6. Block loot is Java behaviour. Conditions use Java state names; Bedrock differs (shelf_mushroom `growth` not `age`,
   `*_double_slab` is its own block, snow `height`, wall/standing variants, one `bed`/`banner` item with colour in
   aux). Bedrock drop counts for old hard-coded blocks are not in any BDS file and were not checked (inferred equal).
7. Item mapping: 1 unmapped drop, `dynamic:sherds` (cracked decorated_pot drops its four sherd/brick items).
   Renames are hand-written in `bedrock_names.py` (web, deadbush, melon_block, snow/snow_layer, wooden_door,
   normal_stone_slab, small_dripleaf_block, azalea_leaves_flowered, quartz_ore, bed, banner); all other names are
   identity matches against the Bedrock list, not semantically checked one by one.
8. `tall_grass`/`large_fern`: Java checks the other half is present; rows mean "once per plant".
