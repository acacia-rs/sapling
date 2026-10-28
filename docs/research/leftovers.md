# Leftovers: grid map crafts, item repair, mounts, turtles

Small items earlier workstreams left (crafting-storage.md, mob-life.md). BDS read with bdsre `c` on
`bds-blocks/bedrock_server.exe`. No BDS drill was run: everything below is in "Not drilled".

## Special recipes in the grid (inventory/special/)

Each multi recipe's vtable is set in `Recipes::init` 141aff700; slot 1 is `assemble`, slot 5 `matches`.
The map-cloning, map-extending and map-upgrading classes are each built twice by one constructor (crafting
table, then cartography table): extending 142c80560 (vtable 14a7d3090), upgrading 142c81de0 (14a7d3190).
The full uuids are in the exe as text (`D392B075-4BA1-40AE-8789-AF868D56F6CE`,
`AECD2294-4B94-434B-8667-4499BB2C9327`).

- **Map extending** (`d392b075-…`; matches 142c806e0, assemble 142c80c10), special/maps.rs:
  - In a crafting table the filled map must be in grid slot 4 (the middle); at a cartography table anywhere.
    Every other filled cell must be paper: 8 at a table, 1 at a cartography table. Anything else refuses.
  - The map must be known to the level, its item damage below 3 (`*aux < 3`), not locked (`+0x7b`) and of
    scale below 4 (`+0x2c < 4`).
  - The result is the map item copied, count 1, with `map_uuid` the new map's id and `map_is_init` 1. The new
    map record is made inside `assemble` (the level's map call at vtable +0x140), so it exists at craft time.
    sapling: the same as the cartography zoom (`id + 1`, `maps::charted`), done when the request succeeds.
- **Map upgrading** (`aecd2294-…`; matches 142c81f60, assemble 142c82530): exactly one map and one compass,
  nothing else, any grid.
  - A filled map known and not locked, whose tag lacks `map_display_players` true (141a671f0): the copy,
    count 1, gets `map_display_players` 1 (1436f66a0).
  - An empty map of damage other than 2 and without that tag field (1436f6aa0): a new `empty_map`, damage 2,
    count 1, no tag.
- **Item repair** (`…-000000000001`, vtable 14a772f90; matches 141b56330, assemble 141b55170), special/repair.rs:
  - Matches exactly two filled cells, the same item, each of count 1, and the item damageable (item vfunc
    +0x100).
  - An item with `minecraft:repairable` naming itself goes through `RepairableItemComponent::repair` 14718c080
    (keeps the first item's tag). Otherwise: a new item of that kind, count 1, worn
    `max(0, d1 + d2 - M - M / 20)` (the `* -0x6667 >> 19` is `/ 20`), M the max damage (vfunc +0x120).
  - Only the curses 27 (binding) and 28 (vanishing) of either item are added back, each (id, level) once
    (141b55db0); everything else in the tags is dropped.

## Mounts (riding/strength.rs, riding/packs.rs)

- **Llama strength** (ED `llama.json`, `trader_llama.json`): `minecraft:add_attributes`, triggered by every spawn
  event, adds `strength_1` to `strength_5` at weights 32, 32, 32, 2, 2; each sets `minecraft:strength` with
  `max` 5. The tamed group's `minecraft:inventory` has `additional_slots_per_strength` 3, so the chest holds
  3 × strength. Kept as `Strength` and `StrengthMax` (ints; the `StrengthMax` key is read in `Actor::load`
  140d48140). sapling shows it as the `Strength` (75) and `MaxStrength` (76) metadata after the mob's
  announcement, and rolls it for a stored llama that kept none.
- **Carpets**: slot 1 of a llama (`minecraft:equippable` slot 1, `carpet`) is worn as the horse's armour is:
  `MobArmorEquipment`'s body slot. A tamed grown llama takes a held carpet by a click (the tamed groups'
  `equip_item_slot`, sound `armor.equip_generic`).
- **Death**: a dying mount lets go of its tack, its goods and, when chested, the chest (JAVA
  `AbstractChestedHorse.dropEquipment`, outside the `doMobLoot` gate), where its loot appears.

## Turtles (creatures/husbandry/nesting.rs, creatures/stomping.rs, blocks/eggs/nests.rs)

bdsre finds the goal definitions (`LayEggGoalDefinition`, `StompEggGoalDefinition`, `GoHomeGoalDefinition`: their
`_registerFillDefinitionFromComprehensive` lambdas 148883050, 1488483b0, 148993850) but not the goals' ticks, so
what follows is the pack (ED `turtle.json`, `zombie.json` and kin) and the Java game.

- **Breeding**: `minecraft:breedable` has `causes_pregnancy`: the bred turtle gets `minecraft:pregnant` (event
  `become_pregnant`) and no baby. sapling sets the `PREGNANT` flag and keeps the group in `definitions`; a
  turtle with an egg does not fall in love (JAVA `Turtle.canFallInLove`).
- **Going home** (`behavior.go_home`: interval 700, goal radius 4, speed ×1): taken as one tick in 700, then a
  walk home; within 4, `go_lay_egg` swaps `pregnant` for `wants_to_lay_egg`. Home (`minecraft:home`) is the
  egg's cell for a hatched turtle, else the cell it was first ticked in (JAVA: its spawn position).
- **Laying** (`behavior.lay_egg`: search 16 across, 4 up or down, goal radius 1.5): the nearest open cell on
  sand (the `LayEggGoalDefinition` defaults name sand and air above; the schema at 1430bef60 has a
  `lay_egg_sound`), a walk there, `LAYING_EGG` for 200 ticks (JAVA `TurtleLayEggGoal`, Bedrock's `lay_seconds`
  default taken as 10), then 1 to 4 eggs (JAVA `nextInt(4) + 1`), sound `lay_egg`, both flags off and
  `laid_egg` drops `wants_to_lay_egg`.
- **Stomping** (`behavior.stomp_turtle_egg` on zombie, husk, drowned, zombie villager (both) and zombie
  pigman: interval 20, search 10 across and 2 up or down, goal radius 1.14): one tick in 20 a zombie that
  does not hunt picks the nearest turtle egg and walks there. At the eggs it hops (0.3 up while on the
  ground) and after 60 ticks every egg breaks (JAVA `RemoveBlockGoal`/`ZombieAttackTurtleEggGoal`); it gives
  up after 1200 ticks of walking. Only with `mobGriefing` (JAVA).

## Not drilled

- Grid map extending, upgrading and item repair: all read from the decompile, none crafted on a live BDS.
  Repair through `minecraft:repairable` (data-driven items naming themselves) is not ported.
- The llama's carpet in `MobArmorEquipment`'s body slot, and whether BDS sends `Strength`/`MaxStrength`
  in the mob's announcement or after it.
- A bred llama's strength: sapling rolls afresh (Java takes the higher parent's with a chance of more).
- Goods and chest spilled on a mount's death regardless of `doMobLoot`.
- Turtles: the 1 in 700 home start, the 10 s dig, the 1 to 4 egg count, the nearest-sand pick, the nest
  search's 20-tick look, and home for a turtle never hatched: all ED or JAVA.
- Stomping: the 1 in 20 start, the hop, the 60-tick stomp out, the 1200-tick give-up and the walking
  speed (the mob's stroll speed).
