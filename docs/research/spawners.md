# Monster spawners, trial spawners and vaults (BDS)

What sapling copies of BDS's three spawner blocks (`crates/sapling-shard/src/spawners*`), from where, and
what is left. All decompiled from the bds-blocks build (`BDS_EXE=acacia/.testserver/bds-blocks/bedrock_server.exe`,
cache `6ab54bee`; labels in `bdsre/names.user.json`). [U] = unverified (believed, no evidence here). Nothing
here was drilled against a running BDS.

## Monster spawner (`MobSpawner`)

| VA | What |
|---|---|
| 14165cf60 | `BaseMobSpawner` ctor (defaults) |
| 14165d820 | `BaseMobSpawner::tick` |
| 14165d590 | `BaseMobSpawner::_delay` |
| 14165e190 / 14165f3f0 | load / save |
| 14118caf0 | `Spawner::spawnMob` |
| 1429ea2e0 / 14298fc00 / 14298f930 | `Mob::checkSpawnRules` / `Monster::checkSpawnRules` / `Monster::isDarkEnoughToSpawn` |
| 143906990 | after a spawn: LevelEvent 2004 |
| 14486c6e0 | `MobSpawnerBlockActor::setMob` (a spawn egg) |
| 142a76040 | `SpawnEggItem::_useOn` |

- NBT: `EntityIdentifier` String; `Delay`, `MinSpawnDelay`, `MaxSpawnDelay`, `SpawnCount`, `MaxNearbyEntities`,
  `RequiredPlayerRange`, `SpawnRange` all **Short** (defaults 20, 200, 800, 4, 6, 16, 4); `DisplayEntityWidth`,
  `Height`, `Scale` Float (0.8, 1.8, 1.0); `SpawnData` Compound (`TypeId`, `Weight`, `IsBaby`, `Size`,
  `equipment`) and `SpawnPotentials` List, which name the next kind over `EntityIdentifier`. BDS reads the
  delay group only when `MinSpawnDelay` is a Short, the range group only when `MaxNearbyEntities` is.
- An empty `EntityIdentifier` leaves the spawner inert: no countdown.
- Tick: a player (creative counts, spectators do not) with `dist²(player pos, block + 0.5) < range²` (a player's
  position is its eyes [U]); then `Delay > 0` counts down; `-1` resets; `0` makes `SpawnCount` attempts. Any
  spawn resets the delay; none leaves it at 0, to try next tick.
- Reset: `Delay = Min + nextInt(Max - Min)` (when Max > Min), a weighted pick of `SpawnPotentials` into
  `SpawnData`, and BlockEvent(pos, 1, 0) to clients (the client's own countdown). The countdown itself is
  never sent.
- Attempt: `x = R·(f - f) + bx + 0.5`, `y = by + nextInt(3) - 1`, `z` as x. Skipped where the cell is a full
  solid block or `|dx| + |dy| + |dz| > R` (cells). Mobs of the kind in the box `(b - 2R, by - 4) .. (b + 2R,
  by + 4)` from the block's corner (16 × 8 × 16 at R 4, not Java's 9³) at `MaxNearbyEntities` or more end the
  attempts with a reset.
- The kind's own C++ check, not its JSON spawn rules: no collision in its box; a monster also needs raw
  brightness under 8 (a storm reckoned with sky darkening 10) and, outside the nether, block light 0.
- After a spawn: LevelEvent 2004 (`ParticleSpawn`) at the block's corner, data `ceil(width) + (ceil(height) << 8)`.
  The mob is a natural spawn: nothing makes it persistent, and sapling lets it despawn as one (`spawning::Natural`).
- Neither `doMobSpawning` nor difficulty is read. On peaceful a monster spawns and removes itself on its
  first tick; sapling, which does not remove monsters on peaceful, counts it spawned and spawns nothing.
- Spawn egg on it: any game mode, spent outside creative, no sound; sets `EntityIdentifier` and sends the
  entity (BlockActorData) at once. The delay and any `SpawnData` stay, so a `SpawnData` still wins (copied).

## Trial spawner (`TrialSpawner`)

| VA | What |
|---|---|
| 1419d7000 | `TrialSpawnerConfigRegistry` ctor: every built-in config |
| 141191210 | `TrialSpawnerConfig` default ctor |
| 141195c10 / 141196380 | `TrialSpawner::load` / `save` |
| 14488a0b0 | `TrialSpawnerBlockActor` tick (`TrialSpawner::tick` inlined) |
| 14119a530 | `_tryRegisterNewPlayers` |
| 141198120 | `_spawnMob` |
| 141191c10 | `getOrCreateNextSpawnData` |

