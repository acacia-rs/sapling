# What players loose and throw

Bows, crossbows, tridents and thrown things on BDS 1.26.x, from two sources: the vanilla behaviour
pack's entity definitions (`behavior_packs/vanilla*/entities.brarchive`, the `minecraft:projectile`
component of each kind), and bdsre's decompile of `.testserver/bds-blocks/bedrock_server.exe`. That
build has no ported names, so the function names below are labels given while reading. Its
addresses are listed so the code can be found again. Nothing here was drilled against a running
BDS; see "Unverified".

## The pack's projectiles

| kind | power | gravity | inertia / in liquid | other |
|---|---|---|---|---|
| `arrow` (`player_arrow` group) | 3.0 | 0.05 | 0.99 / 0.6 | `power_multiplier` 2, `ceil_pre_critical_damage`, box 0.25 |
| `arrow` (`player_crossbow_arrow`) | 3.15 | 0.05 | 0.99 / 0.6 | as above |
| `thrown_trident` | 4.0 | 0.1 | 0.99 / 0.99 | damage 8, box 0.25 x 0.35, sounds `item.trident.hit`, `item.trident.hit_ground` |
| `snowball` | 1.5 | 0.03 | 0.99 / 0.6 | 3 damage to blazes only, knockback |
| `egg` | 1.5 | 0.03 | 0.99 / 0.6 | spawns a baby chicken with chance 1/8; then 4 with chance 1/32 |
| `ender_pearl` | 1.5 | 0.025 | 1 / 1 | `teleport_owner`, endermite with chance 5% |
| `splash_potion`, `lingering_potion` | 0.5 | 0.05 | 0.99 / 0.6 | `angle_offset` -20, glass sound |
| `xp_bottle` | 0.5 | 0.05 | 0.99 / 0.6 | `angle_offset` -20, 3 to 11 experience |
| `wind_charge_projectile` | 1.5 | 0 | 1 / 1 | box 0.3125, burst `knockback_scaling` 1.22, damage 1 |

A thrown wind charge leaves at 2.25 a tick, its item's `launch_power_scale` 1.5 over the power
(bds-q3wind.txt, 2.26). Meeting a block, it bursts 0.25 back from the face (particle data 1) and
pushes mobs as a blast of reach 2.4 pushes them, 1.22 times as hard: measured from the feet less
half the width, towards 0.9 up the body, added to the motion (two cows, 0.338 and 0.375, to 0.01).
Not copied: BDS's `disappeared`/`reappeared` sounds and its removal and `wind_charge.burst` a tick
after the burst.

A lingering potion leaves a cloud with `duration` 30, `radius` 3, `radius_on_use` -0.5 and
`reapplication_delay` 40.

## Bow: `BowItem::releaseUsing` 0x142eee8d0

- The draw is `t / 20` seconds of use, turned into `(s² + 2s) / 3` and capped at 1. A draw under 0.1
  shoots nothing. This is the Java game's curve.
- The arrow leaves at `power` (3.0) times the draw. Each axis of its aim is off by a gaussian
  × 0.0075 × uncertainty (1 for a player) before the power multiplies it (`ProjectileComponent::shoot`
  0x1424020c0). The shooter's own motion is not added: arrows loosed walking east left at 2.99 and
  3.02, as standing (research/captures/bds-t2ebow.txt). Sapling draws the gaussian (`shots/aim.rs`).
- A tipped arrow shows its item data (potion + 1) as a Byte under metadata key 18: an `arrow` of
  data 26 showed 26 (bds-t2epack.txt), as sapling's does.
- Only a full draw (exactly 1) sets the arrow's CRITICAL flag (flag 13).
- Power stores its level and multiplies the hit by `1.25 + 0.25 × level`. This is not the Java
  game's `+ 0.5 × level + 0.5`.
- Punch's strength is `1.6 × level`.
- Flame sets the arrow on fire for 100 ticks.
- Infinity marks the arrow as not pickable, but only for a plain arrow. A tipped arrow is still
  spent.
- Outside creative the bow wears by 1 a shot, and the offhand's arrows are spent before the
  inventory's. In creative nothing is spent and nothing wears. The bow sound is `LevelSoundEvent`
  0x15.

## Crossbow: `CrossbowItem::use` 0x1436ec1a0

- Loading takes `25 - 5 × Quick Charge` ticks.
- With Multishot it fires three arrows. Arrow `i` is turned `i × spread - spread` degrees; the spread
  is a field of the item that was not read, and sapling uses 10, the Java game's. Every arrow is
  critical, and only the middle one can be picked up.
- Piercing stores the level, and `level + 1` as the number of bodies the arrow may hit.
- The crossbow wears 3 with Multishot and 1 without. The shoot sound is 0xf8.

