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
  dragon's own movement moved twice per tick (AI doc §3), so check with a capture.
- `reflect_on_hurt: true`: a hit on the fireball sends it back (not read).

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
       does. The fetch order is the region's entity order (not read).
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

## Left open
- RadiusPerTick's default (slot 0x308); whether the 0x14cc… flags change the entity inflate; anchor "middle"'s
  value.
- Potion 23's definition and the instant-damage buff amount; the cloud's removal function 1429b1c10.
- Whether `mainBS` reads chunks near the far gateway (main gateway exit, §1).
- The generic per-tick move of the fireball, if any; reflect_on_hurt; hit sound.
- Fetch order in the cloud pulse, which decides who the owner-stop rule skips.
