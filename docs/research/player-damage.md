# A player harmed, killed and risen

What BDS sends, from one run of `bds_command_probe hurt` (2026-10-10, research/captures/bds-hurt.txt): a
survival player (the bot, runtime id 1) and a watcher; breath and blocks from `smother`
(research/captures/bds-smother.txt). No decompile. Sapling builds it in `sapling-shard/src/vitals.rs` and
`vitals/death.rs`; where it differs is under "Sapling" below.

## Harm

- Each harm is told at once as `EntityEvent` `HurtAnimation` to the player and to those who see it, and the
  health follows as `UpdateAttributes` `minecraft:health` a tick later.
- The event's `data` is the cause: -1 for `/damage` without one, 2 `entity_attack`, 5 `fall`, 6 `fire`,
  7 burning on, 8 `lava`, 9 `drowning`. A fall's event carries `fire_at_position`, the player's eyes.
- `/damage @s 1` twice a second apart took 1 each; two within the 10 tick guard were not tried.
- A fall (the bot's own falling, after `/tp` up): 6 blocks cost 3, 12 cost 8, 40 killed, with the sounds
  `fall.big` and `fall` at the feet.
- Fire: 1 on stepping in (cause 6) and the burning flag, then 1 every 10 ticks (cause 7) while in it; once
  out, 1 every 20 ticks for 8 seconds more, then the flag clears.
- Lava: 1 as fire (cause 6), then 4 every 10 ticks (cause 8).

## Breath and blocks

- With water at its eyes a player's `Air` (`SetEntityData`, told every tick) falls 1 a tick from 300. At
  0 it goes on to -20, the player is hurt 2 (cause 9) and `Air` is 0 again: 2 every 20 ticks.
- Out of the water `Air` rises 4 a tick to 300.
- `BREATHING` (Flags bit 35) is cleared the tick the eyes go under and set the tick they come out, told
  in one SetEntityData with that tick's `Air` ([Flags, Air]) to the player and its viewers
  (bds-m3breath.txt). Sapling: `Pose::holding_breath`, `vitals/stand.rs`.
- A block set over the head alone hurts nothing: BDS lays the player down (`BoundingboxHeight` 0.6 and a
  flag in `FlagsExtended`) until the block is gone.
- Both of its blocks filled with stone: 2 (cause 4) from about the 8th tick, then every 10 ticks, while
  `Air` falls as under water.

## Mending

- With hunger at 20 a hurt player gains 1 health every 80 ticks. Each gain adds 6 to
  `minecraft:player.exhaustion`; every 4 of exhaustion takes 1 from `player.saturation` (5 at first), and
  with saturation at 0 from `player.hunger`. All four are told by `UpdateAttributes`.

From `bds_command_probe hunger` (research/captures/bds-hunger.txt):

- The mending runs while hunger is 18 or more: hurt 14, a player mended six times and stopped at hunger 16.
- Exhaustion is spent 4 a tick, in the tick it is there: a mending's 6 leave 2, the next leaves 4 and the
  tick after 0. `/damage` adds no exhaustion.
- The hunger effect at amplifier 255 adds 1.28 of exhaustion a tick.

From `bds_command_probe tire` (research/captures/bds-tire.txt), the bot holding keys:

- A jump adds 0.05 of exhaustion, a jump while sprinting 0.2.
- Sprinting adds 0.1 a block run, counted only in ticks that end on the ground: between two sprinting
  jumps only the landing's step is added.
- Walking adds none.

From `bds_command_probe toil` (research/captures/bds-toil.txt) and `bite`:

- A block of dirt broken by hand adds 0.005. A block of stone broken by hand, which leaves
  nothing, adds none.
- A hit that lands on a cow adds 0.1, and so does one on an armour stand.
- Being hurt adds none: not by `/damage`, not by a zombie.

## Armour

From `bds_command_probe armour` (research/captures/bds-armour.txt), `/damage` on a player dressed
by `/replaceitem entity @s slot.armor.<head|chest|legs|feet>`:

- 5 as an entity's attack took 5 bare, 4.88 in a leather chestplate, 4.3 in an iron one, 2.5 in
  all of iron, 1.25 in all of diamond and 1.2 in all of netherite; 2 in the netherite took 0.432.
- That is the Java game's rule with its points and toughness: the hit is cut by
  `clamp(points - hit / (2 + toughness / 4), points / 5, 20) / 25`.
- Stopped the same way (`armour`, and `worn`, research/captures/bds-worn.txt, all of iron
  against 2): `/damage` with no cause, `fire`, `lava`, `contact`, `projectile`,
  `entity_explosion`, `block_explosion`, `lightning`, `thorns`, `magma`, `campfire`,
  `soul_campfire`.
- Not stopped: `fall`, `drowning`, `fire_tick`, `magic`, `wither`, `starve`, `void`, `freezing`,
  `stalagmite`, `sonic_boom`, `fly_into_wall`.
- `anvil`, `falling_block` and `stalactite` took 0.645 where the others took 0.88: a quarter
  goes to the helmet first (1.5 behind 15 points), and the helmet's slot is sent again.
- `/damage @s 2 suffocation` answers `commands.damage.failed` with the player, dressed or bare.
- No other hit by `/damage` changed an armour slot, and neither did six hits of a zombie (2.5
  each, 1.125 through the iron): wear goes in another packet ("Armour wear").
- Each cause flashes with its number: contact 1, projectile 3, block explosion 10, entity
  explosion 11, void 12, magic 14, wither 15, starve 16, anvil 17, thorns 18, falling block 19,
  fly into wall 21, magma 22, lightning 24, freezing 27, stalactite 28, stalagmite 29, sonic
  boom 31, campfire 32, soul campfire 33.

## Starving

- With hunger at 0 a player is hurt 1 (cause 16) every 80 ticks, first 80 ticks after the hunger ran out,
  down to 10 health on the drilled server (easy), and no further.
- Between 1 and 17 of hunger nothing mends and nothing hurts.
- The saturation effect at amplifier 9 for a tick gave 10 hunger.

## Eating

From `bds_command_probe eat` (research/captures/bds-eat.txt), the bot holding use on an apple:

- The click in the air sets the using flag (`Flags` bit 4) for the player and its viewers, and sends the
  player every attribute.
- The client's `EntityEvent` `EatingItem` comes back to it and goes to its viewers.
- 32 server ticks after the click BDS ends the use itself: `CompletedUsingItem` with the item's network id
  and `Eat`, the inventory, hunger and saturation, the flag cleared, and the sound `burp` 0.1 under the
  eyes.
- An apple gave 4 hunger and 2.4 saturation; the saturation is kept up to the hunger.
- With a full hunger the bot does not try, so what BDS answers then is not seen.

A creative player eats and drinks too (bds-b1eat.txt, `b1eat` in fourteenth1b_drill.rs): every attribute is told
as each use starts, potion and milk included; nothing is used up, and the whole inventory (four windows) is sent
as it ends; a potion leaves a glass bottle beside it unheard, honey and milk nothing; the burp, the vibration and
a chorus fruit's teleport are as in survival (`vitals/eating.rs` `keep`; test
`a_creative_player_eats_and_drinks_what_it_keeps_and_a_potion_leaves_a_bottle_as_well`). Left (REMAINING4): BDS
started no second meal of a food the held use asked for after the first, while it drank a drink twice; sapling
eats both twice. No rule was found.

