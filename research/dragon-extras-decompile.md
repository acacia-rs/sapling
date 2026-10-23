# BDS End gateway spawn search, end island, dragon fireball and breath (bdsre reading)

Build 6ab54bee (BDS 1.26.52.3, `.testserver/bds-molang`). Static reading only, no drill yet. Same house style as
dragon-fight-decompile.md (fight manager, "fight doc") and dragon-ai-decompile.md ("AI doc"). "Unverified" marks a
reading with no second source. Offsets checked against `dis` where it mattered (findValidSpawn and its call sites
were read from `dis` only).

Block ids as in the fight doc: 14ccd9718 bedrock, 14cce12e8 end_stone, 14ccf9950 air. **14ccd5810 is
`minecraft:end_gateway`** (its hash is stored at 14260083c in the VanillaBlockTypeIds initialiser 1425f41e0, right
before the `'minecraft:end_gateway'` name).

Two block predicates recur:
- `fullSolid(b)`: material `+1 == 0` (mNeverBuildable) and material `+3 & 1` (mBlocksMotion), **and**
  `BlockType+0x130 & 0x40000` (the solid-blocking property; see worldgen-bds-caves.md, and pile.rs in
  sapling-worldgen for the same test). In `scripts/data/block_types.tsv` chorus_flower, chorus_plant, end_stone,
  bedrock and obsidian all have 0x40000 with blocksMotion 1. end_rod has 0x80000 only, so it fails.
- `liquid(b)`: material `+2` (mLiquid).

## 1. `findValidSpawn(out, region, pos, endStoneOnly, radius, surfaceFallback)` 1487fde00

### Call sites
| site | call | args |
|---|---|---|
| 148276f8c (gateway job, after both regions load) | far island spawn | `(exitBS, exit, endStoneOnly=1, radius=1, fallback=0)` |
| 148277190 | main gateway's exit | `(exitBS, farGatewayPos, 0, 1, 0)` |
| 1482772d3 | far gateway's exit | `(mainBS, mainGatewayPos, 0, 3, 0)` |
| 14827523c / 148275288 (relink 148274e90) | re-find an exit | `(r, p, 1, 1, 0)`; none: island at p (no y=75), then `(r, p, 1, 1, 1)` |

`exitBS` (rbp+0xb40) and `mainBS` (rbp+0x9c8) are BlockSources (1415f33f0) built from the job's two chunk-view
sources at fight+0xb0 (exit) and +0x90 (main). The fight doc's "`findValidSpawn(mainSource, main pos, 0, 3)`" is
the third row: the `3` is the radius.

### Helper `topSolidUnder(pos)` 141603330 (y in/out)
1. Chunk at pos (`region vt 0x140`); none: false.
2. `y = min(y, chunkTop)` where chunkTop (1409f7640) = dimension min + 16 × (index of the highest sub-chunk that
   is not all air, + 1).
3. Walk down: read block at y, then y−1; stop once the block read was **air** (y is now one below that air) or the
   read y was ≤ the dimension min (`region+0x3a`): then false.
4. Walk down from there: the first y whose **material** is solid-blocking (`+1 == 0 && +3 & 1`, no 0x40000 test
   here) returns true with pos.y = that y. Reaching the minimum: false.

So it is "the first solid-blocking block below the first air at or under y". The caller starts at y = 127.

### Scan
- R = radius × 16. Columns x from `pos.x−R` to `pos.x+R` (outer), z from `pos.z−R` to `pos.z+R` (inner), both
  inclusive, ascending (1487fde6e..ded3, loop tail 1487fe4fe). Radius 1 is 33×33 columns, radius 3 is 97×97.
