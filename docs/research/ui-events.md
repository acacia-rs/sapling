# Plugin UI and the hand and world notices

What workstreams G2 (UI API) and G3 (events) built. Each rule names its source. The tests are in
`crates/sapling-shard/tests/integration/`: `plugin_ui.rs`, `plugin_dialogue.rs` and `hand_events.rs`.
How events are raised in general: `plugin-api.md`.

## UI (G2)

### Text on the hotbar

- `send_popup`, `send_tip` and `send_jukebox_popup` each send a Text packet:
  - popup: type Popup, category Parameters
  - tip: type Tip, category MessageOnly
  - jukebox popup: type JukeboxPopup, category Parameters
- None of them is translated.
- Source: dragonfly `session/text.go`, gophertunnel `packet/text.go`.

### Name and score tags (`labels.rs`)

- `set_name_tag` and `set_score_tag` work on a player or an entity (`Actor`).
- Who sees the actor gets a SetEntityData. Anyone who comes to see it later gets the tags again,
  right after its AddPlayer or AddEntity (`labels::packet`).
- A name tag also sets `AlwaysShowNametag` to 1. Clearing it sends the actor's own name back: the
  player's name, or the Nametag in a mob's announcement. For a mob it also sets
  `AlwaysShowNametag` to 0.
- A score tag is metadata key 84 (`ScoreTag`). Clearing it sends "".
- Source: dragonfly `session/entity_metadata.go` (`NameTag`, `ScoreTag`, `AlwaysShowNameTag`).

### Hiding per viewer (`hidden.rs`)

- Each viewer has a `Hidden` set of entities. The player and mob visibility diffs
  (`visibility::update`, `mobs::update`) leave those entities out, so hiding one sends RemoveEntity
  and showing it again sends the full spawn.
- Source: dragonfly `Player.HideEntity` / `ShowEntity`, which go through its viewer removal path.

### Floating text

- A floating text is dragonfly's text entity: a `minecraft:falling_block` of air with flags
  `NO_AI`, `CAN_SHOW_NAMETAG` and `ALWAYS_SHOW_NAMETAG`, a bounding box of 0×0, and its text as the
  name tag.
- It is a mob statue (`mobs::spawn_shown`), so it shows and hides with distance like any mob.
- `set_floating_text` rewrites its announcement and tells whoever sees it.
- `remove_entity` takes it away.
- Source: dragonfly `entity/text.go` and `session/entity_metadata.go`.

### Extra boss bars (`extra_bars.rs`)

- A client shows one bar per actor id, and a bar needs an actor the client knows. So each extra
  bar (`set_extra_boss_bar(player, key, bar)`) gets its own entity, which only that player is told
  of. It is an invisible floating text with empty text, moved after the player once the player is
  more than 8 blocks away on any axis.
- The player's own bar is unchanged: it stays on the player's own id (DESIGN.md "Boss bar").
- Bars are shown in key order.
- Game bars that belong to no boss entity take a slot of their own as well
  (`extra_bars::BarKey::Raid`), so a raid's bar does not overwrite the plugin bar on the player's id.
  The dragon's and the wither's bars stay on their own entity ids, as on BDS.
- A removed bar gets HideBar, then RemoveEntity for its entity.
- Source: gophertunnel `BossEvent` (the id is an entity unique id), and the vanilla client's
  per-actor bar map. PocketMine's bossbar plugins work the same way.

### Debug shapes (`debug_shapes.rs`)

- `draw_shape` and `erase_shape` send ServerScriptDebugDrawer with one item per shape:
  - shapes: line, box, sphere, circle, text, arrow
  - colour is ARGB
  - an optional lifetime in seconds
- An item with no shape type erases that id. This is how acacia's flat item encodes a remove.
- Source: acacia `ServerScriptDebugDrawer`, and the script API `@minecraft/debug-utilities`
  (`DebugShape.remove`).

### NPC dialogue (`dialogue.rs`)

