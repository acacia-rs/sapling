# bds-player-omens findings

Capture: `research/captures/bds-player-omens.txt` (drill `player_omens`, foes_drill.rs). Times are ms, ~50 ms
a tick. Both streams use rid 1 = actor (CmdActor), rid 2 = watcher; actor-stream times are quoted, the
watcher's match within 1-2 ms. Actor in survival, feet (0.5, -19, 0.5); its MovePlayer y is -17.37999 =
**feet + 1.62: BDS's player position is the eye**, and every spawn point below is that position plus an
offset, not the feet. Watcher feet (3.5, -19, 0.5).
Not recorded by this probe: **UpdateSubchunkBlocks** (none in the file, not even for the setup fills),
so cobweb placement is seen only through the LevelEvents and the cleanup fill's count. MobEffect *is*
recorded here (unlike bds-effects3).

## Common to every death

- `/kill @s`: EntityEvent DeathAnimation + DeathInfo `death.attack.generic` at once (command time).
- Next tick: UpdateAttributes health 0, `Respawn` state 0 at (0.5, -17.37999, 0.5), SetEntityData
  **BoundingboxWidth/Height 0.2/0.2** (+ PlayerLastDeathPosition (0, -18, 0), PlayerHasDied). The
  effect's packets ride in this batch or the next.
- ~1 s on: `disappeared` sound at (0.5, -18.9, 0.5); at client respawn (~2.1 s) Respawn state 1 at
  world spawn, MobEffect Remove, EntityEvent Respawn, `reappeared`, box back to 0.6 x 1.8.

## Wind charged (killed 17024)

- 17049-17050 (next tick, with health 0 / box 0.2, entity tick 389): SetEntityMotion **watcher (rid 2)
  (0.54194486, -0.096464895, 0)**, then `LevelEvent ParticleWindExplosion` at **(0.5, -17.27999, 0.5),
  data 3**. 17100 (next tick): `wind_charge.burst` at the same point, entity_type "", unique -1.
- **Burst y = feet + 1.72, not feet + 0.9**: it is position (eye) + 0.1 = half the *dead* 0.2 box. Same
  rule as the zombie (position + height/2), but the player's position is its eye and its box has
  already shrunk. Zombie was data 4, this is **data 3**.
- Push = motion + 0.0784 = (0.541945, -0.018065, 0). y/x = -0.03333 = -0.1/3: **direction =
  unit(watcher eye - burst) exactly** (eye (3.5, -17.38) vs burst y -17.28). |push| **0.54225**.
  - 0.82(1 - d/9.8): d = 3.0017 (eye) gives **0.5688**, d = 3.4581 (feet) gives 0.5306. Observed sits
    between (0.953 x the eye value); the formula holds only if d = 3.32. Not a fit.
  - bds-effects3's D 8, k 1.2 gives 0.681 (feet d) / 0.750 (eye d): also off. A straight line through
    both samples (feet d) needs k 0.638, D 23: implausible, so something other than distance (exposure,
    mode, resistance) differs between the samples. Needs a distance sweep with one target.
- Watcher x 3.5 -> 4.04194 (tick 386, exactly + 0.54194) -> 4.69371 by 18214: 1.194 in all, z 0.
- **The dying player is not pushed**: no SetEntityMotion for rid 1 anywhere around the death.
- No damage to the watcher.

## Weaving (killed 32051)

- 32073 (next tick, same batch as Respawn state 0 / box 0.2, entity tick 642-643): two `LevelEvent
  AnimationSpawnCobweb`, data 0, at **(1, -19, 0)** and **(0, -19, 1)**.
