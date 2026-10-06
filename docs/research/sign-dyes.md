# Dyes, glow ink, ink sac and honeycomb on signs (2026-10-06)

What BDS does when a player clicks a sign (standing, wall, hanging: one handler) with an item. Conventions and tags
as block-entities.md: **BDS** = bdsre on `acacia/.testserver/bds-molang/bedrock_server.exe` (VA), **JAVA** =
server-26.3.jar via javap, **GEY** = Geyser-Standalone.jar, **PROTO** = acacia-proto generated enums.

Handler: `SignBlock` subscribes `BlockPlayerInteractEvent` in its ctor `142fce6c0`; the handler is `148b55920`
(subscription thunk `148b555c0`). Item name checks compare against the vanilla name HashedStrings
(`14ccc5f80` honeycomb, `14ccbda08` glow_ink_sac, `14ccc98c0` ink_sac; set in `14222a750`).

## Sign text side fields (in-memory -> NBT)

| Offset in side | NBT (side save `144883590`) | Notes |
|---|---|---|
| +0x140..0x14c | `SignTextColor` Int | 4 floats r,g,b,a in 0..1; saved as `(int)(a*255)<<24 + (int)(r*255)<<16 + (int)(g*255)<<8 + (int)(b*255)` (each `cvttss2si`, clamped 0..1) |
| +0x150 | `IgnoreLighting` Byte | glowing text |
| +0x151 | `HideGlowOutline` Byte | this handler only ever writes 0 |
| sign +0x100 | `IsWaxed` Byte | whole sign, not per side |
| sign +0x108 | `LockedForEditingBy` | editor lock (see block-entities.md) |

Side picked: `144886220(sign, player)` = back when `(player - shape centre) . (-sin r, cos r) < 0` (x,z; r = block
`getYRotation`), else front. Same rule sapling `signs.rs::front` uses for the editor. Legacy worlds (base game
version below a constant, not read) always get front.

## Order of checks in `148b55920`

| # | Condition | Effect | Evidence |
|---|---|---|---|
| 0 | Item has tag `minecraft:hanging_sign` (tag string init `1402cf2f0`) | item's own use (vt+0x418, places a hanging sign); if it did, stop, click not consumed | BDS |
| 1 | Player game type Adventure (2, or Default with world Adventure) | nothing (result 0x100) | BDS `141cd3970` + level vt+0x460 |
| 2 | Lock id set and the locking player exists | nothing; a stale lock is cleared | BDS |
| 3 | `IsWaxed` = 1 (modern world) | **any** item or empty hand: LevelSoundEvent **468 `WaxedSignInteractFail`** at the block position, extraData -1, no entity; click consumed (0x101); nothing changes, no editor | BDS `148b55920` (`0x1d4`); PROTO `SoundType::WaxedSignInteractFail = 468`; BDS string `block.sign.waxed_interact_fail`; GEY sounds.json playsound `block.sign.waxed_interact_fail` |
| 4a | Holding `minecraft:honeycomb`, wax check `148b57760` passes (not waxed, not adventure, lock free; **no text needed**) | non-creative: stack count - 1 (explicit game-type check, creative = 1 or Default+world creative); `IsWaxed=1`; **LevelEvent 2030 `ParticleWaxOn`** at block centre (x+.5,y+.5,z+.5), data 0 | BDS; PROTO `LevelEventEvent::ParticleWaxOn = 2030` |
| 4b | Holding `minecraft:glow_ink_sac`, not waxed, not adventure (`140d5db90`, ActorGameTypeComponent), facing side `IgnoreLighting`=0 and its text non-empty (`144886fb0`, size != 0) | side `IgnoreLighting=1`, `HideGlowOutline=0`; `Player::useItem(item, Place(6), consume=true)`; **LevelEvent 1066 `InkSackUsed`** at the block position (corner), data 0 | BDS; PROTO `1066 => InkSackUsed` |
| 4c | Holding `minecraft:ink_sac`, ink check `148b57890`: not waxed, not adventure, lock free, side `IgnoreLighting`=1, text non-empty | side `IgnoreLighting=0`, `HideGlowOutline=0`; useItem(Place, consume); **LevelEvent 1066** (same as glow) | BDS |
| 4d | Item `isDye` (vt+0x80) and dye check `148b57970`: not waxed, not adventure, lock free, colour differs (each channel x255 truncated compared), text non-empty | side colour = table below, alpha 1.0; `HideGlowOutline=0`; glow unchanged; useItem(Place, consume); **LevelEvent 1065 `DyeUsed`** at block position, data 0 | BDS; PROTO `1065 => DyeUsed` |
| 5 | Anything else (empty hand, other item, or a 4x check that failed), sign not waxed | opens the editor: player vt+0x668 (`OpenSign`, `is_front` = side == front) | BDS |

So: glow ink on an already glowing side, ink sac on a non-glowing side, a dye of the current colour, or any of
them on a side with empty text **open the editor** (step 5) instead of doing nothing; no sound. After 4a-4d the
click is consumed (no editor, no item use). Every change sets the entity's changed bit (+0x4c |= 1), i.e. resent.

