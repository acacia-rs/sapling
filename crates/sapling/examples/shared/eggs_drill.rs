//! Drills of eggs and nests (docs/research/mob-life.md, leftovers.md "Turtles"): turtle eggs hatching,
//! trodden and stomped, frogspawn losing its water, sniffer eggs, turtles laying. On creature_drill.rs's
//! floor (stone at y -27), walled with glass so nothing walks off it.

use super::creatures::{floor, waits, with};

/// Packets shown of these drills.
pub const EGGS_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityData", "EntityEvent", "ActorEvent", "LevelSoundEvent", "LevelEvent", "UpdateBlock", "AddItemEntity"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "eggs" => eggs(),
            "eggtrample" => egg_trample(),
            "nesting" => nesting(),
            "stomping" => stomping(),
            "sniffers" => sniffers(),
            _ => return None,
        },
        EGGS_SHOWN,
    ))
}

/// The floor walled in glass two high, a second layer of stone under it so water stays.
pub fn penned(more: &[&str]) -> Vec<String> {
    let walls = ["fill -8 -28 -8 16 -28 8 stone", "fill -8 -26 -8 16 -25 -8 glass", "fill -8 -26 8 16 -25 8 glass", "fill -8 -26 -8 -8 -25 8 glass", "fill 16 -26 -8 16 -25 8 glass", "gamerule mobgriefing true", "gamerule domobloot true"];
    with(with(floor(), &walls), more)
}

const SAND_EGG: &str = "turtle_egg [\"turtle_egg_count\"=\"four_egg\",\"cracked_state\"=\"max_cracked\"]";

/// Turtle eggs at dawn, held there: four and one cracked twice on sand (hatch at the first tick,
/// 1000 ticks on), two uncracked on sand, one cracked twice on stone, and one on sand set again with
/// only its cracks changed 5 s in. Frogspawn on a water cell whose water goes 3 s in, and one set on stone.
pub fn eggs() -> Vec<String> {
    let mut lines = penned(&[
        "gamerule dodaylightcycle false",
        "time set 21500",
        "setblock 4 -27 4 sand",
        &format!("setblock 4 -26 4 {SAND_EGG}"),
        "setblock 6 -27 4 sand",
        "setblock 6 -26 4 turtle_egg [\"cracked_state\"=\"max_cracked\"]",
        "setblock 8 -27 4 sand",
        "setblock 8 -26 4 turtle_egg [\"turtle_egg_count\"=\"two_egg\"]",
        "setblock 10 -26 4 turtle_egg [\"cracked_state\"=\"max_cracked\"]",
        "setblock 12 -27 4 sand",
        "setblock 12 -26 4 turtle_egg",
        "setblock 4 -27 -4 water",
        "setblock 4 -26 -4 frog_spawn",
        "setblock 8 -26 -4 frog_spawn",
    ]);
    waits(&mut lines, 3);
    lines.push("setblock 4 -27 -4 stone".into());
    waits(&mut lines, 2);
    lines.push("setblock 12 -26 4 turtle_egg [\"cracked_state\"=\"cracked\"]".into());
    waits(&mut lines, 56);
    with(lines, &["gamerule dodaylightcycle true"])
}

