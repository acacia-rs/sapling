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
  inactive, active, unlocking, ejecting.
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
Players are known by their kept `UniqueID` (`tags::Marks`).

Left out:
- Spawners in worlds sapling generates have no block entity (worldgen writes blocks only), so they do
  nothing until a spawn egg sets them; BDS's dungeon picks its mob from `nextInt(4)` (`monster_room.rs`).
- Bad Omen and Trial Omen (no such effects yet), so a spawner turns ominous only as a world holds it; the
  ominous item spawner. Ominous vaults use `reward_ominous.json` and the ominous key [U: BDS has these from
  their template's NBT].
- `SpawnData` equipment, `IsBaby` and slime sizes (spawned plain); the 3 blocks a non-fire-immune mob may not
  spawn on and `walkTargetValue` (unknown); the client-side flame and smoke; the vault's luck and its seeded
  loot RNG.

## Unverified

- A player's position as its eyes for every range check here; the 3615 charged-detection event; that a
  trial spawner's line of sight is a clip through any block with collision (BDS: a real ray clip, max 200).
- The eject velocity's odd x/z term BDS's trial spawner adds (left out); the vault's per-axis spread.
- What a spawn egg does to a trial spawner's configs (`14488a3a0`; here its kind goes into `spawn_data`, kept
  over the cooldown while the config names none).
- Positions of the vault's and the trial spawner's level events (sent at the block's middle).