- **2 cobwebs: (+1, 0, +0) and (+0, 0, +1) of the feet cell**; the player's own cell stayed free this
  time (the zombie's had one). Both on the floor layer: the cube is about the eye cell (0, -18, 0) or the
  feet cell; either way only y -19 has a sturdy face below. Can't tell which from one sample.
- No block packets (see above). The cleanup `fill ... air` at 39055 reports **`["2"]`** (the wind and
  oozing cleanups report `["0"]`): both webs were placed and stayed.

## Oozing (killed 47061)

- 47121-47122 (2 batches on, as for the zombie): **2 slimes at (0.5, -16.87999, 0.5) = position (eye) +
  0.5 = feet + 2.12**, AddEntity vel (0, -1e-5, 0), yaws -68.9 and -123.75.
- Final size for both **Variant 2, box 1.04, health 4/4** (SetEntityData + UpdateAttributes 1-2 ms on).
  AddEntity again stale: rid 5 came as Variant 4 / box 2.08 / health 16 / movement 0.6, rid 4 as Variant
  1 / box 0.52 / health 1 / movement 0.3. `spawn` sounds at y +1.768 and +0.442 (0.85 x the stale box).
- They fall 2.12: y -16.958 (47170), motion -0.2305 (47231), -0.4475 (47370), land at -19.0 (47530,
  on_ground) at (0.729, 0.702) and (0.675, 0.249). Landing plays `attack` at each slime's middle (no hurt
  anywhere).
- `kill @e[type=!player]` 53068 (`"Slime, Slime"`): item 391 x2 dropped (53120); **5 tiny splits** at
  54131 in two stacks, 3 at (-1.5, -18.5, 0.5) and 2 at (-4.5, -18.5, 1.5) (block-centre, feet + 0.5,
  as in bds-effects3), Variant 1 / box 0.52. They lived through the infested trial (killed 88114).

## Infested (effect 60077, ten `/damage 1` + heal)

- Each hurt: HurtAnimation data -1, health 20 -> 19 (no armour), healed back by instant_health.
- **Silverfish on 1 of 10 hurts: hurt 6 (72080), two at once** (Java's 1-2 a trigger; the zombie got one
  per trigger, twice). 72121-72122, 41 ms after the HurtAnimation, one batch after the health-19 sync
  (tick 1344); the zombie's came 10-38 ms on.
- Position **(0.5, -16.47999, 0.5) = position (eye) + 0.9 = feet + 2.52**: "position + height/2" again,
  with the live 1.8 box on an eye-level position. Not the player's middle.
- **Velocity horizontal 0.3 exactly, y 0**: (0.29786, 0, 0.03578) and (0.03902, 0, 0.29745): 83.2 and
  7.5 deg off +z, the player's yaw 0 look, so within +-90 deg as Java's rule.
- `spawn` (y +0.255) and `hurt` sound at the spawn point, as for the zombie.
- Both **burrowed**: rid 13 SilverfishSpawnAnimation 73472 -> UpdateBlock (1, -20, 0) 2988963749
  (infested stone) + RemoveEntity 73520; rid 12 75423 -> (0, -20, 2) at 75471.
- The kill (82100): nothing extra.

## Effects through death

- **No MobEffect Remove and no VisibleMobEffects 0 at death.** The effect stays until respawn, where
  MobEffect Remove (19150 id 32, 34180 id 33, 49180 id 34, 84279 id 35) and VisibleMobEffects 0 come.
- The removed duration is frozen at death: 1160 for the first three (1200 - the 40 ticks between effect
  and kill), 767 for infested (~433 ticks alive). The ~2 s dead didn't count down.
- Nothing fired twice: one burst, one cobweb set, one slime pair, no second trigger at respawn.

## Surprises

1. BDS's player position is the eye, so the omen spawns sit high: burst feet + 1.72 (dead 0.2 box), slimes
   feet + 2.12 (they fall in), silverfish feet + 2.52. All follow the mob rules applied to that position.
2. Wind burst data 3 here against 4 on the zombie; the push (0.542 at 3) fits neither 0.82(1-d/9.8) nor
   effects3's D 8 / k 1.2.
3. Two silverfish from one trigger.
4. The first oozing slime and first silverfish share yaw -68.90625 exactly (a seeded or reused random?).
5. The probe recorded no UpdateSubchunkBlocks; add it before the next block-placing drill.
