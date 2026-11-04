//! Drills of the fourth follow-up wave's group 3 (docs/research mob-damage.md, structure-block-entities.md):
//! which mobs a player is shown and ticked at each `tick-distance` (the server's, set before the run), and
//! what a potted sapling keeps.

use super::creatures::{floor, waits, with};

const CIRCLE_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "UpdateAttributes", "MoveEntityDelta", "MoveEntityAbsolute"];
const POTS_SHOWN: &[&str] = &["UpdateBlock", "BlockEntityData", "InventorySlot"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "tickcircle" => (tick_circle(), CIRCLE_SHOWN),
        "potsapling" => (pot_sapling(), POTS_SHOWN),
        _ => return None,
    })
}

/// Each kind's chunk offset from the bots' chunk (3, 0), past the circles of `tick-distance` 4 to 10
/// (dx² + dz²: 16, 18, 25, 34, 36, 41, 49, 61, 64, 65, 72, 81, 100, 104).
const FAR: [(&str, i32, i32); 14] = [
    ("cow", 4, 0),
    ("pig", 3, 3),
    ("sheep", 5, 0),
    ("mooshroom", 5, 3),
    ("goat", 6, 0),
    ("donkey", 5, 4),
    ("mule", 7, 0),
    ("horse", 6, 5),
    ("chicken", 8, 0),
    ("camel", 7, 4),
    ("turtle", 6, 6),
    ("sniffer", 9, 0),
    ("panda", 10, 0),
    ("armadillo", 10, 2),
];

/// Mobs held by slowness are moved from beside the bots, each onto a stone pad (sapling's flat ground
/// ends at z ±48) at its offset in `FAR`; there they are poisoned, which only a ticked mob feels, and
/// moved back after 6 s: the health a hidden mob comes back with says whether it was ticked.
fn tick_circle() -> Vec<String> {
    let at = |dx: i32, dz: i32| ((3 + dx) * 16 + 8, dz * 16 + 8);
    let mut lines = with(floor(), &["tp @s 52.5 -60 0.5", "tp CmdWatcher 52.5 -60 2.5", ""]);
    for (kind, dx, dz) in FAR {
        let (x, z) = at(dx, dz);
        lines.extend([
            format!("fill {} -61 {} {} -61 {} stone", x - 2, z - 2, x + 2, z + 2),
            format!("summon {kind} 56.5 -60 0.5"),
            format!("effect @e[type={kind}] slowness 120 255 true"),
        ]);
    }
    for (kind, dx, dz) in FAR {
        let (x, z) = at(dx, dz);
        lines.push(format!("tp @e[type={kind}] {x}.5 -60 {z}.5"));
    }
    waits(&mut lines, 2);
    lines.push("effect @e[type=!player] poison 30 0 true".to_owned());
    waits(&mut lines, 6);
    for (kind, ..) in FAR {
        lines.push(format!("tp @e[type={kind}] 56.5 -60 0.5"));
    }
    waits(&mut lines, 2);
    lines.extend(with(Vec::new(), &["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""]));
    lines
}

/// The actor clicks a pot a block east of it with each plant in hand; the pots stay for the world's
/// `PlantBlock`s to be read.
fn pot_sapling() -> Vec<String> {
    let plants = ["oak_sapling", "cherry_sapling", "mangrove_propagule", "bamboo", "cactus", "poppy"];
    let mut lines = floor();
    for (z, plant) in (-6..).step_by(2).zip(plants) {
        lines.extend([
            format!("tp @s 0.5 -26 {} -90 0", z as f32 + 0.5),
            format!("setblock 1 -26 {z} flower_pot"),
            format!("replaceitem entity @s slot.weapon.mainhand 0 {plant}"),
            "!click 1 0 0".to_owned(),
            String::new(),
        ]);
    }
    lines.extend(with(Vec::new(), &["clear @s", "tp @s 0.5 -26 0.5", ""]));
    lines
}
