# Breakpoints on a live BDS (Windows)

When the decompile and the oracle disagree, ask the running server. `crates/sapling-worldgen/scripts/bds_dbg.py`
is a small Win32 debugger (Python ctypes, no install: the machine has no cdb, WinDbg or gdb) that starts
`bedrock_server.exe` with `DEBUG_ONLY_THIS_PROCESS`, plants int3 breakpoints at static addresses (the bdsre ones,
image base 0x140000000; ASLR is handled) and logs registers, memory, strings and call stacks per hit.
`scripts/bds_dbg_run.sh` wraps it in the worldgen-oracle run (same pack, own ports), so a chunk region is generated
under the debugger and the saved world is kept beside the log.

```
WORLDGEN_BDS=<your own BDS copy> crates/sapling-worldgen/scripts/bds_dbg_run.sh 20261003 <out dir> cfg.json 171,348,171,348
```

A one-chunk region takes about a minute. Use your own BDS copy and one run at a time per copy (a second run
rewrites `server.properties` and the pack under the first).

## Config
One JSON object, `{"breakpoints": [...]}`; the fields are listed in the script's docstring. What each is for:
- `filter` on a `BlockPos` argument keeps the log to one tree or one cell: `["r8", 8, [[x, z]]]` for features
  (`place(this, result, ctx)`: the position is at ctx+8, the random at ctx+0x18), `["rdx", 0, ...]` for
  `BlockSource::setBlock` (1415f6ae0: rdx position, r8 block). `{"box": [x0, z0, x1, z1]}` takes an area.
- `filter2` takes a `ChunkPos` (two ints); `filter_format: "fff"` reads the filter's three values as floats.
- `dump`, `dump_ptr` and `dump_on_return` write memory to files: a range at a register, the range a stored
  pointer names, and an output buffer once the call has returned.
- `strings: [["rax", 3]]` names a `Block*` (three pointer hops reach `minecraft:<name>`); air prints unrelated
  names (a wall or pumpkin string), so read it as "air" when the same pointer shows for obviously empty cells.
