# Bedrock liquid flow, from BDS (bdsre) with a Java 26.3 cross-check

Read 2026-10-05 from `acacia/.testserver/bds/bedrock_server.exe` with `py -3 bdsre.py c <va>` (pseudo-C) and
`dis`/`win`. Functions were found through the RTTI of their lambdas (`find "LiquidBlock"`), names from the
LeviLamina headers. No `name` labels were added (bdsre is outside the allowed write area); suggested labels below.
Bedrock is the pre-1.13 Java algorithm (`BlockDynamicLiquid.updateTick`), not modern `FlowingFluid`.

| VA | Suggested label | What |
|---|---|---|
| 148b5c4c0 | `LiquidBlock::LiquidBlock` | subscribes onPlaceBase, tick, (lava) randomTick |
| 148b5c980 | `LiquidBlock::tick` | `_tick(region, pos, random, false)` |
| 148b5d060 | `LiquidBlock::_tick` | the whole flow rule |
| 148b5cd30 | `LiquidBlock::neighborChanged` | solidify + queue a tick |
| 148b5ed00 | `LiquidBlockBase::getDepth` | -1 if not this material |
| 148b5ee50 | `LiquidBlock::_isLiquidBlocking` | |
| 148b5f020 | `LiquidBlock::_getSlopeDistance` | |
| 148b5e580 | `LiquidBlock::_trySpreadTo` | |
| 148b5f900 / 148b5cf60 | `LiquidBlockBase::onPlaceBase` / `::neighborChanged` | 5x solidify |
| 148b609f0 / 148b60f90 | `LiquidBlockBase::solidify` / `::_solidify` | lava + water contact |
| 148b61310, 148b617c0, 148b61d20 | `LiquidBlockDynamic` ctor, `::onPlace`, `::tick` | |
| 148b5cd10 | `LiquidBlock::randomTick` | lava sets fire (not covered) |
| 142ed68c0 | `Block::isLiquidSource?` | has `liquid_depth` and it is 0 |

## Constants (verified on BDS)

| | water | lava | evidence |
|---|---|---|---|
| tick delay | 5 | 30; 10 if `Dimension+0x1a6` | 148b5d060, 148b5e580, 148b5cd30 (same inline code) |
| drop-off per block | 1 | 2; 1 if `Dimension+0x1a6` | 148b5d060 `2 - dim[0x1a6]` |
| slope search limit | 4 | 2 (no dimension test) | 148b5f020 `pass < (mat != 6)*2 + 2` |
| infinite source | yes | no | 148b5d060, guarded by material == 5 |
| recede slowdown | none | 3 in 4 chance of delay x4 when old<8, new<8, old<new | 148b5d060 |

Material type byte 5 = Water, 6 = Lava, 0x12 = Bubble; `Material+5` = `mSolid` (LeviLamina `MaterialType.h`,
`Material.h`). `Dimension+0x1a6` = `mUltraWarm` (nether) is **inferred** from the header field order.
Block name globals resolved with `win` on their initialisers: 14cce16a8 obsidian, 14cce3a00 cobblestone,
14cce38b0 stone, 14ccdafe8 `minecraft:lava`, 14cce2b78 blue_ice, 14cce4240 soul_soil, 14cce0b98 basalt.

## Block identity and `liquid_depth` (verified)

- `LiquidBlock` is `minecraft:water` / `minecraft:lava`; it does all the flowing and every cell it writes is its own
  type (`this+0x240` default state with `liquid_depth` set). Evidence: `_tick` tests the ticking block's name against
  `minecraft:lava`.
- `LiquidBlockDynamic` (`flowing_water` / `flowing_lava`) is transient: `onPlace` and `tick` replace it by the
  `LiquidBlock` with the same states (flags 2); `onPlace` also queues a tick. `water` placed directly queues nothing
  (so `setblock water` is still until a neighbour changes).
- So: always write `minecraft:water[liquid_depth=n]`. 0 = source, 1..7 = flowing, 8..15 = falling.
  Falling = `8 + (depth of the cell above, if < 8)`, or the above cell's value unchanged if it is already >= 8;
  so 8 is "under a source or under falling-from-source", 9..15 keep the level of the stream that fell.
  A neighbour with depth >= 8 counts as depth 0 for feeding but not as a source.

## Pseudo-code (verified on BDS unless marked)

