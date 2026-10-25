# sapling — what to do next

The work list, in order. What is built and how is in `DESIGN.md`; every gap it knows of is in its
"Open items" and in the `Not done` notes of each section. What the game itself does is in
`research/` (decompiled BDS, the Java jar's data); read that before guessing a rule.

## 1. Entities, then vanilla commands

Decided 2026-10-06, so that the drill probe (`examples/bds_mechanism_probe.rs`) runs on sapling as
it does on BDS and the two captures can be diffed:

1. **An entity API.** `EntityId` handles like `PlayerId`, plain-data answers: `spawn_entity(kind,
   pos)`, `entity(id)`, `entities()`, `move_entity`, `set_velocity`, `remove_entity`. Bodies are
   simulated on the server with BDS's gravity and drag and collide with blocks; no AI. One model for
   spawned mobs, the mobs a world folder stores (today statues, `mobs.rs`), lying items and falling
   blocks: built (`body.rs`, `sync.rs`, `entities.rs`, `creatures.rs`, `actors.rs`), stored mobs
   saved back where they are. Items let go, orbs, falling blocks and players' stuck arrows and tridents are saved with their column (`drops.rs`, `orbs.rs`,
   `blocks/falling/flight.rs`, `creatures/flight.rs`); a falling block's `Time` is kept as its fall began.
2. **Vanilla commands**: built (`sapling::vanilla`); the drill probe runs on sapling
   (`tools/sapling-drill.sh`, on testbox `tools/remote-drill.sh`). Left: what the BDS-against-sapling
   diff of the drill captures still shows (`research/captures/sapling-vs-bds-findings.md`).
   `/weather` and `/toggledownfall` set a world's weather, told as BDS told it (`weather.rs`,
   `bds_command_probe weather`); where `doWeatherCycle` is on it turns of itself, by the Java
   game's times, which were not checked against BDS; BDS's `query`, which lags a change behind, is
   not copied.
   `/time`, `/xp` and `/tp` to a target answer as BDS did (`vanilla/clock.rs`, `bds_command_probe
   clock`: the answers agree line for line; BDS writes its experience attributes in another
   order). A sapling capture that answers `commands.generic.unknown` or `.syntax` where BDS's
   does not names a command or a form still missing.
   The spawn, region (`/clone`, `/fill`'s modes), item (`/enchant`, `/give` and `/clear` in
   full), selecting and server commands answer as BDS did (`worldcmds`, `itemcmds`). Left there:
   `/give`'s `can_destroy` and `can_place_on`, data-value block overloads, what BDS guesses are
   listed in `docs/research/world-commands.md`.
   `/scoreboard` and `/tag` answer as BDS did (`bds_command_probe scoreboard|scoreboard2`); left:
   the unverified list in `docs/research/scoreboard.md`, and saving the board only when its shard closes or panics (no periodic world saves yet).

   What a click does to a block (`docs/research/block-uses.md`, `clicks`), primed TNT (`tnt`) and
   bone meal (`docs/research/bone-meal.md`, `bonemeal`) are drilled and built. Saplings,
   propagules and azaleas grow worldgen's trees by bone meal and random tick, by BDS's decompiled
   rules (`docs/research/saplings.md`, `blocks/saplings.rs`), in light of 9 or more; not drilled
   against BDS. The server reckons light now (DESIGN.md "Light (as built)"), and crops, stems,
   bamboo, grass and mycelium, ice and snow, freezing and snowfall, mushrooms, daylight detectors
   and the sun's burning ask it by BDS's decompiled rules (docs/research/growth.md "Light"; not
   drilled), as does mob spawning (`spawning.rs`, docs/research/mob-spawning.md, whose "Left"
   lists what is not built: slimes, built now, still never spawn naturally, as BDS's code gates
   them by slime chunk and swamp moon phase, not by spawn rules alone); left there: mushrooms spreading, vines and cave vines; the flood's grid
   looks a column up by hash per cell (~0.7 µs a cell optimised: three columns arriving cost a tick
   15 ms), worth caching. Fire is lit, spreads,
   burns and is put out (`docs/research/fire.md`, `fire`, `quench`), its spread by the Java game's odds. Crops, stems and berry bushes
   grow by themselves at BDS's pace, a ripe stem bears, cane, cactus and bamboo grow and
   farmland dries and is trodden down, grass and mycelium spread to open dirt, leaves decay when their log
   goes, and cocoa, nether wart and pitcher crops grow (`docs/research/growth.md`, `crops`,
   `grows`, `grows2`, `leaves`, `trample`);
   left: the rest of what random ticks do.

