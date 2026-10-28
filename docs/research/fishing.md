# BDS fishing (2026-10-27)

Sources: BDSRE on `acacia/.testserver/bds-molang/bedrock_server.exe` (`FishingRodItem::use` 143700ef0,
`FishingHook::normalTick` 1429c8960, `retrieve` 1429c6b50, `_pullCloser` 1429c69b0); capture
`research/captures/bds-fish.txt` (drill `fish` of `bds_command_probe`, BDS 1.26 on devbox). The `!fish` sections'
timestamps are bunched: the probe reads nothing while the bot waits for a bite, so order holds but times do not.
sapling: `crates/sapling-shard/src/fishing.rs` and `fishing/`, loot `crates/sapling-data/src/mob_loot/fishing.rs`.

## Cast (capture)

- The hook leaves the drop point, 0.3 under the eyes and 0.15 ahead (horizontal), plus the fisher's motion
  (a standing player's `(0, -0.0784, 0)`), at **1.3 along the look, no spread** (three casts: |v| = 1.3, z = 0).
- AddEntity `minecraft:fishing_hook` with `OwnerEid` = the fisher's unique id, yaw 90 for a cast east.
- LevelSoundEvent `throw` (entity `minecraft:player`) at the drop point, then CompletedUsingItem {395, -1}, the
  four inventory windows and PlayerHotbar (select false). A reel answers the same way.

## Flight and bob

Per tick (normalTick, then the projectile's own move):
1. friction 0.92 (0.5 on the ground); out of water `vy -= 0.04`, in water by the five-slice wet part
   (`vy += (2 wp - 1) * 0.04`, then `f *= 0.9`, `vy = min((target - bob) * 0.15, vy * 0.8 + 0.05)`).
2. `v *= f`, move; then move by `v` again, `v *= 0.99`, `vy -= 0.05`.
3. A move cut short by a block stops it dead (SetEntityMotion 0, no second move); that tick counts as in the
   ground. The next tick's friction is 0.92 again.
Exact against the capture through the first tick in water; later bob ticks drift about 1e-3.

## Wait and bite (BDSRE; capture shows the events)

Lure base `nextInt(25) + 5 - 5 * Lure` seconds (quartered near water animals), then teases (FishX/FishZ data +
ActorEvent 14) more often as the fish nears, `hooked = nextInt(60) + 20` ticks of FishX/FishZ/FishAngle data
with ActorEvent 12 while the fish's spot is water, ActorEvent 13 at the bite, then 10-29 ticks of nibble.

## Reel

- Rod wear: hooked entity 3, biting fish 1, nothing 0, against a block 2 (overrides). Capture: stone cast 0 -> 2,
  early reel in water + 0, catches + 1, a hook against the pig's platform + 2.
- A catch: AddItemEntity at the hook, `is_from_fishing` set, velocity toward the fisher:
  `d = (feet + 2.52 - item middle) * 0.1` per axis (y also `+ sqrt(|d|) * 0.08`); three catches agree to 1e-6.
- Experience: `nextInt(5) + 1` per catch, as orbs at the feet + 0.7 up and half a block along the look, split
  as any orbs are (4 comes as 3 + 1: the capture's one or two orbs per catch are that split, not a rule).
- Cast (fishing/cast.rs): a `disappeared` sound at the fisher's feet + 0.9 before `throw`; the hook comes in
  flagged Invisible, a tick later Invisible gives way to CanShowNametag and Moving, a tick after that Moving
  goes with a `reappeared` sound at the hook + 0.075. The fisher shows using an item (flag 4) for the use's
  tick only.

## sapling gaps

- The nibble's downward tug sinks sapling's hook deeper than BDS's (catches 0.1-0.6 under the surface on BDS,
  up to 1.3 in sapling, 2026-10-28). The decompile adds `-0.5 * r1 * r2 * r3` to motion.y after the hook's
  own move; moving the tug in between sapling's two moves did not change the depth.
- A hooked mob or player: from BDSRE only; the drill's pig walked off its platform.