## An arrow's hit: `ImpactDamageSubcomponent::doOnHitEffect` 0x1424050a0

In order:

1. `|velocity| × power_multiplier`.
2. × Power's factor.
3. Impaling adds `+2.5 × level` against a target in water or rain.
4. Rounded up (`ceil`).
5. A critical arrow adds `(d × 0.5 + 2) × random`.

Punch throws the target `k × 0.6` across and 0.1 up. A tipped arrow gives each of its potion's
effects for `max(1, duration / 8)` (0x142410e20).

## Picking up: `AbstractArrow::playerTouch` 0x1424af530, `canPickup` 0x1424af800

- Only an arrow a player shot (`player` set) can be picked up.
- One lying on the ground can be picked up by any player.
- An Infinity, Multishot-side or creative arrow is removed with a sound and gives no item.

## Ender pearl

- `getCooldownDuration` 0x142cd0b00 is 20 ticks.
- On landing (`TeleportToSubcomponent` 0x14240a840) the owner is teleported to the hit point only if
  it is a player in the same dimension and not sleeping. It then takes 5 fall damage (cause 5),
  unless it is in creative.

## Trident: `TridentItem::releaseUsing` 0x141fa6b10

- A throw needs 10 ticks of use or more.
- The trident wears by 1 and leaves the hand outside creative.
- Loyalty stores the owner on the thrown trident.
- Riptide (0x141c644b0) works only in water or rain and not while riding, and then no trident is
  thrown.

## Other numbers

- A bottle o' enchanting gives `min + rand(max - min)` experience (0x142408580): 3 to 10.
- A cloud's `duration` is in seconds, stored as ticks: 600 (0x142409a00).

## What sapling does

Each kind's numbers are in `sapling_data::shots` and `enchant/archery.rs`. Flight, hits and landings
are in `creatures/flight.rs`, `hits.rs` and `landing.rs`. Using an item is in `shots/`.

Drilled 2026-11-05 (`bds_command_probe r1shots`, `r1stuck`; research/captures/bds-r1shots.txt, bds-r1stuck.txt):
- A stuck arrow nobody took went 60 s after it was shot, survival's and creative's alike (1200 ticks).
- A Loyalty III trident lay 5 ticks after it landed (`item.trident.hit_ground` there), then came home by Java's
  numbers: the motion kept 0.95 and pulled 0.05 a level along the way to the eyes, the position lifted besides by
  0.015 a level of the height to the eyes (`ThrownTrident.tick`); `item.trident.return` was told at the thrower's
  eyes with the second tick home. sapling: `shots/tridents.rs`.
- Sounds, each with no entity type and unique id -1, at the feet + 0.9: `bow`, `crossbow.loading.start`,
  `.middle`, `.end`, `crossbow.shoot`, `item.trident.throw`, and Riptide's `item.trident.riptide_<level>`
  (`riptide_3` at level III).
- A crossbow held in use was heard `crossbow.loading.start`, `.middle` and `.end` 4, 14 and 24 ticks in and was
  loaded the tick after, `ChargedItem` to the player and its watcher, before it was let go. sapling counts the use
  from 1 and sounds at a fifth, three fifths and the whole of the load (Quick Charge's fractions assumed), then
  loads (`shots/crossbows.rs` `tick`).

A stuck arrow or trident that a player may pick up is kept with its column, with the fields
`player`, `isCreative`, `auxValue` and `Trident` (sapling-leveldb `actors/stuck.rs`). Any other stuck
shot goes after 1200 ticks.

## Unverified

These are the Java game's, a guess, or not found in the decompile:

- the Multishot spread of 10 degrees;
- where a shot leaves from: 0.1 under the eyes;
- BDS's short pickup delay (Actor +0x1bc): drilled 2026-11-05 (bds-f1pickup.txt), a stuck arrow was
  taken 8 ticks after it lay still, as the 7-tick shake ran out; sapling waits the same since
  (`creatures/stuck.rs` `shaking`). The same drill's dispenser arrow bounced off the creative
  watcher, so an arrow now meets creative players (`flight/in_way.rs`);
- the wind charge cooldown (10), and the cooldowns of snowballs, eggs and potions (none);
- the lingering cloud's shrinking over its life (its pulse and shares were read: items.md, "Area
  effect clouds");
- a player's arrow throwing a mob 0.4, as a melee hit does;
- Riptide's launch (`3 × (1 + level) / 4`);
- the `chargedItem` tag of a loaded crossbow;
- most effect colours, which are Java's (sapling-data `shots.rs`);
- the five chick-hatching numbers, the snowball's 3 against a blaze and the bottle's 3 to 10 are the
  pack's, but were not seen in play.

Firework rockets as crossbow ammunition and Channeling are in wither-lightning-fireworks.md; both
drilled the same (research/captures/bds-t2efire.txt).