## 2. Survival gaps still open

Each needs its rule looked up first (`bdsre` for BDS, the Java jar for data), then a shard test.

1. **Water and lava for entities**: items float and burn, orbs bob and are thrown out of lava,
   mobs sink, as on BDS (`body/liquid.rs`, `docs/research/water-movement.md` 7 and 8); concrete
   powder sets (`bds_command_probe concrete`). Mobs have health; lava, fire, falls, a
   block their head is in and water without breath hurt and kill them, cooked if on fire
   (`creatures/harm.rs`, `creatures/health.rs`, `docs/research/mob-damage.md`, `bds_command_probe
   burn` and `harm`). A stored mob's health and fire are
   saved with it. Water puts a fire out with BDS's fizz (`douse`). A player's hit
   hurts and throws a mob, and its kill leaves experience (`creatures/attacks.rs`, `hit`). Left:
   what the doc lists as not drilled. A zombie and seven kinds like it take the nearest survival player, walk at it
   and hit it, a creeper blows up by it (`creatures/blast.rs`, `boom`) and a skeleton shoots at it (`creatures/arrows.rs`, `shoot`) (`creatures/hunting.rs`, `docs/research/mob-attacks.md`, `bite`, `slain`, `kinds`); and every kind that walks strolls when it has nothing to do and flees a hit (`creatures/strolling.rs`, `stroll`, `flee`); left
   there: the kinds that shoot other things than arrows (a blaze and a witch are built; a witch drinks, `witch-drinks`, and mobs have effects, splashed too, `mob-effects`; an evoker's fangs and vexes are built, `evoker`; a shulker's bullets are built, `shulker`, a breeze's wind charges and leaps, `breeze`, and a ghast's flight and fireballs, `ghast`; docs/research/mob-kinds.md), the other kinds that turn on their hitter (`provoke`, `provoke2` and `provoke3`: a wolf, a zombified piglin, a polar bear, spiders by day, a llama's spit, a panda, a bee and an enderman, `ender`, are built; an enderman's stare is built, `stare`; a dolphin swims and bites back, `dolphin`; an enderman's calm teleports and water are built; left there rain on an enderman, a dolphin's breath, breaching and shared anger, and of pandas' genes a lazy one's pace, a worried one's keeping away, a playful one's roll and an aggressive one's anger passed on (`pandas`); a goat's ram is built, of players and mobs, `goat`, `goat-cows`). Survival players have health: falls,
   fire, lava, water over the head and a block around them hurt, `/damage` and `/kill` too; they die
   leaving items and experience and rise by their bed or at the world's spawn (`vitals.rs`,
   `docs/research/player-damage.md`, `bds_command_probe hurt` and `smother`). A wounded player mends by its
   hunger and an empty hunger starves it (`vitals/hunger.rs`, `hunger`). Food in the hand is eaten
   (`vitals/eating.rs`, `eat`). Players draw bows, load crossbows and throw tridents, pearls, potions and the rest (`shots.rs`, `docs/research/projectiles.md`; not drilled against BDS). Jumps, sprints, harvested blocks and hits tire a player (`tire`, `toil`). Armour stops part of a hit of the causes it stopped on BDS (`armour`, `worn`). Twenty-five effects by `/effect` (`vitals/effects.rs`, `docs/research/effects.md`, `effects`); food and potions give theirs and milk ends them (`docs/research/food-effects.md`, `sick`, `drink`). A player hits a player where `pvp` is on (`creatures/duels.rs`, `pvp`, `duel`). Enchantments of weapons and armour, Mending and the curses act by BDS's decompiled rules (`sapling_data::enchant`, `docs/research/enchantments.md`; not drilled: a `bds_*` drill of a Sharpness hit, a Protection hit and a Thorns hit would check them, and of Knockback's throw). The enchanting table, anvil, grindstone and smithing table craft by BDS's decompiled rules (`workstations.rs`, `inventory/forge.rs`, `docs/research/workstations.md`, `workstations`; not drilled: a `bds_*` drill of a table's offers for a known seed and of an anvil's costs would check them; the doc's "Unverified" list). The newest four effects act on mobs and players (`effects3`, `player-omens`, docs/research/mob-kinds.md). Left there: the doc's "Sapling" list.
