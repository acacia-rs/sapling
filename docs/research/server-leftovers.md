# Server leftovers: /list, /stop, /save, /reload, /permission

BDS 1.26.52 (`.testserver/bds-blocks/bedrock_server.exe`), read with bdsre `dis` and, later, `c`. Captures: research/captures/bds-cmdlist.txt, bds-ui-operator.txt,
bds-console-cmds.txt.

## Levels

AvailableCommands (bds-ui-operator.txt): `/list` `any`; `/permission` (alias enum `permissionCommandAliases`
= permission, ops; 2 overloads) and `/reload` (1 overload) `admin`; `/save` (1 overload) and `/stop` `owner`.
An operator running an `owner` command gets `commands.generic.error.permissions [name]` (bds-worldcmds.txt,
`/setmaxplayers`). sapling has two levels; like `/setmaxplayers` and `/allowlist`, these are for operators.

## /list

bds-cmdlist.txt: `commands.players.list ["<online>", "<max>"]` then `commands.players.list.names ["A, B"]`,
both success, count 1; operators see the usual grey echo of both. Names in the order the players joined
(a rejoin moves a player last). sapling: the shard's players, max = the listener's current limit.

## /save (fn 0x1400b88a0)

FileArchiver keeps one global state: 0 idle, 1 saving, 2 ready, 3 error.
- `hold`: FileArchiver hold (0x1400b29c0) refuses in states 1 and 2 → `commands.generic.running` (error);
  else state 1 → `commands.save.start` (success).
- `query`: state 3 → `commands.save-all.error` (error); state 2 → `commands.save-all.success` then a second
  line `%%s` whose parameter is the file list (`path:size`, `, `-joined; nothing when the list is empty);
  states 0 and 1 → `commands.save-on.notDone` (error). The command's data gets
  `###* {"command":"save-all","result":[...]}` (formatter 0x1403d69f0).
- `resume`: FileArchiver resume (0x1400b30e0) only from state 2 (→ 0) → `commands.save-on.success`; any other
  state `commands.save-on.notDone` (error).

