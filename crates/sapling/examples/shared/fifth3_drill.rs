//! The fifth wave's rain and knockback drills (research/parity/FIFTH-BRIEF.md "Group 3"): bees
//! sheltering as rain sets in and clears (`beerain`), sleep and the night's skip as rain sets in
//! and clears (`rainnight`), how fast rain fills a chunk of cauldrons (`cauldronrate`), and the
//! pillagers' prey drill with their moves and the actor's knockback shown (`pillswitch2`). On
//! open sky: `SAPLING_DRILL_WORLD=open` on sapling.

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
    let (mut lines, shown) = super::vitality::drill("conduit2")?;
    let framed = lines.iter().position(|line| line == "tp @s 20.5 -20 8.5")?;
    lines.truncate(framed + 1);
    let held = "effect @e[type=zombie] slowness 60 255 true";
    lines.extend(["time set midnight", "fill 24 -21 -1 26 -21 1 stone", "fill 24 -20 -1 26 -19 1 water", "weather rain", "", "", "", "summon zombie 20.5 -20 -6.5", held].map(String::from));
    waits(&mut lines, 12);
    lines.extend(["kill @e[type=zombie]", "summon zombie 25.5 -20 0.5", held].map(String::from));
    waits(&mut lines, 16);
    lines.extend(["kill @e[type=zombie]", "summon zombie 25.5 -20 0.5", held].map(String::from));
    waits(&mut lines, 12);
    lines.extend(["kill @e[type=!player]", "weather clear", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", ""].map(String::from));
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