```
liq(p)      = layer-1 block if it is a liquid, else layer-0 block       # BlockSource::getLiquidBlock
depth(p)    = liq(p) is my material ? liquid_depth : -1                # 148b5ed00
is_src(p)   = liq(p) is my material && liquid_depth == 0
SIDES       = [(-x, face 5), (+x, face 4), (-z, face 3), (+z, face 2)]  # face = face of the target cell entered
OPP(face)   = opposite face

blocking(q, from, face):                                               # 148b5ee50
    b = block0(q)
    if liq(from) is lava: return b.type.vcall_0xC8()                   # not decoded
    if b.liquid_reaction == Blocking(2): return true
    if b.blocked_faces has face or !b.canFlowInto(face): return true   # Block+0xb9 mask, BlockType vt+0x88
    s = block0(from)                                                   # the cell the liquid leaves
    return s.blocked_faces has OPP(face) or !s.canFlowInto(OPP(face))

tick(p):                                                               # 148b5d060, onlyDetectStatic = false
    d = depth(p); delay = DELAY; changed = false
    if d >= 1:
        hi = NONE; sources = 0
        for (dir, face) in SIDES:                                      # only "can enter n through face" is tested
            n = p + dir
            if block0(n).blocked_faces has face or !block0(n).canFlowInto(face): continue
            nd = depth(n); if nd < 0: continue
            if nd == 0: sources += 1
            hi = nd >= 8 ? 0 : min(hi, nd)                             # NONE acts as +infinity
        new = (hi == NONE || hi + DROP > 7) ? -1 : hi + DROP
        a = depth(p.up)
        if a >= 0: new = a >= 8 ? a : a + 8
        if WATER and (sources >= 2 or (sources == 1 and a == 0)):      # after the falling rule: can override it
            if material(block0(p.down)).solid or is_src(p.down): new = 0
        if LAVA and d < 8 and 0 <= new < 8 and d < new and rand(4) != 0: delay *= 4
        if new != d:
            changed = true
            layer = block0(p) is this liquid ? 0 : 1                   # waterlogged liquid lives in layer 1
            if new < 0: set(layer, p, air, flags 3)
            else:       set(layer, p, me[new], flags 3); update_neighbours(p); schedule(p, delay)
            d = new
    if !changed:                                                       # also every tick of a source
        if liq(p) is minecraft:lava with depth 0 and liq(p.up) is water: set(p, obsidian, flags 2); return
    if d < 0: return                                                   # inferred guard, see Unverified
    b = p.down
    if b.y >= min_y and has_block(b) and liq(b) is neither my material nor lava and !blocking(b, p, UP):
        if LAVA and (liq(b) is water or material(b) == Bubble): set(b, stone, flags 3); fizz(b); return
        spread_to(b, d >= 8 ? d : d + 8, p, UP); return                # falling: never also sideways
    if d != 0 and !blocking(b, p, UP): return                          # flowing over liquid/hole: no sideways
    for i, (dir, face) in SIDES:
        n = p + dir; cost[i] = 1000
        if blocking(n, p, face) or is_src(n): continue
        if !blocking(n.down, p, UP): cost[i] = 0                       # hole right beside (water below counts)
        else: own = liquid_depth of liq(p) now
              cost[i] = slope(n, own in 5..=7 ? own - 3 : 1, i)
    nd = d >= 8 ? 1 : d + DROP
    if nd > 7: return
    for i in 0..4 where cost[i] == min(cost): spread_to(p + dir_i, nd, p, face_i)   # all ties, order -x,+x,-z,+z

slope(q, pass, from):                                                  # 148b5f020; from = index of the step taken
    best = 1000
    for i, (dir, face) in SIDES, skipping the step straight back (i == from ^ 1):
        n = q + dir
        if blocking(n, q, face) or is_src(n): continue
        if !blocking(n.down, q, UP): return pass
        if pass < SLOPE_LIMIT: best = min(best, slope(n, pass + 1, i))
    return best

spread_to(n, nd, from, face):                                          # 148b5e580
    if n.y < min_y or !has_block(n): return
    if liq(n) is my material or lava: return                           # never overwrites same liquid, any depth
    if blocking(n, from, face): return
    b0 = block0(n); layer = 0
    if b0 is not air (and not world-gen instatick):
        if LAVA: fizz(n)
        elif b0.liquid_reaction == Popped(1): drop_loot(b0)            # fcn.142cd6f20, chance 1.0
        elif b0.liquid_reaction == Noreaction(3): layer = 1            # waterloggable: block stays, water in layer 1
        # Broken(0): replaced, no drop
        if block1(n) is not air and block1(n).liquid_reaction == Popped: drop_loot(block1(n))
    set(layer, n, me[nd], flags 3); schedule(n, DELAY)

neighbor_changed(p):                                                   # 148b5cd30 (water and lava)
    for n in [+z, -z, +x, -x, +y]: solidify(p, n)
    if liq(p).type is me and no tick is queued for p at exactly now + DELAY: schedule(p, DELAY)

solidify(p, n):                                                        # 148b60f90; lava only, n never below p
    if block0(p) is not this lava or n.y < p.y: return
    if liq(n) is water: set(p, depth(p) == 0 ? obsidian : cobblestone, flags 3); fizz(p)
    elif block0(n) is blue_ice and block0(p.down) is soul_soil: set(p, basalt, flags 3); fizz(p)
```

