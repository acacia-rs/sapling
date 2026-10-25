# Scoreboard and tags on BDS 1.26.52

Evidence: `research/captures/bds-scoreboard.txt`, `bds-scoreboard2.txt` (`bds_command_probe
scoreboard|scoreboard2`, actor and watcher bots, flat world), the world's `scoreboard` record read
with the `records` example, BDSRE (`Tags` key in 1401ac2d0), and the recorded AvailableCommands
overloads (11 for `/scoreboard`, 2 for `/tag`).

## Storage

- One board per level: key `scoreboard`, little-endian NBT. Root fields: `Criteria` (empty list: no
  non-dummy criteria), `DisplayObjectives` [{`Name` slot, `ObjectiveName`, `SortOrder` byte 0 asc / 1
  desc}], `Entries` [{`IdentityType` byte 1 player / 2 entity / 3 fake, `ScoreboardId` long, and
  `PlayerId` / `EntityID` long or `FakePlayerName` string}], `LastUniqueID` long, `Objectives`
  [{`Name`, `DisplayName`, `Criteria` "dummy", `Scores` [{`Score` int, `ScoreboardId` long}]}].
- Ids count from 1; `LastUniqueID` is the last given. Only holders with a score are entries.
- Tags: `Tags`, a list of strings in the actor's (and player's) compound.

## Answers

- `objectives add` success / `alreadyExists`; criteria other than `dummy` is a syntax error at it;
  names with `.` and quoted names with spaces are fine; no length limit seen (36).
- `objectives list`: `§a%commands.scoreboard.objectives.list.count` [n] then one `.entry` [name,
  display, criteria] per objective by name, success count n; `.list.empty` when none.
- `setdisplay <slot> <obj> [ascending|descending]` (default descending; `belowname` takes no
  order): `successSet` [slot, obj]; `setdisplay <slot>`: `successCleared` [slot].
- `players set|add|remove`: one target `.set.success` [obj, name, score], `.add/.remove.success`
  [amount, obj, name, new]; more `.set.multiple.success` [obj, n, score], `.add.multiple.success`
  [amount, obj, n]. An add that overflows i32 gets no answer at all. A missing score starts at 0.
- `players list`: `§a%commands.scoreboard.players.list.count` [n] and a line `%%s, %%s, ...` with
  the names (entities by `UniqueID`), success count 1; `.list.empty`. `list <target>`: per holder
  `§a%...list.player.count` [n, name] and `.list.player.entry` [score, display, obj]; a name with
  no score `.list.player.empty` [name].
- `test`: `.test.success` / `.test.failed` [score, min, max]; `*` min is i32::MIN, no or `*` max
  i32::MAX; min is required.
- `random min max`: `.random.invalidRange` [min, max] unless min < max; inclusive; more than one
  target answers `.operation.success` [obj, n].
- `operation`: `+= -= *= /= %= = < > ><` (C++ ints: truncating `/` and `%`, by zero leaves the
  target); a target without a score starts at 0, a source without one is passed over; nothing done
  is `commands.scoreboard.allMatchesFailed`, after `.list.player.empty` for an unknown source name.
- `reset <target> [obj]`: `.reset.success` [name] / `.resetscore.success` [obj, name]; `*` answers
  every holder, also those without that score.
- `/tag`: `commands.tag.add.success.single` [tag, name] / `.multiple` [tag, n] / `.add.failed`;
  same for `remove`; `list.single.empty` [name], `list.single.success` [name, n, "§atag§r, ..."],
  `list.multiple.success` [targets, tags, joined], `list.multiple.empty` [targets]. Tags list newest
  first.
- Selector `scores=`: a holder without the score fails, `!` or not; all objectives must pass.
- Rawtext `score`: a selector's scores `, `-joined; holders without one left out; nobody: the part
  is dropped; `*` is the reader, left raw for a reader without a score.

## Packets

- SetDisplayObjective (slot, name, display, "dummy", sort) to everyone; replacing a slot's objective
  sends a cleared one first (empty strings, sort 0); then a SetScore with all its scores. Clearing
  an empty slot sends nothing.
- SetScore per change only while the objective is shown, even when the score stays; players
  `changeplayer` with their actor unique id, entities `changeentity`, fakes `changefakeplayer`.
  Resetting sends `remove` entries with the objective name.
- RemoveObjective to everyone on `objectives remove`, shown or not.
- SetScoreboardIdentity was never sent.

## Unverified

- Order of `players list` names and of a holder's objectives (BDS: hash order); sapling lists
  holders newest first and objectives in creation order.
- A holder with scores but none on the tested objective (`test`): sapling answers
  `commands.scoreboard.players.score.notFound` [obj, name].
- Offline players' names (`commands.scoreboard.players.offlinePlayerName`) and their wire ids.
- `/titleraw` `*` score parts; whether `/tag` names a single entity by its type.
- Numbers past i32: BDS answers `commands.generic.num.invalid` [word]; sapling a syntax error.
- The `operation` syntax error token for `^=` (BDS points at `^`).
