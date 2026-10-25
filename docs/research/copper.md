# Bedrock copper: oxidation, waxing, scraping, lightning, from BDS (bdsre)

Read 2026-10-25 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (PE stamp 6ab54bee) with bdsre
(`rtti|str|c|dis|win|callers|refs`) plus two throwaway scans (lea targets into a vtable range; `mov rax,[rax+0x80]`
vcalls). No `name` labels added. Layouts from LeviLamina `ChestBlockActor.h`. Every rule is [decompile] unless
marked otherwise; nothing here was drilled on the test server.

Notation: `cb` = the block type's `CopperBehavior` (embedded in the block type at +0x378/+0x380/+0x388/+0x3b0
depending on the base class): `cb+0` pointer to an 8-name table `[age0, age1, age2, age3, waxed age0..age3]`,
`cb+8` age (u8, 0 unaffected .. 3 oxidized), `cb+9` waxed (u8). `BlockType vt+0xa8` returns `cb` (null for
non-copper); `BlockType vt+0x80` is the lightning-hit hook. `swap(old, new)` = 148b38390 (rule S1).

## 1. Who is copper

| VA | What |
|---|---|
| 14713ab80 | `CopperBlock<BlockType>` ctor: builds `cb`, subscribes PlayerInteract → 14713a990, RandomTick → 14713a270 |
| 148b37fd0 | `CopperBehavior` ctor from the block's name (string compares; sets `cb+8`, `cb+9`) |
| 1487c20c0 | copper bulb registration: calls 14713ab80, then the bulb's own queued tick (148b39700) |
| 148b5c240 | `CopperBlock<LightningRodBlock>` ctor; 1487b5ab0 builds the final rod class (vtable 14a94f9b0, size 0x390) |

RTTI of the `BlockEventExecutor` lambdas: `CopperBlock<T>` exists for `BlockType` (full, cut, chiseled, grate,
bulb), `StairBlock`, `SlabBlock` (slab and double slab), `DoorBlock`, `TrapDoorBlock`, `ThinFenceBlock` (bars),
`ChainBlock`, `LanternBlock`, `LightningRodBlock`, `ChestBlock` (copper chest, plus `CopperChestBlock` handlers),
`ActorBlockBase<BlockType>` (golem statue, plus `CopperGolemStatueBlock` handlers). All of them, waxed ones
included, carry a `cb`; the random-tick handlers all call 148b38710 (callers 14713a270, 147139050, 1487d72c0,
1487d7440, chest 147138210).

## 2. Oxidation (random tick)

| VA | What |
|---|---|
| 14713a270 | RandomTick handler: `a = rand.nextFloat(); b = rand.nextFloat(); tryOxidize(region, pos, a, b, cb)` |
| 148b38710 | `tryOxidize` |
| 1408865c0 / 1408863d0 | Manhattan-ball iterator (filter / step), same walk as Java `BlockPos.withinManhattan` |
| 14a9455c0 | modifier table `{1.0, 0.75}` |

- O1 Both floats are drawn on every random tick of every copper block, waxed or oxidized too (two `nextFloat`
  calls before any check; first one is `a`).
- O2 Stop if `cb.waxed`. Stop if `a >= 0.06826667` (f32 0x3d8bce65; Java's is 64/1125 = 0.0568889, BDS is 1.2x
  that). Stop if `cb.age == 3`.
- O3 Neighbours: every position at Manhattan distance 1..4 from `pos` (centre excluded, 128 cells; iterator set up
  with sizes (4,4,4), max depth 12, filtered to distance <= 4). A neighbour counts only if its block type has a
  `cb` and is **not waxed**. For a counted neighbour of age `n` vs own age `m`: `n < m` → stop at once (no
  oxidation); `n == m` → `same += 1`; `n > m` → `higher += 1`. Iteration order does not matter for the result.
- O4 `f = (higher + 1) / (higher + 1 + same)`; chance `= f * f * (age == 0 ? 0.75 : 1.0)`; oxidize iff
  `b < chance`.
- O5 The new block is `names[age + 1]` (unwaxed next stage), looked up by name (1419be4c0), all states copied by
  name from the old block (S1).
- O6 Copper chest (147138210): only ticks when the chest block actor exists, is a chest (`isType(2)`) and its bit
  at +0x2dc is set (`mPairLead` by the header layout). On success the paired chest at `mLargeChestPairedPosition`
  (+0x308, if `mLargeChestPaired` +0x300 is non-null) is also set to `names[age + 1]` of the ticking chest.
- O7 Doors: each half random-ticks on its own; a success rewrites both halves (S1). Both halves count as neighbours.

