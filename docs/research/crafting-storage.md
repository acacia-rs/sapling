# Special crafting recipes and old world storage

How BDS crafts its special (multi) recipes, and how sapling reads palettes and sub-chunks older
than the ones it writes. Code: `sapling-shard/src/inventory/special/`, `sapling-leveldb/src/upgrade/`.
BDS read with bdsre on `bds-blocks/bedrock_server.exe` (Linux has no r2 decompiler: `dis` only).

## Special recipes

`CraftingData` lists 14 multi recipes: a uuid and a network id each, no cells and no result. The
client names one in `CraftRecipe`; BDS looks the recipe up by that id, asks its `matches`, and hands
out what its `assemble` returns. sapling does the same: the named recipe's rule alone is asked
(`special.rs`), so a grid another recipe would take is refused.

Which uuid is which: `Recipes::init` 141aff700 builds each recipe after its `*_recipeId` string, with
the uuid globals a static initializer (142cafa40) parses from text.

| uuid | recipe id | sapling |
|---|---|---|
| `b5c5d105-…` | `banner_duplicate` | yes |
| `d81aaeaf-…` | `banner_add_pattern` | the loom's (`CraftLoom`), not this |
| `…-0000000000c8` | `shield_banner` (vtable 14a773110; uuid by exclusion, acacia-bot crafted with it live) | yes |
| `…-000000000002` | fireworks (rocket, star, fade) | yes |
| `85939755-…` | `map_cloning_crafting` | yes |
| `442d85ed-…` | `map_cloning_cartography` | cartography.rs |
| `d392b075-…` | `map_extending_crafting` | no: needs a new map record (maps.rs `charted`) at craft time |
| `8b36268c-…` | `map_extending_cartography` | cartography.rs |
| `602234e4-…` | `map_locking_cartography` | cartography.rs |
| `aecd2294-…` | `map_upgrading_crafting` | no |
| `98c84b38-…` | `map_upgrading_cartography` | no |
| `…-000000000001` | `repair_item` | no |
| `d1ca6b84-…` | `book_cloning` | yes |
| `685a742a-…` | `decorated_pot` | yes |

There is no armour dye recipe: no multi recipe, and no shaped or shapeless recipe outputs leather
armour from leather armour. BDS dyes leather armour in a cauldron only (blocks/cauldrons.rs).

What each makes, as its `assemble` reads (vtable slot 1):

- **Decorated pot** (142c7de60): the grid's slots 1, 3, 5, 7 (the table's edge middles) are read
  in that order into `sherds`, each the item's full name. 1457ed9b0 writes `{sherds: [4 strings]}`
  as the item's tag unless 1457edc20 finds all four the brick, then no tag.
- **Book cloning** (142c6d3a0): the written book's tag copied with `generation` one more (absent
  counts 0), count the number of writable books; a `generation` over 1 makes nothing (`cmp
  [rax+8], 1; jg`). The original book, count set to 1, is pushed as a second result.
- **Banner duplicate** (142c6a1e0): the first banner with a non-empty `Patterns` is copied with
  count 2 (`setCount(2)`) as the only result: both banners are used up.
- **Shield banner** (141b57e60): the shield's tag merged with `Base` (the banner's colour) and the
  banner's `Patterns`; an ominous banner (`Type` 1) takes another branch.

## Not drilled

- Every special recipe's `matches`: sapling's grid rules are Java's (one patterned and one blank
  banner of a colour; one written book and writable books; one filled map and empty maps; a shield
  without patterns) and the read of `assemble` above.
- The shield's ominous-banner branch and whether a patterned shield is refused.
- Map cloning in the grid: the copies' count and tag (Java's: one more than the empty maps).
- Book cloning's call through vtable slot 0x300 on the copy (custom name?).