- Configs live in the server, not the packs: `normal_config` / `ominous_config` name one
  (`minecraft:trial_chamber/<kind>/normal|ominous`) or hold one inline. Default: 6 mobs in all, 2 at once,
  +2 and +1 per player after the first, 20 ticks between, range 4, cooldown 36000; rewards `spawners/
  trial_chamber/key.json` and `consumables.json` 1:1 (ominous: the `spawners/ominous/` pair 3:7). The kinds
  (total / at once / +total / +at once / ticks; `-` default):

  | config | mob | normal | ominous |
  |---|---|---|---|
  | breeze | breeze | 2/1/1/0.5/- | 4/2/1/0.5/- |
  | melee/husk, melee/zombie | husk, zombie | -/3/-/0.5/- | same |
  | melee/spider | spider | -/3/-/0.5/- | 12/4/-/0.5/- |
  | ranged/{poison_skeleton,skeleton,stray} | bogged, skeleton, stray | -/3/-/0.5/- | same |
  | slow_ranged/{same three} | as ranged | -/4/-/2/160 | same |
  | small_melee/baby_zombie | baby zombie | -/-/-/0.5/- | same |
  | small_melee/{cave_spider,silverfish,slime} | as named | -/3/-/0.5/- | 12/4/-/0.5/- |

