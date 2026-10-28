# /loot, /camera, /camerashake, /fog, /hud, /music, /playanimation, /daylock, /recipe, /inputpermission, /mobevent on BDS

Captures (`bds_command_probe <word>` against BDS 1.26.52): `bds-lootcmd.txt`, `bds-lootcmd2.txt`
(`lootcmd`, `lootcmd2`), `bds-screen.txt` (`screen`), `bds-music.txt` (`music`), `bds-controls.txt`,
`bds-controls2.txt` (`controls`, `controls2`). Sapling's answers and packets match where the worlds
agree (BDS's floor at -20, sapling's at 64; loot rolls differ), checked with the same drills
against `drill_server`.

Every command:
- A syntax error keeps ten characters each side of the token: `["t entities", "/", "armor_stan"]`.

/loot:
- A table is a string: unquoted, `/` ends it and is the syntax error; quoted, any path, any case
  (`"Entities/Chicken"`). Unknown: `commands.loot.failure.invalidLootTable ["loot_tables/<path>.json"]`.
- Success: `commands.loot.success [stacks made]`, summed over `give`'s players (`@a` of 2 → "4");
  an empty table "0".
- Tool word: any word; `mainhand`/`offhand` are the sender's hands, an unknown item is no tool.
- `kill`: one target (`noTargetMatch`, `commands.generic.tooManyTargets []`); a player gives "0";
  nothing dies. `mine`: the block stays; stone by hand 0, wooden pickaxe 1; air or outside 0.
- `insert`/`replace block` without a container: `failure.noContainer ["Pos(x,y,z)"]`.
- `replace`: a slot outside the type, `failure.invalidSlotIdForSlotType [id, "0", last, type]`
  (hotbar last 8; a block's type "none"). Success `replaceSuccess [type, id, name, a, b]` and
  `replaceSuccess.block ["slot.container", id, a, b]`; the default count is one slot.

/daylock (/alwaysday): no command output. Every player gets Json Text
`commands.always.day.locked|unlocked`. Locking a running day: SyncWorldClocks at the next noon,
then GameRulesChanged doDayLightCycle=false. Already locked: the text only.

/camerashake: CameraShake {intensity 0.5, 1 s, positional} by default; intensity above 4 is
`num.tooBig ["5.00","4.00"]`. `stop` without a player stops every player's and names them all.

/camera: presets by CameraPresets.bin order (free = 3). `pos` and `facing` resolve `~`;
`facing <entity>` is its box middle (player feet + 0.9); `rot` is pitch, yaw; `default` sets
`default` and `remove_ignore_starting_values`. `unknown.preset`, `invalid.pitch` (|pitch| > 90),
`invalid.time` (fade seconds outside 0..10). BDS's fade and fov bodies (2026-10-28 `screen` rerun,
dump.rs printing undecodable bodies): `fade` `0000010000000000000000`, `fade time 1 2 3`
`000001010000803f000000400000404000000000000000`, `fade color 0 255 0`
`0000010001000000000000803f00000000000000000000`, `fov_set 60` `…01 00007042 00000000 06"linear" 00…`,
`fov_set 60 2 linear` ease time `00000040`, `fov_clear` `…01 00000000 00000000 06"linear" 01…`. So fade
is optional time then optional colour (channel/255), and fov's ease is a string; acacia-proto reads
them so from 2026-10-28 (its docs/proto.md). The `screen` lines above that show decode errors predate it.

/fog: PlayerFog carries the whole stack; pop takes the topmost entry of the id, remove all;
none: `commands.fog.invalidUserId [name, id]`. Any fog id is taken.

/hud: SetHud, element and visibility zigzagged (hotbar 5 shows as VehicleHealth; reset as
Unknown(2)). No element is all thirteen in BDS's order.

/music: LevelEventGeneric to every player, ids 1900 queue, 1901 play, 1902 stop, 1903 volume,
fields `fadeSeconds`, `repeatMode`, `trackName`, `volume`. Volume above 1: `num.tooBig`.

/recipe: players start with UnlockedRecipes.bin's list. give: NewlyUnlocked [id],
`unlockedRecipe`; known: `playerHasRecipe` (failure). take: RemoveUnlocked; a starting recipe
comes back at once (InitiallyUnlocked [id]); unknown to the player: `playerDoesNotHaveRecipe`.
`take *`: RemoveAllUnlocked [] then InitiallyUnlocked [the starting list]. Unknown id
(`stick`): `notFound []`. Nobody: `noTargetMatch` and `commands.generic.player.notFound`.
The answer comes before the packet on BDS; sapling sends it after.

/inputpermission: plain-text answers. set: UpdateClientInputLocks only on a change,
`"<Shown> input enabled|disabled for <name or N players>  " ["N"]`; the shown names of
lateral_movement, mount, dismount and move_forward end in a space. query: `"<<Shown>>: E enabled
| D disabled" [enabled names]`, count 1 (also for nobody); with a state `"<<Shown>>: N state"
[names]`, count 1 only when every target is in it. Lock bits: camera 2, movement 4, lateral 16,
sneak 32, jump 64, mount 128, dismount 256, forward 512, backward 1024, left 2048, right 4096.
The `haspermission` selector filter reads the same locks.

/mobevent: `eventIsTrue|IsFalse|SetToTrue|SetToFalse ["%entity.<pillager|wandering_trader|ender_dragon>.name", event]`;
`events_enabled`: `eventsEnabled...`. /ability does not exist (`commands.generic.unknown`).
/event and /dialogue answers are recorded in bds-controls.txt but not built.
