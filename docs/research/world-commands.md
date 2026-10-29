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
- `/spreadplayers` places at random tries within the range; BDS's own spreading was not drilled.
  `maxHeight` caps where it looks for ground. The failure's fourth parameter is always `0.00`.
- `ParamKind::Value` is sent as type 4 (minecraft-data's number).
- A cloned chest: BDS adds `forceunpair`; sapling copies the block entity as it is.
