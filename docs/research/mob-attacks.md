# Mobs that hit back

What BDS does when a zombie and a survival player share a floor, from `bds_command_probe bite`,
`slain` and `chase` (`research/captures/bds-bite.txt`, `bds-slain.txt`, `bds-chase.txt`; sapling's
are beside them), and the
zombie's definition in the vanilla pack (`entities.brarchive`, `zombie.json`). Easy difficulty.

## Picking a player

- The definition: `nearest_attackable_target` with `max_dist` 35 for players, `must_see`,
  `within_radius` 25; `melee_box_attack`; `attack` 3; `movement` 0.23.
- A zombie summoned 8 or 30 blocks from the player took it as its target 1 to 9 ticks later: it
  looks about every 10 ticks. One 40 blocks off never did.
- It tells its viewers by `SetEntityData` `TargetEid` (the player's unique id, 0 when it lets go)
  and, a tick later, the sound `mob.warning` at 0.85 of its height.
- A player in creative is not taken; turned to survival it is, within the 10 ticks. Turned to
  creative, or dead, it is let go a tick later.
- A wall two high between them, across the whole floor or five wide, and it took no target; a
  step one high did not hide the player.
- With no target the zombie looks around and strolls, at the same speed.

## Walking

- It turns first: its yaw went 0, -30, -90, -149, 180 in four ticks, its head more slowly.
- Each tick its speed across is `0.546 x speed + push`, along its yaw. The push grows over its
  first ten ticks as `0.98 x (0.23 x n/10)^2` and stays `0.98 x 0.23^2 = 0.05184`, so it ends at
  0.11419 blocks a tick.
- The turn carries it up to 0.009 off the line; it is back on it within a block.
- It walks on at full speed through its own hits.
- Another player in its way stops it: the watcher's box held it 0.55 off for seven seconds.
- The actor was sent every move; the watcher, 20 blocks further, every other one.

## A block in the way (`chase`)

- With the face of a block one high 0.46 from its middle (0.57 the tick before) it jumps: up 0.42
  in that tick, then `(v - 0.08) x 0.98` as anything falls, 1.25 at its highest.
- In the air its push is `0.02 x 0.98 x 0.23 = 0.0045` a tick and the drag across is 0.91.
- The drag of a tick goes by whether it stood on the ground before that tick's move: the tick
  after it lands is still slowed by 0.91, the tick after it walks off an edge by 0.546.
- `SetEntityMotion` goes out as its speed changes by a lot, holding the speed after the tick's
  drag.

## Hitting

- It hits in the first tick in which its box, 0.8 wider on each side, overlaps the player's,
  measured after that tick's move (1.4 between their middles), and again 20 ticks later while that
  holds.
- In the hit's tick: `EntityEvent ArmSwing` of the zombie, `SetEntityMotion` of the player and the
  player's `HurtAnimation` with 2 (entity attack); its health follows as after any hit. A tick
  later the sound `attack` at half the zombie's height, where it stood as it hit.
- 2.5 of health on easy: `3 / 2 + 1`.
- The player is thrown at 0.4 away from the zombie across and 0.4 up. The bot landed 2.07 blocks
  on, so a zombie that keeps hitting does so every 20 ticks.
- Being hit costs no exhaustion.
- The hit that kills throws nobody and flashes nothing. The death reads `death.attack.mob` with
  the player's name and `%entity.zombie.name`, in `DeathInfo` and to the world.
- `/difficulty easy` answers `commands.difficulty.success` with `EASY`.

## Not drilled

Normal and hard (taken as 3 and 4.5), a way around a wall, a hole in the way, what hides a
player besides a full block, whether prey that goes behind a wall is kept, a zombie hit
back, armour, daylight, baby zombies, any other kind, and what `within_radius` 25 does: a zombie on
open grass 30 off took and dropped its target every 10 ticks, one on the stone floor kept it.

## Sapling

`creatures/hunting.rs`, for kinds with `attack=` and `speed=` in `mob-traits.tsv` (the zombie).
The first two hits of `bite` and both of `slain` land where BDS's did, to the last digit.
Its jump over the step of `chase` is BDS's to the sixth decimal.
Different: it walks straight at the player (no path around anything), keeps prey it has seen
whatever comes between, sends a mob's speed as it is before the drag, turns 60
degrees a tick with its head, stands still with no target, stops when its box touches the
player's since nothing pushes anything, does not let go as it is killed, hunts nobody on peaceful
(where BDS has no monsters), and tells every viewer every move.
