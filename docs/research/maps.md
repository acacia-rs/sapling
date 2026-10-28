# BDS maps (2026-10-27)

Sources: BDSRE on `acacia/.testserver/bds-molang/bedrock_server.exe` (map update 141a64d60, render
141a633d0, decorations 14371cdf0); captures `research/captures/bds-maps.txt` (drill `maps` of
`bds_command_probe`), `bds-mapwater.txt` (`mapwater`), `bds-mapgrid*` (`mapgrid`, `mapgrid-raised`, with the
`tools/map-colours` pack), `bds-maptints.cells` (`maptints`). sapling: `crates/sapling-shard/src/maps.rs` and
`maps/`, colours `crates/sapling-data/src/map_colours.rs` (README "Map colours").

## Filling

- Using an empty map (net id 526) in the air makes a `minecraft:filled_map` (423) with tag
  `{map_name_index: Int, map_uuid: Long}`, keys sorted; a locator map (damage 2) adds `map_display_players: Byte(1)`.
  `map_name_index` counts maps made in the level from 1.
- Ids: BDS unique ids, top half all ones (`-4294967279`); a new map takes five, its own and its zoom levels'
  (`included_in`), so the next map is 5 on.
- Centre: Java's `calculateCenter`: `floor((x + 64) / s) * s + s / 2 - 64`, `s = 128 << scale`; `origin.y` 0.
- Order: ClientboundMapItemData `{included_in: Some(5 ids), scale: Some(0), objects, decorations, pixels: None}` to
  everyone in the world, the four inventory windows and PlayerHotbar (select true); next tick the whole map,
  128x128 of 0 with empty object and decoration lists, then LevelSoundEvent `ui.cartography_table.take_result`
  at the floored eyes; the tick after, the first markers.

## Ticking

Only the map in a player's selected slot ticks; a map swapped out stops.
- Redraw: a per-map count goes up each tick; a tick that finds it at 20 restarts it at `nextInt(6)` and redraws
  (first drawing 21 ticks after filling).
- Packets: a tick whose redraw (or a client's pixels) changed pixels sends them, the changed rectangle only,
  with the last markers; any other tick counts, and every fifth count recomputes and sends the markers. So a
  pixel packet slips the marker cadence by a tick (25929, 26183 P, 26232, 26477 in `bds-maps.txt`). Both go to
  every player in the world, the holder or not.

## Drawing (decompiled render)

Java's `MapItem.update`: `f = 1 << scale`, `r = 128 / f` (halved under a ceiling), holder pixel
`p = (floor(x) - centre) / f + 64` (C division). x over `max(px - r + 1, 0)..min(px + r, 128)`, z over
`max(pz - r - 1, -1)..min(pz + r, 128)`; written when `z >= 0`, `d² < r²` and (`d² <= (r - 2)²` or `(x + z)` odd).
- A pixel samples `f x f` columns at `((centre / f) + x - 64) * f`: the highest block with a map colour, the most
  common colour winning (first on a tie), height `+= y / f²` (integer division).
- Shade (f32): `step = (h - h_north) * 4 / (f + 4) + (((x + z) & 1) - 0.5) * 0.4`; `> 0.6` full (1.0),
  `< -0.6` low (0.7058824), else 0.8627451. A one-block rise on an even cell sits exactly at 0.6: normal. The
  first row compares with itself.
- Water is always at full brightness, any depth (1 to 16 in `bds-mapwater.txt`), unlike Java.
- Pixel: `trunc(f32(c / 255 * shade) * 255)` per channel, ABGR with alpha FF; a column with no coloured block
  is `0xFF000000`. Tinted colour `c = own / 255 * tint / 255 * 255`.
- Nether: Java's noise `(x + z * 231871)` squared, dirt or stone, height 100.

## Markers (decompiled 14371cdf0, capture)

- Every player in the world: `u = round((pos - centre) / f)` (half away from zero) per axis. Within
  `-64..=63` on both: MarkerWhite at `2u`, `u > 62` → 127, `u < -62` → -128; otherwise SquareWhite with the same
  clamp. `|u| > 319`: not tracked.
- Rotation `(i8)(int)((yaw ± 8) * 16 / 360)`, the sign of the yaw's; kept for SquareWhite too.
- Colour ABGR: the holder -1, others -11206657. The order of tracked objects and markers changed between captures
  (`bds-maps.txt` holder last, `bds-stations.txt` holder first); sapling lists the newest player first.
- Plain and locator maps both showed markers.

## MapInfoRequest

- No pixels: to the asker only, the creation packet (with `included_in`) and the whole map with the last markers.
- With `client_pixels`: each written at `index % 128, index / 128` as given, told as a changed rectangle.

## Cartography (captures `bds-maps.txt`, `bds-stations.txt`)

- ContainerOpen window type Cartography; the client puts the map in `CartographyInput` 12 and the added item in
  `CartographyAdditional` 13, then `CraftRecipeOptional` (the multi recipe's network id, filter string 0, cause
  CartographyText), Consume of both, Place from CreatedOutput 50. Multi recipe UUIDs: copy (empty map)
  `442d85ed-…`, zoom (paper) `8b36268c-…`, lock (glass pane) `602234e4-…`; a rename alone names recipe 0.
- Copy: two of the map, tag kept. Zoom: map id + 1 (its `included_in`), scale + 1, centred anew from the old
  centre (`(256, -256)` → `(320, -192)`), tag `{map_is_init: 1, map_name_index (kept), map_uuid}`; BDS sent the
  zoomed map whole, blank. Lock: a new id (BDS: 255902174), `locked: true`, pixels copied, damage 6 on the item.
- A renamed result's response slot carries `custom_name`; BDS resent no inventory.
- The bot's reads were paused during the table's actions, so who got the result packet and when is not known.

## Open

- Tints: BDS blends biome tints across neighbours; `map-tints.tsv` holds one probed spot per biome.
- Only default block states were drawn; state-dependent colours (log axis, beds) use the default's.
- Saving (sapling-leveldb `maps.rs`, from a BDS world's `map_<id>` records): BDS saved all five scales of a
  filled map at once, each `parentMapId` the id one up (-1 at scale 4); sapling saves the maps it made.
  Unread: the RGBA byte order of `colors` (the BDS records seen were blank), BDS's `map_name_index` count
  after a restart, and `decorations`.
- Locked maps' ids: `bds_command_probe maplock` (bds-maplock.txt) locked three maps (unique ids -4294967272,
  -4294967267, -4294967262) to 1396458477, 2162056098 and 3185328418, after 255902174 in the first capture:
  positive, 32 bits wide, in no order, apart from the unique-id run. Where BDS draws them was not found
  (the cartography handlers 142c80c10 and 142c81720 call no random); sapling hashes the tick and the map
  count into a free id of that range (`maps::locked_id`). The locked map's `included_in` was its own id and
  the zoom ids of the map it came from. A second lock of a locked map: status 9.
