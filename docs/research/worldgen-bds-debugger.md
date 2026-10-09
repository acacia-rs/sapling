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
  `BlockSource::setBlock` (1415f6ae0: rdx position, r8 block). `{"box": [x0, z0, x1, z1]}` takes an area,
  `{"boxes": [...]}` several (one run over many single-chunk regions).
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

- Is a feature in sync, call for call: log its entry and the entries of what it calls with one filter box around
  the chunk's 3x3 (`snap_to_surface` 143bb7720, pointed/cluster/large speleothems 149d62d10/149d61dc0/149e94510,
  `growPointedDripstone` 14829e980 with r8/r9 = facing/height at rdx, vegetation patch 143bc8310 and its accepted
  cell at 143bc8468 = `[rbp+0x68]` with the depth at `[rbp+0x84]`, growing plant 144c15200), run
  `examples/feature_trace` on the same chunk and give both to `scripts/dbg_calls.py`. The first differing call
  names the try; the number of raw draws between two calls (the scatter's next position decoded from the rule's
  stream) says how many draws BDS spent in between. A phase-0 chunk run as a one-chunk region sees bare terrain, as
  our model of it does: when every call matches there, what the oracle shows for that chunk comes from BDS's run
  (same-phase neighbours decorated before it, thread-local draws), not from the port.
- In which order chunks were decorated: one entry breakpoint with a box 100 blocks wider than the region, and
  `scripts/dbg_order.py`. Cherry grove's 4x4 region decorated 241 chunks, outer ones first, and all
  856 neighbour pairs kept the parity phases.
- Reading the run's world: `<out dir>/world` opens like an oracle world (`examples/bds_blocks`, `BDS_LIST=<name>`
  lists cells). A run over many regions under the debugger can miss the script's load deadline ("tiles failed"):
  the hits are complete, the saved world is not.

`WORLDGEN_PORT` picks the server port (default 19290) so two sessions' copies can run at once. A run holds about
0.5 GB; on 2026-10-09 the harness killed one for low memory with the lock directory left behind.

## Limits
- Breakpoints on a `push` (most function entries) are emulated and never leave memory. Others are stepped over
  with the original byte restored for one instruction while every other thread is suspended (before 2026-10-09
  they ran on and most hits of a mid-function breakpoint were lost: 10 of about 50 patch cells showed).
- Every hit costs a round trip through Python: a filterless breakpoint on `setBlock` is millions of hits. Filter
  by position, or break on a rarer caller.
- In two of about a dozen early runs an expected hit was absent (a tree's volume cells, a single-block try): most
  likely the step-over window above, since closed. A hit can still be missing because BDS's same-phase chunk order
  differs between runs.
- Linux BDS on testbox was not tried: its addresses differ from the Windows decompile's.
