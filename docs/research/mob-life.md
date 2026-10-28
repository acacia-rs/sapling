# Eggs, tadpoles, mount inventories and sniffers: ground truth for sapling (2026-10-28)

Tags: **BDSRE** (bdsre on BDS 1.26.52, `acacia/.testserver/bds-blocks/bedrock_server.exe`; VAs as given), **ED** (the
vanilla behaviour pack's entity JSONs, `research/bedrock-samples-ui/behavior_pack/entities`, and its loot tables),
**RP** (the same samples' resource pack: what the client animates by), **JAVA** (the Java game's rules, used only where
BDS was not read). No BDS drill was run for any of this. Items under "Not drilled" at the end are what sapling guesses.

## Egg blocks

- `TurtleEggBlock`, `FrogSpawnBlock` and `SnifferEggBlock` subscribe to the block place and queued tick events (BDSRE:
  the `BlockEventExecutor<...>::subscribe` lambdas; turtle ctor 14257d680, subscribe helper 14257dce0). They do not
  random tick. Sapling raises "place" from `blocks/send.rs` `write` when the cell's block name changes, and queues
  `Job::Hatch` (blocks/queued.rs, persisted with the other queued ticks).
- Sounds are named by the sound table 14346a2b0 (BDSRE): `block.turtle_egg.crack/.hatch/.break`,
  `block.frog_spawn.hatch`, `block.sniffer_egg.crack/.hatch` (ids 225, 226, 433, 466, 467).
- A hatched mob is spawned with `entity_born`, a baby (sapling: `SpawnReason::Hatched`, then `husbandry::be_born`).

### Turtle eggs (blocks/eggs/turtle.rs)

- Place 14257dae0: on sand, red sand or suspicious sand, `LevelEvent` 2034 (`ParticleTurtleEgg`); then a tick in
  1000.
- Tick 14257e040: due when `getTimeOfDay` is in (0.65, 0.69) (f64 compares) or `nextInt(500) == 0`, and only on sand.
  - Below `max_cracked`: sound 225, `cracked_state` one further.
  - At `max_cracked`: sound 226, the block goes; per egg `LevelEvent` 2001 (destroy particles) and a baby turtle.
  - Then the next tick in `750 + nextInt(750)`.
- A hatched turtle's home is the egg's cell: `HomePos` (float list) and `HomeDimensionId` in its compound
  (sapling-leveldb `actor_home.rs`).
- Trampling (JAVA): any living thing but a turtle or bat, a mob only with `mobGriefing`, breaks one egg with odds of 1
  in 100 per tick on it, or 1 in 3 when it lands on it. A player does not break eggs while sneaking. Sound `.break`.

### Frogspawn (blocks/eggs/frogspawn.rs)

- Place 14715fcb0: a tick in `3600 + nextInt(8400)`.
- Tick 147160310: sound 433. The block goes, then 2 + `nextInt(5)` tadpoles appear at
  `(x + nextFloat, y - 1 + 0.5, z + nextFloat)`, each with `LevelEvent` 2001.
- Tadpole to frog (ED `tadpole.json`): `minecraft:ageable` over 1200 s, `minecraft:transformation` into
  `minecraft:frog` with sound `convert_to_frog`. Slime balls feed it. Sapling keeps the tadpole's name.
- Frog variant (ED `frog.json` `entity_spawned`):
  - Cold (1) in biomes tagged cold or mountains, except meadow, cherry grove and stony peaks.
  - Warm (2) in warm biomes.
  - Temperate (0) elsewhere.
  - Sapling keeps the variant as the frog's `Variant` (creatures/husbandry/tadpoles.rs, birth.rs).

### Sniffer eggs (blocks/eggs/sniffer.rs)

- Place 146607830 (BDSRE `c`): on moss (the block under it), `LevelEvent` 0x7f2 = 2034. The delay is
  `7200 + nextInt(1600)`, a float halved on moss.
- Tick 146607e10:
  - Below `max_cracked`: `LevelEvent` 2034, sound 466, one crack more, and the same delay again.
  - At `max_cracked`: sound 467, the block goes, and a baby sniffer appears (`entity_born`).

## Mount inventories (riding/packs.rs)

- No BDS capture of the horse screen exists. BDSRE found `HorseContainerManagerModel` (lambdas 1480929b0, 148092d00) and
  `ServerPlayer::openInventory` 140756800, but they were not read further.
- Sapling's layout:
  - Slot 0 is the saddle and slot 1 the armour (horse) or carpet (llama).
  - Chest slots follow from 2: 15 for a donkey or mule, 3 × strength for a llama (strength 3 here).
- The screen sapling sends:
  - `ContainerOpen` with `WindowType::Horse` and the mob's runtime id, then `InventoryContent` (`containers::show_carried`).
  - Then `UpdateEquipment` (window, type Horse, size, entity id). Its NBT `slots` list gives each tack slot's
    `slotNumber`, its outline `item` (`horsesaddle`, `horsearmor`, `carpet`) and `acceptedItems`.
- What opens it:
  - A sneaking click on a tamed mount.
  - The inventory key (`Interact OpenInventory`) while riding a mob. A ride without an inventory opens the player's own.
- A chest used on a tamed grown donkey, mule or llama sets `Chested` (ED: `minecraft:equippable` / chest groups) with
  sound `add.chest`.
- Stack requests: `ContainerSlotType::HorseEquip` (27) names the tack slots and `Container` the chest slots. Both are
  `Place::Container(slot)` (inventory/requests.rs).
- Kept in the compound:
  - `Saddled` and `Chested`.
  - `ChestItems` by slot, tack included; slot 1's armour is worn again on load.

## Sniffers (creatures/husbandry/sniffing.rs)

- ED `sniffer.json`:
  - `behavior.timer_flag_1`: cooldown 400–500 s, lasts 2 s, ends in `on_scenting_success`, which adds
    `sniffer_search_and_dig` to an adult.
  - `behavior.random_search_and_dig`:
    - Search: speed ×1.25, 5 position retries, range 20 across and 3 up or down, goal radius 2.
    - Dig: 8–10 s, the item 6 s in, 2.25 ahead, from `loot_tables/gameplay/entities/sniffer_seeds.json`
      (torchflower seeds or pitcher pod, even odds).
    - Target blocks: dirt, coarse dirt, grass, podzol, dirt with roots, moss, pale moss, mud, muddy mangrove roots.
  - On an item found: `feeling_happy`, `timer_flag_3` for 2–5 s.
  - On success or a failed dig: `stand_up`, `timer_flag_2` for 2–5 s.
- RP: the client animates by `query.is_searching`, `query.is_digging` and `query.timer_flag_1..3`. Those are the
  extended flags `SEARCHING`, `DIGGING` and `TIMER_FLAG_1..3`.
- Breeding (ED): `causes_pregnancy`. The pregnant sniffer's `spawn_entity` drops a `sniffer_egg` item with sound
  `plop` and no baby (creatures/husbandry/breeding.rs `LAYS_EGG`).

## Not drilled

- Turtle trampling odds and who tramples (JAVA). The hatched turtles' places, 0.3 + 0.2 per egg along x (JAVA).
- Whether frogspawn with no water under it breaks or waits for its next tick. Sapling breaks it, with particles and
  no tadpoles.
- The sniffer egg's hatch position (sapling: the cell's centre) and whether `BlockPlaceEvent` fires on a state change.
- The whole horse screen:
  - Packet order and `UpdateEquipment` NBT.
  - Whether `ChestItems` keeps the tack.
  - The llama's strength roll, carpet display, and the chest's goods spilling on death.
- The sniffer:
  - Its extended flags' base word (sapling: `1 << 25`, as breezes and creakings were recorded).
  - How a search position is chosen (sapling: the highest open soil in a random column within reach).
  - The search's give-up time (sapling: 30 s, not in the pack), and whether dug spots are remembered.
  - None of its state is kept in a world folder.