2. **Redstone**: plates, buttons, levers, blocks of redstone and torches power dust, lamps, doors,
   trapdoors, gates and hopper locks (`blocks/redstone.rs`, `docs/research/redstone.md`; plugins
   read `ctx.power_at`), checked against `bds_command_probe redstone`. Circuits a world folder
   holds are found as their columns load. Repeaters pass, delay
   and hold (`bds_command_probe repeaters`). Comparators
   compare, subtract and read containers (`bds_command_probe comparators`). Observers
   pulse (`bds_command_probe observers`). Pistons push, break and
   draw back (`blocks/pistons.rs`, `bds_command_probe pistons`). Slime and honey take what they touch
   (`blocks/pistons/plan.rs`, `bds_command_probe slime`). What stands in a piston's way is pushed before it, a player too (`blocks/pistons/shove.rs`, `shove`, `pushed`). Left: the circuits of an arena's
   template, and what a comparator reads of a lectern or a wall frame (a jukebox is read by its disc, `discs`; a
   decorated pot by its stack and a flat frame not at all, `holders`). Dust does not lag a lever or
   button worked by hand (`byhand`; docs/research/redstone.md, "Observers").

## 3. Things assumed, not verified

- Checked 2026-10-24 (docs/research/assumptions-checked.md): mirrored shaped recipes match BDS for every
  vanilla recipe (its `assume_symmetry` is never false; sapling does not read it); BDS keeps a container
  screen open however far its player goes and closes it when the block goes (`reach-close`, now copied);
  orbs are taken by BDS's pickup area with no age gate, lava stops at BDS's `isLavaBlocking` list
  (`blocks/liquid/lava.rs`), the 16 fire-resistant items are listed by name, and `stripped_bamboo_block`
  burns; the orb touch runs from the player's side (`orbs::touch`). Left there: whether the 2-tick
  spacing between orbs taken is BDS's `+0x3c4` lookup.
