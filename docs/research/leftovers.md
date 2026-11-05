# Leftovers: grid map crafts, item repair, mounts, turtles

Small items earlier workstreams left (crafting-storage.md, mob-life.md). BDS read with bdsre `c` on
`bds-blocks/bedrock_server.exe`. Mounts and turtles were drilled on BDS 2026-10-29 ("Drilled" at the end); the
grid crafts were not.

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
  140d48140). sapling shows it as the `Strength` (75), `MaxStrength` (76) and
  `ContainerExtraSlotsPerStrength` (3) metadata in the mob's announcement and again after it, and rolls it
  for a stored llama that kept none.
- **Carpets**: slot 1 of a llama (`minecraft:equippable` slot 1, `carpet`) is worn as the horse's armour is:
  `MobArmorEquipment`'s body slot. A tamed grown llama takes a held carpet by a click (the tamed groups'
  `equip_item_slot`, sound `armor.equip_generic`).
- **Death**: a dying mount lets go of its goods outside the `doMobLoot` gate, then its chest, then a saddle
  put on it (the horse family and a camel); its armour and carpet stay with it (`bds-tack.txt`,
  `bds-mountdeath.txt`).

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
  ground), and the eggs break as "Drilled" says (JAVA `RemoveBlockGoal`/`ZombieAttackTurtleEggGoal`). It gives up
  after 1200 ticks of walking. Only with `mobGriefing` (JAVA).

## Drilled (2026-10-29)

Scenes `tack`, `llamabreed`, `nesting`, `stomping` (`examples/shared/tack_drill.rs`, `eggs_drill.rs`); captures in
`research/captures/`.

- Llama strength (`bds-tack.txt`, `bds-llamabreed.txt`): it comes in a `SetEntityData` right after the announcement.
  Sixteen summoned llamas fit the weights. Drilled same. Bred babies got 2, 1, 3 and 3, which is too few to tell
  sapling's fresh roll from Java's rule; sapling keeps its roll. They come with `born` and experience.
- The carpet (`bds-tack.txt`): taken by a click and shown in `MobArmorEquipment`'s body slot. Fixed: it is heard as
  `armor` and then `armor.equip_generic`.
- Fixed: `/summon <mount> ~ ~ ~ 0 0 minecraft:spawn_tame_adult` comes tamed, with no owner. A sneaking click opens
  the screen even holding a chest, and takes no chest (`bds-tack.txt`).
