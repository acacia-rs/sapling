# Lush-cave, temple and old-growth residue (2026-10-10, worldgen/lush2)

Continues [worldgen-bds-residue.md](worldgen-bds-residue.md); corrects its "Same-phase sources and the jungle
temple" (the pair), its "Left" list (the snow layers) and worldgen-bds-caves.md's "no port difference is known".

All2 (395e611) -> lush2, wrong / missing / deterministic per world (`tools/oracle.sh`, logs `base` and `lv2`):

| world | wrong | missing | deterministic |
|---|---|---|---|
| near origin | 2018 -> 2347 | 313 -> 301 | 2024 -> 2039 |
| far out | 259 -> 281 | 30 -> 28 | 260 -> 260 |
| negative seed | 98 -> 126 | 12 -> 10 | 78 -> 78 |
| biome regions | 5620 -> 5588 | 1134 -> 1100 | 1817 -> 1660 |
| tree regions | 3385 -> 2978 | 928 -> 813 | 3018 -> 2577 |
| structures | 17873 -> 17208 | 5236 -> 4885 | 10793 -> 10057 |

Wrong + missing over all 59 regions 36906 -> 35665. By region: mansion 4373 -> 2907, old growth spruce taiga
571 -> 69, meadow 413 -> 13, warm ocean 71 -> 26, dark forest 207 -> 164, old growth pine taiga 619 -> 587;
worse: near origin +317, bamboo jungle 319 -> 577, ancient city +104, pale garden 671 -> 744, stronghold +66,
trail ruins +57, windswept hills 17 -> 62, and ten to thirty cells in most other regions.

### Glow lichen always spreads (the one change to the generator)
A lush patch whose every moss block matched while every cave vine hung one column further on (mansion chunk
-508,998, first ceiling patch; three BDS runs alike) had one column fewer in BDS: a glow lichen sat under its
ceiling. That lichen is the spread of the multiface feature. BDS spreads whenever the gate passes and any
direction can take it and picks the direction with its thread-local generator; sapling spread only when one
direction could, which is almost never (`examples/lichen_spreads` near the origin: 288 of 292 spreads have four
candidates; BDS's pick falls on them 69, 55, 68, 66 times, so uniformly; in 41 neither run has any of them). So
BDS had one lichen more per spread, and a patch around it drew one depth fewer: every later vegetation draw of
the patch moved by one cell. Now the first candidate takes it. The lichen is on BDS's cell one time in four
(each miss is one more wrong cell, which is the rise near the origin: its deterministic column did not move),
but the count is right and a patch is off only between our cell and BDS's.
How it was found: `scripts/patch_cells.py` (our `cell` trace lines of one try against BDS's moss per column,
the moss listed by `BDS_LIST=moss_block examples/bds_blocks`): depths agreed up to the 90th cell, where one
run has a lichen and the twin has it two cells away. Chunk -508,998 went from 503 wrong cells to 47.
The vein feature keeps the old rule (struct4's negative result; sculk is another round's).

### Same-phase order: an experiment switch, and the jungle temple's pair
`SAPLING_SAME_PHASE_ORDER="x,z:x,z:..."` (feature/order.rs) decorates the listed sources in that order, each
over the writes of the listed same-phase sources two chunks away that come before it. A diagnostic: the order
is BDS's load pattern, not the seed's.
- Jungle temple with `78,8:78,6`: 3104 -> 389 wrong + missing (412 with the lichen change). The other seven
  neighbours of 78,6 change nothing, 80,6 included: "80,6 before 78,6" in the section below was the order of
  that debugger run and is not what the oracle's cells need. What happens: ring source 78,6 writes a vine at
  (1271,72,107); 79,6's second tree stands there and BDS grows it. With 78,8's leaves in chunk 78,7 the walker
  of 78,6 draws otherwise and the vine is elsewhere. Everything else in 79,6, 79,7, 78,5 and 79,5 follows that
  tree. BDS's two oracle runs agree on it (47 unstable cells in the region), so the order is stable for a load
  pattern; a debugger-slowed run has another.
- Vines do block trees: letting `prepareSpawn` pass vines took jungle from 1 to 27384. Its volume test is the
  descriptor list alone (1438eac70 loops `BlockDescriptor::matches` over +0x68), no material test.
- What is left of the temple with the pair set: dripstone 210, vines 89 (chunk 79,7), bubble columns 26.

### What each region's residue is (wrong + missing after the change)
- Mansion 2907: lush caves over 30 chunks (cave vines 720, moss 715, clay 351), 218 liquid cells. Not traced
  further: each patch near a lichen of ours that is not on BDS's cell is still off between the two.
- Old growth birch forest 1075, never traced before: one chunk, -309,285 (439 cells), and the water that runs
  out of it (260 liquid cells down to y -34 in -310,282 and -310,283). Source -309,285's first clay pool
  (try 17 at -4944,0,4568) has the clay right and two dripleaves elsewhere, the second pool is then another
  size, and a moss patch of ours lies along a mineshaft corridor (y 6, x -4940) where BDS has rails in other
  cells than ours and no moss. The four phase-0 sources around it match BDS write for write. The pool's cells
  need four sides and a floor that support, and the corridor's planks, fences and rails are the mineshaft's
  draw drift (worldgen-bds-structures.md): structure order, not lush code.
- Old growth pine taiga 587: chunk -334,339 (412, clay) and -334,338, both on the region's west edge.
- Old growth spruce taiga 69: was one moss floor patch (ground right, vegetation shifted) and its lava; the
  lichen.
- Savanna plateau 474: sculk 233 (another round's), dripstone 148, coal 46 (follows the dripstone).
- Bamboo jungle 577: a moss ceiling in 172,346 now shifts where our lichen is not on BDS's cell.
- Igloo, snowy plains: the "117 snow layers over short grass" of the struct4 list were fixed by 0357d11
  (feat5) before this round; the region's 336 are a mineshaft (129) and dripstone (187).

### The thread-local generator is seeded from the system's random source
The generator at `gs:[0x58]` +0xe60 wraps a `Random` at +0x460 that is default-constructed on a thread's first
use (143b84667 -> 1411889d0). That constructor asks a platform object for four random bytes (141ccbe30) and
seeds the MT with them. Live, a breakpoint on the store (141188a43, ecx) logged each worker thread's seed in
two runs of one region: 9bc23ad8, 5f4a8a61, 44478521, ... and 126ff94f, e82c568a, a481c7b3, ... Nothing of it
comes from the world seed, the thread or the chunk: lichen faces, cells and spread directions cannot be
reproduced, whatever the chunk order per thread.

