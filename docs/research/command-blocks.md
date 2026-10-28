# Command blocks (2026-10-28)

How BDS keeps and runs command blocks, and what sapling copies (`crates/sapling-shard/src/blocks/command_blocks.rs`,
`command_blocks/run.rs`). Captures: `research/captures/bds-cmdblocks.txt`, `bds-cmdchains.txt`, `bds-cmdlinks.txt`,
`bds-cmdlinks2.txt`, `bds-cmdrules.txt` (drills in `crates/sapling/examples/shared/command_block_drill.rs` and
`command_link_drill.rs`). Code: bdsre on `bds-molang/bedrock_server.exe` (VAs below; struct offsets from LeviLamina's
`CommandBlockActor`/`BaseCommandBlock`).

## Functions

| Label | VA | Role |
|---|---|---|
| `CommandBlock::tick` | `148b2cac0` | the queued run (below) |
| `CommandBlock::_onRedstoneUpdate` | `148b2d7e0` | keeps `powered`; a rise in redstone mode, not chain: mark condition, queue a tick of `ExecuteOnFirstTick ? 0 : TickDelay` |
| `CommandBlock::_executeChain` | `148b2e220` | follows `facing_direction` through chain blocks, at most `maxCommandChainLength` of them |
| `CommandBlock::_executeChainBlock` | `148b2e400` | one link (below) |
| `CommandBlockActor::markConditionMet` | `141f38350` | met unless `conditional_bit`; then met = the block behind is a command block with `SuccessCount > 0` |
| `CommandBlockActor` setAutomatic | `141f38d10` | made always active, or made repeating: unpowered and not chain, mark condition and queue as on a rise |
| `BaseCommandBlock::performCommand` | `141f11390` | once a tick (`LastExecution`); off under `commandBlocksEnabled` (`SuccessCount` 0, false); empty command `SuccessCount` 0; runs, keeps `SuccessCount`, tracked output |

`tick`: impulse: condition met: run, then the chain; not met and conditional: `SuccessCount` 0. Repeating: marks its
condition first, as impulse, then queues itself again after `TickDelay` while powered or always active. Chain: its link
run from the tick, then the chain on from it.

`_executeChainBlock`: unpowered and needing power: passed over (chain goes on). Not from its own tick with a delay:
queued after `TickDelay`, chain ends. Condition not met: conditional `SuccessCount` 0, chain goes on. Else run; a run
refused (ran this tick, blocks off) ends the chain.

## Captured

- Entity of a block put where none was: `BlockEntityVersion` 0, `Command` "", `CustomName` "", `ExecuteOnFirstTick`
  (1 for repeating), `LPCommandMode` Int 0, `LPCondionalMode` 0 (sic), `LPRedstoneMode` 0, `LastExecution` Long 0,
  `LastOutput` "", `LastOutputParams` empty list (tag 0), `SuccessCount` 0, `TickDelay` 0, `TrackOutput` 1, `Version`
  52, `auto` (1 for chain), `conditionMet` 0, `conditionalMode` 0, `powered` 0. Shown with the block, and both again a
  tick later. A repeating block put over an impulse one kept its entity, shown once.
- Hand placing: creative operators only (survival refused); faces the placer (looking east: `facing_direction` 4).
  A creative click: BlockEntityData, then ContainerOpen `CommandBlock`, entity id -1, a command block in hand or
  not (it built nothing). A survival click: nothing.
- CommandBlockUpdate: answered by UpdateBlock and BlockEntityData; a command that does not compile keeps
  `commands.generic.syntax` with `["", word, ""]` as `LastOutput`. `FilteredCustomName` is added when named. Runs
  show nothing of the entity.
- Timing: a block of redstone put at tick T ran a delay-0 impulse block at T+2, delay 5 at T+6. Chains ran in the
  same tick as their head.
- Output: `/say` from a block: `[!§r] text` (named: `[Bob§r] text`). Successes go to operators as
  `§7§o[` + translate `!§r` + `: ` + the line + `]`, under `commandBlockOutput`; failures are silent. `say` and
  `daylock` counted `SuccessCount` 1 with no output line.
- Permission: `op`, `deop`, `allowlist`, `setmaxplayers`, `transfer`, `place` answered
  `commands.generic.error.permissions` `[name]`; `kick`, `list`, `tp`, `gamerule`, `time` ran.
- Rules: `maxCommandChainLength` 2 ran two links of three; -1 (the test server's world had it) ran none, so
  `cmdchains` and `cmdrules` set the three rules to their defaults first (recaptured 2026-10-28; that world's
  redstone clocks and sculk sensors add UpdateBlocks that are not the drill's).
  `commandBlocksEnabled` false ran nothing. `commandBlockOutput` false kept `say` but no echo.
- Only the chain block a block faces runs after it (bds-cmdlinks2.txt).

## Not copied

- The sender's name: sapling's `Sender` has no block variant, so `/say` from a block is `[Server]` (needs a new
  public `Sender` variant or `Executed` field).
- A repeating block made always active by its editor first ran ~8 ticks into a 10-tick delay on BDS; sapling waits 9.
- Minecarts with command blocks.
- `say hi @s` from a block: BDS said `[!§r] hi ` (bds-saysel.txt), sapling `[Server] hi `: the selector resolves
  as BDS's did, the name is the console's (above).
- `seed` and `version` are sapling commands, so a block runs them; BDS has neither (a syntax error).
