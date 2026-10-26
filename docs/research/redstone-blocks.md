# Bedrock redstone blocks: note block, jukebox, tripwire, target, lightning rod, copper bulb, comparator overrides

Read 2026-10-25 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (PE stamp 6ab54bee, packs up to
vanilla_1.26.52) with bdsre `c|dis|vt|refs|callers|str|rtti` plus private scratch scripts: a registration walker for
the note-block instrument component, a vtable-override differ, and a simulator for the RecordItem switch. BlockType slot
names come from LeviLamina `BlockType.h` (header virtual index − 13 = vtable slot; 132 slots). Event layouts come from
LeviLamina `block_events/*.h`. Comparator/circuit vocabulary as in `redstone.md` (`Producer 0x80000`, `comp+0x53`
allowPowerUp, `comp+0x64` mAttachedAllowed, `comp vt+0x20` setStrength). Daylight detector and observer are out of
scope. Tags: [decompile] = read from code; [inferred] = deduced, not read directly. No drill was run.

Shared VAs: `BlockSource::blockEvent(pos,b0,b1)` 1415fa670 (runs `Block::triggerEvent`, slot 85, on the server, then
the listeners send BlockEventPacket); `addToTickingQueue(queue,region,pos,block,delay,0)` 1415f50f0; find circuit
component 1414febc0; `CircuitSystem::setStrength` 144bd6b90; Producer factory 14257d0e0(circuit,pos,region,type,dir);
Level `playSound(region,LSE,pos,data,…)` = Level vt+0x6a8 (via 14a59a850); Level `broadcastLevelEvent` = vt+0x698.
State globals: powered_bit 14cd246a0, attached_bit 14cd26488, disarmed_bit 14cd24700, suspended_bit 14cd24640,
direction 14cd24f50, facing_direction 14cd26368, lit 14cd25b50, sculk_sensor_phase 14cd242e0.

## Note block (vt 14a8b5030, legacy id 25)

| Handler | VA |
|---|---|
| interact (subscribed) | 1465edb80 |
| redstone update (subscribed) | 1465ee2a0 |
| attack (slot 81) | 1465ee420 |
| triggerEvent (slot 85) | 1465ee6f0 |
| canPlay / instrument | 1448705c0 |
| MusicBlockActor load / save | 144870540 / 1448704e0 |
| instrument component setter (registration) | 144c2c560; lookup 144c2c600; default data 14cb31fc0 |
| instrument names | 146fc8e50, table 14a8d8c80 |

- B1 Redstone: skip when `mIsFirstTime`; `powered = signal > 0`; act only if `powered != actor.powered (+0xf1)`; play
  only when the new value is true; always store `actor.powered = powered`. So it plays on the rising edge only.
  [decompile]
- B2 Interact (server): `note = (note + 1) % 25`, mark the actor changed, then try to play. The note advances even
  when playing is blocked. Result 0x101. Head bypass: if `block+0x378 == 1` (registration writes 0) and the event
  flag +0x34 is set and the held item's block is a mob head → return 0x100 without tuning. [decompile]
- B3 Attack: plays the current note without advancing, if `player+0x269 == 0`. [decompile]; what that flag means is
  unverified.
- B4 canPlay looks at y+1:
  - a mob head (list 14ccdb7d0..d8, not 14ccda528) → the head's own instrument, plays (behind a version gate,
    globals 14cc48618/06/00);
  - air → plays, instrument = the block below's component byte (no component = harp 0);
  - anything else → silent.
  [decompile]
- B5 Block actor NBT: only byte `note` (load clamps it to ≤ 24). `powered` is NOT saved, so it starts as false
  after a load. [decompile]
- B6 Play = `blockEvent(pos, b0 = instrument, b1 = note)` plus game event 14cc898a8. triggerEvent then:
  - instrument ≤ 19: `playSound(LSE Note = 81, pos = block corner, data = instrument << 8 | note)` and a note
    particle at (x+.5, y+1.2, z+.5) coloured note/24;
  - instrument > 19 (heads): `data = instrument << 8 | 9`, no particle.
  The BlockEventPacket goes out after the sound, with eventType = instrument and data = note. [decompile]; packet
  field mapping [inferred] from the chest (type 1 = open).
