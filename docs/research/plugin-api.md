# Plugin events: how they are raised

How sapling offers what happens to plugins, for anyone adding an event. DESIGN.md's "Plugin API" row
has the decision; this is the practice. UI calls and the newer hand and world notices are in
`ui-events.md`.

## The model

- A plugin's `build` registers typed listeners: `plugin.on::<E>(Priority, |ctx, event| ..)`. Each
  gets a `&mut ShardCtx` and the event mutably. No bevy type crosses the API.
- An event is a notice (`impl Event for E {}`, nothing to stop) or cancellable (`cancel()`,
  `is_cancelled()`, `Event::is_stopped`). Player events get both from `player_events!` in
  `sapling-api/src/player_event.rs`, world events from `stoppable!` in `block_event.rs`.
- Some events carry public fields a listener may change (`ItemDamageEvent::damage`,
  `LecternPageTurnEvent::to`). The mechanic reads them back after the offer.

## Raising one

- Each event is raised by one call in its mechanic's module, into a small shard module that builds
  and offers it:
  - `player_events.rs`: inputs and state
  - `life_events.rs`: body and hands
  - `hand_events.rs`: breaks, picks, wear, containers, lecterns, fire, diagnostics
  - `world_events.rs` and `world_notices.rs`: the world on its own
- An event nobody listens to is not built. `life_events::ask` and `allowed` check
  `plugins::listening::<E>` first, and so does `plugins::stopped_in`, because building one can cost
  a lookup or a clone.
- A player's event is offered with the player's world in focus (`worlds::focus_on`). A world's is
  offered with that world in focus (`plugins::offer_in`), so `ctx` calls act on the right world.

## A cancel undoes what the client predicted

The client already shows the player's own action, so a cancel must undo that on its screen:

| Kind | How the client is put right |
| --- | --- |
| Block changes | `send::resync`: both layers, plus the block entity |
| Inventory | the refusal answer, `send_all`, `containers::resend` |
| A crack | `BlockStopBreak` |
| Movement | a correction |

Events the server alone decides (wear, despawn, a sound) need nothing sent back.

## Event families

| File (`sapling-api/src`) | Events |
| --- | --- |
| `player_event.rs` | jump, punch air, sprint, sneak, held slot, teleport, game mode, skin |
| `life_event.rs` | pickup, consume, experience, food, heal, sleep, sign edit, container open/close |
| `hand_event.rs` | start break, block pick, item release, item damage, container slot, lectern page, set on fire, fire extinguish, diagnostics |
| `block_event.rs`, `world_event.rs` | liquid harden, crop trample, leaves decay, portal create, redstone, spread, grow, explosion, entity spawn |
| `world_notice.rs` | liquid decay, sound, entity despawn, world close |