- S1 `swap` 148b38390(region, oldBlock, pos, newType): new block = newType's default with every state of the old
  block copied over. If the old type has flag `+0x131 & 2` (doors) it finds both halves via `upper_block_bit`
  (148a24730), writes both with `setBlock(…, flags 2)` then calls 1415f8e60 on upper (0) and lower (1) (neighbour
  updates, inferred). Otherwise one `setBlock(pos, new, flags 3)` (1415f7140).

## 3. Waxing and scraping (player interact)

| VA | What |
|---|---|
| 14713a990 | PlayerInteract handler: `useCopper(player, pos, consume=true, cb)`, result → event `+0x20` |
| 148b38940 | `useCopper` |
| 147137d90 | copper chest handler; 148b39520 = "sneaking and holding an `is_axe` item or honeycomb" |
| 14ccc5f80 | item name `minecraft:honeycomb` (init at 14222fbe8) |
| 14cc29c28 | item tag `minecraft:is_axe` (init at 1402cf5c5) |

Server path only (client path just predicts the result):
- W1 Wax: if `!cb.waxed` and held item is `minecraft:honeycomb`: if `consume` and the player is not creative
  (game type 1, or 5 = default with world default 1) the held stack's count is set to count-1 and written back
  (`vt+0x260`); then `swap` to `names[4 + age]`; `region.postGameEvent(player, GameEvent@14cc8a9e4, pos)`;
  `LevelEvent 2030 (ParticleWaxOn)` at `pos + 0.5`, data 0; a player event hook (148b392d0, stats/achievement).
  Any age, oxidized included, can be waxed. Returns true.
- W2 Otherwise, if `cb.waxed` or `cb.age > 0`, and the held item has tag `minecraft:is_axe`: if `consume` and not
  creative, copy the stack, `hurtAndBreak(1, player)` (141a56c80, inferred name) and write it back. Then:
  waxed → `swap` to `names[age]` (same stage, unwaxed), `LevelEvent 2031 (ParticleWaxOff)` at `pos + 0.5`, data
  0, player event hook 148b392d0; unwaxed → `swap` to `names[age - 1]`, `LevelEvent 2032 (ParticleScrape)` at
  `pos + 0.5`, data 0. Both then post the same GameEvent. Returns true.
- W3 Anything else (unwaxed age 0 with an axe, honeycomb on waxed, other items) → false; nothing changes.
- W4 No sound is sent by the server: the `copper.wax.on`/`copper.wax.off`/`scrape` strings are only in the
  LevelSoundEvent name table (14346a2b0); the client plays them from the LevelEvent (inferred).
- W5 Order inside BDS: item cost, block swap, GameEvent, LevelEvent. (`tools.rs` sends the LevelEvent before
  UpdateBlock, from capture; consistent if UpdateBlock is flushed later in the tick.)
- W6 Doors: both halves change (S1). Trapdoors, stairs, slabs: one block, states kept.
- W7 Copper chest: only when the player is sneaking (actor flag 1, 142535e40, inferred) **and** holds an axe or
  honeycomb; otherwise the chest opens (148b25a10). On success, `useCopper(player, pairedPos, consume=false,
  this chest's cb)` runs on the paired chest: no second item cost, a second LevelEvent. That second call re-reads
  the hand, so a survival player waxing with their last honeycomb leaves the paired half unwaxed (edge).
- W8 Golem statue (1471397a0): with an empty / non-usable hand it rotates the statue pose (`+0x37 = (+0x37+1)&3`)
  and plays a sound (LevelSoundEvent 0x21a); waxing/scraping otherwise as W1/W2 (not fully read).

## 4. Lightning

| VA | What |
|---|---|
| 142f981b0 | `LightningBolt` tick: strike, copper clearing, fire, damage |
| 148b39420 | generic `CopperBlock` lightning hook (vt+0x80, thunks 14714c9c0 / 14714c9a0 / 1487d7360): one-step deoxidize |
| 148b5c410 | copper lightning rod hook (vt+0x80 of 14a94f9b0) |
| 148b5b4f0 | `LightningRodBlock` hit (powering), called by 148b5c410 |
| 1471383f0 | copper chest hook: 148b39420 on itself, then the same step on the paired chest |

- L1 Generic hook `deox(region, pos)`: if `!waxed && age > 0`: `swap` to `names[age - 1]`, `LevelEvent 2033
  (ParticleElectricSpark)` at the integer block corner `pos` (no +0.5), data 0. Else nothing.
- L2 Copper lightning rod hook: sets a level flag (`level+0x200 = 1`), if `!waxed && age > 0` swaps to
  `names[age - 1]` **without** an event, runs the rod's normal hit (power), clears the flag. Waxed rods still power.
