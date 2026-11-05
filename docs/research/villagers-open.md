# Villagers: not drilled and left out

What of BDS's villagers sapling has not checked or does not copy; split from villagers.md.

## Not drilled

- Gathering and the other schedule goals' walks beyond the home and bed counts above; standing's
  numbers (no packet shows them).
- Fear in the open: `n1fear` (bds-n1fear.txt, sapling-n1fear.txt, 2026-11-04) held a zombie still in
  the open by two villagers: BDS's both stayed within 8 of it for 15 s, one within 4 for 9 s; sapling's
  fled (0 or 1 within 8). Not fixed: as with the glass cage, BDS's `behavior.avoid_mob_type` did not
  fire for a still zombie; its `canUse` (`probability_per_strength`, the target's strength) was not
  found in the binary, and a hunting zombie was not drilled.
  `f3fear` (2026-11-04, bds-f3fear.txt, sapling-f3fear.txt; villagers within 8 of the zombie each
  second for 15 s): in the village by a hunting zombie BDS `2 2 1 0 ...`, sapling `1 2 2 1 1 1 0 0 1 ...`;
  200 east with no beds or bell, still, BDS `1 0 0 ...`, sapling `0 0 ...` (both fled); there hunting, BDS
  `3 0 0 ...` (gone in a second), sapling `3 1 3 3 2 2 2 1 ...` (never shook it off). The avoid goal was
  decompiled (vtable 14a75d720: `canUse` 14190daa0, `findNearest` 14190d490 takes the first fitting
  mob in its box, not the nearest; `canUse` then needs sight, a `RandomPos::getPosAvoid` spot
  (14597bd90, kept to the mob's home and leash restriction) farther from the threat, and a path).
  `f3fear2` (bds-f3fear2.txt; the zombie summoned 60 off, then put 4 from the pair) undid any village
  rule: in the village both fled from a still zombie and a hunting one within 1 to 2 s, while alone one
  villager stayed within 8 of a still zombie for 8 s and of a hunting one for most of 15 s. BDS's
  fleeing is as often late as not, which fits a random spot search that fails; sapling flees straight
  away at once. Not fixed: one run per case does not fit the search.
  Re-drilled 2026-11-05 (twelfth wave, group 11): three BDS runs (bds-f3fear2.txt, -2, -3) and two
  sapling runs (sapling-f3fear2.txt, -2); villagers within 8 each second for 15 s, per case (village
  still, village hunting, alone still, alone hunting):
  BDS `2 0 0 …`, `1 2 1 1 1 1 1 1 1 1 1 1 1 1 0`, `1 0 0 …`; `2 2 0 …` three times; `2 1 1 1 1 1 1 1 1 0 0 0 0 1 1`,
  `1 0 0 …`, `2 1 1 1 0 …`; `1 0 0 1 1 1 1 1 1 1 1 1 2 1 1`, `2 2 2 1 1 1 1 0 1 0 …`, `2 2 0 …`.
  Sapling `1 1 0 …`, `0 …`; `2 2 0 0 1 1 1 1 1 1 1 1 1 1 1`, `0 …`; `1 0 …`, `1 1 0 0 0 1 0 0 1 1 0 …`;
  `1 0 …`, `1 1 1 0 1 1 1 1 1 1 1 1 1 1 1`. Both sides range from gone in a second to lingering most of the
  15 s in each case, so the one-run gap was chance: the same as BDS within the spread, left as it is.
- The schedule's re-check delay (`max_delay_secs`, up to 10 s); here every tick, which matched the
  drilled sleep and wake ticks.
- Farmers farming, cartographers' explorer maps (map items that locate a structure) and entity
  vibration listeners (the allay's; the warden's hearing is in creatures/warden/hearing.rs) were not
  reached in the twelfth wave's group 8 (2026-11-05); the schedule walks' counts are strolls, random on
  both sides. Group 11 (2026-11-05) spent its runs on breeding and fear and did not reach them either:
  each is a port with no sapling side yet (no farming goal, map item or vibration listener for mobs),
  not a drill of something built. The walks were seen in passing: in the `g11love` runs both sides'
  pairs strolled 10 to 35 apart within a minute (their `MoveEntityDelta`s).
- Job-site range: `f3poi` (villagers.md "Unverified"; a held villager took none).
- A baby's hurt standing (-3, ten hits to -30) moved no BDS golem within 30 s, not explained
  (villagers.md "Standing": the defend goal's code path read, the cause not found).
- The aggressor a golem defends against: any attacker of a villager or golem goes on the village's
  list (villagers.md "Standing"); only a player's hand hit on a grown villager is copied (`golems.rs`),
  not a mob attacker's. Ranged kills, the founding reward and the cure's +5: done (villagers.md). Standing is kept as BDS's `VILLAGE_<id>_PLAYERS` (`Players`, each `ID` long and
  `S` int; 0x142a2c140), the id sapling's village uuid and `ID` the low half of the player's uuid (sapling
  players keep no `ActorUniqueID`).

## Left out

Gossip (Bedrock has none: its discounts are the cured ones of `economy_trade_table`, which a drilled
cure did not give on BDS, `g8cured`), villages as BDS's dweller-claimed POIs, mingling partners, babies
growing up, explorer maps (their trades are never offered), the llamas' leashes and their going with
their trader, a villager sharing food (`behavior.share_items`; villagers.md "Breeding"), farmers
farming, cats spawning, the old trading screen, a villager's pantry kept in its actor compound.
