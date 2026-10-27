# /execute on BDS

Captures: `bds-execute.txt`, `bds-execute-if.txt`, `bds-execute-run.txt`, `bds-execute2.txt`
(`bds_command_probe execute execute-if execute-run execute2`); syntax in `bds-command-syntax.txt`.
Sapling's answers match line for line where the worlds agree (BDS's drill world has its floor at
-20 and the bot sinks; sapling's at 64), checked with the same drills against `drill_server`.

- The whole line is read first, the command after `run` included: a wrong word there is a syntax
  error of the whole line (`execute run execute run banana` → `["ecute run ", "banana", ""]`), with
  no `commands.execute.failed`.
- Each frame runs the command as itself; the answers are passed on and, after every run that
  answered without success, `commands.execute.failed [command, executor]`. The executor is the
  entity's display name (`Pig`, `Armor Stand`). A run that answers nothing (`say`) adds nothing.
  A nested `/execute` that fails adds its own `failed` (`["execute", "CmdActor"]`).
- Success count: the sum of the runs' counts. Without `run`, one `trueCondition` per frame left
  (`trueConditionWithCount [n]` after `if blocks`).
- A frame failing a test answers `falseCondition [if|unless, block|blocks|entity|score]`.
  Errors before it: `outRangedDetectPosition [x, y, z]`, `ifUnlessBLocks.tooManyBlocks
  ["32768", product of the corner differences]`, `ifUnlessScore.targets.empty [typed]`,
  `objectiveNotFound [objective]`, `targets.tooMany [n]` + `commands.selector.error.moreThanOne`.
- `as`/`at` naming nothing: `noTargetMatch`. `align` with a bad word: `commands.execute.align.invalidInput`.
  `run op`: `commands.generic.error.permissions ["op"]` and `failed`.
- `anchored eyes` is 1.52 above the feet for a player, pig, cow, chicken and armour stand alike, and
  survives `positioned`, `positioned as` and `at`. `facing` turns from the feet even when anchored at
  the eyes; the `^` offsets then start at the eyes.
- Score ranges take `!`; `==` is a syntax error at the second `=`.
- `kill` counts every entity it kills (`kill @e[type=cow]` with two cows: success count 2).
- Not matched: `execute run function nosuch` answers `commands.function.functionNameNotFound` on BDS;
  sapling has no `/function` and answers a syntax error.
