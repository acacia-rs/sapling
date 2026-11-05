//! Group T's `z2hand`: blocks that hold an entity put by hand on the floor, a column at a time from a stand
//! 1.5 west of it, the far cells first so nothing put stands in the bot's sight; empty dispensers put by hand
//! beside blocks of redstone and set beside them; a hanging sign put on a block's side and under it; a
//! campfire set and broken by `setblock ... destroy` three times (its charcoal).

use super::owned;
use super::super::creatures::{floor, waits};

const HAND_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelSoundEvent", "LevelEvent", "AddEntity", "AddItemEntity", "CommandOutput"];

/// The items put down, a column of seven each.
const COLUMNS: [[&str; 7]; 4] = [
    ["dispenser", "dropper", "hopper", "furnace", "smoker", "blast_furnace", "barrel"],
    ["chest", "crafter", "brewing_stand", "lectern", "jukebox", "noteblock", "bell"],
    ["campfire", "skeleton_skull", "cauldron", "enchanting_table", "ender_chest", "decorated_pot", "chiseled_bookshelf"],
    ["flower_pot", "daylight_detector", "comparator", "sculk_sensor", "beacon", "trapped_chest", "beehive"],
];
/// The z of each of a column's seven, in the order they are put.
const ORDER: [i32; 7] = [3, -3, 2, -2, 1, -1, 0];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    (name == "z2hand").then(|| (hand(), HAND_SHOWN))
}

fn hand() -> Vec<String> {
    let mut lines = floor();
    for (column, items) in COLUMNS.iter().enumerate() {
        let x = 2 + 3 * column as i32;
        lines.push(format!("tp @s {}.5 -26 0.5 -90 0", x - 2));
        for z in ORDER {
            let item = items[(z + 3) as usize];
            lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {item} 1"), format!("!place 2 -1 {z}"), String::new()]);
        }
    }
    lines.extend(owned(["tp @s 0.5 -26 0.5 -90 0", "fill 1 -26 -3 12 -24 3 air", "kill @e[type=item]", ""]));
    lines.extend(dispensers());
    lines.extend(owned(["setblock 3 -26 -6 stone", "setblock 3 -24 -6 stone", "replaceitem entity @s slot.weapon.mainhand 0 oak_hanging_sign 2", "!place 3 0 -6 west", "", "!place 3 2 -6 down", ""]));
    lines.extend(owned(["fill 2 -26 -6 3 -24 -6 air", "clear @s", ""]));
    for _ in 0..3 {
        lines.extend(owned(["setblock 4 -26 6 campfire", "setblock 4 -26 6 air destroy", ""]));
    }
    lines.extend(owned(["kill @e[type=item]", ""]));
    lines
}

/// Blocks of redstone at x 2; at z 5 to 7 a dispenser put by hand east of each, at z -5 to -7 one set there.
fn dispensers() -> Vec<String> {
    let mut lines = owned(["fill 2 -26 -7 2 -26 -5 redstone_block", "fill 2 -26 5 2 -26 7 redstone_block", "replaceitem entity @s slot.weapon.mainhand 0 dispenser 4", ""]);
    for z in [5, 6, 7] {
        lines.extend([format!("!place 3 -1 {z}"), String::new()]);
        waits(&mut lines, 1);
    }
    for z in [-5, -6, -7] {
        lines.extend([format!("setblock 3 -26 {z} dispenser [\"facing_direction\"=5]"), String::new()]);
        waits(&mut lines, 1);
    }
    lines.extend(owned(["fill 2 -26 -7 3 -26 7 air", "clear @s", ""]));
    lines
}
