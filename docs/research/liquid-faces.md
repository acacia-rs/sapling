# Faces water may pass through (BDS `liquidCanFlowIntoFromDirection`)

Read 2026-10-07 from `acacia/.testserver/bds-molang/bedrock_server.exe` (same build as `water.md`) with
`py -3 bdsre.py dis|c|vt|refs|win` plus a scratch pefile scan of block vtables. No `name` labels added.
Facing numbers are Bedrock's: 0 Down, 1 Up, 2 North (-z), 3 South (+z), 4 West (-x), 5 East (+x).

## VAs

| VA | Suggested label | What |
|---|---|---|
| 148b5ee50 | `LiquidBlock::_isLiquidBlocking` | `blocking(q, from, face)` in `water.md` |
| 14119ff40 | its `<lambda_1>` | `getBlock(p)` = `region.getBlock(p)` (vt+0x10, layer 0) |
| 14a8548e8 | face→bit table | `01 02 04 08 10 20` (`blocked_faces` bit = 1 << face) |
| 14a682118 | opposite-face table | `01 00 03 02 05 04` |
| 1400a2880 | `BlockType::liquidCanFlowIntoFromDirection` | default, folded `return true` (264 block vtables) |
| 146615720 | `StairBlock::liquidCanFlowIntoFromDirection` | vtables 14a8bc7b0 (StairBlock), 14a92cef0 (CopperBlock<StairBlock>) |
| 1466155b0 | `StairBlock::_isStairFacing?` (helper) | neighbour is a connecting stair with `weirdo_direction == dir` |
| 146612ef0 | `StairBlock::StairBlock` | stores `mBase` +0x378, `mLeakyCornersFix` +0x380, +0x381 |
| 1466031d0 | `SlabBlock::liquidCanFlowIntoFromDirection` | vtables 14a8b90e0, 14a92d370 (copper slab) |
| 141175090 | `SculkSensorBlock/HeavyCoreBlock::liquidCanFlowIntoFromDirection` | `return face != Down`; 3 vtables 14a766b60 14a7eb1c0 14a94c050 |
| 148b51f50 | `ShelfBlock::liquidCanFlowIntoFromDirection` (by elimination) | vtable 14a94da80 |
| 14cd25070 | state global `weirdo_direction` | static init 1439eb00f |
| 14cd26248 | state global, read as `upside_down_bit` | used only in the stair vertical branch |
| 14cd262a8 | state global `minecraft:vertical_half` | static init 1439e65e2 |
| 14cd25130 | state global `minecraft:cardinal_direction` | static init 1439e610e |

vt+0x88 is BlockType slot 17. LeviLamina `BlockType.h` puts `liquidCanFlowIntoFromDirection(uchar flowIntoFacing,
std::function<Block const&(BlockPos const&)> const& getBlock, BlockPos const& pos)` there; slot 25 (+0xC8) is
`isLavaBlocking`, matching `buckets-lava.md`. Only 5 functions occupy slot 17 across every block vtable found
(anchored on default slot 1 and slot 14, 272 vtables). The header's overriders are the same five classes:
StairBlock, SlabBlock, ShelfBlock, SculkSensorBlock, HeavyCoreBlock.

## `_isLiquidBlocking` (148b5ee50, disassembly)

Arguments: rcx = region, rdx = `pos` (target), r8 = `flowFromPos`, r9b = `face` (the target's face the liquid
enters through, see `SIDES` in `water.md`).

```
blocking(pos, from, face):
    t = region.getBlock(pos)                         # vt+0x10, layer 0
    if region.getLiquidBlock(from).material == lava: return t.type.isLavaBlocking()
    if t.liquid_reaction (Block+0xba) == Blocking: return true
    if t.blocked_faces (Block+0xb9) has bit face: return true
    if !t.type.liquidCanFlowIntoFromDirection(face, getBlock, pos): return true
    s = region.getBlock(from)                        # vt+0x10 on r8: layer 0 of the cell being left
    o = OPP[face]
    if s.blocked_faces has bit o: return true
    return !s.type.liquidCanFlowIntoFromDirection(o, getBlock, from)
```

So the face test is applied on both sides. For water in layer 1 of a waterlogged block, `s` is the container
itself (layer 0 at `from`), tested on the face the water leaves through. Lava ignores faces entirely.

## Per class

```
default (1400a2880):            true for every face

StairBlock (146615720):        # half = upside_down_bit, d = weirdo_direction
    if getBlock(pos).type != this: return false
    face Down: return half == top                  # bottom stairs: Down closed
    face Up:   return half == bottom               # upside-down stairs: Up closed
    back = [East, West, South, North][d]           # d: 0 east, 1 west, 2 south, 3 north
    front = OPP(back)
    if face == back:  return false
    if face == front: return true
    # side face: closed only for an inner corner (block in front fills that side)
    n = pos + offset(front)
    return !(connects(n) and n.weirdo_direction == weirdo_whose_back_is(face))
    connects(n) = mLeakyCornersFix ? n.type.flags(+0x130) bit0 : n.type == this.type
    # upside_down_bit of n is NOT compared

SlabBlock (1466031d0):
    if face is horizontal: return true
    if !type.isSlabBlock() or type.isDouble (+0x378): return true
    top = vertical_half != 0
    return top ? face != Up : face != Down

SculkSensor / CalibratedSculkSensor / HeavyCore (141175090):   return face != Down

Shelf (148b51f50):              return face != OPP[cardinal_direction]   # state holds a Facing 2..5; closes the back
```

Side-face corner table (stair, `d` → side face → front neighbour offset → neighbour `weirdo_direction` that closes it):
d0: South→(x-1)→2, North→(x-1)→3. d1: South→(x+1)→2, North→(x+1)→3. d2: West→(z-1)→1, East→(z-1)→0.
d3: West→(z+1)→1, East→(z+1)→0.

Every other class, including fences, walls, panes, trapdoors, doors, signs, chests and leaves, keeps the default:
all six faces open. `blocked_faces` (Block+0xb9) is 0 for every vanilla block (`waterlogging.md`), so those blocks
never stop water through a face.

## Consequences

- D4 (east-facing stairs, d=0): water leaving east has target face West, so `o` = East = back → blocked. North and
  south are side faces, open unless there is an inner corner. West (front) is open. This matches the BDS capture.
- Down: `blocking(p.down, p, Up)` tests `s` with `o` = Down. A waterlogged bottom slab or right-side-up stair keeps
  its water (it never falls). A top slab or upside-down stair lets it fall. Water entering from above tests the
  target's Up face, so a top slab or upside-down stair below refuses it.
- With Down closed, `blocking(b, p, Up)` is true, so a flowing (d != 0) waterlogged bottom slab/stair still spreads
  sideways (`water.md` line "flowing over liquid/hole: no sideways" does not trigger).

## Unverified

- Which stairs set `mLeakyCornersFix`: the ctor's arg varies by call site (148aa4de5 passes 1, 14879b11d a computed
  value; probably version/experiment gated). The `+0x130 bit0` flag is assumed to mean "is a stair".
- 14cd26248 = `upside_down_bit`: inferred from use (it is the only boolean the vertical branch reads); its static
  initialiser was not located.
- 148b51f50 = ShelfBlock: by elimination (only remaining header overrider, reads `cardinal_direction`).
- Vtables that override both anchor slots (1 and 14) would be missed by the scan.
- `tick`'s inflow loop (`water.md` line 75) checks only the neighbour's face, not the waterlogged cell's own face,
  per `water.md`; not re-read here.
