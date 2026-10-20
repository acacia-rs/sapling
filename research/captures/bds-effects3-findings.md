# bds-effects3 findings

Capture: `research/captures/bds-effects3.txt` (drill `effects3`, foes_drill.rs). Times are ms, ~50 ms a
tick. Actor and watcher streams match packet for packet for the mobs; actor-stream times are quoted.
The actor (creative) stood at (0.5, -19, 0.5) on the stone floor (top y -20) until the wind burst moved
it to (-0.6065, -19, 0.3831), so trials 2-4 summon at **(3.3935, -19, 0.3831)**, cell (3, -19, 0).
Incidental: a sculk catalyst at (12, -24, 0) and sensor at (14, -24, 0) are left from older drills
under the floor. Deaths within ~8 of the catalyst (zombie 1, the oozing slimes, the last kill) bloom
it (UpdateBlock 1049514107 -> 4289174832) and spread sculk over the floor (y -20 -> 1041212874, veins at
y -19 = 1406367373, `block.sculk.spread`); the sensor toggles on vibrations. None of it is the effects.

## Ids, packets

| effect | CommandOutput key | VisibleMobEffects (with slowness) | own slot | id |
|---|---|---|---|---|
| slowness 255 | `%potion.moveSlowdown` | 4 | 4 | 2 |
| wind_charged | `%effect.wind_charged` | **576** = 4<<7 \| 64 | 64 | **32** |
| weaving | `%effect.weaving` | **578** = 4<<7 \| 66 | 66 | **33** |
| oozing | `%effect.oozing` | **580** = 4<<7 \| 68 | 68 | **34** |
| infested | `%effect.infested` | **582** = 4<<7 \| 70 | 70 | **35** |

- Slot packing as in bds-mob-effects (`(id << 1) | ambient`, lowest id in the highest slot). The new
  four use the `%effect.<name>` key; old effects keep `%potion.<x>`. SetEntityData 20-40 ms after the
  command, on the next tick.
- **MobEffect: this capture holds none at all**, not even slowness's known Add (bds-mob-effects), so the
  probe did not record that packet type: undecided here. Slowness's UpdateAttributes movement 0.0 is in.
- No VisibleMobEffects 0 after any of the four deaths (bds-mob-effects saw one ~10 ticks on).
- Every effect fires on the death tick (with the loot drop), not at removal 20 ticks later.

## Wind charged (zombie at (4.9722, -19, 0.9724), killed 15660)

- 15680, same batch as the rotten-flesh drop: SetEntityMotion on the watcher (rid 4) and the actor
  (rid 1), both `(-0.50234, -0.00314, -0.05307)`, then `LevelEvent ParticleWindExplosion` at
  **(4.972212, -18.05, 0.972438) = zombie feet + 0.95 (half its 1.9 height)**, `data: 4`.
- 15733 (next tick): `LevelSoundEvent wind_charge.burst` at the same point, entity_type "", unique -1.
- Push = motion + 0.0784 (resting y) = (-0.50234, 0.07526, -0.05307). **Direction = unit(feet + (0,
  1.62, 0) - c) to 1e-6** (the breeze-burst rule). |push| **0.51071** at d = |feet - c| = 4.5963.
  `k (1 - d/D)`: D 6 needs k 2.183; **D 8, k 1.2 gives 0.51055** (3e-4 off, one point, not unique).
  Java's 1.22 x (1 - d/6) would be 0.285, so BDS pushes harder or farther. Needs a distance sweep.
