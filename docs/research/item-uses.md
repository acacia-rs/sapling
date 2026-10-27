# Items in use: elytra, buckets, books, stations, small items

What BDS did, drilled with `bds_command_probe` (`crates/sapling/examples/shared/items_drill.rs`) on
devbox's BDS; the captures are `research/captures/bds-<drill>.txt`, sapling's `sapling-<drill>.txt`.

## Elytra (`glide`; `gliding.rs`)

- **Start**: the client's `StartGliding` in an input off the ground. BDS answered with SetEntityData
  `{Flags + GLIDING, BoundingboxHeight 0.6}` stamped with the input's tick, to the player and its
  viewers. No PlayerAction.
- **Stop**: `StopGliding` in the air, or the server's own: SetEntityData `{Flags, BoundingboxHeight 1.8}`.
  With the elytra worn to its last use (Damage 431 of 432) BDS stopped the glide at once by itself and
  then corrected the falling client (CorrectPlayerMovePrediction) each 300 ms; the elytra stayed on.
- **Wear**: PlayerArmorDamage with the chest alone (slot 2 on the wire, decoded `Leggings`) and its new
  Damage, once a second of gliding (the first 18 to 20 ticks in; now and then two 50 ms apart, BDS
  catching up ticks). Survival only in sapling, as Java's `hurtAndBreak` spares creative.
- **Boost**: the rocket's ClickAir while gliding: MovementEffect `GLIDEBOOST` with duration twice the
  rocket's life (44 for a 22-tick rocket), stamped with the input tick; the windows resent with
  PlayerHotbar `select_slot true`, one rocket spent; AddEntity `fireworks_rocket` at the glider with
  only the launch wobble for velocity, `launch` heard; it bursts at the end of its life as any rocket.
  BDS also sent the glider's whole UpdateAttributes and SetEntityData then, which sapling does not.
- Not drilled: water, levitation, riding and creative flight ending a glide; the fly-into-wall damage
  (combat's).