Search reach: with start pass 1 and limit 4, a hole is seen up to 5 cells from `p`; a thin stream (depth 5/6/7)
starts at pass 2/3/4 and sees 4/3/2 cells. All four costs 1000 means "spread every open way".
Recede: a cut-off cell becomes `min(side depths) + 1` on its own tick, so a stream's numbers climb by one per
5 ticks per cell and vanish past 7; a cell with no liquid beside or above goes to air at once.
Ticks: only (a) `spread_to`, (b) a depth change, (c) `neighbor_changed`, (d) placing `flowing_*` queue one.

## Answers

1. Delay 5 / 30 / 10 (table); scheduled by the four events above; unchanged cells stop ticking.
2. See "Block identity". Always `minecraft:water`; `flowing_water` exists only until its first event.
3. `new = min side depth + drop`, falling `above | 8`; infinite source: water only, 2 side sources, or 1 side
   source plus a source directly above, over solid material or a water source. Cut-off: recompute, not vanish.
4. Down first and exclusively; sideways if source or the cell below blocks; cheapest-drop search as above.
   What gives way is per block data (`liquid_reaction`, `Block+0xba`; `LiquidReaction.h`: Broken 0, Popped 1,
   Blocking 2, Noreaction 3): Popped drops its loot, Broken does not, Noreaction gets waterlogged.
5. Lava source with water beside/above: obsidian; flowing lava (any depth) with water beside/above: cobblestone
   (no "depth <= 4" test); lava falling onto water or a bubble column: stone on the lower cell; lava source that
   stays unchanged with water above: obsidian. Water never flows into a lava cell (`spread_to` returns); the lava
   reacts through `neighbor_changed`. Lava may flow sideways into a water cell and then solidifies on place.
6. Layer-1 water is ordinary water to every rule (`getLiquidBlock`): it feeds, counts as a source and ticks; its
   writes go to layer 1. It leaves only through faces its container allows (`blocked_faces`, `canFlowInto`).

## Java 26.3 differences (FlowingFluid / WaterFluid / LavaFluid, Vineflower from client-26.3.jar)

- Same: delays 5 / 30 / 10, drop-off 1 / 2 / 1, lava x4 recede (`LavaFluid.getSpreadDelay`), stone under falling
  lava (`LavaFluid.spreadTo`). Different, **Bedrock wins**:
- Java lava slope distance is 4 in the nether (`getSlopeFindDistance`); BDS is always 2.
- Java infinite source needs 2 side sources (`getNewLiquid`); BDS also accepts 1 side + source above.
- Java falling is a flag with amount 8; BDS keeps the stream level in 8..15.
- Java also spreads sideways while falling when 3+ side sources (`spread`); BDS never does.
- Java `spreadTo` drops every washed-away block (`beforeDestroyingBlock`); BDS only `Popped` ones.
- Java search starts at pass 1 always; BDS shortens it for depth 5..7.

## Unverified / inferred

- Which vanilla blocks are Broken / Popped / Blocking / Noreaction and their `blocked_faces`: per-block data, not
  dumped. Java only: non-solid replaceable blocks wash away with drops; doors, signs, ladders, sugar cane,
  portals, structure void and anything motion-blocking stop flow (`FlowingFluid.canHoldAnyFluid`).
- `blocking()` when the flowing liquid is lava (`BlockType` vt+0xC8) was not decompiled.
- `d < 0` guard after a cell dries: BDS falls through to the spread code; its signed/unsigned compares there were
  not disassembled. In practice the cell below already holds liquid. Keep the guard.
- BlockSource vt+0x1d8 read as `isInstaticking` (inferred from use); side faces in `tick`'s cost loop are taken
  from `slope` (the switch table was not recovered). `air` global 14ccf9950 inferred from use.
- Lava fire spread (`randomTick`), fizz sound/particles, bucket placement: not covered.
