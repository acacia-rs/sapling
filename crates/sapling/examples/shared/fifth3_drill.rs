//! The fifth wave's rain and knockback drills (research/parity/FIFTH-BRIEF.md "Group 3"): bees
//! sheltering as rain sets in and clears (`beerain`), sleep and the night's skip as rain sets in
//! and clears (`rainnight`), how fast rain fills a chunk of cauldrons (`cauldronrate`), and the
//! pillagers' prey drill with their moves and the actor's knockback shown (`pillswitch2`), and
//! which wet mobs a conduit hunts (`conduitrain`, `conduitwater`, `conduitdepth`). On open sky:
//! `SAPLING_DRILL_WORLD=open` on sapling.

use super::creatures::{floor, waits, with};

const BEES_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "LevelSoundEvent", "LevelEvent"];
const NIGHT_SHOWN: &[&str] = &["SetTime", "SyncWorldClocks", "LevelEvent", "LevelEventGeneric", "Text", "PlayerAction", "Animate"];
const CAULDRONS_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "UpdateSubChunkBlocks", "LevelEvent"];
const PREY_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "MovePlayer", "EntityEvent", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "AddItemEntity", "RemoveEntity"];
/// A chunk's 256 columns: cauldrons on stone, open to the sky, two chunks east of the bots.
const CAULDRONS: &str = "32 -27 0 47 -27 15";

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "beerain" => (bee_rain(), BEES_SHOWN),
        "rainnight" => (rain_night(), NIGHT_SHOWN),
        "cauldronrate" => (cauldron_rate(), CAULDRONS_SHOWN),
        "pillswitch2" => (super::last::drill("pillswitch")?.0, PREY_SHOWN),
        "conduitrain" => conduit_rain()?,
        "conduitwater" => conduit_water()?,
        "conduitdepth" => conduit_depth()?,
        "conduitspill" => conduit_spill()?,
        "conduitreach" => conduit_reach()?,
        _ => return None,
    })
}

/// Three bees by a hive at noon with no flower near; rain: they go in. 35 s of rain, past their
/// stay; clear: they come out.
fn bee_rain() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule dodaylightcycle false", "time set noon", "setblock 6 -26 0 beehive [\"direction\"=3]", "summon bee 5.5 -25 -1.5", "summon bee 5.5 -24 -2.5", "summon bee 7.5 -25 1.5"]);
    waits(&mut lines, 6);
    lines.push("weather rain".into());
    waits(&mut lines, 35);
    lines.push("weather clear".into());
    waits(&mut lines, 15);
    with(lines, &["kill @e[type=!player]", "setblock 6 -26 0 air", "gamerule dodaylightcycle true", ""])
}

/// The actor back on its mark and into the bed east of it.
const SLEEP: [&str; 2] = ["tp @s 0.5 -26 0.5", "!sleep 1 0 0"];

fn sleep_then(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(SLEEP.map(String::from));
    waits(lines, seconds);
    lines.extend(["!wake", ""].map(String::from));
}

/// The rain's own window of night (12009 on, the clear one 12541): at 12200 clear; rain begun a
/// second before, then 5 s before, slept 7 s; slept in full rain that clears; at 23800 in full
/// rain, slept 7 s; a clear night for the skip's own packets.
fn rain_night() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule playerssleepingpercentage 50", "gamerule dodaylightcycle true", "gamerule doweathercycle true", "gamemode survival @s", "effect @s resistance 1000 4 true", "tp CmdWatcher -3.5 -26 0.5", "setblock 1 -26 0 bed", ""]);
    lines.extend(["time set 12200", ""].map(String::from));
    sleep_then(&mut lines, 1);
    lines.extend(["time set 12200", "weather rain"].map(String::from));
    sleep_then(&mut lines, 1);
    lines.extend(["time set 12200", "", "", ""].map(String::from));
    sleep_then(&mut lines, 7);
    lines.extend(["weather rain", "", "", "", "", "", "time set 12200"].map(String::from));
    lines.extend(SLEEP.map(String::from));
    lines.push("weather clear".into());
    waits(&mut lines, 7);
    lines.extend(["!wake", "weather rain", "", "", "", "", "", "time set 23800"].map(String::from));
    sleep_then(&mut lines, 7);
    lines.extend(["weather clear", "time set 18000", ""].map(String::from));
    sleep_then(&mut lines, 7);
    with(lines, &["gamemode creative @s", "gamerule playerssleepingpercentage 100", "gamerule doweathercycle false", "weather clear", "time set 1000", "setblock 1 -26 0 air", ""])
}

