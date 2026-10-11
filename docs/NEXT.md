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
   saved back where they are. Items let go and orbs are saved with their column (`drops.rs`, `orbs.rs`). Left: saving falling blocks.
2. **Vanilla commands**: built (`sapling::vanilla`); the drill probe runs on sapling
   (`tools/sapling-drill.sh`, on testbox `tools/remote-drill.sh`). Left: what the BDS-against-sapling
   diff of the drill captures still shows (`research/captures/sapling-vs-bds-findings.md`).
   `/weather` and `/toggledownfall` set a world's weather, told as BDS told it (`weather.rs`,
   `bds_command_probe weather`); it never changes of itself, and BDS's `query`, which lags a change
   behind, is not copied.
   `/time`, `/xp` and `/tp` to a target answer as BDS did (`vanilla/clock.rs`, `bds_command_probe
   clock`: the answers agree line for line; BDS writes its experience attributes in another
   order). A sapling capture that answers `commands.generic.unknown` or `.syntax` where BDS's
   does not names a command or a form still missing.

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
   there: the kinds that shoot other things than arrows, the other kinds that turn on their hitter (`provoke` drilled a wolf and a zombified piglin). Survival players have health: falls,
   fire, lava, water over the head and a block around them hurt, `/damage` and `/kill` too; they die
   leaving items and experience and rise by their bed or at the world's spawn (`vitals.rs`,
   `docs/research/player-damage.md`, `bds_command_probe hurt` and `smother`). A wounded player mends by its
   hunger and an empty hunger starves it (`vitals/hunger.rs`, `hunger`). Food in the hand is eaten
   (`vitals/eating.rs`, `eat`). Jumps, sprints, harvested blocks and hits tire a player (`tire`, `toil`). Armour stops part of a hit of the causes it stopped on BDS (`armour`, `worn`). Twenty-five effects by `/effect` (`vitals/effects.rs`, `docs/research/effects.md`, `effects`); food and potions give theirs and milk ends them (`docs/research/food-effects.md`, `sick`, `drink`). A player hits a player where `pvp` is on (`creatures/duels.rs`, `pvp`, `duel`). Left there: the newest four effects and their potions, and the doc's "Sapling" list.
2. **Redstone**: plates, buttons, levers, blocks of redstone and torches power dust, lamps, doors,
   trapdoors, gates and hopper locks (`blocks/redstone.rs`, `docs/research/redstone.md`; plugins
   read `ctx.power_at`), checked against `bds_command_probe redstone`. Circuits a world folder
   holds are found as their columns load. Repeaters pass, delay
   and hold (`bds_command_probe repeaters`). Comparators
   compare, subtract and read containers (`bds_command_probe comparators`). Observers
   pulse (`bds_command_probe observers`). Pistons push, break and
   draw back (`blocks/pistons.rs`, `bds_command_probe pistons`). Slime and honey take what they touch
   (`blocks/pistons/plan.rs`, `bds_command_probe slime`). What stands in a piston's way is pushed before it, a player too (`blocks/pistons/shove.rs`, `shove`, `pushed`). Left: the circuits of an arena's
   template, and what a comparator reads of a jukebox, lectern or item frame. Dust does not lag a lever or
   button worked by hand (`byhand`; docs/research/redstone.md, "Observers").

## 3. Things assumed, not verified

- Mirrored shaped recipes are accepted (`Recipe::is_laid_out`) from memory of the game; decompile
  `ShapedRecipe::matches` to confirm.
- A door's hinge was checked against BDS with a block and with a door on either side (`bds_command_probe
  sounds`); the Java game's tie-break by where the click fell is still left out.
- The sounds of a door, trapdoor, gate, chest, barrel and ender chest opening were heard on BDS and
  agree (`sounds`); those of a chest, barrel and ender chest shutting were not heard there.
- Block loot is the Java game's tables under Bedrock names (`sapling-data/data/block-loot.tsv`);
  26 blocks broken with a diamond pickaxe left the same items in the same numbers on BDS
  (`bds_command_probe loot`, research/captures/bds-loot.txt), but copper ore, which left 3 each of
  three times there and 2 to 5 here, and 2 to 5 there too in twelve more (`loot2`). Ores leave experience and ice water as there
  (`blocks/after_break.rs`); the experience of 20 broken ores there was within the Java game's range
  for each (coal 0 to 2, diamond 3 to 7, redstone 1 to 4, lapis 2 to 5, quartz 3 and 4, nether gold 0 and 1).
- 75 blocks put down by hand from one stance took the same state on BDS and here in 65
  (`bds_command_probe placing`, research/captures/bds-placing.txt), after dispensers, pistons,
  observers, gates, anvils, bells, grindstones, amethyst, frames and rails were set right. Left: a
  sea pickle out of water (dead on BDS), scaffolding's `stability_check`, and a trapdoor, whose row was spoilt. A second run of 51 more (`placing2`) agreed in 31
  and set right a crafter, a decorated pot, a chiseled bookshelf, a calibrated sculk sensor, what
  clings (lichen, sculk vein, resin) and what hangs (moss, roots, a spore blossom, a vine, a
  hanging sign); a pitcher plant, which BDS stood two high on stone, is not placed here, and a
  vault is not woken. BDS's commands take old item names too (`chain`
  for `iron_chain`): sapling's `/give` and `/replaceitem` take the 31 that were tried and found
  (`vanilla.rs` `OLD_ITEM_NAMES`, `bds_command_probe item-names`). Which way a dispenser or a
  piston turns up or down is the old game's rule, of which BDS showed one case.
- BDS's `/setblock` and `/testforblock` take old block names; sapling takes the 34 that were
  tried and found (`vanilla/blocks.rs` `OLD_NAMES`, `loot2`, `names`) and no others. A name BDS
  does not know is a syntax error there and `commands.setblock.noChange` here.
- Whether a vanilla client names a hopper's slots `Container` is the bot's assumption.
- A placed shulker box's `facing` is the face clicked (`shulker-ender.md`).
- An orb is taken within Java's pickup box once 7 ticks old (`orbs.rs`): one recorded pickup; BDS's
  touch rule was not decompiled.
- Wooden blocks burn by a list of names where BDS reads a material (`furnaces/fuel.rs`).
- Who fences, panes and walls join, and what gives a rail, wire or repeater support, are the Java
  game's lists and collision boxes; the dirt and sand tags sugar cane asks for are Java's
  (`docs/research/standing-and-shapes.md`, "Open checks").
- Stair corners and hanging-sign states by Java's rules; a placed skull's `SkullType` (-1 as BDS
  leaves it), the cauldron's `PotionId` defaults (`docs/research/block-entities.md`, "Open checks").
- The distance at which BDS closes an open container, whether a double chest asks both halves
  above, and the handler for `PlayerAction StopSleeping` (`docs/research/beds-and-containers.md`).
- Which block each liquid rule belongs to was read by where the block's name sits in BDS's code:
  sure for whole families (stairs, slabs, walls), not for single blocks (`waterlogging.md`).
- Lava burns what has no collision; BDS asks each block (`isLavaBlocking`, 23 overriding classes).
- Which items lava does not burn: netherite's and ancient debris, by name (`drops.rs` `fireproof`);
  BDS reads a flag of each item.

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