- Death (`bds-tack.txt`), fixed: the goods spill with `doMobLoot` on or off; the carpet, horse armour and saddle do
  not (a horse's saddle comes from its loot).
- Nesting (`bds-nesting.txt`): a bred turtle set off 36 and 51 s after breeding (1 in 700 a tick: 35 s on average).
  It dug 200 ticks, then laid. Fixed: while digging it throws the sand's 2001 dust at the nest cell's corner every
  5 ticks, the first as it starts.
- Stomping (`bds-stomping.txt`), fixed: at the eggs a zombie crumbles them every tick (`ParticleBreakingEgg` at the
  corner, data the plain egg). It hops and is heard (`block.turtle_egg.attack`, with its type and id) every 6 ticks
  from its third. After 48 to 51 ticks the block goes with 2003 (`ParticleDestroyEgg`), then the break sound with its
  type. Zombies also tread eggs as any mob does.

Drilled 2026-11-03 (research/captures/bds-gridmend.txt, sapling-gridmend.txt; `bds_command_probe gridmend`,
the special crafts sent by hand as crafting-storage.md "Grid crafts" says): two worn iron swords (150 with
Sharpness II and Vanishing, 130) made one worn 18 keeping only Vanishing; two of 10 and 5 one with
`Damage: 0` (sapling left no tag; fixed, special/repair.rs); a filled map and eight paper the map under
the next id with `map_is_init` 1; a filled map and a compass the map with `map_display_players` 1; an
empty map and a compass a bare `empty_map` of damage 2; an iron and a golden sword status 35
`MismatchedRecipeForInputGridItems`, nothing resent (sapling answered Error and resent; fixed,
`crafting::mismatches_grid`). The rest as built. Seen on the way: `/enchant` of Vanishing then
Sharpness left BDS's `ench` as [Sharpness, Vanishing], sapling's in the order given. Drilled and fixed
2026-11-03 (research/captures/bds-enchorder.txt, `bds_command_probe enchorder`): BDS keeps an item's
enchantments by activation, worn ones first, then held, then the item's own, each group in the order
given, and a raised level stays in place (`enchant::place`). Swords gave [Sharpness 3, Unbreaking,
Looting, Vanishing, Knockback, Mending], a bow [Flame, Unbreaking, Power 3, Infinity, Punch]: Flame acts
worn and Punch and Infinity as the item's own, which sapling had as held; changed in `enchant/kinds.rs`,
so the table's offers list them there too (not seen in an offer). Re-drilled on sapling
(sapling-enchorder.txt): all four items' `ench` lists the same as BDS's.

Drilled 2026-11-03 (`bds_command_probe mountflags`, `mountslots`, `mountdeath`, examples/shared/mount_plant_drill.rs;
research/captures/bds-*.txt against sapling-*; tests in tests/integration/mounts_drilled.rs):

- Fixed: a saddle shows WASD_CONTROLLED and CAN_POWER_JUMP on the horse family, WASD_CONTROLLED and CAN_DASH on a
  camel, in the flag word sent with it (`riding::saddle_flags`); words after it match BDS's. `slot.saddle air`
  answers success and leaves the saddle on, sending nothing.
- Fixed: a `/summon` naming a spawn event plays no `spawn` (BDS silent at all 112 such summons in the captures);
  one without an event still does.
- Fixed: `slot.chest` on a chested donkey takes 0 to 13 and refuses 14, its last goods slot, as
  `badSlotNumber [14, 0, 14]`; `slot.inventory` takes the tack then the goods, to 15, refusing 16 as
  `[16, 0, 15]`. A llama's bounds follow its strength (BDS's had 2: bound 5); sapling's for its own strength.
- Fixed: a llama's announcement carries its own `Strength`, `MaxStrength` and `ContainerExtraSlotsPerStrength` 3;
  sapling's showed the recorded llama's (2, and 0 slots a point) until the SetEntityData after, which now also
  carries the 3 (`riding/strength.rs`).
