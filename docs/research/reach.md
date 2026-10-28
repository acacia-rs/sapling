# Reach

What BDS 1.26.52.3 accepts from a player's eyes. Measured 2026-10-27 with acacia's
`crates/acacia-bot/examples/reach_drills.rs`, which sends the packets directly, past acacia's own reach
guards. The static reading is in acacia's `docs/confirmed-actions.md` "Reach and line of sight".

- **Block clicks** (`ItemUseTransaction`): the eye to the block's minimum corner `(x, y, z)`, with no +0.5,
  at most 7.2 in survival and 12.5 in creative.
  - Survival: corner 7.10 (middle 7.56) was placed; corner 7.30 (middle 7.76) was refused, and the area
    was resent.
  - Creative: 12.40 was placed and 12.60 refused.
  - The older middle-based readings (6.4 and 12.2 taken, 7.3 and 13.2 refused) fit this too.
- **Hits** (`ItemUseOnEntity` Attack): the eye to the target's position (a mob's feet), at most 6.7 in
  survival and 12 in creative. A cow 6.40 off was hurt; one 7.00 off got no answer.
  - BDS also wants the packet's `fromPos` within 6.0 of where it has the player. sapling does not check this.
- **Breaking** (server-authoritative): 6.7 or 12 × `server-authoritative-block-breaking-pick-range-scalar`
  (1.5 by default), from the eye to the corner.
- **Line of sight:** BDS checks none for clicks, breaks or hits. A block or mob behind a wall is reachable.

sapling: `blocks.rs` `reaches` checks clicks and `creatures/attacks.rs` checks hits. Player hits
(`duels.rs`) keep their own 4.0.