- Per column, `p = (x, 127, z)`; `topSolidUnder(p)` false: skip. B = block at p.
  1. **B is bedrock and the block at p.y−2 is end_gateway** (a gateway's top cap, see below): p.y −= 2 and
     `topSolidUnder(p)` again (it passes through the gateway and its lower bedrock, then the air under it, to the
     surface beneath); false: skip. Then skip if `fullSolid(p+1)` or `fullSolid(p+2)`. Go to the clear test.
  2. Otherwise (including bedrock without a gateway below), when **endStoneOnly**: B must be end_stone, else skip;
     go straight to the clear test (no p+1/p+2 test on this path).
  3. Otherwise (endStoneOnly = 0): bedrock: skip. Skip if `fullSolid(p+1)` or `fullSolid(p+2)`. The column is a
     candidate but **never clear** (jumps to scoring with the not-clear flag set, 1487fe442).
  - Clear test (1487fe0d1): `liquid(p+1)`: not clear. Else the 8 neighbours at p.y, in the order (+1,0), (−1,0),
    (0,−1), (0,+1), (+1,−1), (+1,+1), (−1,−1), (−1,+1) (dx,dz): any of them **air**: not clear. All 8 non-air:
    clear.
- Score (1487fe45f), f32: `d = ((y+.5)−(py+.5))² + ((x+.5)−(px+.5))²` then `+ ((z+.5)−(pz+.5))²`, where each
  term is `float(int)+0.5` minus the same for the search centre `pos` (y included, so it is 3D).
  - `any`: replaced when none yet or `d < best` (strict).
  - `clear`: replaced when the column is clear and (none yet or `d < best`) (strict).
  - Ties keep the first column in scan order (lowest x, then lowest z).
- Result: `clear` if any; else, fallback = 0: `any`, or **(0,0,0)** if none. Fallback = 1: `any`; if none, from
  `pos` (y clamped up to the dimension min) go up to the first `fullSolid` block, then up to the first air; result =
  the cell under that air. Without a loaded chunk, or past the dimension's max height (`dimension+0xca`), the
  result is `pos` itself.

Why bedrock-then-gateway: `EndGatewayFeature` writes the Java pattern (gateway at G, bedrock at G±2 centre and the
plus shape at G±1), so a column through a gateway tops out at bedrock G+2 with the gateway at G. The plus-shape
columns top out at bedrock G+1 with bedrock at G−1, so they fail case 1 and are skipped by cases 2 and 3.

### What the job does with it (148276f0d..1482772fd)
1. `s = findValidSpawn(exitBS, exit, 1, 1, 0)`. **s == (0,0,0)**: `s = (exit.x, 75, exit.z)` and the end island at s
   (section 3).
2. `far = findTallestBlock(exitBS, s, 16, allowBedrock = 1)`, then `far.y += 10`. EndGatewayFeature at the main
   pos (if +0x88) with mainBS, and at `far` (if +0xa8) with exitBS (144c120f0).
3. Main gateway block actor (type 0x18 in mainBS's chunk): `e = findValidSpawn(exitBS, far, 0, 1, 0)`, then
   `t = findTallestBlock(mainBS, e, 5, allowBedrock = 0)`; exit = `t + (0,1,0)`. Note the **mainBS** here: it
   reads blocks near the far gateway through the source built round the main gateway. Whether those chunks read
   as loaded through it is **unverified** (if they read as air, `t` falls back to `e`).
4. Far gateway block actor (type 0x18 in exitBS's chunk): exit = `findValidSpawn(mainBS, mainPos, 0, 3, 0)`
   itself, no +1. Columns over the void return nothing, and the main gateway's own columns are skipped, so this is
   the nearest (3D) top of the main island within ±48 of the main gateway. (0,0,0) if there is none.
5. Both get `+0x106 = 0, +0x107 = 1` (word 0x100) and changed flag `+0x4c |= 1`. State 2.

## 2. `findTallestBlock(out, region, pos, radius, allowBedrock)` 1487fdbb0
- Best starts at (0,0,0). dx = −r..r (outer, x), dz = −r..r (inner, z).
- allowBedrock = 0: the centre column (dx = dz = 0) is **skipped** and bedrock never counts. allowBedrock = 1: the
  centre is included and bedrock counts (Java's rule).
- Per column: y from `region vt 0x120` (max height) − 1 down, **while y > best.y** (best.y read before the column);
  the first block with `fullSolid` (and, if bedrock is not allowed, not bedrock) becomes the best. So a later
  column wins only when strictly higher; ties keep the first column in scan order. Nothing above y 0 counts.
- Result: best, or `pos` when best is still (0,0,0).
- Counted: any `fullSolid` block, so **chorus_flower and chorus_plant count** (0x40000, blocksMotion; chorus is in
  the solid-blocking property set, worldgen-bds-order.md), as do end_stone, obsidian, bedrock (when allowed).
  Not counted: end_rod, air, liquids, non-blocking blocks. The tsv does not record mNeverBuildable; assumed 0 for
  chorus (unverified). This matches the evidence that BDS puts the far gateway 10 over a chorus flower.

## 3. End island when no spawn is found: 148274c80
- Feature: `EndIslandFeature` (vtable 14a9252b0, buffer +0x18 null so it writes straight to the region),
  place = 144c12ac0.
- Random: a fresh **MT19937** (Bedrock `Random`, vtable 14a73ebe0) seeded with a hash of the BlockPos
  `(exit.x, 75, exit.z)`:
  - `f(v)` = 64-bit FNV-1a over the 4 little-endian bytes of the int (offset 0xcbf29ce484222325, prime
    0x100000001b3).
  - `h = f(x) + 0x9e3779b9`; `h = (f(y) + 0x9e3779b9 + (h<<6) + (h>>2)) ^ h`; `h = (f(z) + 0x9e3779b9 + (h<<6) +
    (h>>2)) ^ h` (boost hash_combine, u64).
  - seed = low 32 bits of h. Standard MT init (`mt[i] = 0x6c078965*(mt[i−1]^(mt[i−1]>>30)) + i`; the init is lazy,
    first 398 words, output unchanged).
- Shape (Java's EndIslandFeature):
  - `f = nextInt(3) + 4.0` (f32), dy = 0.
  - While `f > 0.5`: for x in `floor(−f)..ceil(f)` (outer), z in the same range (inner): if `x² + z² ≤ (f+1)²` set
    end_stone at (pos.x+x, pos.y+dy, pos.z+z), setBlock flags 3. Then `f −= nextInt(2) + 0.5`, dy −= 1.
  - Draws: 1 + one per layer. Top layer at y = 75, layers go down.
- Then `findTallestBlock(..., 16, true)` finds the island's top (75) unless a taller fullSolid block is within 16,
  and the gateway goes 10 above it.

## 4. Dragon fireball and the breath clouds

### 4.1 Firing: StrafePlayer → ShooterComponent → ProjectileFactory
- The strafe tick calls ShooterComponent 1424324f0 and then `performRangedAttack` (vt 0x1a8). That slot is
  **not overridden** (140d29710): it only fires a gameplay event. The 3.0 is not used by it.
- 1424324f0: the component's conditional projectile list (+8..+0x10) is empty for the dragon, so no
  `level vt 0xa70` draw. It calls 1424327b0 with the default def (+0x20) and aux (+0xd0).
- 1424327b0: owner must be a Mob (0x100). `ProjectileFactory::shoot` 1424b75d0 (level vt 0xb8) with def
  `minecraft:dragon_fireball`, no target direction (zero vec), power = ShooterComponent+0 (no JSON `power`;
  presumably 0, unverified), then the shooter's sound (+0xd4) at the owner's attach pos 2. The JSON sets no sound.
- 1424b75d0, owner D, target T = D's target actor (D+0x290), ProjectileComponent `pc`:
  - Spawn the actor at D's position with D's rotation, owner = D.
  - Anchor y: `pc+0x1c4` == 2 → `getAttachPos(1).y`, == 1 → `getAttachPos(6).y`, else D.y. JSON `anchor: "middle"`;
    which value it maps to is unverified. Attach 1 on the dragon is pos + (0,2,0) per the AI doc. Only y is used.
  - Offset (0, 0.5, 0) rotated by D's yaw (`a = yaw*−0.017453292`): rx, rz are 0 for this offset.
  - Aim (shoot-at-target byte `pc+0x1d5` set, which must be the case here): `dx = T.x − D.x − rx`,
    `dz = T.z − D.z − rz`, `dy = (T.aabb.minY + (T.aabb.maxY − T.aabb.minY)*0.5) − anchorY − 0.5`. So it aims at the
    **vertical centre of T's box**, at T's feet x/z.
  - `h = sqrt(dz² + dx²)`; h < 1e-7: returns null (the actor already exists).
  - power = 1.3 (`pc+0x100`) since the passed power ≤ 0.
  - Rotation `(atan2(dz,dx)*57.295776 − 90, −atan2(dy,h)*57.295776)`.
  - **Spawn position** `(D.x + rx + dx/h, anchorY + 0.5, D.z + rz + dz/h)`: one block towards T horizontally from
    D's feet x/z (Java spawns at the head). `moveTo` 140d30f00.
  - Direction = `normalize(dx, dy, dz)` (zero below 1e-4). Gravity `pc+0x118` is 0, so no ballistic correction.
  - Uncertainty = `max(0, pc+0x1a0 (10) − difficulty × pc+0x1a4)`. The multiplier is not in the JSON
    (default presumably 0, unverified), so 10.
  - `ProjectileComponent::shoot` 1424020c0, then the shoot sound `pc+0x1d0` at D's attach pos 2 (level 0x14a59a850,
    JSON sets none), then owner set again.
- 1424020c0 (`shoot(dir, power, uncertainty)`):
  - `v = dir`, normalised again (≥ 0.001, else zero).
  - Then per axis x, y, z in order: `v += gauss() * 0.0075 * uncertainty`.
  - `v *= power` (0 if `pc+0x201`), plus a zero vector (14a654978). Position += the same zero vector.
  - Rotation: pitch = `atan2(v.y, sqrt(v.z²+v.x²))*57.3`, yaw = `atan2(v.x, v.z)*57.3`. Both are copied into the
    previous rotation.
  - Game event 0x14cc8a96c at the floored position (probably projectile_shoot).
- `gauss()` uses the **level random** (`level vt 0xa68`, MT at +0x9f8, as the dragon's other draws), Marsaglia polar
  with a cached second value (+0x9d4, flag +0x9e4). Per attempt `u = (f32)(u32 * 2^-32)` computed in double then
  rounded, `v1 = 2u1 − 1`, `v2 = 2u2 − 1` (f32), `s = v1² + v2²`; retry while `s ≥ 1` or `s == 0`.
  `m = sqrt(−2*log(s)/s)`; return `v1*m`, cache `v2*m`. The log is 14a5bcc40 (float log assumed). Three gaussians
  per shot: the cache carries over between calls.
- The JSON `uncertainty_base 10` gives sigma 0.075 per axis before ×1.3.

### 4.2 Flight: `ProjectileSystem::tick` 143b4dc00 → per entity 143b4e3c0
Read only far enough to port a gravity-0, inertia-1 projectile. In order each tick:
1. Removed: nothing. A stuck projectile (`+0x26c`) only re-checks every 5 ticks. Not relevant in flight.
2. If the previous rotation is zero (first tick), the rotation is set from the velocity.
3. **Owner overlap latch** `+0x266` (until set, the owner can't be hit): while 0, take the projectile box expanded
   by the velocity and grown by 1, fetch entities (`region vt 0xf8`). The latch becomes 1 once the owner is gone or
   no entity there shares the owner's root vehicle (PickComponent walk). This is Java's "left owner" rule.
4. Block clip from pos to `pos + v` (`region vt 0x198`, 200 steps, hit-liquid flag `pc+0x1db`), then entities
   along the segment (142ff6c90) with a box inflate of **0.3**. Under some experiment/gamerule flags not identified
   (0x14cc48798, 0x14cc48786), the inflate is instead `clamp((age − 2)*0.05, 0, 0.3)`.
5. Hits are walked in order (sorted results). A block or entity hit (types 0/1) moves the projectile to the hit
   point (145e33650) and runs on_hit (1423fe9f0). The loop stops when the projectile was removed.
6. Movement 1424039f0 (`hit` = something was hit this tick):
   - No hit: `pos += v` (one move).
   - Rotation follows `v` and is smoothed: `rot = prev + wrap(rot − prev)*0.2`.
   - `v *= inertia` (`pc+0x1bc`, or `liquid_inertia` `pc+0x1c0` with WasInWaterFlag), then `v.y −= gravity`.
   - With inertia 1 and gravity 0, **v is constant**, so the fireball flies straight at 1.3 × |dir + noise| per tick.
7. Age `+0x30` (f32 slot `[0xc]`) += 1.
- Whether the generic actor mover also moves it (it has `minecraft:physics`, collision box 0.31) is unverified. The
  dragon's own movement moved twice per tick (AI doc §3), so check with a capture. It is pushed by overlapping
  Mobs (5.3).
- `reflect_on_hurt: true`: see 5.1 (melee does nothing; another projectile reflects it).

### 4.3 On hit: SpawnAoECloudSubcomponent (vtable 14a7a0ae0, factory lambda_69 1424c5f20)
- Fields: +8 potion (default −1), +0xc duration (0), +0x10 particle (default 35), +0x14 reapplication_delay (0),
  +0x18 radius (0.0), +0x1c radius_on_use (default −1.0), +0x30 affect_owner (default 1). Names from its writer
  142409c40.
- `doOnHitEffect` 142409ee0: potion −1 → nothing. Potion 23 = Potion table 0x14ccc0498[23]. Its first effect's
  MobEffect is looked up (0x14ccb6c78[id]). Colour from the potion's effects.
- Creates `area_effect_cloud` (0x5f) at the projectile's position (already moved to the hit point). Owner = the
  projectile's owner (the dragon). Synched data:
  - duration (slot 0x2f8) = 120; radius (0x1e8) = max(6, 0); cloud radius_on_use (+0x3d4) = 0; potion (0x120) = 23.
  - effects: cleared, then a copy of each of the potion's MobEffectInstances (1429afe10).
  - particle (0x1f8) = dragonbreath; colour (1429b1db0); affect_owner = 0.
  - reapplication delay: when the first effect is **instantaneous** (harming is) it is **0**, except for a
    `dragon_fireball` projectile (type 0x40004F), which keeps the JSON value **20**.
- Then `level vt 0x3e8` at the projectile's position with the colour (a splash particle event, unverified).
- RadiusPerTick (slot 0x308) is **not** written here, so it keeps the cloud's default, which was **not found**.
  Leads: 141c81cd0/141c82570 store −0.005 into a component (from 141c81fe0). The JSON `area_effect_cloud` has
  only physics and bandwidth components.

### 4.4 The area_effect_cloud: tick 1429b01a0 (cloud vtable slot 188, load 1429b2930)
- Radius (1429afbe0) = `radius(0x1e8) + age × radiusPerTick(0x308) + pickupCount(0x318) ×
  radiusChangeOnPickup(0x310)` (f32), `age = levelTick − spawnTick(+0x3c0)`. So the radius over time is linear in
  age, at the unknown default rate (4.3).
- Each tick (server):
  1. First tick (spawnTick < 1): spawnTick := the level tick, return.
  2. `age = max(0, now − spawnTick)`. Duration d (slot 0x2f8): removed when `d != −1` and `d < age`, so it lives
     ages 0..d.
  3. Radius clamped ≥ 0; below **0.5**: removed.
  4. **Only when age % 5 == 0** (an inverse-multiply test by 0xcccc…cd in the server branch), otherwise return.
  5. Victims map (+0x3f0): entries whose expiry ≤ age are dropped.
  6. No effects: 1429b1c10 (removal, unverified), return.
  7. Box = the cloud's AABB grown by (r, 0.5r, r) (1429b1cf0). Fetch actors of type Mob 0x100 (players included)
     in it (1415fcd30).
  8. For each entity e in fetch order:
     - affect_owner == 0 and e is the owner: **the loop stops** (`je 1429b0402`, the loop exit, at 1429b0758).
       Entities after the owner in the list are not affected this pulse. Probably a BDS bug, but it is what the code
       does. Fetch order: 5.4 (non-player mobs first, then players).
     - e in the victims map: skip. Not affectable (140d5b950): skip. No effect e can take (`vt 0x338`): skip.
     - 3D squared distance between the cloud position and e's position (feet), **≤ r²**, else skip.
     - victims[e] = age + reapplication delay (+0x3c8).
     - Each effect: non-instant ones are added with **duration × 0.25** (int; −1 stays −1). Instant ones:
       `MobEffect vt 0x30 applyInstantaneousEffect(cloud, owner, e, amplifier, 0.5)`.
     - radius_on_use (+0x3d4) != 0: radius += it, below 0.5 removed. duration_on_use (+0x3cc) != 0: duration +=
       it, ≤ 0 removed. Both are 0 for the dragon's clouds.
- So with reapplication 20 an entity is hit at most once per 20 ticks (every 4th pulse). With 0 it is hit on
  every pulse (every 5 ticks).
- Damage: effect 7 `instant_damage` ("potion.harm", registered at 141c4f6a5). Instant effects go through the
  MobEffect's attribute buffs (141c56080: an `InstantaneousAttributeBuff` on health, amount from the buff's value
  or its amplifier function, scaled by the 0.5 passed). The buff values were **not read**. Java's rule
  `(6 << amp) * 0.5 = 3` damage for amplifier 0 is the likely reading (unverified). Undead get the cause swapped
  (cause 3 → 4, 2 → 7 at 141c56080: harm heals undead).
- Potion 23 is harming in Bedrock's potion id list. Its definition (Potion table init) was **not found** by string,
  so "instant_damage amplifier 0" is assumed, not read.

### 4.5 Flaming-phase breath (DragonFlamingGoal::tick 148a3bdd0, AI doc §2 Flaming)
At flame tick c == 10 (`flame_time`):
- Position: as in the AI doc. `px = H.x + n.x*radius*0.5`, `pz = H.z + n.z*radius*0.5` (floats, 2.5 off the head
  towards the side away from the body). Column (floor px, H.y, floor pz) walked down while air, to no lower than the
  dimension min. Cloud y = `floor(y) + 1`.
- `area_effect_cloud` (0x5f), owner = the dragon. Synched data:
  - duration 200 (goal+0x50), radius 5 (goal+0x54, clamped ≥ 0).
  - RadiusChangeOnPickup (slot 0x310) = **−0.5**: this is the AI doc's −0.5 field, the per-bottle shrink, not
    radius-on-use.
  - radius_on_use (+0x3d4) = 0, potion (0x120) = 23.
  - effects cleared, then one MobEffectInstance: id **7** (instant_damage), amplifier 0, the rest zero, visibility
    flag from MobEffect 7 +0xf0.
  - particle 0x31 (49, dragon breath), colour "#DC00EF" (goal+0x58), affect_owner 0, **reapplication delay 0**.
- RadiusPerTick is again not set (default unknown, 4.3). Java grows its breath cloud by (7−5)/200 per tick; BDS
  sets no rate.
- With reapplication 0, a player in range takes the instant-damage hit every 5 ticks. While the sitting dragon is
  inside the grown box and comes before the player in the fetch order, the owner-stop rule (4.4) skips the player.
  This should be **checked live**.

## 5. Follow-ups

The pack JSON quoted here is `behavior_packs/vanilla_1.26.50/__brarchive/entities.brarchive` (newest copy). The
full dragon_fireball entry: collision box 0.31, `physics {}`, **`pushable_by_entity {}`**, `pushable_by_block {}`,
projectile `anchor middle, gravity 0, reflect_on_hurt true, hit_sound explode, power 1.3, inertia 1, offset
(0,0.5,0), uncertainty_base 10, isolated_physics false`, on_hit **`remove_on_hit {}`** + `spawn_aoe_cloud` (§4.3).
ender_dragon has no pushable components and no `tick_world`.

### 5.1 reflect_on_hurt
Field map (JSON reader 1423fc450, then the definition→component copy at 1423fd895): definition +0x1e9
reflect_on_hurt → component **+0x1d7**, +0x1ea destroy_on_hurt → +0x1d8, +0x1eb stop_on_hurt → +0x1d9,
+0x1ec crit_particle_on_hurt → +0x1da, def +0x214 `reflect_immunity` (default 0.0) → +0xfc = `(int)floor(x*20)`
ticks, +0x1f1 multiple_targets → +0x1dd, `ignored_entities` → the type vector at +0x1e8.

**`reflect(pc, proj, source)` 142400b00.** Runs only if `source.getEntityCategories() & Mob (2)` (players count) or
cause == SelfDestruct (13), and +0x1d7, and `pc+0xfc <= proj tick (+0x198)`. Then:
1. `A = level.fetchEntity(source.getEntityUniqueID())`. `v = A.getViewVector(1.0)` (140d2bdb0: Java's, pitch/yaw
   through the Mth sine table), normalised (zero below 1e-4); A missing: v = 0.
2. `posDelta = v`. **Speed 1.0 per tick, not ×power**, aimed along A's look, not reversed.
3. `setOwner(A id)` (Actor vt 0x1b0); pc+8 (owner id) := A id; pc+0x1d6 := 1 (lets the owner be hit, 5.2);
   pc+0x26c (stuck) := 0; the stored hit result (+0x270..) reset; pc+0xf9 := 1 (unread). No sound, no event.
   The leave-owner latch +0x266 is not reset.

Callers: base `Actor::_hurt` 140d397e0 (via `ProjectileComponent` hurt handler 1424013f0: reflect, then
destroy_on_hurt removal and the crit particle event 0x7dc) and `on_hit` 1423fe9f0 (5.2 step 4).

**A player's melee does nothing to a dragon fireball.** Type 0x40004F is built by factory 141ec3d00 (registration
at 141ea0d1d) into class vtable 14a79dc40 (a Fireball subclass, base ctor 1424b39d0). Its `_hurt` (slot 133) is
14120a500, which returns {0, 1} and does nothing, so the ProjectileComponent hurt handler is never reached: no
reflect, no owner change, no sound. This matches LeviLamina's `DragonFireball::_hurt` override and Java
(`DragonFireball.hurt` returns false). The JSON `reflect_on_hurt` is dead for melee.

**It can still be reflected by another projectile.** When projectile P hits an entity that has a
ProjectileComponent (on_hit 1423ff1d6), P builds a source `ActorDamageByChildActorSource(P, owner(P),
Projectile)` (140d90c80; P alone if it has no owner, 140d55310) and calls `reflect` on the hit one. For an arrow
shot by a player, the source's entity is the player, so the fireball takes the **player's view vector at speed
1** and the player as owner. An ownerless P fails the Mob-category test. The fireball is a valid target for P
because the Fireball ctor adds a PickComponent (1424b39d0). Unverified live.

**A reflected fireball does not hurt the dragon.** It can hit the dragon (no longer the owner; Mobs are
pickable). On hit it runs `remove_on_hit` and the cloud; there is no `impact_damage`, so no direct hurt. The
cloud's owner is now the player. The cloud skips the dragon: its per-effect test (`vt 0x338`, slot 103) is
overridden on the dragon (vtable 14a88bde0) by 140076010 (`xor eax,eax`), so the dragon is never affected by
effects. The generic Mob version is 140d5a920. The §4 BlockExplosion/player gate is never reached.

### 5.2 Projectile entity hits (143b4e3c0 → 142ff6c90)
- Candidates: `region vt 0xf8` (fetchEntities, the projectile excluded) in the box from pos to pos+v, grown on
  each side by the projectile's AABB size + 1. Per candidate, the predicate (default 143b558c0; the flagged
  variant 143b55660, 4.2), in order:
  1. Must have **PickComponent**, else skip.
  2. Its unique id equals the owner id (pc+8) or `proj.getSourceUniqueID()` (vt 0x298): skip unless pc+0x1d6 (set
     by reflect or shield block). So the dragon can **never** hit its own fireball's owner before a reflect,
     latched or not.
  3. Latch +0x266 still 0 and the owner exists: skip when the candidate's root vehicle is the owner's root
     vehicle (1023-step walk via 140d26680; no vehicle means itself). The latch only adds this test.
  4. Type (140d25840) in `ignored_entities`: skip (dragon_fireball has none).
  - Flagged variant: no step 3; step 2 also needs `pc+0x34 >= 5`.
- Who has a PickComponent (adder 141ccef40, value = pick inflate). Every **Mob** gets 0.1 from the Mob ctor
  1429df250, which covers players and the dragon. A Mob tick at 142cb87b0 removes it while `vt 0xf8` is true
  (probably spectator, unverified). Also: the Fireball family (1.0, ctor 1424b39d0: fireball, small_fireball,
  dragon_fireball, wither_skull), shulker bullet (0.1), boat 14a7e8160, minecart 14a8883c0, primed TNT 14a8899f0,
  vtable 14a88b9a0 (probably the ender crystal), and 14a7e7d20/14a79f200/14a779950/14a8876e0 (unnamed).
  - **Item entities, XP orbs, arrows, tridents, snowballs and area_effect_clouds were not seen adding one**, so
    they cannot be hit. Inferred from the adder list, not from their ctors.
- Shape tested: an actor with a **SubBBsComponent** (the dragon's ctor reaches its emplace, 141ccfc90) is tested
  per sub box grown by the inflate. Otherwise HitboxComponent boxes, otherwise the AABB (+0x220).
- On an entity hit, `on_hit` 1423fe9f0 does, in order:
  1. Return at once if the target's `canInteractWithOtherEntitiesInGame` (vt 0x120) is false.
  2. The script/gameplay hit event.
  3. A target with a projectile-deflect component (a type list; breeze-like) re-aims the projectile, then
     return.
  4. A target with a ProjectileComponent is reflected (5.1).
  5. The on_hit subcomponents in JSON order (+0x120 vector, vt 0x18): for dragon_fireball, remove_on_hit and
     spawn_aoe_cloud.
  6. Removal/bounce bookkeeping (+0x260..+0x265, +0x1e4).
  7. Game event 0x14cc8a964.
  8. If the target is creative (game type 1, or 5 with level default 1): stop here.
  9. hit_sound (pc+0x1c8, `explode`) at the projectile's position.
  10. Shield/stop_on_hurt handling (target `isBlocking` vt 0x160, or +0x1d9).
- So for a hit mob or player, the only effect is the cloud at the hit point, plus the removal and the explode
  sound. No damage, knockback or fire.

### 5.3 Push by overlapping actors: Java's `Entity.push` (yes)
- **`PushActorsSystem::tick` 143b5bba0 → 143b50bc0**: for each entity with ActorOwner, ActorMovementTickNeeded,
  **MobFlag** and PushActorsRequest (emplaced each tick by a post-move system 141935fe0, registered from
  `registerPostMoveTravelVelocitySystems` lambda_7; its view was not read), walked in reverse storage order: actor
  vt 0x480 = **`Mob::pushActors` 1429f2250**, which the dragon does not override. Only Mobs push. The fireball is
  pushed; it pushes nothing.
- `pushActors(M)` 1429f2250:
  1. Skip if M fails `canInteractWithOtherEntitiesInGame`.
  2. `fetchEntities(except M, M.AABB grown 0.2 in x and z, y unchanged)`.
  3. For each e in fetch order: skip riders and vehicles of M (ridingPrev and root-vehicle tests) and e with flag
     +0x210 & 0x400. If e has PushableByEntity: `push(e, M)` 142fa1d50.
- `push(e, M)` 142fa1d50 with the preset (no presets in the JSON → the component's default, 142fa26e0; defaults
  from the struct initialiser at 148f8b390: mode 1, strength 0.05, min 0.1, max FLT_MAX, needs-overlap 1, scale
  e 1.0, scale M 1.0):
  1. `dx = M.pos.x − e.pos.x`, `dz = M.pos.z − e.pos.z`. **StateVector positions, i.e. feet; M's main position,
     not a part.**
  2. Mode 1 distance `m = max(|dx|, |dz|)`. Require `m ≥ 0.1` and `m − M.AABB.width/2 ≤ FLT_MAX`.
  3. **Require M.AABB to intersect e.AABB** (142f77680, the needs-overlap flag). M.AABB is the dragon's main
     13 × 4 collision box (+0x220), not its parts.
  4. Mode 1 (142fa30c0), f32: `s = sqrt(m)`, `inv = 1/s`, `k = min(inv, 1) * 0.05`, `r = 1 − e+0x194`
     (e+0x194 is probably push-through, 0 here, unverified). `e.posDelta += (−dx*inv*k*r*1.0, 0, −dz*inv*k*r*1.0)`
     (142fa31d0: a plain add to posDelta for non-Mobs).
  5. Then e `vt 0x3e0(M)`. M is pushed back only if M has PushableByEntity; the dragon has none.
- For m ≥ 1 the major axis gains exactly 0.05 and the minor `0.05 * minor/major`, which matches the captures.
  The push stops once the fireball's box leaves the dragon's 13-wide box (6.5 + 0.155 from its feet x/z at
  ≈1.3/tick, starting 1 block out), about 4 ticks. Since the fireball's inertia is 1, the velocity then stays.
  In tick 1 (m < 1), the gain is `0.05*axis/sqrt(m)`.
- Players (Mobs) overlapping a fireball push it the same way.
- Not read: where PushActorsSystem runs relative to ProjectileSystem and the dragon's moves in the tick, so
  whether the push sees the fireball before or after its own move this tick. Fit it on a capture.

### 5.4 Cloud pulse fetch order (1415fcd30, type 0x100)
- `BlockSource::fetchEntities(type, box)`:
  1. Clear the output vector.
  2. Chunks `cx = floor((minX−2)/16) .. floor((maxX+2)/16)` (**outer, ascending**), `cz` the same on z (inner,
     ascending).
  3. Each loaded chunk (`vt 0x148`) appends its own entity list in **list order** (LevelChunk +0x1360, via
     1409f8a10).
  4. Per entity: has the type (140d27800) and its AABB strictly overlaps the box.
  5. Then, when the type is 0x100 (Mob) or has low byte 0x3f (Player), the **dimension's players** are
     appended (`vt 0x128 → vt 0x50` forEachPlayer, lambda 141613430, strict AABB overlap).
- Players are not in the chunk lists (inferred: that is why the second pass exists). So in a pulse **every
  non-player mob comes before every player**, in chunk-then-list order. With the owner-stop rule (verified: the
  `je 1429b0402` at 1429b0758 jumps to the same exit as an empty list at 1429b06fa), once the dragon is in the
  box, **no player is affected that pulse**.
- The chunk list order is insertion order. Re-adding on chunk change is presumed, not read.
- Box (1429b1cf0): the cloud's AABB grown by (r, r/2, r). The cloud's AABB is the default 0.6 × 1.8 (capture
  metadata BoundingBoxWidth/Height on the breath clouds in `captures/bds-fireball-hit.txt`). The same metadata
  shows `AreaEffectCloudChangeRate −0.0`, i.e. **RadiusPerTick = 0** for the breath cloud (answers 4.3 for that
  cloud; the fireball cloud's value was not captured).
- **Dragon on the portal.** Captured breath clouds sat at (7.5, 62.0, −0.005), r = 5, so the box is x
  2.2..12.8, y 59.5..66.3, z −5.3..5.3. The sitting dragon (drill: feet 0.06, 66.236, 0.13; AABB ±6.5 × 4) has
  minY 66.236 < 66.3, x −6.44..6.56 and z −6.37..6.63, so it **is inside**, by 0.064 in y.
  - So while the dragon sits there and r ≥ ~4.97, the breath cloud damages no player.
  - One bottle pickup (r 4.5, maxY 66.05) or a cloud 1 lower takes the dragon out of the box.
  - Computed from the static rules and capture numbers; check live (damage taken while the dragon sits).

### 5.5 Path node chunk test (findClosestNode 145997df0)
- The call is BlockSource (vtable 14a753900) slot 55, 1415f6270 = `areChunksFullyLoaded(pos, r)` (two args;
  LeviLamina's signature), → 1415f62f0 over the block box `pos ± r`.
- Chunks: x `(−60)>>4 .. 60>>4` = −4..3, z the same: **8 × 8 chunks, blocks −64..63**, any y. Walked x inner, z
  outer; the first failure returns false.
- Per chunk:
  1. Skipped (passes) when `ChunkSource::isWithinWorldLimit` (ChunkSource vt 0xe8) is false.
  2. Else `BlockSource::getChunk` (vt 0x148, 1415f5900), which returns null unless the chunk's load state
     (+0xf0) is **≥ 11 Loaded** (≥ 2 only for BlockSources with +0x1c set, worldgen ones). Null: false.
  3. A Loaded chunk passes only if +0x152e (unidentified) or **+0x10d1** is set. +0x10d1 is written 1 once at the
     end of 140a0a700, a once-only chunk on-load step that places block entities and entity data; probably
     `mOnChunkLoadedCalled`.
  - So: **all 64 chunks Loaded and through their on-load step**. Generated-only or ticking state does not matter.
- **Correction to the AI doc §1** (`dis` 145997e55..f2e):
  - With the latch +0x7e0 clear: `ok = areChunksFullyLoaded(0, 60)`. If !ok and nodes already exist, keep them.
    Otherwise clear the vector, rebuild all 24 nodes, and set **latch := ok**.
  - So the nodes are built on the very first call even when the area is not loaded (heightmaps of missing chunks;
    the `vt 0x170` fallback value was not read). They are rebuilt once more on the first call where the test
    passes, then frozen. The node vector is never empty after the first call.
- Who loads those chunks:
  - Only players. The player view source (1402a50d0 → 1409ea4d0) centres on floor(player pos) with radius
    **player+0xb94 chunks × 16** (the negotiated chunk radius), plus 112 blocks only on a client.
  - 1409ea4d0's other callers are the gateway jobs (fight +0x90/+0xb0, radius 0xa0, temporary), 14074fe40,
    1402a5330 (also player) and 14371cc30 (likely ticking areas, unread).
  - Neither TheEndDimension nor the dragon creates a view source, and the dragon JSON has no `tick_world`.
  - The fight's own gate (fight doc §1 step 5) needs only chunks (−1|0, −1|0), plus a player within ~200.
  - So the nodes freeze from real terrain once some player's view covers chunks −4..3 (a radius of about 5+
    chunks from the island centre). Before that, the dragon flies on nodes built from unloaded columns.
  - Whether the view source loads a square or a circle was not read (1409ea4d0).

## Left open
- RadiusPerTick's default for the fireball cloud (slot 0x308; the breath cloud's is 0, 5.4); whether the 0x14cc…
  flags change the entity inflate; anchor "middle"'s value.
- Potion 23's definition and the instant-damage buff amount; the cloud's removal function 1429b1c10.
- Whether `mainBS` reads chunks near the far gateway (main gateway exit, §1).
- The generic per-tick move of the fireball, if any, and where PushActorsSystem runs in the tick (5.3).
- +0x152e on LevelChunk; `vt 0x170`'s value for an unloaded column (first node build, 5.5).
