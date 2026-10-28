# BDS brewing stands (2026-10-27)

Sources: BDSRE on `acacia/.testserver/bds-molang/bedrock_server.exe`; capture `research/captures/bds-brew.txt`
(drill `brew` of `bds_command_probe`, BDS 1.26 on devbox). sapling: `crates/sapling-shard/src/brewing.rs`, recipes
`crates/sapling-data/src/recipes/brewing.rs`.

## Tick (`BrewingStandBlockActor::tick` 141f22400)

In order, every tick:
1. Fuel 0 and slot 4 holds something: fuel = fuel total = 20 if it is `blaze_powder`, else 0; one item is taken
   either way.
2. *Can brew*: slot 0 holds an ingredient (`isIngredient`), fuel > 0, and some bottle in slots 1-3 has a mix.
3. Brew time < 1: if it can brew, time = 400 and the ingredient's item is recorded. No fuel is spent here.
4. Else time - 1. Can brew and time 0: finish. Else can brew with the same item (data not compared): go on.
   Else time = 0.
5. Finish: each bottle with a mix is replaced (container mixes are tried before potion mixes), tagged
   `wasJustBrewed: 1b`; the ingredient loses one (no leftovers); fuel - 1; LevelSoundEvent `potion.brewed` at the
   block centre.

Mixes are BDS's own `CraftingData` `potion_type_recipes` / `potion_container_recipes` (sapling-data reads them from
`CraftingData.bin`).

## What the screen is told (capture)

- Open: ContainerSetData 0, 1, 2 (brew time, fuel, fuel total), all three even when 0.
- Powder put in with fuel 0: properties 1 and 2 = 20 at once, the powder taken (refuel needs no ingredient).
- Ingredient put in: property 0 = 400 the same tick, then one a tick, 399 down to 1.
- Finish (tick of value 0): InventorySlot 1, 2, 3 (the new potions, tagged), InventorySlot 0 (the ingredient), then
  property 0 = 0 and property 1 = fuel left. The sound comes **one tick later**, to everyone near.
- Ingredient taken out mid-brew: property 0 = 0.
- An ingredient with no mix for the bottles (glowstone on splash awkward): nothing starts.
- A second powder while fuel > 0: nothing sent; it waits in slot 4.
- No UpdateBlock at the finish: the bottle bits change only when slots 1-3 go from empty to full or back.

## Hoppers (`canPushInItem` 141f219d0, `canPullOutItem` 141f21ce0)

Slot 0 only from above, and only an ingredient; slot 4 from any face but up, only blaze powder; slots 1-3 from
any face but up, potions and glass bottles, only into an empty slot. A hopper below takes from slots 1-3 only.

## Block entity

`CookTime`, `FuelAmount`, `FuelTotal` (shorts) next to `Items`.

## sapling gaps

- A stand nobody looks into starts brewing what a hopper brought only at the next ticking sweep.
- Finish order on the wire: sapling tells slot 0 before 1-3.
