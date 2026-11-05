# Structure templates, /place, /locate, ticking areas, hardcore and the border on BDS 1.26.52

Evidence: `research/captures/bds-structures.txt`, `bds-locate.txt`, `bds-ticking.txt`,
`bds-place.txt`, `bds-hardcore.txt` (`bds_command_probe structures|locate|ticking|place|hardcore`,
`crates/sapling/examples/shared/structure_drill.rs`), and strings of the BDS executable.

## Structure templates

- Stored as little-endian NBT: `format_version` 1, `size`, `structure_world_origin`, `structure`
  {`block_indices` (two layers, -1 for none, index `(x*sy + y)*sz + z`), `entities`,
  `palette.default` {`block_palette`, `block_position_data` {"index": {`block_entity_data`}}}}.
  In the world's LevelDB under `structuretemplate_<namespace>:<name>`; `.mcstructure` files in the
  world's `structures` folder are read too (`sapling-leveldb/src/structure.rs`).
- A name without a namespace is `mystructure:`; answers name it so, except `load.success`, which
  repeats the name as typed.
- `save name from to [includeEntities] [memory|disk] [includeBlocks]` and `save name from to disk`.
  Over 64x384x64 → `size.tooBig` ["64","384","64",sx,sy,sz]; entities and blocks both left out →
  `empty.save` (a box of air alone saves).
- `load name to [rotation] [mirror] [animationMode animationSeconds] [includeEntities]
  [includeBlocks] [waterlogged] [integrity 0-100] [seed]`: integrity out of range →
  `invalid.integrity`; the top past 320 → `y.tooHigh` [320 - sy]; below -64 → `y.tooLow` ["-64"];
  both includes false → `empty.load`; a missing name → `notFound` [namespaced].
- Turned and mirrored as the capture shows: a chest facing east faces south at 90°, west at 180°,
  north at 270°; stairs' `weirdo_direction` 0 goes 2, 1, 3; a log's `pillar_axis` swaps x and z on
  a quarter turn; mirror `x` flips z and `z` flips x; an entity's yaw turns 90° a quarter
  (`sapling-shard/src/structures/turn.rs`).
- A load is sent as one UpdateSubchunkBlocks; a chest's block entity gets `forceunpair` 1; water
  placed flows after. The same seed keeps the same cells at the same integrity.
- `delete` → `delete.success` [namespaced], then `notFound`; a delete of a name a load just missed
  answers `delete.success` once.

## /locate

- `structure <name> [useNewChunksOnly]`: `commands.locate.structure.success` [minecraft:name, x, z,
  distance], x and z the middle of the start chunk (a stronghold where an eye flies; trial chambers and
  trail ruins the middle of their start piece), distance the 2D one from the sender rounded down. Unknown
  or absent kinds (abandoned camp, fortress, end city, bastion in the overworld) →
  `structure.fail.nostructurefound` []. All 13 answers of the capture's seed-12345 world are reproduced
  (`sapling-world/src/generated/tests.rs`).
- `biome <minecraft:name>`: a name without the namespace or unknown is a syntax error; success
  [name, x, y, z, distance]; not found → `biome.fail` [name]. The search [D] (143aad6a0, 143aaa5d0,
  145807000, 145808020; `sapling-world/src/locate.rs`): a box 4000 out from the sender in x and z, from the
  bottom + 5 to the top, on a 64-block lattice from its corner, cut into blocks of 16x16x16 points; blocks in
  shells round the one holding the sender at the middle height, each block's points in shells round its
  point nearest the sender (x, then y, then z innermost); the first point of the biome wins. Its column's
  height (one above the noise shape's top block, 145807a20, for an unloaded chunk) is tested next and
  answered if the biome is there; else a 16-block lattice 64 round it under that height, then the highest
  open cell (it and the one above air, the biome in either) within 16 of that hit, in steps of two down
  from 16 above. Nothing found: the whole search again from 32 further in x, y and z. The capture's two
  answers and two failures are reproduced. Not drilled: a hit whose surface is not the biome (the
  16-block step), and loaded chunks, whose height BDS reads from their blocks.

## /place

- Needs operator level 4 (`op-permission-level=4`; at 2 → `commands.generic.error.permissions`).
- Success is `commands.place.success` [] for every form.
- `feature <name>`: a name without `minecraft:` or not a known feature is a syntax error at the
  name; so were both feature rules tried (`featurerule minecraft:overworld_surface_oak_tree`).
  A y of 400 → `invalidposition.error` [].
- `structure <name>`: `minecraft:trail_ruins`, `abandoned_camp_forest`, `trial_chambers` placed;
  `minecraft:village` and unknown names → `invalidstructure.error` []. A place 9000 blocks off,
  unloaded, placed too.
- Names (bds-t1dplace.txt, bds-t1denums.txt): `featurerule` takes the 170 of BDS's `featureRules`
  enum (`minecraft:minecraft:savanna_mutated_...` among them), else a syntax error;
  `structure` a soft enum, any name; `liquidSettings` `apply_waterlogging|ignore_waterlogging`,
  `banana` a syntax error. Without level 4 the parsed line is `generic.error.permissions ["place"]`.
- `jigsaw <pool> <target> <maxDepth>`: depth 0 and 30 → `maxdepth.error` []; an unknown pool or a
  target the pool lacks → `generation.error` []; `trail_ruins/tower road_anchor 3` and
  `village/plains/town_centers bottom 2` placed. bds-k1jig.txt (2026-11-05, every name namespaced
  but as said): a pool or target without `minecraft:` → `generation.error` (sapling the same,
  `vanilla/worldgen.rs`). A town centre one deep (seed 20261018, y -60): one iron golem made with
  `doMobSpawning` off, none with `includeEntities` false (sapling: the templates' marker mobs, made
  unless it is false, `generation.rs` `write`; test `a_placed_village_names_its_marker_mobs`);
  `keepJigsaws` true left 32 jigsaw blocks (a fill replacing them counted 32), false none. No
  villager in a town centre.
- `keepJigsaws` (bds-n13jig.txt): each kept jigsaw is `jigsaw[facing_direction=F,rotation=0]`
  (up 1, sides 2-5; rotation always 0) with a `JigsawBlock` entity: `final_state`, `joint`
  (`rollable`), `name`, `target`, `target_pool`, `selection_priority`, `placement_priority`.
  sapling files them as `BlockEntity::Jigsaw` in on-demand placements only
  (`StructureWrites::jigsaws`, `jigsaw/kept.rs`) and writes them when asked (sapling-world
  `PlacedStructure::of`). A village pool through `/place jigsaw` writes nothing on sapling: its
  elements are legacy templates the v2 placer skips (TODO in `jigsaw/piece.rs`).
- `ignoreStartHeight` (bds-n13jig.txt): trail ruins from y -50 spanned -57..-35, from -40
  -51..-25 (top 15 over the given y); without it they stood at their own height, buried at the
  floor. sapling: v2 kinds start at the given y (`place_structure_at`; test
  `ignore_start_height_stands_trail_ruins_at_the_given_height`).
- `liquidSettings` (bds-n13liq.txt, plains houses into water pools): `apply_waterlogging` left
  water in layer 1 under the waterloggable blocks placed (stairs, slabs, fences, panes, doors),
  `ignore_waterlogging` none. sapling: apply is what any write does (`blocks/waterlog.rs`);
  ignore clears layer 1 under every placed block (`sapling-shard/src/generation.rs` `write`).

## /tickingarea

- Up to 10 areas a dimension, 100 chunks an area, a circle's radius up to 4; key strings in
  `sapling/src/vanilla/tickingarea.rs`. Bounds are shown widened to whole chunks; a circle by its
  centre chunk's middle and radius. Lists join lines with "\n"; what a change touched is listed
  with each line ended by "\n", then `inuse` [count, "10"].
- The `inuse` count is every dimension's areas (a Nether and an overworld area: `["2", "10"]` from
  either). `list all-dimensions` is one line per dimension, `%dimension.dimensionName<n>: ` then
  its areas, all one output line; BDS listed the Nether (added first) before the overworld, which
  sapling follows as the order areas came (which BDS keeps is not settled) [bds-h1cmds.txt].
