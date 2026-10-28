# BDS bundles (2026-10-27)

Source: capture `research/captures/bds-bundles.txt` (drill `bundles` of `bds_command_probe`: raw
ItemStackRequests from `crates/sapling/examples/shared/bundle_drill.rs`). sapling:
`crates/sapling-shard/src/inventory/bundles.rs`.

## Numbering

- A given bundle (net id 864) arrives untagged; about a tick later BDS gives it `{bundle_id: Int(n)}`, `n`
  counting up in the level from 0, under a new stack id, resends the four windows with PlayerHotbar
  `select_slot: true`, then InventoryContent of window 125 for the bundle (container `Dynamic(n)`,
  storage item the bundle).
- The client's copy of the tag never shows the contents; the bundle's own stack id never changes after.

## Requests

- Into a bundle: Place to `Dynamic` with `dynamic_container_id = bundle_id`, any slot the client picks; each
  slot holds its own stack (cobblestone in 0 and 2). Onto a held slot the destination must carry that slot's
  stack id. Out: Take from the same container.
- Weight: an item weighs 64 / its stack size (cobblestone 1, ender pearl 4, sword 64); the total may be 64.
  40 cobblestone + 4 pearls (56) took 1 more cobblestone, refused 10 more and a sword, each **status 55**
  with no containers (the whole request, nothing partial). A bundle into a bundle: 55 too.
- The answer names `Hotbar` and `Dynamic(id)` slots in the order the request named them; after it BDS
  sends InventorySlot `{window 125, slot, container: Dynamic(id), storage_item: the bundle (no stack id),
  item: the slot's stack with its id}` per changed slot (an emptied slot: no item).
- `/clear` counted no bundle contents.
- Taking from an empty bundle slot: **status 49**, no containers, nothing resent (bds-bundlekeep.txt, twice).
- Numbers across a restart (bds-bundlekeep2.txt, KEEP_WORLD): a new bundle after the restart was numbered 0
  again, so BDS keeps no level counter. The drill's own bundles were cleared before the restart, so whether
  a player's kept bundle 0 then shares its number with a new one was not seen; sapling skips past the
  numbers the player holds.
- After a rejoin sapling sends each numbered bundle's contents as InventoryContent window 125 (no BDS capture
  of a rejoin with a filled bundle yet).

## Container numbers on the wire and cleanup (2026-10-28)

- `dynamic_container_id` is little-endian: bundle 1's `ContainerRegistryCleanup` body was
  `01 3f 01 01000000` (research/captures/bds-container-cleanup.txt), which acacia-proto before
  2026-10-28 (big-endian `u32`) decoded as 16777216; a request the bot sent for bundle 1 as big-endian 1
  got status 50 (bds-hints.txt). acacia-proto reads it little-endian since (its docs/proto.md).
- `ContainerRegistryCleanup` goes to every player, also those that never held the bundle, at a sweep
  every 15 s (two runs: 15.05 s apart, phase unrelated to the drill). It lists every shown bundle no
  longer anywhere: cleared, replaced by air, or thrown and then killed as an item; one lying as an item
  stayed. Two at once came newest first. sapling: `inventory/bundle_cleanup.rs`.

## Open

- Shulker boxes in bundles were not drilled (sapling refuses them, as Java does).
- A rejoin with a filled bundle on BDS, to confirm the window 125 content sapling sends. `bundlekeep3`
  (bds-bundlekeep3.txt) tried: BDS let the probe back in as a new player, its inventory empty (the
  offline probe sends an empty `SelfSignedId`, likely why BDS found no saved player), yet a take from the old bundle's
  container 0 by its old stack id still answered Ok. A capture needs a client BDS saves as the same player.