- Fixed, correcting the death entry of 2026-10-29: a dying horse, donkey, mule, zombie horse, chested donkey
  and camel drop the saddle `slot.saddle` put on them, after any chest (`bds-mountdeath.txt`, item 374; the
  horse's in `bds-tack.txt` was this, not loot, which is leather alone), with `doMobLoot` off too. Horse
  armour put by `slot.armor.body` stays; armour in the inventory's slot 1 is loot (corrected 2026-11-04 below).
  A horse summoned with `minecraft:horse_saddled` drops none.
- Fixed: `/replaceitem ... slot.armor.body` puts a horse's armour or a llama's carpet as `slot.armor` does;
  sapling's parser refused it (`vanilla/containers.rs`). Corrected by `bds-saddlerule.txt` below: BDS's
  `slot.armor.body` is not `slot.armor`.
- Right after a summon BDS's flag word lacks BREATHING and carries TAMED at once: fixed 2026-11-04 below.

Drilled 2026-11-03, third wave (`bds_command_probe saddleevents`, `saddlerule`, examples/shared/saddle_drill.rs;
research/captures/bds-saddle*.txt; sapling's re-drill in the report, not kept; tests in
tests/integration/mounts_drilled.rs):

- Fixed: a summon naming a kind's saddling event (`horse_saddled` for a horse or zombie horse,
  `donkey_saddled`, `mule_saddled`, `camel_saddled`) shows SADDLED and the controls and is heard as `saddle`
  (the event's `play_sound`), tamed or not; the mob holds no saddle and, as the summon runs that event
  alone and never the one adding the `<kind>_adult` group its loot table sits in, it dies leaving nothing
  (BDS's zombie horse: no rotten flesh, whose table's least is 2). `riding/evented.rs`. SADDLED in the
  announcement's own word: fixed 2026-11-04 below.
- Fixed: a pig's or strider's `on_saddled` shows SADDLED, unheard; a saddled pig or strider, by the event or
  by a click with a saddle, dies leaving the saddle first, then its meat or string (the `pig_saddled`
  and `strider_saddled` tables), and nothing with `doMobLoot` off. `slot.saddle` on a pig is
  `commands.generic.exception` (sapling's already was).
- Fixed, the saddle rule: `slot.saddle` takes only a grown, tamed horse, donkey, mule or zombie horse, and a
  grown camel; an untamed one (`spawn_adult`) and a foal are `commands.generic.exception`. A skeleton
  horse is refused always, though tamed from birth: its pack gives it no `minecraft:equippable` at all.
  The 2026-10-29 sample was this, not untamed.
- BDS's `slot.armor.body` is the generic body slot, not `slot.armor`'s tack: fixed 2026-11-04 below.
- Fixed: hand items given by `/replaceitem` are kept in the mob's `Mainhand` and `Offhand`
  (`creatures/command_gear.rs`, test `hands_given_by_command_are_saved_with_the_mob`), as BDS's actor format
  keeps them; the drill cannot read BDS's world folder, so the save itself was not compared.

Drilled 2026-11-04, fourth wave (`bds_command_probe bodyslot`, examples/shared/last_drill.rs;
research/captures/bds-bodyslot.txt, bds-mountflags.txt, bds-saddleevents.txt; sapling's re-drills matched
and are not kept; tests in tests/integration/mounts_drilled.rs):

- Fixed: `slot.armor.body` (`MobSlot::Body`; `slot.armor` is now `MobSlot::MountArmor`) is a body slot any
  mob has. Horse or wolf armour goes on any mob, tame or wild: horse, donkey, mule, zombie horse, camel,
  llama, pig, wolf (wolf armour too), shown in MobArmorEquipment, unheard, and it stays with the dead.
  Anything else (stone, a diamond helmet, a carpet, a llama's too) answers success and shows nothing; air
  sends an empty body even where there was none. `riding/body.rs`.
- Fixed: `slot.armor` writes the inventory's slot 1 and the body slot both (MobArmorEquipment, `armor`);
  `slot.armor.body` after it changes only what shows. An untamed horse's `slot.armor` is
  `commands.generic.exception` (no inventory); `slot.armor air` answers success and leaves the armour on.
- Fixed, correcting the death entries above: the armour in a tamed horse's slot 1 drops as loot after the
  leather, with `doMobLoot` on (`riding::looted_armor`); a clicked llama carpet stayed
  in `bds-tack.txt` with `doMobLoot` on, so a carpet is not loot.
- Drilled 2026-11-04 (`e1loot`, bds-e1loot.txt): a tamed horse with saddle and iron armour killed with
  `doMobLoot` off dropped its saddle, then its armour, in the tick it died, and no leather; with it on,
  leather, armour, then saddle the tick after. Sapling dropped the saddle alone a tick late with the
  rule off, and the saddle first with it on. Fixed: `creatures/death.rs` `die` (the tack tossed at
  once without loot, after the loot with it), test
  `a_tamed_horse_leaves_its_tack_after_its_loot_or_at_once_without_mob_loot`. A chested mount's goods
  and chest go with the saddle either way, not drilled with the rule off. Drilled 2026-11-04 (twelfth
  wave, `t12goods`, bds-t12goods.txt): a tamed donkey chested by a click, saddled, then given
  diamonds by `slot.inventory 0` (which took the saddle's slot: SADDLED went off) and emeralds in
  slot 2, killed with `doMobLoot` off, left the diamonds, emeralds and chest at once and no saddle
  (made in that order by their unique ids, sent newest first; sapling sends them oldest first).
  Left (twelfth wave): sending every new mob newest first broke four drilled orders (e1loot's tack
  in one tick came oldest first, saddleevents, the golems, the sand), so BDS's order is no one rule by
  id; the donkey's case needs BDS's send order traced first. Traced as far as the captures go
  (2026-11-05, twelfth wave), runtime ids made against the order sent: t12goods' donkey 18, 19, 20
  sent 20, 19, 18; e1loot's horse with the rule on (saddle 19, armour 20, leather 21) sent 21, 20,
  19; saddleevents' two pigs (porkchop 25 then saddle 26; 31, 32) sent saddle first; only e1loot's
  horse with the rule off (saddle 16, armour 17) sent 16, 17, the armour in the next batch, with
  the horse's own SetEntityData. Their SetEntityData and moves after come in yet other orders
  (20, 18, 19; 17, 16), and no MSVC unordered_map of runtime or unique ids (FNV-1a, 8 to 512
  buckets, the insert-in-front rule) gives the AddItemEntity orders. So drops made together are
  sent newest first but for that one pair, which points to the rule-off armour being made in a
  later step than the saddle. The sender is a virtual `Actor::tryCreateAddActorPacket`
  (140d2cc70, vtable +0xb8), reached only through vtables; its caller was not found. Left: a
  breakpoint there on a Windows BDS would name the step. Broken on the Linux BDS 2026-11-05
  (thirteenth2e; slot 24 of Actor and ItemActor, `bds-e1loot-gdb.txt`, `-hits.txt`): every AddActor
  came through one six-frame call path, so no later step makes the armour; and this run sent the rule-off
  pair newest first (armour 23, saddle 22) in one batch. The pair's order varies between runs, so
  newest first stands with no exception; sapling keeps its by-id order
  (its own making order differs from BDS's), which gives e1loot's, saddleevents', the golems' and
  the sand's sends.
  Sapling's (sapling-t12goods.txt) left the emeralds and chest and lost the slot-0 diamonds. Fixed:
  `riding/replaced.rs` keeps goods put by `slot.inventory 0` in the pack's slot 0 (unsaddling it,
  flags as BDS's), and `riding/packs.rs` `spilled` lets go of them; test
  `a_chested_donkey_without_mob_loot_spills_the_goods_put_over_its_saddle_then_the_rest_and_its_chest`.
- Fixed: a summon's spawn event (tame, saddle, baby) shows in the AddEntity's own flag word, and the first
  word after it, with BREATHING, carries it too (`creatures/birth.rs` `arrive`): the words now match
  BDS's for every summon in mountflags and saddleevents. The SetEntityData after every AddEntity and the
  death's armour: fixed in the fifth wave, below.

Drilled 2026-11-04, fifth wave (`bds_command_probe bodysave`, examples/shared/fifth1_drill.rs;
research/captures/bds-bodysave.txt, its world read with sapling-leveldb's `actor_dump`; tests in
tests/integration/mob_packets.rs):

- Fixed: BDS keeps any mob's body slot as `Armor`'s fifth entry (a pig's and a husk's horse armour, a
  wolf's wolf armour, the husk's helmet still first), and a tamed horse's `slot.armor` in `ChestItems`
  slot 1 besides: gold by `slot.armor` then diamond by `slot.armor.body` saved `Armor[4]` diamond and
  slot 1 gold. `riding/body.rs` keeps the body (`bodied_actor` now keeps the four pieces), `packs::keep`
  slot 1, and a loaded mob wears both again (`body::adopt`, `packs::adopt`). Sapling's older saves, with
  slot 1's armour only in `Armor[4]`, load it as the body alone.
- Fixed: BDS follows every new entity's AddEntity with one SetEntityData (`entities/first_data.rs`):
  not the flag word alone but the whole announced metadata, in order, less the items still as every
  actor is made (scale 1, a 0.6 x 1.8 box, a zero seat, no shooter, extended flags 0 and the like), with
  the AddEntity's properties. It is not sent when a mob is shown again or to a player who comes later;
  for a mob loaded from a world folder it was not drilled and sapling sends none. The first data's
  values: sixth wave, below. Tests read changes past it (`common/data.rs`).
- Fixed: at a mob's death, after its two emptied hands, a MobArmorEquipment of what it wears with its
  body slot (`creatures/worn.rs` `shown_dead`), as bds-bodyslot.txt's horse (body iron at death).
  Re-drilled on sapling (bodyslot, mountflags; sapling-mountflags.txt): each summon's two SetEntityData
  and each death's MobEquipment, MobEquipment, MobArmorEquipment now line up with BDS's. The
  SetEntityData at a death: sixth wave, below.

Drilled 2026-11-04, sixth wave (`bds_command_probe firstdata`, examples/shared/sixth3_drill.rs;
research/captures/bds-firstdata.txt, sapling-firstdata.txt; tests in tests/integration/first_data.rs
and tnt.rs):

- Corrected: the fifth wave read bds-tnt.txt as "the first data holds the values after the first
  tick". Drilled over a cow, a zombie in the sun, a summoned arrow and snowball, an orb, TNT by
  command and by signal, dropped items and five mounts, every first data equals its announcement
  less the as-made items, but a TNT's: it is announced unlit with `FuseLength` -1 and its first data
  holds IGNITED and 79, the first tick's burn, which is not told again. A mob's first-tick change (a
  cow's or horse's BREATHING, the zombie's catching fire with it) follows the first data alone, in the
  same tick, as sapling's did. Fixed: TNT (`tnt.rs`, `entities::show_first`); an item's metadata again
  after its AddItemEntity leaves out the as-made items too (64 announced, 59 again; `mobs::item_shown`);
  a llama's lone strength SetEntityData before its first data is gone (the announcement holds it).
  The "sometimes a tick late" of bds-beesout.txt's bee was not seen again.
- Fixed: BDS's data 16 (`MinecartDisplayBlock` on the wire) is a horse-family mob's open mouth: horse,
  donkey, mule, skeleton and zombie horse, not a llama or camel. Any hurt, `/damage` or `/kill`, sets
  it to 128 in the hurt's tick; it goes back to 0 29 ticks later (all four drilled: 1447-1452 ms). A
  death opens it as a hurt does, told after the loot's AddItemEntity, and a saddled one's death clears
  SADDLED, WASD_CONTROLLED and CAN_POWER_JUMP in that same packet (bds-mountdeath.txt; a spawn-event
  saddle, which drops nothing, shows no flag change). A zombie horse by day opened it at its first burn
  and never shut it while burning (bds-saddlerule.txt), so its death showed nothing more.
  `riding/mouth.rs`. Re-drilled: sapling-firstdata.txt's mouths open and shut on the same ticks.
- Mount inventories and boxes (2026-11-04, `s7mounts`, bds-s7mounts.txt against
  sapling-s7mounts.txt). Fixed:
  - A mount tamed by its spawn event (`spawn_tame_adult`, `on_tame`) is announced with ContainerType
    12 and ContainerBaseSize 2 (horse, zombie horse) or 16 (donkey, mule, llama). An untamed one, a
    foal and a skeleton horse show 0 and 0. A camel shows 12 and 5 once grown by its group
    (`spawn_adult`, plain); `camel_saddled` adds no age group, so it shows 0 and 0.
  - A trader llama has no `spawn_tame_adult`: it comes wild.
  - A zombie or skeleton horse whose event adds no age group (`horse_saddled`, `spawn_adult` on a
    skeleton horse) keeps the 0.6 x 1.8 box every actor is made with.
  - Such a zombie horse is tamed the tick after by its pack's `upgrade_to_1_21_130`: one
    SetEntityData of the word with TAMED, ContainerType 12 and ContainerBaseSize 2.
  - Where: `riding/inventory.rs`, `Raised::ungrouped`, `owners::tame_by_upgrade`. Test
    `a_summoned_mount_shows_the_inventory_and_box_its_event_gives`.
  - A mount tamed by riding (bds-n3ride.txt, read 2026-11-04): `TameSuccess`, then one SetEntityData
    of the word with TAMED and `OwnerEid`, then the tick after ContainerType and ContainerBaseSize
    alone. Sapling told the inventory with TAMED; fixed (`owners::tame_by_riding`,
    `riding/breaking.rs` `furnish`), test
    `a_horse_tamed_by_a_ride_shows_its_owner_with_the_flag_and_its_inventory_the_tick_after`.
  - Re-drilled (sapling-s7mounts.txt): every announcement's and data's container and box as BDS's.
- Flag words and moves of shots, TNT and items (2026-11-04, `s7shots`, bds-s7shots.txt; TNT in
  bds-firstdata.txt). Fixed:
  - A summoned arrow, snowball, egg, splash potion or bottle stays put the tick it comes. It falls from
    the next tick by its own weight and drag (arrow 0.05, thrown 0.03, both 0.99); sapling made it a mob
    falling 0.08.
  - The tick after any shot comes, BDS clears INVISIBLE and sets CAN_SHOW_NAMETAG. MOVING then follows
    the moving rule and goes once the shot lies.
  - `/summon ender_pearl` and `thrown_trident` are `commands.generic.syntax` on BDS.
  - A TNT's fuse is told every tick, with the flag word only when it changed (lit, MOVING on through
    its hop and off once it lies).
  - Items: the moving rule and the move counts match. The arcs differed only by the random throw.
    Drilled same.
  - Where: `flight::summon`, `flight::flags`, `entities::step_flagged`, tnt.rs. Tests
    `a_summoned_shot_falls_from_the_tick_after_and_is_shown_then`,
    `tnt_by_a_block_of_redstone_hops_burns_eighty_ticks_and_bursts`,
    `world_and_item_commands_answer_as_bds_answered`.
  - Re-drilled (sapling-s7shots.txt, sapling-firstdata.txt): each shot's flag words, positions and
    move count, and each TNT's words, are BDS's.
  - Left: a mob's shot moves in the tick it is loosed on BDS, a tick later on sapling. BDS turns a
    summoned shot to its fall at its first move (pitch byte 192); sapling's stays at 0. BDS sends a
    shot's SetEntityMotion one move sooner than sapling (sync.rs's motion rule, all entities).
  - Fixed 2026-11-04 (eleventh wave): a shot tells the motion of its next move, drag and weight
    taken (BDS keeps it so after the move), so its SetEntityMotion comes a move sooner as BDS's; a
    summoned shot turns to its fall at its first move (byte 192). `creatures/flight.rs` `fly`; the
    hit test moved to `flight/in_way.rs`. Still left: the loosed tick's move (moving it there made
    a shot that met its prey at once go unannounced; the TODO in arrows.rs `loose`), and BDS turning
    a loosed shot in flight (bytes 2, 1, 0, 255, 252 from the fourth move; not fitted).
  - Fitted 2026-11-05 (thirteenth wave, group Q) on bds-shoot.txt's skeleton arrow: each move the
    shot's pitch goes a fifth of the way to that move's own angle (the Java game's `lerpRotation`),
    and all the way at once from a shot announced at yaw and pitch 0 (the summoned one's byte 192);
    bytes cut toward zero, every byte of the capture's 12 moves matched. `creatures/flight/pitch.rs`,
    test `a_skeletons_arrow_is_told_pitching_down_as_it_flies`. The yaw was not seen to turn and is
    left as announced.
  - Fixed 2026-11-05 (thirteenth wave): a mob's arrow and a dispenser's shot make their first move
    in the tick they are loosed (`creatures/arrows.rs` `first_move`). The instant-hit regression
    came from removing an arrow in the tick it hit, before it was announced; BDS removes an arrow
    or snowball that hit a body the tick after the hit and sends no move in the hit tick
    (bds-shoot.txt, bds-n3knock.txt, bds-e1arrow.txt), so sapling now marks it `hits::Struck` and
    removes it at the next tick (`creatures/flight.rs`). Re-drilled (sapling-shoot.txt,
    sapling-e1arrow.txt): AddEntity, the first move and SetEntityMotion in one tick, a point-blank
    arrow announced with its hit and gone the tick after, as BDS. Small fireballs, llama spit and
    witch potions: re-drilled 2026-11-05 (blaze, provoke2, witch), each moves and is told its
    motion in its loosing tick, as BDS's. Still left: BDS announces a dispenser's arrow after its
    first move (AddEntity at the moved spot, n3knock, f1pickup) and its speed, yaw and pitch differ.

Drilled 2026-11-04, seventh wave (examples/shared/seventh1_drill.rs):

- The zombie that did not burn: not sapling's sun rule. sixth3's sapling run used drill_server's
  default world, whose stone roof at y 60-63 shades the floor at -27 (`GROUND`); BDS's floor is in
  the open. Re-drilled on sapling with `SAPLING_DRILL_WORLD=open` (sapling-firstdata.txt): the zombie
  burns from its first tick, as BDS's (sapling's had strolled to 3.5 blocks of the TNT and died of
  the blast; BDS's, further off, was put out for a tick and lit again). Drills of the sun or rain on
  sapling need `open`. Still different, Group 3's: sapling
  announces the zombie already burning (its summon tick runs before `mobs::update` and `reshow`
  rebuilds the AddEntity) and then tells BREATHING and the fire in two packets; BDS announces it
  unlit and tells both in one SetEntityData after the first data. Re-drilled 2026-11-04 on `open`
  (sapling-firstdata.txt): still so (AddEntity word ...561 lit, then ...928, ...929). Fixed
  (twelfth wave): `creatures::tick` leaves a new mob's announcement as made and rebuilds it the tick
  after, and `entities/tell.rs` `keep_last_flags` sends only the last flag word held for a new
  entity; re-drilled (sapling-firstdata.txt): AddEntity and first data ...560, then one ...929, as
  BDS's. Test `a_zombie_summoned_in_the_sun_is_announced_unlit_and_lit_after`.
- Fixed: data 16's 32 is `HorseFlags::Eating` (LeviLamina `HorseFlags.h`: Saddle 4, Eating 32,
  Standing 64, OpenMouth 128), not rearing. Read in `Horse::postAiStep` (0x142a49530; the Horse
  vtable 0x14a7c72f0 is the one family vtable overriding `Actor::setStanding`, slot 0x1d0;
  `setHorseEating` 0x142a4a020 is `_setHorseFlag(0x20, b)`): a horse not eating and carrying no one
  draws 1 in 300 a tick, and eats if the block under its feet is `minecraft:grass_block` (global
  0x14ccdfc38); it stops when its count passes 50, or in the first tick its navigation has a path,
  so a horse that draws while walking sets and clears it in one tick, which still sends data 16 (an
  unchanged 0). It also heals 1 a tick in 900: drilled 2026-11-04 (`e1heal`, bds-e1heal.txt), six
  held horses hurt 6 healed 11 times in 100 s, 1 at a time, two cows never; ported
  (`riding/grazing.rs` `heal`, its own draws; test `a_hurt_horse_heals_one_now_and_then_and_a_cow_never`). Drilled
  (`horseeat`, bds-horseeat.txt): eleven horse-family mobs on grass and three horses on stone, 180 s
  by night: every horse kind but the llama ate, meals of 50 ticks or cut short by a stroll (2 to 43),
  unchanged 0s sent 2 to 11 times a mob, a zombie horse killed while eating showed 160, the stone
  horses never ate. Fixed: `riding/grazing.rs` (told with the mouth by `riding/mouth.rs`; its own
  draws), stopped by a stroll or flight under way (`strolling::is_under_way`); test
  `grazing::a_horse_on_grass_grazes_for_50_ticks_now_and_then_and_one_on_stone_never`. Re-drilled
  (sapling-horseeat.txt): the same kinds eat 50-tick meals or shorter ones, the llama and the stone
  horses never, a mule killed eating showed 160.

Drilled 2026-11-04, ninth wave (examples/shared/ninth3_drill.rs):

- Standing on a turtle egg (n3eggs, griefing off): a cow, a pig, a zombie and a chicken summoned
  over single eggs landed at y -25.55, the egg's top, on BDS and on sapling alike.
- Home for a turtle never hatched (n3home, four pregnant turtles summoned on sand and teleported
  10 blocks east): each walked back west to where it was summoned, setting off 12 to 70 s later
  (`go_home`'s interval 700), at about 0.37 blocks a second. Sapling's `nesting.rs` makes the
  first cell it is seen in its home, as BDS did, but its turtles (sapling-n3home.txt) strolled about
  and wandered back only slowly within the 100 s, where BDS's walked straight home once set off.
  Fixed 2026-11-04 (`husbandry/nesting.rs`): a pregnant turtle away from home stands until it sets
  off, then walks straight to 0.3 of its home's middle (BDS's ended about 0.3 off); test
  `a_pregnant_turtle_taken_away_stands_then_walks_straight_home`. Stomping odds stay undrilled.
- Frogs (n3frogs, eight on open stone for 90 s): BDS's waits from a landing to the next leap were
  7.3 to 9.2 s (a frog's first, 5.0 to 6.7); sapling's 7.1 to 9.0.

## Not drilled

- Repair through `minecraft:repairable` (data-driven items naming themselves), drilled 2026-11-04 on
  spears (research/captures/bds-t2espear.txt): two iron spears worn 150 and 130, one with sharpness
  II, made one worn 30 (no twentieth) that kept sharpness II, in either hotbar order; sapling gave 18
  and dropped it. Fixed (`special/repair.rs`, both items' enchantments kept). Not drilled: two
  enchanted ones (the decompile says the first's tag is kept).
- `/replaceitem` on a mount, drilled (bds-fixreplace.txt) and fixed (`riding/replaced.rs`, test
  `mounts_take_tack_by_slot_as_bds_s_did`):
  - `slot.saddle`: the horse family, `saddle` heard at the feet and the flags; a llama is
    `commands.generic.exception`. Which mounts take it: drilled 2026-11-03 (`bds-saddlerule.txt`), above.
  - `slot.armor`: a horse takes horse armour or a carpet (MobArmorEquipment, `armor` mid-body); a
    llama a carpet the same, anything else unseen; a donkey is an exception.
  - `slot.chest` on an unchested mount: `badSlotNumber ["0","0","-1"]`; a horse is an exception.
  - `slot.inventory 0` is the first tack slot (a horse's saddle, a llama's carpet), so an item there
    unsaddles; the flags are sent only when they change.
  - The flags, the summon's sound and a chested mount's slots past the tack: drilled 2026-11-03, below.
- Turtles: the 1 to 4 egg count (two clutches seen), the nearest-sand pick and the nest search's
  20-tick look. Two runs (n3eggs) failed to show a clutch: eight turtles summoned with `go_lay_egg`
  on sand pens held up by stone, and one in a lane with sand 2 west and 3 east, laid nothing in 50 s;
  four pregnant turtles taken home (n3home) walked back but laid nothing in the 60 s after.
- Stomping: the 1 in 20 start, the 1200-tick give-up, and the walking speed (not tried again).

Drilled since (2026-10-30): a chested mount's chest drops after its goods (mob-life.md, `bds-chestdie.txt`), as
sapling's did. The `lay_egg` sound's 0.34 over a turtle's feet is fixed: 0.85 of its height, from the
bds-nesting.txt capture (mob_life.rs).