- B7 Instrument ids: 0 harp, 1 bd, 2 snare, 3 hat, 4 bass, 5 flute, 6 bell, 7 guitar, 8 chime, 9 xylophone,
  10 iron_xylophone, 11 cow_bell, 12 didgeridoo, 13 bit, 14 banjo, 15 pling, 16 trumpet, 17 trumpet_exposed,
  18 trumpet_weathered, 19 trumpet_oxidized, 20 zombie, 21 skeleton, 22 creeper, 23 enderdragon,
  24 witherskeleton, 25 piglin, 26 none. [decompile]
- B8 Instrument by block below (every other block → harp 0). [decompile]
  - 1 bd: stone and stone-like (stone, ores, cobblestone, bricks, sandstones; stone/deepslate/tuff/blackstone/basalt
    families; netherrack, nylium, quartz, prismarine, purpur, end stone, obsidian, crying obsidian); spawners;
    furnaces; dispenser, dropper, observer, crafter; enchanting table, end portal frame, respawn anchor, lodestone;
    dead corals and coral fans; calcite, amethyst block, dripstone; mud/resin bricks; concrete; all terracotta.
  - 2 snare: sand, red sand, gravel, suspicious sand and gravel, heavy core, concrete powder.
  - 3 hat: glass, panes, stained and hard glass, tinted glass, beacon, sea lantern.
  - 4 bass: all wood blocks (planks, logs, stems, hyphae, stairs, slabs, fences, gates, doors, trapdoors, signs,
    hanging signs, buttons, plates, shelves); note block, bookshelves, chests, crafting table, jukebox, daylight
    detectors, banners, mushroom blocks, lectern, cartography/fletching/smithing tables, barrel, composter, bee nest,
    beehive, bamboo block and mosaic, creaking heart, camera.
  - 5 flute: clay, honeycomb block, infested blocks, resin block.
  - Single blocks: 6 bell = gold block; 7 guitar = wool; 8 chime = packed ice; 9 xylophone = bone block;
    10 iron_xylophone = iron block; 11 cow_bell = soul sand; 12 didgeridoo = pumpkin; 13 bit = emerald block;
    14 banjo = hay block; 15 pling = glowstone.
  - Copper (blocks, cut, stairs, slabs, chiseled; waxed the same as unwaxed): 16/17/18/19 by oxidation, behind a
    version gate (14cc48418); harp otherwise.
- B9 Heads above (y+1): zombie 20, skeleton 21, creeper 22, dragon 23, wither skeleton 24, piglin 25. No player head.
  [decompile]
- B10 No comparator override (reads 0). [decompile]

## Jukebox (vt 14a7edb50; interact 142fd9a80, setupRedstoneComponent 142fda560, getComparatorSignal 142fdaa30)

- B11 Circuit: a Producer with no direction (dir 6), strength 0, `+0x64 = 1`, `+0x53 = 1`, so it powers all six
  sides. [decompile]
- B12 Inserting a disc: strength 15, then setRecord 1457fd3c0. Interacting with a disc inside: strength 0, then eject
  142fda600 (the disc drops at a free neighbour cell and the state is cleared). [decompile]
- B13 Tick 1457fcab0 runs every 20 ticks (counter +0x308):
  - while `ticksPlayed × 0.05 ≤ duration`: note particle plus LSE 441 RecordPlaying;
  - else: mark finished and stop the sound.
  While finished it sets strength 0 every tick. So the signal is 15 from insert until the song ends (found at a
  20-tick check) or the disc is removed. [decompile]
- B14 Comparator = the disc's signal when a disc is in (flag +0x2d2), else 0. Discs (signal, duration s), from the
  RecordItem switch 142ef08b0 [decompile]:

  | Disc | Signal | Duration (s) |
  |---|---|---|
  | 13 | 1 | 178 |
  | cat | 2 | 185 |
  | blocks | 3 | 345 |
  | chirp | 4 | 185 |
  | far | 5 | 174 |
  | mall | 6 | 197 |
  | mellohi | 7 | 96 |
  | stal | 8 | 150 |
  | strad | 9 | 188 |
  | ward | 10 | 251 |
  | 11 | 11 | 71 |
  | wait | 12 | 238 |
  | pigstep | 13 | 149 |
  | otherside | 14 | 195 |
  | 5 | 15 | 178 |
  | relic | 14 | 218 |
  | creator | 12 | 176 |
  | creator_music_box | 11 | 73 |
  | precipice | 13 | 299 |
  | tears | 10 | 175 |
  | lava_chicken | 9 | 134 |
  | bounce | 8 | 234 |

## Tripwire hook (vt 14a7a37b0) and tripwire (vt 14a7a3390)

