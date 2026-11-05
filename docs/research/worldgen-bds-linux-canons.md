# Canons made on the Linux BDS (2026-11-05, parity/thirteenth2aa)

Every canon before `dappled` was a Windows run. The Windows box is gone, so new canons come from the Linux
`bedrock_server`, patched and logged the same way (worldgen-bds-order.md, "A repeatable BDS").

## Making one
```
cp bedrock_server bedrock_server.canon
python3 -s crates/sapling-worldgen/scripts/bds_still.py bedrock_server.canon    # no chunk tick queues
python3 -s crates/sapling-worldgen/scripts/bds_seed0.py bedrock_server.canon    # thread generators seeded 0
WORLDGEN_BDS=<dir> WORLDGEN_EXE=bedrock_server.canon WORLDGEN_THREADS=1 WORLDGEN_PORT=<port> \
    crates/sapling-worldgen/scripts/bds_dbg_run.sh <seed> <out> crates/sapling-worldgen/scripts/data/dbg_canon_linux.json <region>
python3 -I crates/sapling-worldgen/scripts/dbg_loads.py <out>/dbg/dbg.log --order <out>/order.txt --structure-order <out>/structure-order.txt
tools/worldgen-oracle/canon-upload.sh <out> <canon> <world>     # then write env.txt: SAPLING_ORACLE_LINUX=1
```
`bds_dbg_run.sh` runs a non-`.exe` under gdb with `scripts/bds_gdb.py`; one run logs both the decoration and
the structure order. The region is picked with BDS's own `/locate biome` and the saved world's biome ids.

## Addresses (ELF; gdb loads the image at 0x555555554000)
| What | Address |
|---|---|
| `Random::Random()` entropy call (bds_seed0) | 0xed49cbb, target 0xedd9bb0 |
| `tickPendingTicks` / `LevelChunk::tick` calls (bds_still) | 0xc8e4770 / 0xc9e2af7 0xc9e2b20 0xc9e51c9 0xc9e5263 |
| `OverworldGeneratorMultinoise` vtable (Windows slot n is Linux slot n+1) | 0xee9d720 |
| `loadChunk` (LevelChunk in rsi, ChunkPos at +0x50) | 0x517ab30 |
| `decorationPostProcessChunk` (3x3 view corner at rsi+0x1a0, +0x1a8) | 0x517cd10 |
| `decorateWorldGenPostProcess` | 0x5181650 |
| `postProcessStructureFeatures` (chunk in r8d, r9d) | 0xc970f80 |
| `MultifaceFeature::place` face-shuffle draws (eax) | 0xd2ecbfd, 0xd2ece1c |
| MT19937 next (generator in rdi, index at +0x9cc) | 0xed4a030 |

To see every draw of one source chunk, gate the generator breakpoint on that chunk's decoration:
`{"at": "0x517cd10", "mem": [["rsi", 416, "i"], ["rsi", 424, "i"]], "gates": "draws", "when": [[cx-1], [cz-1]]}`
and `{"at": "0xed4a030", "label": "draws", "gated": true, "regs": ["rdi"], "mem": [["rdi", 2508, "i"]], "backtrace": 3}`.
A rule's placing generator is the `rdi` whose index restarts at 624 (and again every 624 draws); its callers
split it into tries. This found the fallen poplar that BDS laid over a red shrub.

## Where Linux generates differently
- Feature rule order within a pass: the rules sit in an `unordered_map`, MSVC's on Windows and libc++'s on
  Linux. libc++ grows its buckets 0, 2, 5, 11, 23... (primes; powers of two kept), inserts into an empty bucket
  at the list front, and on rehash moves a node to the front of its bucket's run unless it continues the
  previous node's bucket. `scripts/features_to_rust.py` (`LibcxxMap`) emits `LINUX_ORDER`; with it the dappled
  canon's ore and stone blob residue (3198 cells) went to 0.
- Mth's sine table: UCRT `sinf` on Windows, glibc's on Linux (acacia-physics `sinf.rs` holds the Linux table).
  Not met in worldgen yet.
- `std::hash<int>`: FNV-1a on Windows, identity on Linux (structure start maps, template markers). Not met yet.

The per-rule seed (boost `hash_combine` of identity values) is the same on both.

## The switch
sapling generates as Windows BDS does (worlds match Windows). `sapling_worldgen::platform::follow_linux_bds()`
switches to Linux behaviour; the oracle replay calls it when `SAPLING_ORACLE_LINUX` is set, which
`tools/replay-canon-env.sh` reads from the canon's `env.txt`. Only the rule order follows it so far.
