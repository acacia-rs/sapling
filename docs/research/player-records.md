# Player records: how BDS keeps a player (2026-10-24)

Tags: **BDSRE** = `acacia/.testserver/bds-molang/bedrock_server.exe` (bdsre cache `6ab54bee`), read with
`py -3 C:/Users/hexay/acacia/bdsre/bdsre.py c <va>`; **DF** = dragonfly `server/world/mcdb/db.go` (MIT);
**inf** = inferred.

## Keys

- `player_server_` + an id: the player's compound (BDSRE 1412725f0 builds the key; `player_server_editor_`
  for editor worlds).
- `player_<id>` → `{MsaId, SelfSignedId, ServerId}`, `ServerId` naming the compound's key (DF 62-136).
  BDS's own matcher (BDSRE 141272c50) also knows `GuestId` and two platform ids. Sapling writes
  `MsaId` = `SelfSignedId` = the login UUID and `ServerId` = `player_server_<uuid>`, as DF does.
- `~local_player`: the host of a local (client) world; a dedicated server does not use it (inf).

## Fields

`Player::addAdditionalSaveData` (BDSRE 1402b2d30) writes, in order: `Offhand`, `Inventory`,
`format_version`, `SelectedInventorySlot` (int), `SelectedContainerId`, `Sleeping`, `SleepTimer`,
`SpawnBlockPositionX/Y/Z`, `SpawnDimension`, `SpawnX/Y/Z` (ints), `EnchantmentSeed`, riding ids,
`PlayerGameMode` (int, the `ActorGameTypeComponent`, -1 without one), `PlayerLevel` (int),
`PlayerLevelProgress` (float), `DimensionId` (int), `EnderChestInventory`, `HasSeenCredits`,
`PlayerUIItems`, `MapIndex`, `Sneaking`, `AgentID`, `DeathPositionX/Y/Z`, `DeathDimension`,
`HasDiedBefore`, permission levels and the recipe-book tabs.

The actor part (BDSRE 140d40ec0) writes `Pos` (3 floats, the position as held: a player's is at its eyes,
feet + 1.62001, inf from its being saved unchanged), `Rotation` (yaw, pitch), `Motion`, `FallDistance`,
`OnGround` and the rest; `Attributes` and `ActiveEffects` come from the mob part.

An effect (BDSRE 141c57070): `Id` and `Amplifier` bytes, `Duration`, `DurationEasy`, `DurationNormal`,
`DurationHard` ints, `Ambient` and `ShowParticles` bytes, `DisplayOnScreenTextureAnimation`, and a
`FactorCalculationData` compound for effects that fade.

No spawn point: `Player` load (BDSRE 1402af220) sets all six spawn ints to `i32::MIN` and the spawn
dimension to 3 when the fields are missing, and the save writes them as held; sapling writes the same.

## Not verified

- No BDS-saved player record could be captured: an offline-mode bot (no XUID) that joined, changed game
  mode, took items and an effect and left on a fresh BDS 1.26.5x world left no `player_` key at all (only
  `AutonomousEntities`, `BiomeData`, `LevelChunkMetaDataDictionary`, `Overworld`, `WorldClocks`, `mobevents`,
  `schedulerWT`, `scoreboard`). BDS seems to keep only players with an account id, which is why the mapping's
  key for an authenticated player (by `MsaId`?) is unchecked. A record written by an Xbox-signed client is the
  check still to make.
- Whether BDS loads a record sapling wrote fresh: sapling writes no `UniqueID`, `identifier` or `Motion`.
- `Pos` at the eyes is read from the code path, not from a saved file.