| Function | VA |
|---|---|
| hook place / tick / setupRedstoneComponent / onRemove | 14257b8d0 / 14257bd70 / 14257cdf0 / 14257d390 |
| hook calculateState(region, pos, wireSource) | 142579770 |
| hook sounds | 14257c4d0 |
| wire place / tick / entityInside | 1425781c0 / 142578b60 / 14257a820 |
| wire gate / checkPressed / notify hooks | 14257a670 / 14257a860 / 142578f30 |
| wire playerWillDestroy / visual shape | 142579260 / 142578ce0 |

- B15 Hook circuit: a Producer facing the attached block. `direction` 0/1/2/3 maps to facing 2/5/3/4 (north, east,
  south, west). Strength 0, `+0x64 = 1`, `+0x53 = 1`. Strength 15 when the hook is powered. So it strongly powers
  the block it hangs on and weakly powers everything else. [decompile]; strong vs weak per `redstone.md` R11.
- B16 Hook place: calculateState(−1); if it can't survive, a tick with delay 1. The tick runs calculateState(−1) and
  destroys the hook if it can't survive. onRemove: if attached or powered, calculateState(−1); then the component is
  removed. [decompile]
- B17 calculateState walks i = 1..41 along the facing:
  - it stops at a hook facing back; that hook's distance is D;
  - each wire is recorded as valid = !disarmed and as powered (counted only if not disarmed);
  - any other block breaks the chain.
  Then `attached = D > 1 && every wire valid`, and `powered = attached && any wire powered`. When a wire calls
  (wireSource ≥ 0), that wire gets a tick with delay 10. [decompile]
- B18 Writes:
  - The far hook (at D) and this hook each get `powered_bit = powered`, `attached_bit = attached` (setBlock flags 3)
    and strength 15/0. Each plays its own sounds.
  - If D > 1 and `attached` changed, every valid wire is rewritten with its powered, disarmed and
    `attached_bit = attached`.
  [decompile]
- B19 Hook sounds, at (x+.5, y+.1, z+.5), first match only: powered rising → LSE 73 PowerOn; powered falling →
  74 PowerOff; attached rising → 75 Attach; attached falling → 76 Detach. Each also posts a game event
  (14cc8a93c, 14cc8a944, 14cc8a9bc, 14cc8a9b4). [decompile]
- B20 Wire: entityInside / shouldTriggerEntityInside fire only on the server, only if the actor's vt+0x120 returns
  true (likely `canInteractWithOtherEntitiesInGame`), and only while the wire is unpowered. They query entities in
  the shape. [decompile]
- B21 Wire shape for entity checks: not suspended → (0,0,0)-(1,0.09375,1); suspended → y 0.0625..0.15625 when the
  second flag is set (likely attached), else y 0..0.5. [decompile]
- B22 checkPressed / tick (delay 10): any entity in the shape counts (no type filter seen). If `powered` differs
  from "entities present", flip powered_bit, setBlock flags 3, notify the hooks and fire the script trip event.
  While entities remain, re-tick in 10. [decompile]
- B23 Notify hooks: scan both ways along the axis up to 41 cells for a hook facing the wire, then calculateState on
  it with the wire's distance. [decompile]
- B24 Breaking a wire:
  - with shears: set powered_bit 0, post the shear game event 14cc8a934;
  - otherwise: set powered_bit 1.
  Then notify the hooks. Nothing sets disarmed_bit here. [decompile]

## Target (vt 14a8eead0; onProjectileHit 1471f22a0, core 1471f2530, tick 1471f20b0, setupRedstoneComponent 1471f2200)

- B25 Hit point: the ProjectileComponent hit position. A fireworks rocket (actor type 0x48) uses actor+0x218.
  Any other actor without a ProjectileComponent is ignored. [decompile]; which entity types carry the component
  (arrow, trident, snowball, egg, ...) is [inferred].
- B26 Strength: `d = hit − (pos + .5)`; the dominant |axis| is the face; u, v are the other two;
  `s = clamp(trunc((0.70710677 − sqrt(u² + v²)) × 16 / 0.70710677), 1, 15)`. [decompile]
- B27 Strength is held by a Producer with no direction (dir 6), recreated per hit, `+0x53 = 1`. Nothing goes into a
  block state. So it powers all six sides weakly (no strong face). It also fires the owner's achievement/script
  event 142d3ecc0(s) and updates neighbours. [decompile]