- L3 On the bolt's strike tick (life counter `+0x3b4 == 2`), server side only: `p = floor(bolt position)`;
  `block(p).type.vt[0x80](region, p)` (the hit hook on the struck cell itself).
- L4 Start cell `s = p + (0,-1,0)`; if `block(p)` is a lightning rod (any of the rod type list at 14ccd8468),
  `s = p + offset(opposite(facing_direction))` instead (map `{1,0,3,2,5,4}` at 14a788c18, offsets 14a712760).
- L5 If `block(s)` has a `cb` and is not waxed: `w = nextInt(3)`; repeat `w + 3` times: call the hook on `s`
  (so `s` loses one stage per walk, up to 5); `k = nextInt(8)`; then up to `k + 1` steps from `cur = s`: try up to
  10 times `c = cur + (nextInt(3)-1, nextInt(3)-1, nextInt(3)-1)` (x, y, z in that order) until `block(c)` has a
  `cb` that is not waxed; none in 10 tries ends this walk; else call the hook on `c` (L1/L2; age 0 just passes)
  and `cur = c`. RNG is the level's random (`level vt+0xa68`, `nextInt` = vt+8).
- L6 Then `postGameEvent(region, bolt, GameEvent@14cc89898, s)` whether or not copper was found.
- Differences from Java: the start block is not reset to unaffected in one go but takes one step per walk (3..5
  steps, so it ends unaffected anyway); the struck cell itself also gets the hook; the walk's sparks come from L1.

## 5. Names

The 16 oxidizing families, each with 8 ids (`minecraft:` prefix omitted). Stage prefix `""`, `exposed_`,
`weathered_`, `oxidized_`; waxed adds `waxed_` in front of the stage prefix.

| family (stage 0 unwaxed) | pattern |
|---|---|
| `copper_block` | **exception**: stage 0 unwaxed is `copper_block`; others are `{stage}copper`, `waxed_copper`, `waxed_{stage}copper` |
| `cut_copper`, `cut_copper_stairs`, `cut_copper_slab`, `double_cut_copper_slab`, `chiseled_copper` | `[waxed_]{stage}<family>` |
| `copper_grate`, `copper_bulb`, `copper_door`, `copper_trapdoor`, `copper_chest`, `copper_golem_statue`, `copper_bars`, `copper_chain`, `copper_lantern` | `[waxed_]{stage}<family>` |
| `lightning_rod` | `[waxed_]{stage}lightning_rod` (no `copper` in the name) |

Copper ids that do not oxidize: `copper_ore`, `deepslate_copper_ore`, `raw_copper_block`, `copper_torch`.
The table was read from the strings (`str copper`, `str lightning_rod`); all 128 names are present.

## 6. Against `crates/sapling-shard/src/blocks/tools.rs`

- T1 Honeycomb waxing tests `bare.contains("copper")`: lightning rods are never waxed. Use the family table.
- T2 Doors: only the clicked half changes; BDS rewrites both (S1). Same for oxidation once implemented.
- T3 Copper chest: BDS needs sneaking, else opens the chest; the paired chest changes too (W7).
- T4 Events (2030/2031/2032 at `pos + 0.5`, data 0), creative not consuming honeycomb nor wearing the axe, and the
  scrape/unwax mapping match BDS.
- `research/captures/bds-copper.txt` shows no copper UpdateBlock in its window; with a 5x5 layer of fresh copper
  the per-tick chance is about `0.0683 * 0.75 / (same+1)^2`, so none is expected. It does not test O1-O7.

## Unverified

- U1 Whether a random tick reaches waxed / oxidized copper at all (irrelevant: O2 drops them, but two RNG floats
  are consumed if it does).
- U2 `+0x2dc & 1` on the chest actor is `mPairLead` (header layout guess); whether a single chest has it set.
- U3 141a56c80 is `hurtAndBreak` (Unbreaking applies?); 142535e40 tests the SNEAKING flag.
- U4 Whether the generic `BlockPlayerInteractEvent` reaches copper when the player sneaks with an item.
- U5 GameEvent ids 14cc8a9e4 / 14cc89898 not named (likely `block_change` / `lightning_strike`).
- U6 Golem statue interact (W8) and the statue's `ActorInternalEvent` (turning into a golem) not read.
- U7 The rod type list 14ccd8468 contents and the state id 14cd26368 = `facing_direction` are inferred.
- U8 None of this was drilled on BDS; first drill: oxidation rate of an isolated copper block at
  `randomtickspeed` high, and lightning on a copper rod.