/// conduit2's conduit up to its whole frame (vitality_drill.rs), then at midnight in rain: a held
/// zombie on land 7 off before the conduit ever took one, one in the pool 5 off, and another there
/// once that one is dead.
fn conduit_rain() -> Option<(Vec<String>, &'static [&'static str])> {
    let (mut lines, shown) = framed_conduit()?;
    let held = HELD;
    lines.extend(["time set midnight", "fill 24 -21 -1 26 -21 1 stone", "fill 24 -20 -1 26 -19 1 water", "weather rain", "", "", "", "summon zombie 20.5 -20 -6.5", held].map(String::from));
    lines.extend(liquid_at("20 -20 -7"));
    waits(&mut lines, 12);
    lines.extend(["kill @e[type=zombie]", "summon zombie 25.5 -20 0.5", held].map(String::from));
    waits(&mut lines, 16);
    lines.extend(["kill @e[type=zombie]", "summon zombie 25.5 -20 0.5", held].map(String::from));
    waits(&mut lines, 12);
    lines.extend(["kill @e[type=!player]", "weather clear", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", ""].map(String::from));
    Some((lines, shown))
}

const HELD: &str = "effect @e[type=zombie] slowness 60 255 true";

/// Which water, if any, holds `cell`, by `testforblock`.
fn liquid_at(cell: &str) -> [String; 2] {
    ["water", "flowing_water"].map(|name| format!("testforblock {cell} {name}"))
}

/// conduit2's lines up to its whole frame (its cube's water spills over the floor round it).
fn framed_conduit() -> Option<(Vec<String>, &'static [&'static str])> {
    let (mut lines, shown) = super::vitality::drill("conduit2")?;
    let framed = lines.iter().position(|line| line == "tp @s 20.5 -20 8.5")?;
    lines.truncate(framed + 1);
    Some((lines, shown))
}

/// The framed conduit at midnight, clear: a glass channel on a shelf 4 over the floor, a water
/// source at its west end flowing east; a held zombie in the source, then 3 and 5 cells down the
/// flow.
fn conduit_water() -> Option<(Vec<String>, &'static [&'static str])> {
    let (mut lines, shown) = framed_conduit()?;
    lines.extend(["weather clear", "time set midnight", "fill 16 -15 -6 24 -15 -4 stone", "fill 16 -14 -6 24 -14 -4 glass", "fill 17 -14 -5 23 -14 -5 air", "setblock 17 -14 -5 water", "", "", ""].map(String::from));
    for at in ["17.5 -14 -4.5", "20.5 -14 -4.5", "22.5 -14 -4.5"] {
        lines.extend([format!("summon zombie {at}"), HELD.into()]);
        lines.extend(liquid_at(&format!("{} -14 -5", &at[..2])));
        waits(&mut lines, 12);
        lines.push("kill @e[type=zombie]".into());
    }
    lines.extend(["kill @e[type=!player]", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", ""].map(String::from));
    Some((lines, shown))
}

/// The framed conduit at midnight, clear: a held zombie in a glass cell on a shelf 4 over the
/// floor, its water set at `liquid_depth` 4, 3, 2, 1 in turn (a `/setblock` liquid is not ticked
/// on BDS: bds-conduitwater.txt).
fn conduit_depth() -> Option<(Vec<String>, &'static [&'static str])> {
    let (mut lines, shown) = framed_conduit()?;
    lines.extend(["weather clear", "time set midnight", "fill 19 -15 -6 21 -15 -4 stone", "fill 19 -14 -6 21 -13 -4 glass", "setblock 20 -13 -5 air"].map(String::from));
    for depth in [4, 3, 2, 1] {
        lines.extend([format!("setblock 20 -14 -5 water [\"liquid_depth\"={depth}]"), "summon zombie 20.5 -14 -4.5".into(), HELD.into()]);
        waits(&mut lines, 7);
        lines.extend([format!("testforblock 20 -14 -5 water [\"liquid_depth\"={depth}]"), "kill @e[type=zombie]".into()]);
    }
    lines.extend(["kill @e[type=!player]", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", ""].map(String::from));
    Some((lines, shown))
}

/// The framed conduit's spill where `conduitrain`'s first zombie stood, and over it, by depth; at
/// midnight a held zombie in the spill 3 off, then one where the first stood walled in glass, its
/// water set at that depth.
fn conduit_spill() -> Option<(Vec<String>, &'static [&'static str])> {
    let (mut lines, shown) = framed_conduit()?;
    waits(&mut lines, 3);
    for y in [-20, -19] {
        lines.extend((0..16).map(|depth| format!("testforblock 20 {y} -7 water [\"liquid_depth\"={depth}]")));
    }
    lines.extend(["time set midnight", "summon zombie 20.5 -20 -3.5", HELD.into()].map(String::from));
    waits(&mut lines, 8);
    lines.extend(["kill @e[type=zombie]", "fill 19 -20 -8 21 -19 -6 glass", "setblock 20 -20 -7 water [\"liquid_depth\"=4]", "setblock 20 -19 -7 air", "summon zombie 20.5 -20 -6.5", HELD.into()].map(String::from));
    waits(&mut lines, 8);
    lines.extend(["kill @e[type=!player]", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", ""].map(String::from));
    Some((lines, shown))
}

/// The framed conduit at midnight, clear: a held zombie in a two-deep glass well level with it,
/// 6, then 7 blocks north.
fn conduit_reach() -> Option<(Vec<String>, &'static [&'static str])> {
    let (mut lines, shown) = framed_conduit()?;
    lines.extend(["weather clear", "time set midnight", "fill 19 -20 -9 21 -14 -4 air"].map(String::from));
    for dz in [-6, -7] {
        lines.extend([format!("fill 19 -19 {} 21 -16 {} glass", dz - 1, dz + 1), format!("fill 20 -18 {dz} 20 -17 {dz} water"), format!("summon zombie 20.5 -18 {}.5", dz), HELD.into()]);
        waits(&mut lines, 8);
        lines.extend(["kill @e[type=zombie]".to_string(), format!("fill 19 -19 {} 21 -16 {} air", dz - 1, dz + 1)]);
    }
    lines.extend(["kill @e[type=!player]", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", ""].map(String::from));
    Some((lines, shown))
}

/// Full rain over a chunk of empty cauldrons for 20 s, then the rain clearing for 8 s.
fn cauldron_rate() -> Vec<String> {
    let mut lines = with(floor(), &["fill 32 -28 0 47 -28 15 stone", &format!("fill {CAULDRONS} air"), "fill 32 -26 0 47 -16 15 air", "weather rain"]);
    waits(&mut lines, 6);
    lines.push(format!("fill {CAULDRONS} cauldron"));
    waits(&mut lines, 20);
    lines.push("weather clear".into());
    waits(&mut lines, 8);
    with(lines, &[&format!("fill {CAULDRONS} air"), ""])
}
