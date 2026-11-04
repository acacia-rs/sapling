//! The seventh wave's mob movement drills (research/parity/SEVENTH-BRIEF.md "Group 1"): unslowed mobs
//! strolling on a walled floor (`strolls`), penned mobs turning their heads under speed and slowness
//! (`headspeed`), and the horse family left alone on grass and on stone (`horseeat`). All at fixed
//! places in the open sky; drill sapling with `SAPLING_DRILL_WORLD=open`.

use super::creatures::waits;

const STROLLS_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta"];
const HORSES_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityData", "LevelSoundEvent", "ActorEvent", "EntityEvent", "UpdateBlock"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "strolls" => (strolls(), STROLLS_SHOWN),
        "headspeed" => (head_speed(), STROLLS_SHOWN),
        "horseeat" => (horse_eat(), HORSES_SHOWN),
        "strollstart" => (stroll_start(), STROLLS_SHOWN),
        _ => return None,
    })
}

/// A stone floor 25 across walled three high, cleared above, both bots in its middle.
fn walled(time: &str, ground: &str) -> Vec<String> {
    let lines = [
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "weather clear",
        "difficulty easy",
        "gamemode creative @s",
        "gamemode creative CmdWatcher",
        "effect @s clear",
        "fill -13 -27 -13 13 -24 13 stone",
        "fill -12 -26 -12 12 -16 12 air",
    ];
    let mut lines: Vec<String> = lines.map(String::from).into();
    lines.extend([format!("time set {time}"), format!("fill -12 -27 -12 12 -27 12 {ground}")]);
    lines.extend(["tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""].map(String::from));
    lines
}

const END: [&str; 4] = ["kill @e[type=!player]", "fill -13 -27 -13 13 -16 13 air", "time set noon", ""];

/// Two cows, two sheep and two pigs by day for 150 s, then three zombies by night for 120 s,
/// left alone on the walled floor (moves told every tick within 10.7 of a bot).
fn strolls() -> Vec<String> {
    let mut lines = walled("noon", "stone");
    for at in ["cow -6.5 -26 -6.5", "cow 6.5 -26 6.5", "sheep -6.5 -26 6.5", "sheep 6.5 -26 -6.5", "pig 0.5 -26 -8.5", "pig 0.5 -26 8.5"] {
        lines.push(format!("summon {at}"));
    }
    waits(&mut lines, 150);
    lines.extend(["kill @e[type=!player]", "time set midnight", "summon zombie -6.5 -26 0.5", "summon zombie 6.5 -26 -6.5", "summon zombie 6.5 -26 6.5"].map(String::from));
    waits(&mut lines, 120);
    lines.extend(END.map(String::from));
    lines
}

/// How long a summoned mob stands before its first stroll: sixteen cows in a ring 6 blocks about
/// the bots and eight pigs 9 off, 40 s, twice.
fn stroll_start() -> Vec<String> {
    let mut lines = walled("noon", "stone");
    for _ in 0..2 {
        for i in 0..24 {
            let (kind, far, of) = if i < 16 { ("cow", 6.0, 16.0) } else { ("pig", 9.0, 8.0) };
            let turn = std::f32::consts::TAU * i as f32 / of;
            lines.push(format!("summon {kind} {:.1} -26 {:.1}", 0.5 + far * turn.cos(), 0.5 + far * turn.sin()));
        }
        waits(&mut lines, 40);
        lines.push("kill @e[type=!player]".into());
    }
    lines.extend(END.map(String::from));
    lines
}

/// Eight mobs in one-cell pens 3 to 5 blocks from the bots, by night: cows plain, under speed 4,
/// slowness 2 and slowness 255, a wolf (movement 0.3), a villager (0.5), a chicken and a zombie
/// (0.23). The watcher steps from side to side every three seconds, so heads keep turning.
fn head_speed() -> Vec<String> {
    let mut lines = walled("midnight", "stone");
    lines.extend(["fill 3 -26 -5 5 -24 5 stone", "fill -5 -26 -5 -3 -24 5 stone"].map(String::from));
    let pens = [(4, -3, "cow", ""), (4, -1, "cow", "speed 200 4"), (4, 1, "cow", "slowness 200 2"), (4, 3, "cow", "slowness 200 255"), (-4, -3, "wolf", ""), (-4, -1, "villager_v2", ""), (-4, 1, "chicken", ""), (-4, 3, "zombie", "")];
    for (x, z, kind, effect) in pens {
        lines.push(format!("fill {x} -26 {z} {x} -25 {z} air"));
        let (x, z) = (x as f32 + 0.5, z as f32 + 0.5);
        lines.push(format!("summon {kind} {x} -26 {z}"));
        if !effect.is_empty() {
            lines.push(format!("effect @e[type={kind},x={x},y=-26,z={z},r=0.8] {effect} true"));
        }
    }
    for _ in 0..12 {
        lines.extend(["tp CmdWatcher 0.5 -26 -3.5", "", "", "tp CmdWatcher 0.5 -26 4.5", "", ""].map(String::from));
    }
    lines.extend(END.map(String::from));
    lines
}

/// The horse family by night (a zombie horse burns by day) for 180 s: the west half grass, the east
/// stone, a wall between, both bots on the wall. Six horses, a donkey, a mule, a skeleton horse, a
/// zombie horse and a llama on the grass, three horses on the stone.
fn horse_eat() -> Vec<String> {
    let mut lines = walled("midnight", "stone");
    lines.extend(["fill -12 -27 -12 -1 -27 12 grass_block", "fill 0 -26 -12 0 -24 12 stone", "tp @s 0.5 -23 0.5", "tp CmdWatcher 0.5 -23 2.5", ""].map(String::from));
    let grass = ["horse -6.5 -26 -8.5", "horse -6.5 -26 -4.5", "horse -6.5 -26 0.5", "horse -6.5 -26 4.5", "horse -6.5 -26 8.5", "horse -10.5 -26 0.5", "donkey -3.5 -26 -6.5", "mule -3.5 -26 6.5", "skeleton_horse -10.5 -26 -6.5", "zombie_horse -10.5 -26 6.5", "llama -3.5 -26 0.5"];
    for at in grass.iter().chain(&["horse 6.5 -26 -6.5", "horse 6.5 -26 0.5", "horse 6.5 -26 6.5"]) {
        lines.push(format!("summon {at}"));
    }
    waits(&mut lines, 180);
    lines.extend(END.map(String::from));
    lines
}