sapling: a hold writes back what the shard keeps (players' items, actors, ticks, scoreboard, maps), hands
every written column to storage, then holds all writes (columns stay loaded while unsaved; player saves wait
in memory). A query once storage has nothing on its way flushes each world (LevelDB to tables, `level.dat`)
and lists every file under the world folders, from the folder that holds them (`Bedrock level/db/CURRENT`),
sorted by path, without LevelDB's `LOCK`, `LOG` and `LOG.old`. Resume releases the writes. A flush failure
is state 3 and releases the hold. The hold covers the worlds of the shard the command ran on.

## /stop

`commands.stop.start` (success). BDS's console handles a typed `stop` itself: `Server stop requested.`,
`Stopping server...`, `Quit correctly` (bds-console-cmds.txt; string at 0x14007a42a). sapling answers the
command and resolves `Server::stop_requested`; the binary shuts the server down (examples/hub.rs).

## /reload (fn 0x143af3fb0)

Plain `/reload`: `commands.reload.success`, `commands.reload.error` on failure. `/reload all` reloads the
world for a host: `commands.reload.onlyHost` is among its answers, and a dedicated server has no host.
sapling: the behavior packs given to `ServerBuilder::behavior_packs` read again from disk (each pack,
folder and world list as given); every shard takes the new functions at its next `/function` or tick.

## /permission (fn 0x143ae6b60)

Mode at `[this+0x20]`: 0 list, 1 reload, 2 another (not a client overload; not built).
- reload: re-reads the file, then `commands.ops.reloaded` and `commands.permissions.reloaded`, both success;
  no file: `command.permissions.reload.fail.filenotfound` (error; note `command.`, not in en_US.lang).
- list: the XUIDs of the entries whose level is operator (byte 2), as tagged JSON
  `###* {"command":"ops","result":[xuids]}` and ` *###` (formatter 0x1403d69f0); no file:
  `command.permissions.list.fail.filenotfound` (error).
sapling applies a reloaded role to each online player with an XUID, as on join.

## Soft enums

`AvailableCommands.dynamic_enums` carries them, and the parameter flag is 0x410 (`SoftEnum`, acacia-proto).
`UpdateSoftEnum` add/remove/update changes them. sapling sends a player the enums changed since the build
right after the list, so a player sent the list late still gets the current words.

## Peaceful (bdsre `c`, pseudo-C now runs on Linux)

- `Monster`'s constructor (0x14298f180) emplaces `MonsterFlagComponent` and `RemoveInPeacefulFlagComponent`.
- `Monster::normalTick` (0x14298f410) calls `Mob::normalTick`. Then, if the level's difficulty (vfn 0x120)
  is 0 and the actor has the flag, it calls the actor's vfn 0x58 (remove). No drop, death or name check.
- The `setDifficulty` lambda (0x140847630) walks the level's actors and collects every flagged one, so a
  change to peaceful takes them at once.
- These constructors erase the flag: `HumanoidMonster` (piglin, 0x142990d10) and `EnderDragon`
  (0x145993350). `PiglinBrute` (0x1429910f0) puts it back. Hoglin is not a `Monster`.
- Walking the actor factory 0x141e9b120 found these type ids reaching `Monster`'s constructor:
  32-45, 49, 50, 52, 53, 55, 57-59, 104, 105, 114, 123, 126, 127, 131, 140, 146, 153.
- sapling uses the packs' `monster` family minus piglin and dragon (spawning/peaceful.rs).

## Breath and burning kept

A player's record is the Actor save (0x140d40ec0) plus its own fields. `Fire` (Short, ticks left) is the
field sapling already reads from stored actors (sapling-leveldb actors.rs). `Air` (Short, ticks of
breath, 300 full) is the matching Actor field. sapling writes both, takes them up at load, and tells a
player coming back short of breath its `Air` metadata in the join greeting.

## Drilled 2026-10-29 (research/captures/bds-server.txt, bds-server-console.txt)

- An operator bot gets `commands.generic.error.permissions` for `/reload`, `/reload all`, `/permission`,
  `/save` and `/stop` (owner-level on BDS); `/list` as above.
- Console `save hold`: `Saving...`; a `save query` 1.5 s later was ready: `Data saved. Files are now ready to
  be copied.` then one line `blocks/db/CURRENT:16, blocks/db/MANIFEST-000002:50, blocks/db/000003.log:21948,
  blocks/level.dat:3008, blocks/level.dat_old:3008, blocks/levelname.txt:6` (world folder first, sorted,
  `, `-joined, no LOCK/LOG). `save resume`: `Changes to the world are resumed.`
- Console `permission list`: `###* {"command":"ops","result":[]}` and ` *###`, then the same for
  `permissions`; `permission reload`: `Ops reloaded from file.` / `Permissions reloaded from file.`
- Console `reload all`: `Function and script files have been reloaded.` (sapling answers `onlyHost`).
- `/difficulty peaceful` took every monster still alive at once, stray, husk, drowned, bogged and wither
  skeleton among them; shulker, sulfur cube, ender dragon, camel husk, skeleton and zombie horses and polar
  bear stayed. A zombie summoned on peaceful is `commands.summon.failed`.
- Against `sapling-server.txt`: sapling took the shulker and summoned a zombie on peaceful, and took
  `/summon zombie_villager_v2` (BDS: `commands.generic.syntax`); all three fixed (`spawning/peaceful.rs`,
  `creatures/backend.rs`, `vanilla/players.rs`). Left as sapling's choice: an operator may `/save` and
  `/stop` (sapling has no owner level; BDS: `commands.generic.error.permissions`), and the probe's sapling
  has no `/reload` or `/permission` registered. Piglin and hoglin were gone on BDS before the switch (likely
  zombified), so their peaceful rule was not judged.

## Not drilled

- Sapling's console side of the above: `tools/remote-drill.sh` feeds a console to BDS only.
- `/stop` typed in the console of a sapling server.
- Peaceful for the kinds that died before the switch in the drill (burnt in daylight or despawned): blaze,
  piglin, silverfish, zoglin, zombie pigman, zombie villager and others; the family rule stands for them.
- `Air` is drilled (`bds-swim.txt`: a Short named `Air`). Whether BDS shows a player's burning flag in its
  first metadata after joining needs the same signed identity twice; not drillable offline.