- `backtrace` is exact (dbghelp `StackWalk64` over the exe's unwind data). `stack` only scans stack words and
  shows stale return addresses too: it misled once, use `backtrace`.
- `trace: true` single-steps the hitting thread until the call returns and writes every instruction's address
  and rax. A refused mega tree is 10-16k steps (seconds). Read it by address prefix (`awk '{print substr($1,1,6)}'
  | uniq -c`) to see the callee sequence, then grep the branch targets of the function's `dis`.

## Recipes that answered questions
- Which check refuses a tree: trace `TreeFeature::place` (143b7ff60) at the trunk position; the last branch
  taken in the trunk's `placeTrunk` names the check, and counting the volume loop's passes names the cell.
- What block is in that cell at that moment: breakpoint on the instruction after the trunk's `getBlock`
  (mega trunk: 1438f2e89, position pointer in r15, cell at rbp+0x50) with `strings` on rax.
- Who wrote it: `setBlock` filtered on the cell with `backtrace`; the frames above name the feature
  (`single_block_feature` 143badfc0, `weighted_random_feature` 143bcfb10, `scatter_feature` 143b9b5d0, the rule
  loop 14580b883, `FeaturePoolElement` v2 place 145e144c0).
- Which chunk was being decorated: log every `TreeFeature::place` (`"log": "all"`, an empty filter list prints
  positions); a thread's tree positions give its source chunk, and the log order gives the order of chunks.
- Whether code runs at all during generation: a breakpoint with no filter and a hit count (the heightmap
  update 1409f3f50 had none).
- Comparing with sapling: `examples/source_writes` (a source chunk's writes), `examples/stages` (a column after
  each stage and per source), and the BDS log of the same feature's hits.

- What a Molang query was asked: break on the call inside its lambda and read the argument struct
  (`query.noise`: 1411a25f1, `"mem": [["rdx", 0, "ff"]]`); arguments that are not chunk corners showed the
  origin was stale.
- Where a walking feature stood: log a callee it runs per cell (`Block::mayPlace` 142cedc30, position in r8,
  face in r9) inside a box, beside the feature's own entry; the per-call end columns against
  `examples/feature_trace` name the call whose draws differ, and the raw draws name by how many.

- In what order chunks are processed: `PostprocessingManager::tryLock` (1438df8f0, chunk at `["r8", 0, "ii"]`,
  `backtrace` 3: caller 148213acc is the structure stage, 1482144e3 decoration), `postProcessStructureFeatures`
  (14196b2c0, chunk at `["rsp", 40, "i"], ["rsp", 48, "i"]`) and `decorateWorldGenPostProcess` (14821f400; its
  chunk is the thread's last decoration `tryLock`), all `"log": "all"`. A 5x5 region is about 75k lock tries.
- What a conversion replaces: a breakpoint inside the function, after the test, with the cell on the stack
  (`SculkVeinBehaviour` 148b4265b: old block in rax, position `["rbp", 44, "iii"]`, also usable as the filter).
- What a template placement is given: `placeInWorld` (141eeb8d0) with `["r9", 0, "fI"]` (integrity, seed) and the
  origin at `["r8", 0, "iii"]`.

- Whether a float computation is exact: dump BDS's buffers and compare them bit for bit, largest unit first.
  `_computeDensityCells` (14822d4b0: rdx the output grid of 5x5x41 floats, r8 the chunk, r9 the climate cache
  {x0, z0, width, entries at +0x10, 0x28 bytes each}, rcx+0x280/+0x2a8/+0x2d0 the blended noise's octave objects)
  with `filter2` on r8 and `dump_on_return`/`dump_ptr`; its three octave sums are on its stack once the fills
  are done (14822d84a: rsp+0x213c main, +0x1138 lower, +0x134 upper; the grid origin at rsp+0x64 is three
  floats, so `filter_format: "fff"`). `examples/density_dump_diff`, `octave_offsets` and `climate_dump_diff`
  read the dumps. The corner densities differed in 632 of 1025 corners, the three sums in all of them, the
  octave objects' offsets by a few f32 steps, and that was the bug (worldgen-bds.md, "Terrain port findings").
- A float inside a function: `xmm: [0, 6]` logs the registers' low floats with their bits at a breakpoint
  placed on the instruction after a call (the result) or before one (an argument).

`WORLDGEN_PORT` picks the server port (default 19290) so two sessions' copies can run at once. A run holds about
0.5 GB; on 2026-10-09 the harness killed one for low memory with the lock directory left behind.

## Limits
- Breakpoints on a `push` (most function entries) are emulated and never leave memory. Others are stepped over
  with the original byte restored for one instruction: another thread can slip through in that window.
- Every hit costs a round trip through Python: a filterless breakpoint on `setBlock` is millions of hits. Filter
  by position, or break on a rarer caller.
- In two of about a dozen runs an expected hit was absent (a tree's volume cells, a single-block try). Not
  established whether BDS's chunk order differed in that run or the step-over window lost the hits. Repeat the
  run before concluding anything from a missing hit.
- A breakpoint on the world-gen API's `setBlock` (1411a01f0) is as hot as `BlockSource::setBlock`: a 3x3 region
  had not reached decoration after ten minutes. Break on the feature's own call site instead.
- The slowed server can miss the oracle script's load deadline; the script retries the tile and BDS generates
  the chunks again in the same process. The log then holds two executions of a chunk (two thread ids running the
  same calls): compare them, they need not agree (worldgen-bds-residue.md, "Same-phase sources").
- Killing the server leaves the run script waiting for its deadline with the lock held: append a line with
  `Quit correctly` to the run's `worldgen.log` to let it finish.
- Linux BDS on testbox was not tried: its addresses differ from the Windows decompile's.