## Shield

From bds-b1shield.txt and bds-b1ride.txt (fourteenth1b_drill.rs `b1shield`, `b1ride`), a sneaking survival bot
with a shield in the off hand:

- A mob's hit or shot turned away stands the player up: `FlagsExtended` raised and struck, then one packet with
  `Flags` without sneaking, the standing height 1.8, the same bits and `ArrowShooterId` 0, then the bits clear. The
  next tick the shield is shown down (`blocking_tick` 0) and the player sneaking (1.49) with the bits clear, then
  the shield goes up again as from a fresh sneak. A player's hit or arrow was not drilled; sapling stands no one
  up for it.
- A rider holding a shield has it up as it sits: raised and moving, the shield shown with that tick, raised alone
  two ticks later. Hits from ahead are turned away as on foot (the sound from the seated eyes); a dismount after
  that shows the bits clear. Without a shield the moving bit stays, as bds-m3rideview.txt had it.
- Thorns and fireworks from ahead are turned away.

Sapling: `vitals/shield.rs` (`blocks_mob`, `stand_up`, `sneak_again`, `tell_ride`, `settle_ride`); tests in
`tests/integration/fourteenth1b.rs`.

`/damage <player> ... entity <source>` (bds-b1shield.txt, bds-h2dmgp.txt): a raised shield facing the source turns
away `entity_attack`, `thorns`, `fireworks` and `projectile` (answer `commands.damage.failed`, no stand-up). Any
cause that lands, all 29 drilled, throws the player 0.4 away from the source across and 0.4 up, told before the
flash; from itself straight up. Sapling: `vitals/struck.rs` `sourced`, `ShardCtx::hurt_player_from`; tests in
`tests/integration/fourteenth2h.rs`. `mace_smash` (bds-m3mace.txt) flashes `HurtAnimation` 34, armour stops part
of it as of `entity_attack` (4 through a diamond chestplate left 17.024), a death without a source is
`death.attack.generic` and with one `death.attack.maceSmash.player` [victim, source] (a zombie's
`%entity.zombie.name`). Sapling: `DamageCause::MaceSmash`; test `fourteenth3m.rs`.