- Stored in the level (`sapling-leveldb/src/ticking_areas.rs`) and kept loaded with nobody near
  (`sapling-shard/src/kept.rs`), as are a plugin's forced chunks.
- Bedrock keeps no spawn chunks: a repeating command block's clock stopped within a second of the
  players going 3000 blocks off, and with `tickingarea add circle 0 -60 0 1` it counted on at 20 a
  second (answer centre `8, 0, 8`) [bds-spawnchunks.txt, `bds_command_probe spawnchunks`]; only
  ticking areas stay. Sapling kept counting: a column stays loaded while its writes are being saved,
  and a repeating block writes every run. Fixed: a repeating block in a column no player sees and no
  ticking area keeps waits (`command_blocks/run.rs`;
  `command_blocks::a_repeating_block_stops_once_nobody_is_near_as_bds_keeps_no_spawn_chunks`). Other
  block ticks in such a column are not checked.

## Hardcore

- `IsHardcore` (byte) in level.dat; sapling sends it as StartGame's hardcore flag (BDS's StartGame
  was not recorded). `bds-hardcore.txt` ran on a world made in creative with `IsHardcore` set
  afterwards: a player killed gets Respawn state 0, and on rising Respawn state 1, then
  UpdatePlayerGameType spectator (to itself and to others) and SetHealth 20.
- Commands are not locked there: `/difficulty easy`, `/gamemode`, `/kill` and the rest of the
  drill's list answered as on any world (an earlier run with `isHardcore=true` in
  server.properties on a fresh world had cheats off, so every command was `generic.disabled`).
  sapling therefore does not lock the difficulty.

## World border

- BDS 1.26.52 has none (no `worldborder` string in the executable). sapling's `WorldBorder`
  (`ctx.set_world_border`) is a plugin API, not vanilla: a player past it is put back inside.

## Unverified

- Animation modes place at once.
- `/place featurerule` places nothing (`invalidfeaturerule.error`): on the drill's flat world BDS
  answered every rule, and `feature minecraft:beehive_feature`, at y -26 with `invalidposition.error`
  (bds-t1dplace4.txt), and its console placed nothing (bds-t1dplacec.txt), so what a rule places is
  unseen. `structure` names beyond those drilled: sapling takes `/locate`'s names, `village` too,
  where BDS's `ancient_city` and `mineshaft` were `invalidstructure.error` and
  `abandoned_camp_meadow` a `generation.error`; a structure is drawn only over loaded columns
  (`chunknotloaded.error`, where BDS placed one 9000 blocks off); `ignoreStartHeight` for kinds
  other than the v2 ones; `includeEntities` is followed for `/place structure`
  as for jigsaws, undrilled there; the depth limit is taken as 20 (1..20; 0 and 30 refused).
  Block entities are placed with their loot tables (`StructureWrites::entities`); what BDS's
  chests held is unseen by command.
- Ticking-area ids are made by sapling (BDS's are random UUIDs).
- Hardcore: StartGame's flag on BDS, and whether a hardcore world made fresh (cheats off) refuses
  `/difficulty` by another rule than cheats.
