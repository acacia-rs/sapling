# MOVING flag (ActorFlags bit 34)

## Rule

`MOVING` shown in tick t = `|mPos - mPosPrev|^2 > 0.001` at the end of tick t-1. That is the squared
3D displacement the entity actually made in its previous movement tick (x and z count). Motion and
on_ground play no part. The check is strict `>`, and the threshold works out to about 0.0316
blocks/tick.

It applies only to entities that have `ActorMovementTickNeededComponent`, i.e. entities that ran a
movement tick. An entity riding a vehicle that has `ControlledByLocalInstanceComponent` is skipped
and keeps its old state.

## BDS (build cache 6ab54bee, same exe as `.testserver/bds-molang`)

- `UpdateMovingFlagSystem::fn0` @ `142c26b80` (tick `142c27030`, vtable `14a7d2680`, registered in
  `1418ba620`) runs after movement. Its view covers `ActorMovementTickNeededComponent` +
  `StateVectorComponent` (stride 0x24: `mPos`, `mPosPrev`, `mPosDelta`). It computes
  `(pos-posPrev)·(pos-posPrev) <= 0.001`. If true it removes `SetMovingFlagRequestComponent`;
  otherwise it emplaces one.
- "CacheMovingState" @ `14191abb0` (std::function vtable `14a75dab0`, registered in `141901a60`
  right after `CurrentSwimAmountSystem`) runs early in the next movement tick. It sets
  `ActorDataFlagComponent` bit 0x22 = has(`SetMovingFlagRequestComponent`), and if the bit changed
  it sets `ActorDataDirtyFlagsComponent |= 1`. It also copies rot to rotPrev. The passenger
  variant "PassengerCacheMovingState" is `14191d4f0`.

The request is computed in one tick and applied in the next. That is where the one-tick lag comes from.

## Fit to research/captures/bds-water-flags.txt

The rule predicts every flag packet for items 18, orb 19 and armor stand 20. I checked each move
packet against the flag state shown next. The only exceptions are packets that are not real
per-tick displacements: a move BDS did not send, and first moves measured from the Add packet.

| event | prev-tick displacement² | result |
|---|---|---|
| item spawn 32058, MOVING set 32109 | 0.033 (spawn tick) | set |
| item lands 33358 | 0.0046 (landing tick) | stays set 33412 |
| 33464 cleared | 0.000116 (33412: dx .0107, dy .0011) | clear |
| 36608 not set | 0.000991 (36563) | off |
| 36657 set | 0.001001 (36608: dy .03160 + dx .00155; dy alone is 0.000999) | set |
| 39957 stays set (gravity change) | 0.0013 (39913) | set |
| 40015 cleared | ~0 (39957 top of bob, no move sent) | clear |
| 40064 set | 0.0024 | set |
| 40311 stays, 40365 cleared | 0.00110 (40263), 0.000966 (40311) | set, clear |
| orb spawn 40063, set 40112 | 0.039 (40064 vs AddActor pos) | set |
| orb 43417 cleared | 0.000998 (43359); 0.00105 before | clear |
| stand spawn 48058, set 48160 | 0 in spawn tick, 0.0061 at 48112 | set at spawn+2 |
| stand lands 49107, cleared 49159 | 0.000203 (landing tick, vs 0.0151 before) | clear |

The data put the threshold between 0.000998 and 0.001001, which matches the constant 0.001.

## Versus sapling

`sapling-shard/src/sync.rs` `flags` sets MOVING = "the previous tick started in the air". BDS
instead uses "the previous tick's position change squared > 0.001". The two differ in these cases:

- **Slow movers in the air or water clear.** Examples: an item settling in water, an orb drifting
  at ~0.03/tick, and the top of a bob.
- **Fast movers on the ground stay set.** Example: an item sliding on the ground.
- **The landing tick itself counts.** It clears at landing+2 for an item only because the landing
  tick still moved more than 0.0316. An armor stand whose landing move is small (0.014) clears at
  landing+1.
- **A spawn tick with no movement gives no flag.** The armor stand did not move in its spawn tick,
  so it was set at spawn+2.