## Death

- To the player, in one tick: `InventoryContent` for the inventory, armour, cursor and off hand (several
  times over), `EntityEvent` `DeathAnimation`, `DeathInfo` with the cause and the player's name, `Text`
  translating the same (`death.attack.lava`, `death.attack.generic` for `/kill`,
  `death.fell.accident.generic`), `SetEntityData` with `PlayerLastDeathPosition`, health 0, and `Respawn`
  with state 0 at the player's eyes. Then `SetEntityData` shrinking the box to 0.2.
- To a watcher: the death animation, the same `Text`, health 0, the shrunken box.
- What it held lies as item entities from the next tick (5 stone: one `AddItemEntity`); 20 ticks after the
  death come its experience orbs (3 for 3 levels) and the sound `disappeared`.
- Rising, once the client answers: `Respawn` state 0 at y 32767, `Respawn` state 1 at the world's spawn,
  `EntityEvent` `Respawn`, `SetHealth` 20, the sound `reappeared`, and `UpdateAttributes` with health 20,
  hunger 20, exhaustion 0, saturation 5, level 0, experience 0. A watcher gets the event, an empty
  `MobEquipment`, the attributes and the box again. `reappeared` is heard where the body lay, not at the
  spawn; from another dimension at (0, 32767.1, 0).
- `bds_command_probe anchor` (research/captures/bds-anchor.txt): the death's `SetEntityData` holds the
  box (0.2) and then only what changed: `PlayerLastDeathPosition` (the cell of the eyes),
  `PlayerLastDeathDimension` (left out while it stays 0) and `PlayerHasDied` (the first death). The
  recovery compass reads these on the client. Kept as `DeathPositionX/Y/Z`, `DeathDimension` and
  `HasDiedBefore` (BDSRE: Player load 1402af220 reads them only together), and told again at join.
- `doImmediateRespawn` only reaches the client in `GameRulesChanged`; the server's answers were alike.

## A respawn anchor's spawn

From `bds_command_probe anchor`: glowstone charges it (`respawn_anchor.charge`); an empty hand on a
charged anchor in the Nether sends `SetSpawnPosition` (`Player`, the feet's cell, dimension 1, the
anchor), the grey `tile.respawn_anchor.respawnSet` and the sound `respawn_anchor.set_spawn`; the same
click again sends nothing. Killed, it rises as by a bed, with `respawn_anchor.deplete` at the anchor
after `Respawn` state 1 and the anchor a charge lower the next tick. An anchor at 0 is forgotten:
`SetSpawnPosition` of `Player` with every field `i32::MIN` and dimension 3 first, the trip home to the
overworld, and after `SetHealth` the white `tile.respawn_anchor.notValid`.

## A bed's spawn

From `bds_command_probe bedspawn` (research/captures/bds-bedspawn.txt): the bot puts a bed down by
day and clicks it.

- The click sends `SetSpawnPosition` of type `Player` with the cell the player stands in and the
  bed's head as the world position, and the grey lines `tile.bed.respawnSet` and `tile.bed.noSleep`.
- Killed, it rises there: `Respawn` state 0 naming that cell (whole numbers), then state 1 at the
  middle of the cell, eyes high. The line with y 32767 of a rise at the world's spawn is not sent.
- With the bed gone it is sent the cell once more, then rises at the world's spawn as before, and
  is told `tile.bed.notValid` (not grey).

## Drilled 2026-10-29 (`hurt2`, `deaths`, `fx3`, `void`)

research/captures/bds-hurt2.txt, bds-deaths.txt, bds-fx3.txt, bds-void.txt:

- An iron helmet given at 164 (`/replaceitem ... iron_helmet 1 164`) wore to 165 by a hit of 4 and
  stayed on: the data value is the wear.
- Exhaustion: a block that breaks at once costs 0.005; a hit on a mob inside its guard still
  costs 0.1; swimming 0.01 a metre.
- An empty hunger hurts down to 10 on easy, 1 on normal, and kills on hard (`death.attack.starve`).
- Out of breath inside a block the bot was pushed out, so whether it is hurt as under water is
  not seen. A `keepInventory` death keeps everything.
- `/damage` words: no cause and `entity_attack`, `contact`, `projectile`, `thorns`, `magma`,
  `campfire` say `death.attack.generic`; `fall` `death.attack.fall`; `block_explosion`
  `death.attack.explosion.by.bed`; the rest their own. `/damage ... entity <mob>` says
  `death.attack.mob`.
- `/damage <target> <n> <cause> entity <who>` (bds-fixmisc.txt): the damager may be a player or a
  mob, and the death names it (`death.attack.mob` with the zombie even for `projectile`); without a
  cause `entity` is a syntax error, as is `entity` with nothing after; a cow damager is
  `noTargetMatch`, two `commands.damage.tooManySources`. Sapling refused the form; fixed
  (`vanilla/damage.rs`, `ShardCtx::hurt_player_by`; tests
  `damage_takes_a_damager_after_a_cause_and_refuses_unknown_words`,
  `a_player_killed_by_damage_from_a_mob_dies_by_that_mob`). Re-drilled: every answer the same.
- A rise at the world's spawn is searched for at y 32767 above its column, then found at a spot
  within the spawn radius.
- Protection IV on an iron chestplate and Feather Falling IV on iron boots (8 points), each hit 10:
  `entity_attack`, `fire`, `projectile` and `block_explosion` took 7.392, `fall` 3.6, `magic` 8.4.
  The Java game's EPF rule: 4 a Protection level, 3 more a Feather Falling level for falls, each
  point 4 % off after the armour. Instant damage ignores the armour. A creative or spectator
  player answers `/damage` with `commands.damage.failed`.
- The void (`/tp` to y -110): 4 every half second (cause 12) until dead, `death.attack.outOfWorld`.

## Not drilled

Drilled 2026-11-04 (research/captures/bds-t2ewear.txt, bds-t2ebreath.txt):
- Armour worn out: each piece of four sets at the edge of its uses broke under `/damage 4 entity_attack`
  on both, after the same wear (53/78/73/63, 163/238/223/193, ...) and health.
- Head in stone for 15 s: `Air` went 299 and back to 300 on BDS, with no hurt; sapling sent no `Air`
  change and no hurt either (the one-tick dip is not copied).
- A bed used in the Nether blew up on both; BDS's player was also burning in that scene, so the
  amounts were not compared.

Drilled 2026-11-05 (research/captures/bds-n13play.txt, bds-n13more.txt, bds-n13items.txt):
- A Nether bed 4 blocks off, no armour, Resistance IV: 6.872 on easy, normal and hard alike, hurt
  cause 10 (`block_explosion`). That is the blast rule measured from the feet (34.36 before
  Resistance), plus 1, with no difficulty scaling. sapling took the eyes and scaled it (3.54,
  6.68, 10.03, cause 11); fixed (`creatures/blast.rs` `reaches`, `Rule::Burning`).
- Primed TNT 2 blocks off on easy, Resistance IV, health 60 (bds-r1ench.txt): 7.106, the blast rule
  unhalved (impact 0.717); Blast Protection IV took about a third off the harm and nothing off the
  throw. Every blast on BDS told the hit's 0.4 knock alone, then `HurtAnimation`, then knock and push
  (bds-boom.txt, bds-ghast.txt, bds-o3wither.txt); sapling told one motion after the hurt and halved
  TNT on easy. With the blast's `LevelEvent` shown, a rerun placed it 1.81 from the feet: 7.769 lost,
  impact 0.774, `1 - feet distance / 8`, the throw towards the eyes; sapling had measured from the
  eyes (0.70). Fixed (`blast.rs` `explode_primed`, `struck.rs` `hit_from`). Knockback I and II on a cow, and `/damage` of 10 by four causes bare, in
  diamond and in Protection IV diamond, were the same on both.
