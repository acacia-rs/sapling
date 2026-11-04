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
  `aimassist controlscheme dialogue event gametest packstack script` (Script API, NPCs, game
  tests, client settings) and the host's `changesetting editor-allowlist permission reload
  reloadconfig reloadpacketlimitconfig sendshowstoreoffer serveridentity wsserver`; an `/execute`
  of the latter is refused as BDS refuses it. `/teleport` is `/tp`'s alias.
- `/spreadplayers` draws from its own random, not BDS's: the same rule, other places.
- `ParamKind::Value` is sent as type 4 (minecraft-data's number).
- A cloned chest: see containers-crafting.md B, `/clone` (sapling copies BDS's since 2026-11-04).