- The actor moved (-1.1065, 0, -0.1169) in all (z/x 0.10564 = the push's). No damage to anyone.
- The drop (rotten flesh, AddItemEntity at feet + 0.075, vel (0.0154, 0.2, 0.0237)) got SetEntityMotion
  **(0, -0.83881, 0)** 1 ms later: straight down, x/z exactly 0, so the drop's motion is replaced.
- `data: 4`: one sample, meaning unknown (breeze bursts send ParticleBreezeWindExplosion data 3).
- No block toggles from the burst (no doors/buttons in range; only the sculk above).

## Weaving (zombie at (3.3935, -19, 0.3831), cell (3, -19, 0), killed 28431)

- 28482 (next tick, with the 2-flesh drop): two `LevelEvent AnimationSpawnCobweb` at the cells' min
  corners **(3, -19, 0)** and **(4, -19, 1)**, data 0. 28526 (one tick on): UpdateSubchunkBlocks,
  flags 3, cobweb (955936010) at both.
- **2 cobwebs: the zombie's own cell (+0, 0, +0) and (+1, 0, +1)**. Both on the one layer with a solid
  floor under it, inside the 3x3x3 around the zombie: fits Java's randomInCube(radius 1), 2-3 webs,
  free cell with a sturdy face below.
- They stayed: still there at the `fill air` 6 s later (34429, `["2"]`, both set to air, flags 2).

## Oozing (zombie at (3.3935, -19, 0.3831), killed 41193)

- 41280-41281 (2 ticks on): **2 slimes at (3.3935, -18.5, 0.3831) = zombie x/z exactly, feet + 0.5**,
  AddEntity vel (0, -1e-5, 0). Final size **Variant 2, box 1.04 x 1.04, health 4/4, movement 0.4**
  for both (SetEntityData + UpdateAttributes 1-2 ms after the AddEntity).
- **AddEntity carries a stale natural size**: rid 10 came as Variant 4, box 2.08, health 16, movement
  0.6, corrected 2 ms later; its `spawn` sound is at y +1.768 (0.85 x 2.08), rid 11's at +0.884.
- Motion: falling 0.5 (-0.2305 at 41379), then the overlap push apart (41626: (+0.152, -0.0784, +0.085)
  and (-0.166, -0.0784, +0.003)), then normal hops (y 0.3332). They wandered to (8.77, 4.73) and
  (-1.31, -1.90) in 5 s.
- **The 6 slimes at 47678 are their splits.** `kill @e[type=!player]` at 46615 killed both
  (`"Slime, Slime"`, health 0 at 46625); 21 ticks later, in the RemoveEntity batch of each parent:
  rid 11 (at 8.768, 4.729) -> **2** slimes at (8.5, -18.5, 4.5); rid 10 (at -1.307, -1.898) -> **4** at
  (-1.5, -18.5, -1.5). **Splits spawn at the parent's block-cell centre, y feet + 0.5**, all stacked, vel
  (0, -1e-5, 0); `born` sound each. Final **Variant 1, box 0.52, health 1/1, movement 0.3**; AddEntity
  again stale (box 2.08, health 20/3.4e38). They came after the `fill air` (47609, `["0"]`) only by timing.
- The 6 tiny ones lived to the last `kill @e[type=!player]` (69330); they split no further and dropped
  5 slimeball stacks (item 391, 7 balls).
- Slime packets: EntityAge byte 2 at spawn, then toggling 2/1 with the hops (squish, client cue).

## Infested (zombie at (3.3935, -19, 0.3831), effect 51662)

- 10 `/damage 1`, 1 s apart: HurtAnimation data -1, **0.94 each** (20 -> 19.06 ... -> 10.6; armor 2 as
  `max(2/5, 2 - 1/2) = 1.5` -> 1 x (1 - 1.5/25)).
- **Silverfish on hits 5 (57970) and 9 (62245)**, one each, 10 ms and 38 ms after the HurtAnimation (same
  tick). 2 of 10, fits Java's 10 % a hit.
- Position **= the zombie's feet x/z exactly, y feet + 0.95 (half its height)**: (3.3935, -18.05, 0.3831)
  and (3.32984, -18.05, 0.27056) (the zombie had been shoved to there by the first silverfish).
- **Velocity horizontal 0.3 exactly, y 0**: (0.14774, 0, 0.26110) and (0.23021, 0, 0.19237); 29.5 and
  50.1 deg off +z, within 90 deg of the zombie's yaw (byte 225 = 316 deg, look (0.69, 0.72)). Fits Java's
  `look x 0.3 (y x 1.5) rotated by random +-90 deg`, pitch 0.
- Sounds: silverfish `spawn` (y +0.255) and **`hurt`** at the spawn point, no damage taken (Java plays
  SILVERFISH_HURT on spawn).
- Each spawn shoves the zombie (overlap): 0.06-0.25 off in 6 ticks, opposite the silverfish.
- Silverfish 1 **burrowed**: EntityEvent SilverfishSpawnAnimation at 58681 (14 ticks after spawn),
  then UpdateBlock (3, -20, 1) -> 2988963749 (infested stone) and RemoveEntity at 58732. That floor
  block stays: the cleanup fills only y >= -19. Silverfish 2 roamed until the last kill.
- The kill (64228): nothing extra, no silverfish, no drops.

## Surprises

1. New effects use `%effect.<name>` keys; ids 32-35 confirmed from VisibleMobEffects (576/578/580/582).
2. Wind-charged burst at feet + half height pushes creative players 4.6 away by 0.51: stronger than
   Java's radius-3 burst; the dropped item's motion is overwritten, not added to.
3. BDS sends slimes' AddEntity with a random natural size and fixes it in the next packet.
4. Oozing slimes keep exact x/z (+0.5 y); split slimes snap to the block centre (+0.5 y).
5. Infested silverfish can merge into the floor (infested stone left behind).
6. Old sculk catalyst/sensor under this floor: clear y -24 before the next run on this world.