- A door's hinge was checked against BDS with a block and with a door on either side (`bds_command_probe
  sounds`); the Java game's tie-break by where the click fell is still left out.
- The sounds of a door, trapdoor, gate, chest, barrel and ender chest opening and shutting were
  heard on BDS and agree, tick for tick (`sounds`, `shut`), with a barrel's top a tick late and
  an open lid shown again 80 ticks from its opening; left there: how late a shulker box sounds.
- Block loot is the Java game's tables under Bedrock names (`sapling-data/data/block-loot.tsv`);
  26 blocks broken with a diamond pickaxe left the same items in the same numbers on BDS
  (`bds_command_probe loot`, research/captures/bds-loot.txt), but copper ore, which left 3 each of
  three times there and 2 to 5 here, and 2 to 5 there too in twelve more (`loot2`). Ores leave experience and ice water as there
  (`blocks/after_break.rs`); the experience of 20 broken ores there was within the Java game's range
  for each (coal 0 to 2, diamond 3 to 7, redstone 1 to 4, lapis 2 to 5, quartz 3 and 4, nether gold 0 and 1).
- 75 blocks put down by hand from one stance took the same state on BDS and here in 65
  (`bds_command_probe placing`, research/captures/bds-placing.txt), after dispensers, pistons,
  observers, gates, anvils, bells, grindstones, amethyst, frames and rails were set right. Scaffolding and trapdoors (`trapdoors`: `oak_trapdoor` is no item, the
  drill's mistake) take BDS's states too. A placer is shown the clicked and the placed cell again,
  each with the cells above and below, and in creative its windows twice, as on BDS
  (`place-faces`: top, side and bottom faces); left: BDS's own copy of the placement comes after
  those six, sapling's before. A second run of 51 more (`placing2`) agreed in 31
  and set right a crafter, a decorated pot, a chiseled bookshelf, a calibrated sculk sensor, what
  clings (lichen, sculk vein, resin) and what hangs (moss, roots, a spore blossom, a vine, a
  hanging sign) and a pitcher plant, which stands on its own lower half; a vault is not woken. BDS's commands take old item names too (`chain`
  for `iron_chain`): sapling's `/give` and `/replaceitem` take the 31 that were tried and found
  (`vanilla.rs` `OLD_ITEM_NAMES`, `bds_command_probe item-names`). Which way a dispenser or a
  piston turns up or down is the old game's rule, of which BDS showed one case.
- BDS's `/setblock` and `/testforblock` take old block names; sapling takes the 34 that were
  tried and found (`vanilla/blocks.rs` `OLD_NAMES`, `loot2`, `names`) and no others. A name BDS
  does not know is a syntax error there and `commands.setblock.noChange` here.
- Whether a vanilla client names a hopper's slots `Container` is the bot's assumption.
- A placed shulker box's `facing` is the face clicked (`shulker-ender.md`).
- Who fences, panes and walls join, and what gives a rail, wire or repeater support, are the Java
  game's lists and collision boxes; the dirt and sand tags sugar cane asks for are Java's
  (`docs/research/standing-and-shapes.md`, "Open checks").
- Stair corners and hanging-sign states by Java's rules; a placed skull's `SkullType` (-1 as BDS
  leaves it), the cauldron's `PotionId` defaults (`docs/research/block-entities.md`, "Open checks").
- A double chest asks both halves above, a slab too, as on BDS (`bds_command_probe lids`). Getting
  up by `StopSleeping`, by day and by `/tp`, and a sleeper's data and box, are told as on BDS
  (`wake`); left there: a player's other `FlagsExtended` bits, which sapling never sends. A block a hand
  uses, a block held or not, is shown as it changed and no more, as on BDS (`held-use`); a door opened
  by hand keeps `open_bit` in its lower half and is shown whole in one UpdateSubchunkBlocks, as
  there. A door `/setblock` puts is shown so too, its upper half with flags 2.
- Which block each liquid rule belongs to was read by where the block's name sits in BDS's code:
  sure for whole families (stairs, slabs, walls), not for single blocks (`waterlogging.md`).

## 4. A vanilla client session

No real client has played against any of the survival code: only acacia's bot has. One session with
the game itself, walking through break, place, doors, beds, chests, double chests, crafting by hand
and by the book, the stonecutter, furnaces and water, and noting every kick, desync and wrong
picture. Needs a person at the client.

## 5. Ground truth from the test BDS

Record the bot doing the same things against the BDS in `acacia/.testserver` and diff the packets
with sapling's (`tools/bds-drill.sh` runs a probe on a fresh world; `bds-blocks` is a copy kept for
the drills, port 19182): chest and furnace traffic was never recorded (`research/containers-crafting.md`
says where it is decompiled only), nor door and bed placement. One agent at a time uses that
server. A hopper's rate, shulker box drops, a furnace's result and fuel slots and experience orbs
were recorded on a testbox BDS by `cargo run -p sapling --example bds_container_probe` (2026-10-06).

## 6. World generation, the rest

The overworld generates block-exact against a one-worker, seed-0 BDS (`tools/replay-all.sh`, every
region 0); the stock oracle (`tools/oracle.sh`) is a regression signal only, since stock BDS does not
repeat itself (`docs/research/worldgen-bds-order.md`). The Nether and the End generate the same way
(`tools/nether-replay.sh`, `tools/end-replay.sh`), with their dimensions, portals and the dragon fight
(`docs/research/nether-dimension.md`, `docs/research/end-dimension.md`, whose "Not done" lists what is
left). Left:

- Wall times on a quiet testbox: every speed result so far compares instructions
  (`tools/bench.sh`, `docs/research/worldgen-perf.md`).
- Cache room for spawn areas beyond the generator's areas, if mass joins matter: spawn priority
  costs +13–30% instructions when 8–16 players join at once (`docs/research/worldgen-server-load.md`).