- NBT: `spawn_data` (the wave's kind, kept for the whole cycle), `required_player_range` Int 14,
  `registered_players` and `current_mobs` (lists of `{uuid: Long}`, `UniqueID`s), `total_mobs_spawned` Int,
  `next_mob_spawns_at` and `cooldown_end_at` Long (game ticks), `selected_loot_table` String. Ominous is the
  block's `ominous`; the state is `trial_spawner_state` 0 inactive, 1 waiting for players, 2 active,
  3 waiting for reward ejection, 4 ejecting, 5 cooldown.
- Every tick: no kind to spawn (`spawn_data`, else the config's potentials) and it does nothing; its
  `current_mobs` lose the dead, the gone and those over 47 from its corner (not on the first tick after a load).
- 0 → 1 once it has a kind. In 1 and 2, peaceful [U: a level query that returns 0] or `doMobSpawning` off
  clears the wave and the players (2 → 1). Players are looked for when `(h²·0x285b825 + h·11 + tick) % 20 == 0`,
  `h = z·0x6ebfff5 ^ x·0x2fc20f ^ y`: not creative or spectator, alive, `dist²(pos, block corner) <= range²`,
  and seen from the block's middle while none is taken on yet. New ones: LevelEvent 3611 (data: how many),
  next spawn no sooner than 40 ticks; 1 → 2.
- 2: a mob while spawned < total + per-player·(n - 1), alive < at-once + per-player·(n - 1) and the time has
  come; then the next after `ticks_between`. Spot as a monster spawner's, but only a solid cell, the Manhattan
  range and a line of sight from the middle stop it: no light or room check. Sound `trial_spawner.spawn_mob`,
  LevelEvent 3612 at the mob and at the block (data: ominous). The mob is persistent (`PersistentComponent`).
  All spawned and none left: sound `trial_spawner.open_shutter`, cooldown from now, → 3.
- 3: 40 ticks on, one of the config's loot tables by weight → 4. 4: every 30 ticks from the wave's end, one
  roll per player taken on, ejected from `(x + .5, y + .875, z + .5)` at `0.2` up plus a normal spread of 0.045,
  sound `trial_spawner.eject_item` and LevelEvent 3613 at `(x + .5, y + 1, z + .5)`; none left: sound
  `trial_spawner.close_shutter` → 5. 5: at the cooldown's end everything clears, `ominous` too → 1.
- Ominous: a player with Bad Omen in range has it turned into Trial Omen (18000 · (level + 1) ticks), and Trial
  Omen makes the spawner ominous (LevelEvent 3616, its mobs removed, the ominous config) and, while active,
  every 160 ticks drops an `ominous_item_spawner` over a player.

## Vault (`Vault`)

| VA | What |
|---|---|
| 14488afe0 | `VaultBlockActor` ctor (default config) |
| 14488dcf0 / 14488e5e0 | load / save |
| 14488bc50 | tick (read with `dis`) |
| 144892210 | `updateConnectedPlayersWithinRange` |
| 144891cd0 | `onTransitionBetweenStates` |
| 14488f770 | `tryInsertKey` |

- NBT: `config` {`loot_table` (`chests/trial_chambers/reward.json`), `activation_range` 4, `deactivation_range`
  4.5 (at least the first), `key_item`, `override_loot_table_to_display`}; `data` (not Java's `server_data`)
  {`display_item`, `state_updating_resumes_at` Long, `items_to_eject`, `rewarded_players` (Longs), 
  `total_ejections_needed` Long}; `shared_data` only `connected_particle_range` is read. `vault_state`
  inactive, active, unlocking, ejecting. Its times (and a trial spawner's `cooldown_end_at`,
  `next_mob_spawns_at`) are on the level's clock (`currentTick`, sapling `Level::age`, spawners.rs `now`);
  a generated vault is dated with the level's age as its column arrives (sapling-leveldb `date_generated_vaults`).
- Every 20 ticks while active the shown item is rolled anew (one item of a roll of the table). At
  `state_updating_resumes_at`: inactive → active when a player (spectators no, creative yes) not yet
  rewarded is within 4 of the block's middle; active → inactive when none is within 4.5; unlocking →
  ejecting; ejecting pops one item each 20 ticks from `(x + .5, y + 1.075, z + .5)` at `(t, 0.2 + t, t) · 0.034455`
  [U: one `t = f - f` per axis], sound `vault.eject_item` with pitch data, LevelEvent 9813; empty: back to
  active or inactive. Transitions: from ejecting `vault.close_shutter`; to inactive LevelEvent 9812 and no
  display; to active 9811; to unlocking `vault.insert_item`; to ejecting `vault.open_shutter`.
- Key: on an active vault, the wrong item `vault.insert_item_fail`, a rewarded player
  `vault.reject_rewarded_player` (each at most once in 15 ticks); else the table is rolled (an empty roll does
  nothing), the key spent outside creative, unlocking for 14 ticks, the player remembered (128 at most).

## Sapling

`spawners.rs` ticks every spawner block whose column a player sees (ticking.rs's sweep) and every one a
plugin or an egg just set; state lives in the block entity and the block state, so a saved world keeps it.
Rewards are rolled from BDS's own tables (`sapling-data` `table_loot`, `mob-loot.json`). Times are this
shard's ticks; a world BDS saved counts BDS's, so stored deadlines are clamped to at most one wait ahead.
Players are known by their kept `UniqueID` (`tags::Marks`). Generated spawners, trial spawners and vaults
get the block entities BDS writes (docs/research/structure-block-entities.md).

Left out:
- Bad Omen and Trial Omen (no such effects yet), so a spawner turns ominous only as a world holds it; the
  ominous item spawner. Ominous vaults use `reward_ominous.json` and the ominous key [U: BDS has these from
  their template's NBT].
- `SpawnData` equipment, `IsBaby` and slime sizes (spawned plain); the 3 blocks a non-fire-immune mob may not
  spawn on and `walkTargetValue` (unknown); the client-side flame and smoke; the vault's luck and its seeded
  loot RNG.

## Unverified

- A player's position as its eyes for every range check here; the 3615 charged-detection event; that a
  trial spawner's line of sight is a clip through any block with collision (BDS: a real ray clip, max 200).
  Drilled 2026-11-05 (`y3trial`, bds-y3trial.txt; the trial spawner idles with `doMobSpawning` off on BDS
  too, as `spawners/trial.rs` has it): a spawner 15 over the actor's feet, its eyes 13.9 under it, took
  the actor (`ParticleTrialSpawnerDetection` data 1), so range is from the eyes; one 8 off behind a stone
  wall did not in 8 s; one behind glass and one in the open did, 6 ticks after the survival switch.
  `y3omen` (bds-y3omen.txt): eight ominous spawners told their detection as event 3615 (the wire's
  `ParticleWolfArmorBreak`) at their corners. Sapling (sapling-y3trial.txt) matched at the eyes, stone and
  open sites but not behind glass. Fixed: glass (any block named `*glass`) no longer blocks the sight, as
  Java's visual-shape clip; panes, leaves and the other see-through blocks were not drilled.
  `ab3sight` (2026-11-05, bds-ab3sight.txt, sapling-ab3sight.txt): BDS's spawners 8 off behind walls of
  glass panes, iron bars, tinted glass and red stained panes each spawned zombies; the one behind
  persistent oak leaves none. Sapling spawned only behind tinted glass. Fixed: panes and bars (names
  ending `glass_pane` or `_bars`) no longer block either, leaves still do (`spawners.rs` `in_sight`, test
  `a_spawner_sees_through_panes_bars_and_tinted_glass_but_not_leaves`).
- The eject velocity's odd x/z term BDS's trial spawner adds (left out); the vault's per-axis spread. One BDS
  reward (bds-t3trial.txt): item at x+0.5, y+1.0, z+0.5, moving (0.028, 0.156, 0.010), `eject_item` there
  with extra data -718719502. Sapling's wave there never ended (its zombies did not burn by day, BDS's did)
  and the vault click missed on both, twice; undrillable here. Re-drilled 2026-11-04 with
  `SAPLING_DRILL_WORLD=open` (sapling-t3trial.txt): the first run was on the roofed default world. In
  the open sapling's zombies burned, the wave ended and the reward came out, from y + 0.875 at
  (−0.040, 0.257, 0.010) with extra data -1; now from y + 1.0 as BDS's, and `eject_item` carries the
  block's hash (`trial.rs` `reward`). The velocity stays a 0.2 up throw with a 0.045 normal spread (one
  BDS reward is too few to fit). The vault click still missed on both.
- What a spawn egg does to a trial spawner's configs (`14488a3a0`; no packet shows them).
- The vault's level events' positions (sent at the block's middle). Settled by x2ran below: its corner.

## Parity drill (2026-11-04, `bds_command_probe t3trial`, bds-t3trial.txt)

- A zombie egg on a bare trial spawner: BDS showed the empty entity, then state 1 with an entity of
  `spawn_data` `{TypeId minecraft:zombie, Weight 1}` alone, as every later one of the run. Sapling showed
  it empty; fixed (`sapling-leveldb` `BlockEntity::updated`).
- The trial spawner's detection, spawning and ejecting events are at the block's corner (sapling: its
  middle), and a spawn is told as both puffs, then the mob, then `trial_spawner.spawn_mob`. Fixed
  (`spawners/trial/waves.rs`, `trial.rs`).
- `/effect @s trial_omen 120 0` by an active spawner turned it ominous on neither in 30 s.

## Parity drill (2026-11-05, `bds_command_probe x2ran`, bds-x2ran.txt)

A vault opened with a trial key and a trial spawner's whole wave (mob spawning on: a trial spawner off it clears
its wave), each cloned after. Matched: the spawner's states 0 to 5, its puffs and `spawn_mob`, its `eject_item`
at `+0.5 +1 +0.5`; the cloned spawner's entity (`spawn_data` alone, shown twice). Fixed (thirteenth2x):
- A vault's sounds and animations are at its corner (`vault.eject_item` at `+0.5 +1.2 +0.5`), and a trial
  spawner's shutters; a state's sound comes after its block and entity (`spawners/vault.rs` `enter`,
  `trial.rs`).
- An active or ejecting vault shows its block and entity again each second, the entity with
  `connected_players` and the shown `display_item` from its `shared_data` (sapling showed `[]` and an empty
  item always: `sapling-leveldb` `BlockEntity::updated`). Test in spawners/vault.rs.
- Fixed (thirteenth2z): a set vault is watched when its entity is made (`blocks/set_entities.rs` `furnish`), so it
  is active the next tick as on BDS, not at the 100-tick sweep; a cloned vault's entity has `connected_players`
  `[]` (`spawners::cloned`) and is shown only after the clone's blocks. Tests in integration/thirteenth2z.rs.
- Fixed (thirteenth2ae, `spawners/vault/shown.rs`): a state the vault's own update enters (active, inactive) is
  written at once and told by its animation, but its block goes out the next tick, listed twice in one
  UpdateSubchunkBlocks, then its entity twice, then its sound (`vault.close_shutter` when leaving ejecting). This
  is the cloned vault going inactive a second on: its timing matched, its packets did not. A key's unlocking is
  shown at the click (block, entity, `vault.insert_item`) and its block and entity again the next tick. Tests
  in spawners/vault.rs.
- The ejected count (4 against 3) is the reward table's roll (1 + 1..3 + 1 items), not a rule difference.
- Fixed (thirteenth2af, bds-af2vault.txt): at a key that unlocks, BDS first resends the clicker alone six layer-0
  cells (the vault, already unlocking, the cells above and below it, the clicked face's neighbour and the cells
  above and below that), then the block, entity and `vault.insert_item`. A refusal sends only its sound
  (`blocks::about_click_first`, shared with cauldrons and creative placing; test
  `a_key_shows_its_user_the_cells_about_the_click_first_and_a_refusal_shows_none`).

## Ominous item spawners

Decompiled 2026-11-05 (twelfth7; TrialSpawner tick 1411923a0, spawn 141197c30, spot 141199cf0, item roll
1411990f0, entity tick 1429fa630); captures bds-u1omen.txt. Ported 2026-11-05 (twelfth10,
`spawners/trial/ominous_items.rs`; test `spawners::trial`); re-drilled, sapling-u1omen.txt drops its spawners
with the same packets, a lingering potion shot at (0, -0.5, 0) and an arrow with spread. The coin for the
wave's mobs, the kept per-position MT19937 roll and the spawn box check ported 2026-11-05 (thirteenth2d,
undrilled: the coin and the picks are on the spawner blocks' dice, not BDS's level random). [U] a mob's
head attach point (sapling: 0.85 of its height).

- When: an active spawner with `ominous` set, at `level.currentTick % 160 == 0` (the world's tick, not the
  spawner's), one at most; it needs a detected player alive, not creative, its head attach point closer than
  `required_player_range` to the spawner's centre (strict `<`).
- Over whom: a level-random coin picks the current mobs if any, else the detected players (the same filter);
  `nextInt(n)` one. From its head attach point A, E = A + (0, nextInt(4) + 2, 0); a block clip A to E: on a
  hit the spot is the hit cell's middle one lower, else E raw. Fails in A's cell or its box. [U] the first
  capture spot's +0.02 in y.
- Item: the ominous config's `items_to_drop_when_ominous` (default
  `loot_tables/spawners/trial_chamber/items_to_drop_when_ominous.json`: a lingering potion of wind_charged,
  oozing, weaving, infested, strength, swiftness or slow_falling; an arrow, poison arrow, strong slowness
  arrow, 1-3 fire or 1-3 wind charges), rolled once and kept, by MT19937 seeded
  `u = floor(z/30) 0x6ebfff5 ^ floor(y/20) ^ floor(x/30) 0x2fc20f`, then `(u 0x285b825 + 0xb) u + seed`; each
  spawn takes one kept stack weighted by count.
- The entity (type 0x91): AddEntity at the spot, flags 0, 0.6 by 1.8, still; `MinecartDisplayBlock` the item
  compound, `MinecartDisplayOffset` its delay, `nextInt(61) + 60`; `spawn_item_begin` there and LevelEvent 3612
  at the trial spawner with 1. At 35 ticks left `about_to_spawn_item`; at 0 the item's projectile shot down
  (0, -1, 0) (an arrow at about 1.29 with spread, a lingering potion at (0, -0.5, 0)), else a one-count item
  dropped, then `spawn_item` and the entity gone in that tick. The throw (twelfth10): the item's
  `createProjectileActor` (an arrow's 142a7cf70) calls `Spawner::spawnProjectile` (14118d160) with no owner,
  which shoots (`ProjectileComponent::shoot` 1424020c0) at the entity JSON's `power` and `uncertainty_base`:
  motion = power (way + 0.0075 uncertainty g) per axis, g normal, not renormalised: arrow 1.1 and 16 (both
  bds-u1omen.txt arrows within 2.5 sigma), lingering potion 0.5 and none (its `angle_offset` unused),
  fire charge 1.3 and 10, wind charge 1.5 and 1 (`shots/dispensed.rs` `shoot_unowned`). Whether a stack of
  charges throws one: drilled 2026-11-05 (`y3omen`, bds-y3omen.txt, eight ominous spawners for 60 s): all
  ten item spawners showed `MinecartDisplayBlock` `Count` 1 (nine lingering potions, one arrow), and each
  threw one shot and was gone. Those eight spawners all sat in one 30x20x30 region, so one kept table.
  Drilled again 2026-11-08 (`au3charge`, bds-au3charge.txt, level seed 20261018 as the drill server's;
  twelve ominous spawners in twelve regions): an item spawner carries its kept stack whole (`Count` 2
  or 3 of `fire_charge` in ten of them) and throws one shot (ten small fireballs), then is gone.
  Its `MinecartDisplayBlock` is `Count`, `Damage`, `Name`, `WasPickedUp` 0, in that order. Sapling
  showed `Count` 1 and its fields unsorted without `WasPickedUp`; fixed (`ominous_items.rs` `pick`,
  `fireworks::display_fields`). Re-drilled (sapling-au3charge.txt): each region's kept items
  (arrows of poison, potions 31, 43, 45, fire charges of 2 and 3) as BDS's, which settles the kept
  per-position MT19937 roll; ten fireballs, one per carried stack.