- Exhaustion: a fall, a zombie's hits and burning in fire cost none; breaking dirt and short grass
  0.005 each, a hit on a cow 0.1; sapling the same (sapling-n13play.txt).
- Wear: an iron sword lost 1 to a hit on a cow and 2 to breaking dirt, an iron pickaxe 2 and 1
  (3 each). A hit's wear was not sent; the break's came as one `InventorySlot` of the hand slot,
  where sapling sent the whole inventory; fixed (`inventory/wear.rs` `wear_tool_in_hand`).
- Leather at 79 of 80 stayed on under `/damage 4 entity_attack` and broke in the bed's blast,
  heard as `break` at the eyes, on both.
- Equip sounds: chainmail `armor.equip_chain`, golden `armor.equip_gold`, turtle shell
  `armor.equip_leather` (sapling said `equip_generic`; fixed), elytra `armor.equip_elytra`; all
  pieces at the slot's height over the feet: head 1.52, chest and legs 0.9, feet 0 (sapling the
  eyes; fixed, `inventory/armour_wear.rs` `SOUND_HEIGHTS`). `/replaceitem` into an armour slot
  sends that slot alone as `InventorySlot` (sapling the whole inventory; fixed, `put_on`).
- Donning over a worn piece (iron under diamond, iron under an elytra, iron with Binding under
  diamond, all in creative): the worn piece went to the hand on both.

## Another player's hit (`pvp`)

The actor hit the watcher, both in survival and on one spot (`research/captures/bds-pvp.txt`):

- A fist took 1, a wooden sword 5 and a diamond sword 8 of the other's 20 on easy: what the item
  costs a mob, with nothing taken off for the difficulty.
- Each hit that landed sent `HurtAnimation` 2 and `SetEntityMotion` (0, 0.4, 0) for the one hit,
  to both, and the sound `attack.strong` from the hitter; two hits at once landed once.
- A player killed by another (`duel`, research/captures/bds-duel.txt) is said to all as `Text` of
  the type `Translation`, `death.attack.player` with its name and its killer's, and told so in
  `DeathInfo`; sapling says every death that way now.
- With `pvp` off nothing was sent, not the sound. On peaceful a hit landed as before. A player in
  creative lost nothing; the sound alone was sent.

## Armour wear (`wear`)

The actor in all of iron (`research/captures/bds-wear.txt`). After a hit the client gets
`PlayerArmorDamage` with what each piece worn has lost in all, and no slot:

- A zombie's hit, 2 by a cactus and 2 by fire wore each piece by 1; 8 by a cactus by 2 and 20
  by 5: a quarter of the hit before the armour, rounded down, and 1 at least. Magic, which armour
  does not stop, wore nothing and sent nothing.
- 2 by an anvil wore each piece by 1, and then the helmet by 8, 8, 8, 10, 11 and 11 more, which
  the packet does not hold: the helmet is sent anew as `InventorySlot` with its `Damage` and a
  new stack id. The Java game's `hit x 4 + random x hit x 2`.
- The packet's slots read 0, 2, 4 and 6 for the four pieces.

## Sapling

- Built: health for survival and adventure players; falls, fire, lava, drowning and smothering; `/damage`,
  `/kill` and the plugin calls `health`, `hurt_player`, `kill_player`; death with what was held and
  experience dropped (not under `keepInventory`), the death screen, rising at the world's spawn.
- A fall costs each block begun past three, less the move the player lands in, which BDS does not
  count: 12 blocks cost the bot 8, its last tick's 1.19 left out; 6 cost 3.
- In lava (bds-g7lava.txt, `bds_command_probe g7lava`, 2026-11-05): BDS tells its catching fire (6, 1)
  the tick it comes, then the next tick lava's (8, 4 over the guard), then 8 every 10 ticks and no fire
  tick (7) till it is out. Sapling told 6 and 8 in one tick (health 16) and 7s in the lava; fixed in
  `vitals/stand.rs` (the lava's hit before the fire's own, once lit), test `vitals` lava; re-drilled the
  same unwarded (sapling-g7lava.txt). Under resistance 255 (g10lava, three rounds each, 2026-11-05): with
  lava set around it BDS told 6, then 8s, and one 7 about 1 s after the lava went, as sapling does
  (sapling-g10lava.txt). Teleported into lava, BDS's rounds differ among themselves (8s only; 6 then 8s; 6 and
  8 by turns), a sub-tick phase not modelled; sapling tells 6 then 8s. The earlier "8 at once, two 7s" was one
  such round.