- B28 Duration: a tick with delay 20 if the actor type has bit 0x800000 (arrow, trident), else 8. The tick
  recreates the component at 0. [decompile]

## Lightning rod (vt 14a94f590; onLightningHit 148b5b4f0, tick 148b5aa20, setupRedstoneComponent 148b5ac70, onProjectileHit 148b5af20)

- B29 Lightning hit:
  - if a tick is already queued (1415f48d0), cancel it (1415f5520);
  - set powered_bit 1 (setBlock flags 3) and strength 15;
  - queue a tick with delay 8;
  - level event 2033 ParticlesElectricSpark at pos.
  [decompile]
- B30 Tick (server, chunk loaded, same block): powered_bit 0, strength 0. So it stays powered for 8 ticks.
  [decompile]
- B31 Circuit: Producer with dir = opposite(facing_direction) (byte map 0x40502030001), which is the block it is
  mounted on. Strength `powered_bit ? 15 : 0`, `+0x64 = 1`, `+0x53 = 1`. So it strongly powers the mounted-on block.
  [decompile]
- B32 onProjectileHit: forwards to the block above when that block is in list 14ccd8468. Otherwise, if
  14119d830(y+1) and 1415f7e40 pass, it spawns a lightning_bolt (actor 0x5d) at pos+.5. This is trident
  channeling: sky + thunder [inferred].

## Copper bulb (vt 14a94a0c0; redstone 148b3a260, tick 148b39a60, comparator 148b3a440, light 148b3a550, onLightningHit 14714c9c0)

- B33 Redstone update: if `(signal == 0) == powered_bit` (the state is stale), queue a tick with delay 1. [decompile]
- B34 Tick (server):
  - `now` = circuit strength != 0; if `now == powered_bit`, return;
  - set `powered_bit = now`;
  - if it was unpowered (rising edge): `lit = !lit`, plus LSE 490 CopperBulbTurnOn (newly lit) or 491
    CopperBulbTurnOff, at the block corner;
  - setBlock flags 3.
  [decompile]
- B35 Light = lit ? L : 0, where L is set per variant at registration: 15/12/8/4 for unoxidised, exposed, weathered,
  oxidised. Waxed variants are the same. [decompile]; registration order mapped to oxidation [inferred].
- B36 Comparator = lit ? 15 : 0 (hasComparatorSignal true). [decompile]
- B37 onLightningHit: the copper behaviour 'scrape' (de-oxidise one step via 148b38390) plus level event 2033.
  [decompile]

## Comparator overrides (slot 90 hasComparatorSignal / slot 91 getComparatorSignal)

- B38 Jukebox 142fdaa30: B14. Copper bulb 148b3a440: B36. Note block, target and tripwire hook: none (0). [decompile]
- B39 Crafter 147141810 = number of disabled slots (popcount of actor+0x101; 9 → 9) + number of non-empty slots.
  [decompile]
- B40 Sculk sensor 148b45650: when `sculk_sensor_phase == 1` (active), the frequency of its last vibration
  (1411b3910); else 0. [decompile]

## Sapling deltas found

- notes.rs sends BlockEvent type 1 (ChangeState) always; Bedrock sends type = instrument (B6).
- notes.rs picks the instrument by block-name heuristic; use B8/B9.
- Notes are not stored in a block actor; Bedrock saves byte `note` only (B5).
- jukebox.rs has no bounce (8, 234 s) and needs the redstone signal 15 while playing (B13).

## Unverified

- U1 The version gates on the head-above and copper-trumpet instruments (14cc48618/14cc48418).
- U2 `comp+0x64` = mAttachedAllowed is from LeviLamina field order, not from a decompiled read.
- U3 Jukebox `+0x2d2` "has disc" flag. Bounce disc LSE id 606.
- U4 Actor type 0x48 = fireworks_rocket. Which projectiles carry ProjectileComponent and count for the target.
- U5 Note-block attack flag player+0x269. BlockEventPacket (type = b0, data = b1) mapping.
- U6 Instrument sites with no name resolved: 1484cd64a (hat), 1484f0477 (bd), 148518eb5 (bass).
- U7 Actor vt+0x120 = canInteractWithOtherEntitiesInGame (wire trigger filter). Wire suspended-shape flag = attached.
- U8 Bulb light order 15/12/8/4 = unoxidised..oxidised (registration order). Lightning rod channeling conditions.
- U9 Whether hook strong power reaches the block it hangs on exactly as for a button (`redstone.md` R11); a drill
  should confirm.