- `send_dialogue(player, npc, dialogue, on_press)` first marks the speaker for that player with
  SetEntityData:
  - `InteractiveTag` = 1 (gophertunnel's HasNPC)
  - `NpcSkinId` = `{"portrait_offsets":{"translate":[0,-eyes*32,0],"rotate":[0,-32,0],"scale":[1,1,1]}}`
    (gophertunnel's NPCData)
- It then sends NpcDialogue Open with scene name "default", the title, the body and the button JSON
  (`button_name`, `text` "", `mode` 0, `type` 1).
- A new dialogue closes the last one: the old callback gets `None`.
- NpcRequest only counts for the open dialogue's actor id:
  - ExecuteAction is a press. Its `action_type` byte is the button index.
  - ExecuteClosingCommands is a close.
- A press runs the button's commands (Operator) as the NPC, with `@initiator` as the presser.
  Scoreboard changes are sent, then the callback gets `Some(index)`.
- Source: dragonfly `session/player.go` `SendDialogue`, `session/handler_npc_request.go`;
  `@initiator` per the vanilla command docs (the player who opened the NPC's dialogue).

## Events (G3)

| Event | Raised in | A cancel |
| --- | --- | --- |
| `StartBreakEvent` | `blocks/request.rs` `punch` | sends `BlockStopBreak` to the breaker; the break is never timed |
| `BlockPickEvent` | `inventory/picking.rs` | nothing comes to the hand; no answer sent (BDS answers only a pick it does) |
| `ItemReleaseEvent` | `shots.rs` `release` | nothing is loosed; the draw ends |
| `ItemDamageEvent` | `inventory/wear.rs` `wear_at`, `wear_hand_by` | no wear; `damage` may be changed |
| `ContainerSlotEvent` | `inventory/requests.rs` `answer`, with the trial copy diffed against the open slots | the request is refused (Error), and every window is sent again |
| `LecternPageTurnEvent` | `blocks/lectern.rs` `turn_pages` | `send::resync` puts the lectern back; `to` may be changed |
| `SetOnFireEvent` | `vitals.rs` `settle`, when a player goes from not burning to burning | the fire is put out |
| `FireExtinguishEvent` | `blocks/fire_punch.rs` | the fire stays, and the cell is resynced |
| `DiagnosticsEvent` | `hand_events::dispatch_diagnostics` (ServerboundDiagnostics) | a notice |
| `LiquidDecayEvent` | `blocks/liquid/flow.rs`: a flow drying up, or getting thinner while not falling | it stays until its next update |
| `SoundEvent` | `view::to_viewers_but`: every LevelSoundEvent handed to viewers | nobody hears it |
| `EntityDespawnEvent` | the remove hook of `entities::Kind`, offered by `world_notices::offer_gone` after handlers each tick | a notice |
| `WorldCloseEvent` | `arenas::remove`, `Shard::close` | a notice |

- Punching fire puts it out. A punch on a face with fire in front of it removes the fire and plays
  `extinguish.fire` instead of starting a break.
  - Source: dragonfly `player.go` `StartBreaking` (`if _, ok := w.Block(pos.Side(face)).(block.Fire)`)
    and the Java game's `Level.extinguishFire`.
- Container slot changes are reported per slot as the difference: a take or place of the count
  that changed, or a take then a place where the stack was swapped. A take of a stack that the
  request throws out is a `Drop`.
- Item release only exists for what sapling draws: bow, crossbow and trident (`shots.rs`).

## Not drilled

None of this was checked against BDS or a live client. These rest on reading:

- Extra boss bars on separate entities: the 8-block follow distance, and whether a vanilla client
  draws several bars at once. Not seen on a live client.
- Mob name tags set `AlwaysShowNametag` but not the `ALWAYS_SHOW_NAMETAG` flag.
- A dialogue stays open after a press until it is closed or replaced; dragonfly behaves the same.
- Floating texts and extra bars are not saved, and do not follow a player to another shard or
  dimension.
- A cancelled `ItemDamageEvent` sends nothing. A client that predicted the wear is corrected only
  by its next inventory update.
- `SoundEvent` sees only sounds handed out through `to_viewers`. Sounds told to one player
  (`vitals::tell`, a broken tool) and `ShardCtx::play_sound` are not offered.
- `EntityDespawnEvent` arrives up to a tick late. It is also raised for entities removed with a
  world, and for floating texts.
- `WorldCloseEvent` on shard close is offered once per layer, so a realm's Nether and End close
  separately.
- `SetOnFireEvent` is for players only. Mobs catching fire are not offered.
