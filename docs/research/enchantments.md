# BDS enchantments: ground truth for sapling (2026-10-24, BDS 1.26.5x)

Tags as in `containers-crafting.md` (BDSRE, wiki, inf). Static reading only; nothing tested live.

- Exe `acacia/.testserver/bds-molang/bedrock_server.exe` (PE 6ab54bee):
  `BDS_EXE=<that> py -3 C:/Users/hexay/acacia/bdsre/bdsre.py c <va>`. Hand labels saved in `names.user.json`.
- Enchant vtable slots (LeviLamina `Enchant.h` order, 18 slots, 0x90 apart): 6 `getDamageProtection` (+0x30),
  8 `getDamageBonus` (+0x40), 9 `doPostAttack` (+0x48), 12 `doPostHurt` (+0x60).
- Enchants are read from the item's `ench` ListTag (`141a56170` splits them into Equipped / Held / Self lists).

## 0. Registry: `Enchant::initEnchants` 141b5be40 (BDSRE)

- Ids = LeviLamina `Enchant::Type`: protection 0, fire_protection 1, feather_falling 2, blast_protection 3,
  projectile_protection 4, thorns 5, respiration 6, depth_strider 7, aqua_affinity 8, sharpness 9, smite 10,
  bane_of_arthropods 11, knockback 12, fire_aspect 13, looting 14, efficiency 15, silk_touch 16, unbreaking 17,
  fortune 18, power 19, punch 20, flame 21, infinity 22, luck_of_the_sea 23, lure 24, frost_walker 25, mending 26,
  binding 27, vanishing 28, impaling 29, riptide 30, loyalty 31, channeling 32, multishot 33, piercing 34,
  quick_charge 35, soul_speed 36, swift_sneak 37, wind_burst 38, density 39, breach 40, lunge 41. Slot i of the
  registry is constructed with type i (ctor `141b5b7f0`). All match the list asked about.
- Vtables: ProtectionEnchant 14a773810 (ids 0-5, thorns included), water 14a7738a0 (6-8),
  MeleeWeaponEnchant 14a773930 (9-13), loot/dig 14a773c00, FrostWalker 14a773db0, Mending 14a773e40,
  curses 14a773ed0.

## 1. Protection family

- **Per enchant** `ProtectionEnchant::getDamageProtection(level, source)` 141af5980 (BDSRE):
  - cause Void(12), SelfDestruct(13), Starve(16): 0.
  - protection: `floor(level*1)`; fire_protection: `floor(level*2)` if `source.isFire()`;
    feather_falling: `floor(level*3)` if `source.isFallDamage()`; blast: `floor(level*2)` if cause is
    BlockExplosion(10) or EntityExplosion(11); projectile: `floor(level*2)` if cause Projectile(3); else 0.
  - thorns (5) also lives here and returns 0.
  - `isFire` 140d88ab0 = causes Fire 6, FireTick 7, Lava 8, Magma 22. `isFallDamage` 140d88b60 = Fall 5,
    Stalagmite 29 (BDSRE).
- **Total** `EnchantUtils::getDamageReduction(source, mob)` 141b64090 (BDSRE): sum of slot-6 over every enchant
  of every armour item (list from `141cd2250`, the armour container: inf), early-out once the sum >= 26; then
  `mult = 1 - 0.04*min(sum, 20)`. No random factor. Max 80 % reduction.
- **Applied** in `Mob::getDamageAfterEnchantReduction` 1429ecb10: `dmg *= mult` only if
  `source.isReducedByEnchantReduction()` (140d88af0: every cause except SonicBoom 31). Order relative to armour
  points not traced here (inf: after armour, as Java).

## 2. Fire protection and blast protection side effects

- **Burn time** `Actor::setOnFire(seconds, ...)` 142f1db30 (BDSRE): ticks = seconds*20; if total fire_protection
  levels `fp > 0` (`getTotalProtectionLevels(1, mob)` 141b64920 = sum of levels over armour, only for actors with
  category bit 0x100): `ticks = roundf(ticks * clamp(1 - 0.15*fp, 0, 1))`. OnFireComponent ticks =
  max(old, new). The same inline appears at every fire source (13 callers of 141b64920, all type 1).
- **Blast protection knockback**: not found. No call of the level helpers with type 3 anywhere; Bedrock may not
  reduce explosion knockback at all. Java: `kb *= 1 - 0.15*maxBlastLevel` (wiki, not decompiled).

## 3. Melee

- `MeleeWeaponEnchant::getDamageBonus(level, victim, attacker)` 141af52f0 (BDSRE), only if victim+0x210 & 2
  (an actor flag; true for mobs: inf):
  - sharpness `1.25*level`; smite `2.5*level` if victim family `undead`; bane `2.5*level` if family
    `arthropod` (family strings from initialiser 141afdbf0).
- `EnchantUtils::getMeleeDamageBonus(victim, attacker)` 141b645d0: sum over the **held** (carried) item's
  enchants, then **floor** (`cvttss2si` - (x<trunc)): sharpness I..V = 1,2,3,5,6; smite/bane I..V = 2,5,7,10,12.
- Damage order `1429e56c0` (BDSRE): attack attribute + item damage + item bonus, + enchant bonus (if victim
  flag), then strength `x*1.3+1` per level+1, weakness `x*0.8-0.5` per level+1.
