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
  distance], x and z the middle of the start chunk (a stronghold where an eye flies), distance the
  2D one from the sender rounded down. Unknown or absent kinds (abandoned camp, fortress, end city,
  bastion in the overworld) → `structure.fail.nostructurefound` []. All 11 answers of the capture's
  seed-12345 world are reproduced (`sapling-world/src/generated/tests.rs`).
- `biome <minecraft:name>`: a name without the namespace or unknown is a syntax error; success
  [name, x, y, z, distance] on a 32-block grid; not found → `biome.fail` [name].

## /place

- Needs operator level 4 (`op-permission-level=4`; at 2 → `commands.generic.error.permissions`).
- Success is `commands.place.success` [] for every form.
- `feature <name>`: a name without `minecraft:` or not a known feature is a syntax error at the
  name; so were both feature rules tried (`featurerule minecraft:overworld_surface_oak_tree`).
  A y of 400 → `invalidposition.error` [].
- `structure <name>`: `minecraft:trail_ruins`, `abandoned_camp_forest`, `trial_chambers` placed;
  `minecraft:village` and unknown names → `invalidstructure.error` []. A place 9000 blocks off,
  unloaded, placed too.
- `jigsaw <pool> <target> <maxDepth>`: depth 0 and 30 → `maxdepth.error` []; an unknown pool or a
  target the pool lacks → `generation.error` []; `trail_ruins/tower road_anchor 3` and
  `village/plains/town_centers bottom 2` placed.

## /tickingarea

- Up to 10 areas a dimension, 100 chunks an area, a circle's radius up to 4; key strings in
  `sapling/src/vanilla/tickingarea.rs`. Bounds are shown widened to whole chunks; a circle by its
  centre chunk's middle and radius. Lists join lines with "\n"; what a change touched is listed
  with each line ended by "\n", then `inuse` [count, "10"].
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

- `commands.structure.error` for a save or load into unloaded columns (BDS queues a load,
  `load.queued`, which is not copied).
- Animation modes place at once.
- The biome search's reach (6400) and heights it tries; BDS's answers are only known to lie on a
  32-block grid and to be nearer than or as near as sapling's.
- Trial chambers and trail ruins: BDS named a place off their start chunk's middle (9 233, 610 30).
- `/place`: no feature rule is known, so every `featurerule` is a syntax error; BDS's own list of
  `structure` names is not copied (sapling takes `/locate`'s names, `village` too, and BDS's
  `abandoned_camp_forest` is not one of them); a structure is drawn only over loaded columns
  (`chunknotloaded.error`, where BDS placed one 9000 blocks off); a jigsaw target is matched by
  jigsaw name, which `village/.../town_centers bottom` is not here; the structure options
  (`ignoreStartHeight`, `keepJigsaws`, `includeEntities`, `liquidSettings`) are taken, not
  followed; the depth limit is taken as 20 (1..20; 0 and 30 refused); no block entities, loot or
  entities are placed with a structure.
- Ticking-area ids are made by sapling (BDS's are random UUIDs).
- Hardcore: StartGame's flag on BDS, and whether a hardcore world made fresh (cheats off) refuses
  `/difficulty` by another rule than cheats.
