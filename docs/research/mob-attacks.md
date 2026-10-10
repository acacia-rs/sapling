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
- Glass hides a player as stone does (`around`, research/captures/bds-around.txt): behind a
  glass wall five wide the zombie strolled until it was past the wall's end, then took its target
  and walked straight at it; in a glass pen it took none.
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

## Other kinds (`kinds`)

Seven more on the same floor at midnight (`research/captures/bds-kinds.txt`). All take a target,
walk and hit as the zombie does, with their own numbers:

| kind | movement | blocks a tick | hit on easy | on a hit |
| --- | --- | --- | --- | --- |
| husk | 0.23 | 0.11419 | 2.5 | hunger, 600 ticks (`Update` on each later hit) |
| zombie villager | 0.23 | 0.11419 | 2.5 | |
| spider | 0.3 | 0.19427 | 2 | |
| cave spider | 0.3 | 0.19422 | 2 | poison for 0 ticks, added and removed |
| silverfish | 0.25 | 0.13491 | 1 | |
| vindicator | 0.35 | 0.26443 | 7.5 | |
| wither skeleton | 0.25 x 1.25 | 0.21080 | 5.5 | wither, 200 ticks |

- The speed is `0.98 x (movement x speed_multiplier)^2 / 0.454` for each: the push is the square
  of the speed, and the attack's `speed_multiplier` is inside it.
- The reach is the box rule with the kind's own width: a spider (1.4 wide) first hit from 1.8.
- The hit on easy is `min(attack, attack / 2 + 1)`, where `attack` adds what the mob holds: a
  vindicator's 8 and its iron axe's 5, a wither skeleton's 4 and its stone sword's 5.
- A silverfish 8 blocks off (its `max_dist`) took no target; 4 off it did.
- `/summon zombie_villager_v2` is a syntax error; `zombie_villager` summons one.
- The sounds come from the same shares of each kind's height.

## Strolling (`stroll`)

A zombie and a cow left alone for a minute on a floor 40 blocks square, the actor in creative
(`research/captures/bds-stroll.txt`):

- Each walked in straight lines, 1 to 10 blocks at a time, six or seven times in the minute, and
  stood between for 1 to 15 seconds.
- The zombie strolled at 0.11419 a tick, the speed it hunts at. The cow went 0.08634: its
  movement 0.25 times its `random_stroll`'s `speed_multiplier` 0.8, by the same rule.
- Each walk ends with one more move of about 0.006.
- A chicken strolled at 0.135 (movement 0.25).

## Fleeing (`flee`)

A cow and a pig, each hit once by the actor in creative (`research/captures/bds-flee.txt`; the
chicken and the zombie had strolled out of reach):

- The hit throws it as measured before: 0.4 in its first tick, 0.218, then nine ticks in the air.
- Landed, it turns and runs: its steps grow over some fifteen ticks to 0.2108, which is
  `0.98 x (0.25 x 1.25)^2 / 0.454`, its movement times its `panic`'s `speed_multiplier`.
- The cow ran one stretch and stood 60 ticks after the hit; the pig ran three, turning between,
  and stood after 80. Then each strolled again at its own pace.

## Not drilled

Normal and hard (taken as 3 and 4.5), a way around a wall (a mob that cannot see takes no
target, so the drill showed none), a hole in the way, what hides a player besides a full block, whether prey that goes behind a wall is kept, a zombie hit
back, daylight (a spider hunts only in the dark), baby zombies, a cave spider's poison on
normal and hard (7 and 15 seconds in its definition), the kinds not named here, and what `within_radius` 25 does: a zombie on
open grass 30 off took and dropped its target every 10 ticks, one on the stone floor kept it.

## Sapling

Every kind that walks strolls (`creatures/strolling.rs`, `strolls=` in `mob-traits.tsv`, the
shard's `mobs_stroll`): a tick in 120 it picks a place up to 10 blocks off along each axis and
walks straight there at its stroll speed, giving up at an edge, at a block in its way or after ten
seconds. How often and how far are fitted by eye to the two mobs above. A mob a player hit, of a
kind with `panics=`, runs for 70 ticks from place to place within 5 blocks at that speed.

`creatures/hunting.rs`, for kinds with `attack=`, `speed=` and `sees=` in `mob-traits.tsv`: the
zombie and the seven kinds above, each with its numbers and what its hit gives (not the cave
spider's poison, which lasts no time on easy).
The first two hits of `bite` and both of `slain` land where BDS's did, to the last digit.
Its jump over the step of `chase` is BDS's to the sixth decimal.
Different: it walks straight at the player (no path around anything), keeps prey it has seen
whatever comes between, sends a mob's speed as it is before the drag, turns 60
degrees a tick with its head, stops when its box touches the
player's since nothing pushes anything, does not let go as it is killed, shows no mob holding
its weapon, lets a spider hunt by day, hunts nobody on peaceful
(where BDS has no monsters), and tells every viewer every move.
