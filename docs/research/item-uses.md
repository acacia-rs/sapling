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
- **Ends** (`glide2`, research/captures/bds-glide2.txt, 2026-10-29): a glider put in water was stopped
  by the server as on landing (flag off, `BoundingboxHeight` 1.8); sapling did not, and now does
  (`gliding.rs` `in_water`). One that started riding was stopped too, as in sapling. Levitation did
  not stop it: BDS corrected the levitating glider each 300 ms and left the flag on, as sapling.
- Not drilled: creative flight ending a glide (the drill bot cannot fly); the fly-into-wall damage
  (combat's).

## Armour from the hand (`fx3`; `inventory/donning.rs`)

A click in the air with a chestplate or boots in the hand put it on (research/captures/bds-fx3.txt):
`InventorySlot` of the armour slot, the inventory sent, `armor.equip_iron` a tick on. Sapling did
nothing; fixed, swapping with what was worn (only an empty slot drilled). An elytra put on by
`/replaceitem` is heard as `armor.equip_elytra` (bds-glide2.txt; sapling was silent; fixed).

## Buckets (`buckets`; `blocks/buckets.rs`, `creatures/scooping.rs`)

- **Powder snow**: an empty bucket on powder snow clears the cell and becomes `powder_snow_bucket`;
  that bucket on a block puts powder snow in the cell on the face. Heard as `bucket.fill.powder_snow`
  and `bucket.empty.powder_snow` at the player's feet; windows resent as for water.
- **Fish**: a water bucket used on a cod (or an axolotl) removed it at once (RemoveEntity), played
  `bucket.fill.fish` at the integer corner of the mob's cell (for the axolotl too) and became
  `cod_bucket` whose tag is the mob's whole actor compound (`Variant`, `MarkVariant`, `Color`,
  attributes and the rest). sapling keeps the stored compound and makes the mob from it again where
  the bucket pours. A `/replaceitem` cod bucket (no tag) poured on BDS (`glide2`, 2026-10-29): the cod
  came, heard `ambient.in.water`, and the pour was `bucket.empty.fish` (extra data -1, no entity type)
  at the pourer's feet, the name sapling uses.

## Books and lecterns (`books`; `books.rs`, `blocks/lectern.rs`)

- **Writing**: BookEdit ReplacePage per page (the client also sends a legacy slot rewrite, which BDS
  answers by resending every window with `select_slot true`, once for the tick's edits). Pages are
  `{photoname, text}`. Opening a new book (ClickAir) gave it `pages: []` (element tag 0), seen by
  viewers in MobEquipment.
- **Signing**: the stack becomes `written_book` with `author` (the client's, "Author Unknown"),
  `generation` 0, `pages`, `title`, `xuid`, keys sorted; windows resent, `select_slot true`.
- **Lectern**: a click with a book on an empty lectern: windows resent (`select_slot true`),
  `item.book.put` at the block's corner, then BlockEntityData `{book, hasBook 1, page 0, totalPages}`.
  A click with the book on it opens ContainerOpen `Lectern`. LecternUpdate sets `page`, resent as
  BlockEntityData. The comparator read 15 with a book at page 0 and at page 1 of 3 (the comparator set
  again, so read afresh): no page dependence, unlike the Java game. A turn to page 0 pulsed
  `powered_bit` from the next tick for two ticks; a turn to another page resent the unpowered block two
  ticks later and showed no power. Sapling pulses on page 0 only and resends nothing. Broken, the lectern dropped the book with its tag.

## Cooldowns and held uses (`small`; `cooldowns.rs`, `instruments.rs`)

- ClientStartItemCooldown: `minecraft:ender_pearl` 20, `minecraft:wind_charge` 10 at the throw;
  `minecraft:goat_horn` 140 at the use and again at its completion; `minecraft:chorusfruit` 20 at
  the start of eating and again when eaten. A second pearl or wind charge thrown after the cooldown
  ran out got no packet on BDS (both drilled once); sapling tells every one.
- Goat horn: SetEntityData USING_ITEM to all, `horn_call0` (the variant's number) from feet + 1.52
  with the player as the entity; 20 ticks on CompletedUsingItem (method -1), windows resent, USING
  cleared. A use while cooling did nothing.
- Spyglass: USING_ITEM set on use, cleared on release; nothing else.

## Loom and beacon (`stations`; `inventory/forge/loom.rs`, `beacons.rs`)

- **Loom**: ContainerOpen `Loom`; the banner Placed into `LoomInput` 9, the dye into `LoomDye` 10;
  `CraftLoom{bo}` + ResultsDeprecated + Consume 9, Consume 10 + Place to the inventory, answered Ok
  with those slots, nothing resent. The banner kept its Damage (15, white) and gained
  `Patterns [{Color 1, Pattern bo}]` before `Type`.
- **Beacon**: placed by /setblock it sent BlockEntityData `{primary 0, secondary 0}` twice (sapling
  makes the entity at the first payment). The payment Placed into `BeaconPayment` 27 drew an
  InventorySlot of window 3 slot 0; `BeaconPayment{1, 0}` + Destroy 27 answered Ok, no BlockEntityData.
  On a one-layer iron pyramid speed came to the payer and the watcher as MobEffect Add, amplifier 0,
  duration 220, ambient, particles, then Update every 80 ticks.
