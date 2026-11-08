# World, server and item commands: what BDS showed and what was guessed

Captures: `research/captures/bds-worldcmds.txt` and `bds-itemcmds.txt` (`bds_command_probe
worldcmds|itemcmds`, an operator bot and a watcher on a flat BDS world), and
`bds-console-cmds.txt` (BDS's console). Code: `crates/sapling/src/vanilla/{spawns,regions,copying,
selecting,session,items,enchanting}.rs`.

## Seen on BDS and copied

- `/setworldspawn` clamps y to -64..319; `/spawnpoint` does not (y 500 kept). Both answer with the
  block; `/spawnpoint` and `/clearspawnpoint` have `.single` and `.multiple` keys, the latter with
  the success count. A cleared spawn point is not told to the client.
- `/difficulty` takes names, `p e n h` and 0-3; another number is
  `commands.generic.parameter.invalid`.
- `/fill` counts the blocks it changed (`hollow` twice: 26, then 0 with success false); `destroy`
  of air changes nothing. `/clone` counts every block copied, changed or not; `filtered` with no
  match is `commands.clone.failed`; overlap without `force` is `noOverlap`; a box out of the world
  is `commands.generic.outOfWorld`. `/testforblocks masked` skips the source's air.
- `/spreadplayers` checks spread < 0, then `maxRange < spread + 1`, both
  `commands.generic.num.tooSmall` with two decimals; its centre is counted from the sender's block
  (`0.00` from x 0.5); success count 1 whatever it moved.
- `/give`: amount 1..32767, data >= 0, an unsupported component answers a plain line,
  `wool <n> 14` is red wool. `/clear`: data -1 is any; `maxCount` 0 answers `.testing`; one line per
  player. `/enchant`: level out of 1..max answers two lines (`num.tooSmall|tooBig`, then
  `invalidLevel` with `%` and the enchantment's key); an incompatible or wrong-item enchantment,
  or a lower level of one already on, is `cantEnchant` with the player's name.
- `/setmaxplayers` and `/transfer` are console/owner commands; an operator gets
  `commands.generic.error.permissions`. `/transfer` takes a name, not a selector.

## Drilled 2026-10-29 (`bds-spread.txt` against `sapling-spread.txt`)

- Old names with data 14: `wool carpet concrete concrete_powder stained_glass stained_glass_pane
  stained_hardened_clay shulker_box` are the red one; `coral_block` is the dead tube block (kind in
  bits 0-2, dead with 8); `glazed_terracotta` is a syntax error. Sapling took shulker boxes white
  and coral alive; fixed (`world_commands.rs`).
- `/give` past 36 stacks: the sender is told the amount asked (5000), the recipient
  `successRecipient` with at most 36 stacks (2304 stone, 576 pearls into a full inventory), and
  nothing is dropped. Sapling told the recipient the amount; fixed. Sapling still sends an
  InventoryContent per stack (148 to BDS's 4).
- `/clear <item> 0 0` with none is `commands.clear.failure.no.items`, not `.testing`; fixed.
- `/spreadplayers` with no room: `commands.spreadplayers.failure.players` with
  `[count, x, z, "0.00"]`, not `.unsafe`; fixed. Success answers and `num.tooSmall` were the same.
- Re-drilled (`sapling-spread.txt`): every answer above now the same.

## Drilled 2026-10-30 (`bds-fixmisc.txt` against `sapling-fixmisc.txt`)

- `/time set noon` sets the next noon to come and answers the new time: 6000 from a fresh 540,
  then 30000, midnight 42000, noon 54000; `set 100` answers 100. Sapling's 30000 was the next noon
  from its later clock; same rule, nothing to fix.
- Relative `/fill` (`~`) answers its count (4) in the overworld, and in the Nether through
  `execute in nether` (49 absolute, 9 relative). Sapling's answers were the same; the old 0 is gone.

## Drilled 2026-11-04 (`bds-t1dspread.txt`, `bds-t1dloot.txt`, `bds-t1denums.txt`)

- `/spreadplayers` is BDS's iteration (bdsre 0x143b47550, `spread` in selecting.rs): random
  points in the square, each pushed a block from the mean of those nearer than the spread and
  kept in the square, up to 10000 iterations; once none moves, a target off safe ground starts
  again. A target lands two blocks over the top block (feet -25 over stone at -27), carpets not
  counted. Fire, water, lava, powder snow and a campfire refused; glass, a carpet and stone did not.
  Out of iterations: `failure.players [n, x, z, closest "%.2f"]`, or with one target (no pair)
  `failure.players.unsafe [1, x, z, "10000"]`. Its value parameters are type 4 as sapling sends.
- `/loot replace entity` puts every stack made (or the count's first) from the slot on: a player
  through its 36 slots (hotbar 7 ran on into the inventory), a mob through mainhand, offhand, then
  armour; a cow's `slot.hotbar` is `invalidSlotTypeForEntity [name, slot type]`. The answer counts
  the items, then the stacks: `["slot.weapon.mainhand", "0", "Cow", "5", "3"]`, `.block` alike.
  A count past the container is `countExceedsContainerSize [slot type's size]`.
- `/loot` takes every table of BDS's packs (fishing, bat, `empty`, barrels); an unknown one is
  `invalidLootTable ["loot_tables/<name>.json"]`, a `.json` or `loot_tables/` in it kept.
- `/playsound` counts and names those who heard it: one of two in range is `.single` with its name.
- `/scriptevent`: 2048 characters (bdsre 0x1403ea1a0, `0x800`); a 2047-character line got the drill
  bot disconnected, so the edge was not sent.
- `/recipe give @s *` unlocked 4092 (bds-t1drecipe.txt): every recipe but the starting 20 and the
  540 whose unlock list of ingredients is empty (barrel, `bed_dye_*`); again, no packet. BDS's
  order is the recipes' network ids, not CraftingData's; sapling kept CraftingData's until
  2026-11-04 and now sorts by network id (`recipes/book.rs`, test `give_all_lists_them_in_bds_order`:
  first, every thousandth and last of BDS's 4092 alike).
- `/gamerule` lists BDS's four overloads: `playerwaypoints`, 32 bool rules, 5 int rules.

## Drilled 2026-11-05 (`bds-h1summon.txt`, `bds-h1camera.txt`, `bds-h1cmds.txt`)

- `/summon pig x y z 90 45`: AddEntity pitch 45, yaw 90; the head comes level 10° a tick from the
  summon's tick on (MoveEntityDelta rot_x 24, 17, 10, 3, 0). `summon_as` takes the pitch
  (`creatures/gaze.rs`; test `summon_looks`).
- `/ride ... summon_rider <kind> <event> <name>`: the rider faces yaw 0 where the ride is, is
  named, and stays though the ride refuses it (`summonRider.failed`, zombie on a pig; bds-ride.txt
  kept two such zombies too). `summon_ride` puts the ride at a player's eyes (feet + 1.62), yaw 0;
  a ride the rider cannot mount stays, unnamed, and the answer is `summonRide.notRideable []`
  (earlier `bds-f1cmds.txt` mounted and named its horse "Ride"). `ShardCtx::name_entity` names a
  mob as a name tag does; boats and minecarts take no name in sapling.
- Names (bds-bg2names.txt): `summon <kind> <name> <pos>` (the name first) works as `summon <kind>
  <pos> ... <name>`; the named mob's AddEntity carries the name and `CAN_SHOW_NAMETAG`
  (`AlwaysShowNametag` 0), and so do its first tick's flags. Answers name a named target by its name
  (`/effect` "a", "v"; `vanilla.rs` `answer_name`). `/event ... minecraft:become_charged` charges a
  creeper as lightning does. A name tag used on a live mob was not drilled: BDS named nothing for the
  probe's click after the anvil, with or without a rejoin between (bds-bg2tag.txt).
- `/camera`: see the run, bdsre 0x140ff03e0. Checks in order: easing with seconds > 0 on a preset
  without it (`invalid.easing`, first_person; third_person's not drilled), `rot` pitch, then
  `view_offset` (|x|, |y| > 64 → `invalid.view.offset`) and `entity_offset` (> 64 on an axis →
  `invalid.entity.offset`), each `invalid.offset.camera` on first_person or third_person. The
  offsets go in the instruction set's `offset` (Vec2) and `entity_offset`.
- `target_entity <one> [target_center_offset x y z]` and `play_spline` want the free camera
  (`set minecraft:free` last), else `generic.wrongTargetCamera` before the target is looked up;
  `attach_to_entity <one>` sends the entity's unique id; `detach_from_entity` and `remove_target`
  send `Some(true)`. Many targets → `generic.tooManyTargets`, none → `generic.noTargetMatch`.
  What `play_spline` does on a free camera was not drilled.
- `can_place_on` / `can_destroy` (`bds-h1lock.txt`): `/give` and `/replaceitem` components, with or
  without `minecraft:`; a block name not known or `blocks` not a list → `Component '<as typed>' could
  not be updated.`; an unknown component → `is not a supported component.`. Each name widens to the
  current names of its legacy id over its metas (granite → the stones; `legacy_group`); BDS lists
  them in one fixed order, sapling in name order. On the wire they go in ItemV4 extra
  `can_place_on`/`can_destroy`, not the NBT; saved at the stack's top level (`CanPlaceOn`,
  `CanDestroy`). Adventure players break only what the held item's `can_destroy` names and place
  only on what its `can_place_on` names (test `adventure_lists`).
- BDS's list order (`k1lock`, bds-k1lock.txt, 2026-11-05): not per name and not the order typed. Two
  lists of 30 and 39 names are both one global order filtered (the 39 minus dirt's and stone's
  groups is the 30 exactly), and h1lock's run, earlier, kept the same relative order: a walk of the
  block registry, the same each run. It is no single sort or bucket order of FNV-1a, FNV-1,
  CityHash or MurmurHash64A of the names, with or without `minecraft:`, at any bucket count under
  200000 (a libc++ table's order also carries its insert and rehash history), and it is the Linux
  build's; the Windows build sapling follows (MSVC tables) would differ. Left: sapling keeps name
  order.
- The adventure places (bds-k1lock.txt): BDS refused the `can_place_on` stone onto grass in
  survival too, and placed plain stone there; acacia's bot sends its held item with empty
  `can_place_on`/`can_destroy` (`interact/wire.rs` `to_wire`), and BDS refuses a held item that does
  not match its own. So a listed stack's place on BDS cannot be drilled with this bot; plain stone in
  adventure was refused, as sapling refuses it. sapling does not compare the claimed item's lists and
  placed the listed stone in survival (sapling-k1lock.txt); a real client sends its lists, so only a
  client that misreports them sees the difference (left as sapling's choice).

## Drilled 2026-11-08 (`bds-m3slots.txt`)

- `/replaceitem entity @s slot.weapon.offhand 1 ...` succeeded (any slot number is the one offhand slot).
- `slot.enderchest` 0 to 26 succeeded and sent nothing until the ender chest was opened; 27 and -1 were
  `badSlotNumber` [n, "0", "26"]. `slot.hotbar 9` → [9, 0, 8], `slot.inventory 27` → [27, 0, 26].
- On a player `slot.saddle`, `slot.armor`, `slot.chest` and `slot.equippable` were `generic.exception`;
  `slot.armor.body` succeeded into Armor window slot 4, which sapling has no slot for (TODO in
  `vanilla/containers.rs`). Sapling: `Slot::EnderChest`; tests in `world_commands.rs`.

## Not seen; sapling's choice

- `/defaultgamemode`, `/seed` and `/version` do not exist on BDS 1.26.52: sapling answers under
  the client's `commands.defaultgamemode.success` and `commands.seed.success` keys, and `/version`
  in plain text. The default game mode is this shard's, for newcomers.
- `/setmaxplayers` and `/transfer` are for operators here (sapling has no owner level).
  BDS took 40 with no bound note; sapling's limit stops at the one the listener started with and
  notes `.success.upperbound` above it.
- `/transfer` without a port uses 19132; the Transfer packet's `reload_world` is false.
- The enchantment table (max levels, what goes on what, exclusive groups) beyond the cases above
  is the game's as players know it (`sapling-data/src/enchant/table.rs`), as is the order of
  `/enchant`'s checks (id, targets, level, item).
- BDS lists commands sapling has not (bds-t1denums.txt `command` rows, sapling-t1denums.txt):
  `aimassist controlscheme dialogue gametest packstack script` (`event` since thirteenth1bb; Script API, NPCs, game
  tests, client settings) and the host's `changesetting editor-allowlist permission reload
  reloadconfig reloadpacketlimitconfig sendshowstoreoffer serveridentity wsserver`; an `/execute`
  of the latter is refused as BDS refuses it. `/teleport` is `/tp`'s alias.
- `/spreadplayers` draws from its own random, not BDS's: the same rule, other places.
- `ParamKind::Value` is sent as type 4 (minecraft-data's number).
- A cloned chest: see containers-crafting.md B, `/clone` (sapling copies BDS's since 2026-11-04).