/// Single eggs on sand in two glass pens, four pigs in one and four turtles in the other, with
/// `mobGriefing` on and then off; then the actor in survival stands on four eggs, walking and sneaking.
pub fn egg_trample() -> Vec<String> {
    let pen = |z0: i32| {
        [
            format!("fill 3 -26 {} 9 -25 {} glass", z0 - 1, z0 + 5),
            format!("fill 4 -27 {z0} 8 -27 {} sand", z0 + 4),
            format!("fill 4 -26 {z0} 8 -25 {} air", z0 + 4),
            format!("fill 4 -26 {z0} 8 -26 {} turtle_egg", z0 + 4),
        ]
    };
    let mut lines = penned(&[]);
    lines.extend(pen(-7));
    lines.extend(pen(2));
    lines.extend((0..4).map(|n| format!("summon pig {}.5 -26 -5.5", 4 + n)));
    lines.extend((0..4).map(|n| format!("summon turtle {}.5 -26 3.5", 4 + n)));
    waits(&mut lines, 25);
    lines.extend(["gamerule mobgriefing false", "fill 4 -26 -7 8 -26 -3 turtle_egg replace air"].map(String::from));
    waits(&mut lines, 20);
    lines.extend(
        [
            "gamerule mobgriefing true",
            "kill @e[type=pig]",
            "kill @e[type=turtle]",
            "setblock -4 -27 0 sand",
            "setblock -4 -26 0 turtle_egg [\"turtle_egg_count\"=\"four_egg\"]",
            "setblock -4 -27 4 sand",
            "setblock -4 -26 4 turtle_egg [\"turtle_egg_count\"=\"four_egg\"]",
            "gamemode survival @s",
            "effect @s resistance 60 4 true",
            "tp @s -3.5 -25.5 0.5",
        ]
        .map(String::from),
    );
    waits(&mut lines, 12);
    lines.extend(["!sneak", "tp @s -3.5 -25.5 4.5"].map(String::from));
    waits(&mut lines, 12);
    with(lines, &["!unsneak", "gamemode creative @s"])
}

/// Eight turtles on a sand floor fed seagrass in pairs, so four are pregnant: whether and when
/// they go home, dig and lay, and how many eggs where. Three minutes.
pub fn nesting() -> Vec<String> {
    let mut lines = penned(&["time set noon", "fill -7 -27 -7 15 -27 7 sand", "give @s seagrass 64"]);
    for (x, z) in [(2, 0), (2, 1), (-1, 2), (0, 2), (0, -2), (1, -2), (-2, 0), (-2, -1)] {
        lines.push(format!("summon turtle {}.5 -26 {}.5", x, z));
    }
    lines.extend(["", "!feedall turtle"].map(String::from));
    waits(&mut lines, 180);
    lines
}

/// Three zombies at night at one end of a glass run, turtle eggs on sand at the other, the actor
/// in creative (not their prey): the walk to the eggs, the hops, the eggs stomped out.
pub fn stomping() -> Vec<String> {
    let mut lines = penned(&[
        "gamerule dodaylightcycle false",
        "time set midnight",
        "fill -1 -26 -5 13 -24 -1 glass",
        "fill 0 -26 -4 12 -24 -2 air",
        "fill 10 -27 -4 12 -27 -2 sand",
        "setblock 11 -26 -3 turtle_egg [\"turtle_egg_count\"=\"three_egg\"]",
        "setblock 11 -26 -4 turtle_egg",
        "summon zombie 0.5 -26 -3.5",
        "summon zombie 1.5 -26 -2.5",
        "summon zombie 1.5 -26 -4.5",
        "effect @e[type=zombie] fire_resistance 200 0 true",
    ]);
    waits(&mut lines, 70);
    with(lines, &["gamerule dodaylightcycle true", "kill @e[type=zombie]"])
}

/// Sniffer eggs on moss, one cracked twice and one not (due 180 to 220 s in), one cracked twice
/// on stone; four adult sniffers on a grass floor for four minutes: their flags, searches and digs.
pub fn sniffers() -> Vec<String> {
    let mut lines = penned(&[
        "time set noon",
        "fill -7 -27 -7 15 -27 7 grass_block",
        "setblock 0 -27 6 moss_block",
        "setblock 0 -26 6 sniffer_egg [\"cracked_state\"=\"max_cracked\"]",
        "setblock 3 -27 6 moss_block",
        "setblock 3 -26 6 sniffer_egg",
        "setblock 6 -26 6 sniffer_egg [\"cracked_state\"=\"max_cracked\"]",
        "summon sniffer 8.5 -26 -4.5",
        "summon sniffer 12.5 -26 -4.5",
        "summon sniffer 8.5 -26 2.5",
        "summon sniffer 12.5 -26 2.5",
    ]);
    waits(&mut lines, 240);
    lines
}