- Run against sapling (research/captures/sapling-hurt.txt, sapling-smother.txt) the drills show the same
  flashes, causes and rhythm; the health differs where BDS mends (those runs were before sapling mended)
  or `/effect` heals, which sapling lacks.
- Glass and leaves smother nothing (glass drilled; leaves taken as glass).
- Mending and starving are built to the numbers above (`vitals/hunger.rs`), and so is what a jump and
  a sprint cost (`vitals.rs`), a block broken with what harvests it and a hit that takes health
  from a mob (an armour stand has none here), swimming, and a hit inside the guard; starving stops
  by the difficulty (`vitals/hunger.rs` `starved`). A durable item's data value is its wear, kept in
  its `Damage` tag (`inventory/plugin.rs`); armour in the hand goes on by a click in the air
  (`inventory/donning.rs`). `/damage` deaths are said as above (`vitals/death.rs` `told_words`);
  the `entity` form is parsed too (67e84e06). A rise at the world's spawn is searched for at y 32767 first,
  then found at a column within `spawnRadius` ("Rise spot" below).
- Armour stops what it stopped on BDS, by the same rule (`creatures/health.rs` `shielded`,
  `sapling-data/src/armour.rs`, `sapling_api::DamageCause::is_stopped_by_armour`); a helmet takes
  its quarter. What it stops wears it as measured (`inventory/armour_wear.rs`); how long a piece
  lasts is the Java game's number, and one that breaks goes silently.
- Eating is built (`vitals/eating.rs`, `sapling-data/src/food.rs`): the bot ate an apple and a steak on
  sapling in the same 32 ticks and packets (research/captures/sapling-eat.txt). Every food's values are
  drilled since (bds-foods.txt, bds-foods2.txt; food-effects.md), as are what else a food does and leaves.
- Run against sapling (research/captures/sapling-hurt2.txt, sapling-deaths.txt, sapling-void.txt,
  sapling-fx3.txt) after the fixes the causes, words, the helmet's 165, the enchanted armour's
  12.608 / 16.4 / 11.6, the search at y 32767 and the void's rhythm and depth match.
  `bedspawn` matches (sapling-bedspawn.txt). `anchor` could not be set up on sapling: its relative
  `/fill` in the Nether filled nothing (sapling-anchor.txt); tests hold the anchor (anchors.rs).
- Crawling under a block (bds-f1crawl.txt): teleported under a one-high ceiling, BDS told the player
  and its viewers, the tick after, `BoundingboxHeight` 0.6 and `FlagsExtended` bit 50; out again,
  1.8 and the bit cleared; the eyes told stayed at 1.62. Built 2026-11-05 (`movement/crawl.rs`).
  A player rises by the bed or Nether anchor it
  last clicked while it stands (`sleep/spawn.rs` `spawn_of`), and either waits in storage with the player's items, as does its last death (`vitals/last_death.rs`). Health, hunger, breath (`Air`) and burning (`Fire`) wait in storage with a player's items (`vitals/kept.rs`), as do effects (`vitals/effects.rs`); one who left dead comes back whole.

## Rise spot (decompiled 2026-11-05, Player::tickWorld 1402beeb0, check 1402b9f50)

- r = gamerule `spawnRadius` (default 10; clamped to 256 only under a level flag that defaults off).
  Inner grid reach `A = min(r, 8)`; outer grid reach `ceil((r - 8) / (2A))` past 8, cells 32 apart.
  Each grid: `n = (2R+1)²`, stride the largest prime ≤ 2R (2 below 3), start `nextInt(n)`, cell
  `(start + i * stride) % n`, x its remainder and z its quotient by the side, less R. Each outer cell
  walks the whole inner grid; an offset is kept if `x² + z² ≤ r²`; feet at the cell + 0.5.
- A candidate must be loaded and pass the safe-position check; none fits → the spawn block. Radius 0 →
  the spawn. bds-deaths.txt's rises (+2,+6), (-3,0), (-6,-2), (-3,-2), (-3,-3) fit the r 10 disc.
- BDS seeds a fresh xoroshiro128++ from the clock, so no pick is predictable; sapling throws its dice.
- sapling (`vitals/rise_spot.rs`): stands on the column's top block if solid and not lava, fire or
  magma. BDS's check (box nudges, headroom, which materials refuse) was not decoded.