Consumption (4b-4d): `Player::useItem` (`1402ab270`, Player vtable slot 0x370) removes one when `consume` and the
player lacks the instabuild ability (AbilitiesComponent); creative keeps the item. It also fires the ItemUsed event.

Sounds: no `dye.use`/`ink_sac.use`/`glow_ink_sac.use` strings exist in BDS; the client plays the sound from the
LevelEvent. GEY maps Java `item.dye.use` -> `sign.dye.use`, `item.ink_sac.use` and `item.glow_ink_sac.use` ->
`sign.ink_sac.use`, `item.honeycomb.wax_on` -> `copper.wax.on` (playsound); Bedrock has no glow-specific event.

## Which items are dyes (vt+0x80 `isDye`, vt+0x88 `getItemColor`)

| Items | Class (vtable, ctor) | isDye | ItemColor |
|---|---|---|---|
| 16 `*_dye` (`black_dye` ... `white_dye`) | `14a805c10`, ctor `14201c750` (16 calls in `141fc2880`) | always true (`1400a2880`) | ctor arg, byte +0x218 |
| `ink_sac` (colour 0), `lapis_lazuli` (colour 4) | `14a806030`, ctor `14201cc10` | byte +0x220 = legacy flag | +0x218 |
| `bone_meal` | `14a908240` (built in `141fc2880` @`141fcb804`) | legacy flag | always 15 (white) |
| `cocoa_beans`, everything else | base Item | false | - |

Legacy flag (`141fcb6b5`): true only for worlds whose base game version is below a constant (same pattern as the
side/waxed checks); a current world (`*`/any) gets false. Ink sac is caught by name in 4c before 4d anyway.

## Colour per dye (BDS table `14a7716e0`, 8-byte entries, colour at +4; read by `148b568aa`)

ItemColor 0 is special-cased to (0,0,0): black dye writes the default black, so on a fresh sign it is a no-op
(-> editor). Saved Int = `0xFF000000 | rgb` (float round trip checked exact for all 16).

| ItemColor | Item | RGB | `SignTextColor` Int |
|---|---|---|---|
| 0 | black_dye | 000000 (table has 1D1D21, unused) | -16777216 |
| 1 | red_dye | B02E26 | -5231066 |
| 2 | green_dye | 5E7C16 | -10585066 |
| 3 | brown_dye | 835432 | -8170446 |
| 4 | blue_dye | 3C44AA | -12827478 |
| 5 | purple_dye | 8932B8 | -7785800 |
| 6 | cyan_dye | 169C9C | -15295332 |
| 7 | light_gray_dye | 9D9D97 | -6447721 |
| 8 | gray_dye | 474F52 | -12103854 |
| 9 | pink_dye | F38BAA | -816214 |
| 10 | lime_dye | 80C71F | -8337633 |
| 11 | yellow_dye | FED83D | -75715 |
| 12 | light_blue_dye | 3AB3DA | -12930086 |
| 13 | magenta_dye | C74EBD | -3715395 |
| 14 | orange_dye | F9801D | -425955 |
| 15 | white_dye | F0F0F0 | -986896 |

Item -> ItemColor: black (name global `14ccc9a30`), red (`14ccc8438`), green, brown (`14ccc9a60`), blue
(`14ccc9a90`), white (`14ccc9ac0`) read directly from the ctor calls; 5-14 by the legacy dye-aux name table
(`142175900`, aux order ink_sac, red, green, cocoa, lapis, purple, cyan, light_gray, gray, pink, lime, yellow,
light_blue, magenta, orange, bone_meal, black, brown, blue, white), whose name globals match the ctor calls.

GEY `SignBlockEntityTranslator.getBedrockSignColor` uses Java's diffuse colours (white F9FFFE, orange F9801D, ...;
no black case -> -16777216, as BDS). All 14 coloured entries equal the BDS table (which also confirms the
name -> ItemColor order above); only white differs (Geyser F9FFFE, BDS F0F0F0). Use the BDS table.

## Java cross-check (JAVA)

`SignBlock.useItemOn`: SignApplicator item, `mayBuild`, not waxed, no other editor, `canApplyToSign` (text has a
message) -> `tryApplyToSign`. `DyeItem`/`GlowInkSacItem`/`InkSacItem.tryApplyToSign` = `updateText` (false when
unchanged) + `DYE_USE`/`GLOW_INK_SAC_USE`/`INK_SAC_USE`. Same rules as BDS; Bedrock differs by opening the editor
on a failed apply and by the waxed-fail sound on any click.

## For sapling (`blocks/signs.rs` TODO, `edit_by_click`)

Before opening the editor: adventure -> nothing; waxed -> LevelSoundEvent 468 at pos and stop; else match the held
item per 4a-4d, edit the facing side's compound, `send::keep_entity`, take one item unless creative, broadcast the
LevelEvent; only when nothing applied, open the editor.