- `MeleeWeaponEnchant::doPostAttack(attacker, victim, level)` 141af5370 (BDSRE), only if victim is alive (vt 0x170):
  - sharpness/smite/bane: server only: broadcast an AnimatePacket-like packet (action 5 = MagicCriticalHit,
    field 15.0) from attacker (vtable 14a74df20; inf on the packet name).
  - **bane**: if victim is `arthropod`: Slowness (effect id 2), duration `20 + nextInt(10*level)` ticks,
    **amplifier 0** (MobEffectInstance+0x20 = 0). Java uses amplifier 3; BDS reads 0.
  - **fire_aspect**: skip if victim creative (game type 1, or default 5 with world default 1) or actor flag 50
    FireImmune; `setOnFire(victim, 4*level seconds)` = 80 ticks/level, reduced by the victim's fire
    protection as in section 2.
  - knockback has no doPostAttack.
- **Knockback** (player attack 1402b84b0, BDSRE): `kb = 1.15*level + (attacker sprinting (flag 3) ? 1 : 0)`,
  level from `getKnockbackLevel_held` 1429e5910 (held item, enchant 12). Stored in the attack's damage event and
  applied in the hurt path (not traced). Sprint-crit `*1.5` damage is in the same function.

## 4. Thorns `ProtectionEnchant::doPostHurt(item, victim, attacker, level)` 141af5a50 (BDSRE)

- Called per armour slot 0..4 (5 slots incl. body) by `EnchantUtils::doPostHurtEffects` 141b64a10; each piece
  rolls separately. Skip if victim and attacker share a unique id.
- Trigger: `nextFloat() < 0.15*level` and attacker alive (vt 0x170).
- Damage: `level > 10 ? level-10 : nextInt(4)+1`; source cause Thorns 18 from the victim; sound
  LevelSoundEvent 82 at the victim's attach pos.
- Wear on that piece: `hurtAndBreak(3)` if triggered, else `hurtAndBreak(1)` (141a56c80, unbreaking applies),
  on top of normal armour wear.

## 5. Looting (BDSRE)

- Level = max(enchant 14 on the loot context's tool item (+0x98), best looting on the **killer's whole
  equipment**, filter All = 4: `getBestEnchantLevel` 1429e8a40), killer only if it has flag +0x210&2.
- `looting_enchant` 14752d800: if level != 0: `count += roundf(level * (min + nextFloat()*(max-min)))`
  (`count` range from the function JSON, floats). Not Java's per-unit nextInt.
- `random_chance_with_looting` 14581d5d0: pass if `nextFloat() < chance + level*looting_multiplier`.
- `killed_by_player`: not read.

## 6. Water and frost walker

- **Respiration** `BreathableSystem` 1483877d0 (BDSRE): best level over armour (filter 3); each air tick
  `air -= (nextInt(level+1) == 0) ? 1 : 0`. At `air <= min` (component field) drowning damage 2.0 and air = 0.
- **Depth strider** 1429e8550 (BDSRE, partial): best level over armour, capped at enchant 7's max level (3);
  feeds the server-side water movement speed with SwimSpeedMultiplier. Formula not extracted; Java blends
  water speed toward land speed by level/3 (wiki). Player movement is client-predicted (inf).
- **Aqua affinity**: not found by these helpers. wiki: removes the 5x underwater dig-speed penalty.
- **Frost walker** `FrostWalkSystem::fn0` 1484399b0 (BDSRE): skips riding (+0x269), horizontal motion
  ~0, spectators (game type 6); level from boots (armour slot 3), capped at 6; `r = level+2`; square loop
  dx,dz in [-r, r] around floor(feet pos): where the block at (x,y,z) is air and (x,y-1,z) is water
  (material 5) source, and frosted_ice may be placed there, set frosted_ice (flags 3). Frosted ice melt: wiki.

## 7. Mending (BDSRE, in `ExperienceOrb::playerTouch` 142f9e210)

- Item: `getRandomDamagedItemWithMending` 141b62840: all equipment (vt 0x4f8 = EquipmentFilter All: main hand,
  offhand, armour), non-empty, has mending, damaged; uniform `nextInt(n)`.
- `repair = min(item.Damage, value*2)`; `value -= repair/2` (int division); `Damage -= repair`; item written
  back to whichever slot matched (main hand, offhand, or armour 0..4). **One item only**, no loop; leftover
  value > 0 goes to `addExperience`.

## 8. Curses (BDSRE)

- **Vanishing**: ServerPlayer death handler 140759180: unless gamerule 13 keepInventory is on, every
  vanishing item without `minecraft:keep_on_death` is set empty in the inventory (`1402cd950`), cursor item,
  ContainerComponent, main hand, offhand and armour before drops. Test: `141a57260` = has vanishing.
- **Binding**: `1402bd9f0` (armour removable?) = item empty, or player creative (`140d5d940`: game type 1, or
  default 5 with world default 1), or no binding. So creative is exempt; survival/adventure locked. Other
  binding checks: 141a81ff0, 14242be40, 14242c670, 148b81dd0 (not read).

## Not settled

1. Knockback enchant application to velocity; blast-protection knockback (likely absent).
2. Aqua affinity dig speed; depth strider formula.
3. Whether getDamageReduction runs before or after the armour-point reduction.
4. What the victim flag +0x210 bit 1 is (gates enchant damage bonus and looting killer).
