//! The fourteenth wave's group I drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). Thirteen supporters (`SUPPORTERS`, kept at z -6 and cloned to z 2): the actor puts
//! torches, levers, buttons, lanterns, carpets and plates on their tops and torches, levers, buttons and
//! ladders on their north faces (`i1place`); the same set by `/fill` and nudged by a neighbour (`i1stay`),
//! then straight walls with a block set over the middle (`i1stay`). Blocks put at four yaws and on floors,
//! sides and ceilings (`i1facing`). The rest (slabs, propagules, a burnt torch, experience, anvils,
//! tripwires) are fourteenth1i_more.rs's.

#[path = "fourteenth1i_more.rs"]
mod more;

use super::creatures::{floor, waits, with};

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "i1place" => (place(), BLOCKS),
        "i1stay" => (stay(), BLOCKS),
        "i1facing" => (facing(), BLOCKS),
        _ => return more::drill(name),
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

const SUPPORTERS: [&str; 13] = [
    "smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]",
    "smooth_stone_slab [\"minecraft:vertical_half\"=\"bottom\"]",
    "oak_leaves [\"persistent_bit\"=true]",
    "glass",
    "oak_fence",
    "cobblestone_wall",
    "cauldron",
    "composter",
    "oak_stairs [\"upside_down_bit\"=true,\"weirdo_direction\"=3]",
    "hopper",
    "iron_bars",
    "trapdoor [\"upside_down_bit\"=true]",
    "chain",
];
pub const FIRST: i32 = -2;
pub const LAST: i32 = FIRST + SUPPORTERS.len() as i32 - 1;

/// The supporters laid at z -6 and copied to z 2, the cells over and north of them emptied.
pub fn supporters() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 14.5 -26 6.5"]);
    lines.extend(SUPPORTERS.iter().zip(FIRST..).map(|(block, x)| format!("setblock {x} -26 -6 {block}")));
    lines
}

pub fn reset() -> Vec<String> {
    [
        format!("clone {FIRST} -26 -6 {LAST} -26 -6 {FIRST} -26 2"),
        format!("fill {FIRST} -25 2 {LAST} -25 2 air"),
        format!("fill {FIRST} -26 1 {LAST} -26 1 air"),
        format!("fill {FIRST} -25 1 {LAST} -25 3 air"),
        "kill @e[type=item]".into(),
    ]
    .into()
}

/// Rounds of an item put on each supporter's top and one on its north face, the actor beside each.
fn place() -> Vec<String> {
    let mut lines = supporters();
    let rounds = [("torch", "torch"), ("lever", "lever"), ("stone_button", "stone_button"), ("lantern", "ladder"), ("white_carpet", ""), ("stone_pressure_plate", "")];
    for (top, side) in rounds {
        lines.extend(reset());
        for x in FIRST..=LAST {
            lines.extend([format!("tp @s {}.5 -26 0.5 180 0", x), hold(top), "!place 0 0 2 up".into()]);
            if !side.is_empty() {
                lines.extend([hold(side), "!place 0 0 2 north".into()]);
            }
        }
        lines.push(String::new());
    }
    lines.extend(reset());
    lines
}

/// Over and on the north face of each supporter by `/fill`, then a neighbour put and taken away.
const ON_TOP: [&str; 8] = [
    "torch [\"torch_facing_direction\"=\"top\"]",
    "lever [\"lever_direction\"=\"up_north_south\"]",
    "stone_button [\"facing_direction\"=1]",
    "lantern [\"hanging\"=false]",
    "white_carpet",
    "stone_pressure_plate",
    "flower_pot",
    "standing_sign",
];
const ON_SIDE: [&str; 4] = ["torch [\"torch_facing_direction\"=\"south\"]", "lever [\"lever_direction\"=\"north\"]", "stone_button [\"facing_direction\"=2]", "ladder [\"facing_direction\"=2]"];
/// What is set over the middle of a straight wall of three.
const OVER_WALLS: [&str; 11] = [
    "torch [\"torch_facing_direction\"=\"top\"]",
    "stone_pressure_plate",
    "standing_sign",
    "lever [\"lever_direction\"=\"up_north_south\"]",
    "stone_button [\"facing_direction\"=1]",
    "white_carpet",
    "flower_pot",
    "lantern [\"hanging\"=false]",
    "stone",
    "oak_fence",
    "redstone_wire",
];

fn stay() -> Vec<String> {
    let mut lines = with(supporters(), &["tp @s 3.5 -26 -3.5 0 0"]);
    for block in ON_TOP {
        lines.extend(reset());
        lines.extend([format!("fill {FIRST} -25 2 {LAST} -25 2 {block}"), String::new(), format!("fill {FIRST} -25 3 {LAST} -25 3 stone"), format!("fill {FIRST} -25 3 {LAST} -25 3 air"), String::new()]);
    }
    for block in ON_SIDE {
        lines.extend(reset());
        lines.extend([format!("fill {FIRST} -26 1 {LAST} -26 1 {block}"), String::new(), format!("fill {FIRST} -25 1 {LAST} -25 1 stone"), format!("fill {FIRST} -25 1 {LAST} -25 1 air"), String::new()]);
    }
    lines.extend(reset());
    for (i, block) in OVER_WALLS.iter().enumerate() {
        let (x, z) = (-6 + 4 * (i as i32 % 6), 5 + 2 * (i as i32 / 6));
        lines.extend([format!("fill {} -26 {z} {} -26 {z} cobblestone_wall", x - 1, x + 1), String::new(), format!("setblock {x} -25 {z} {block}"), String::new()]);
    }
    with(lines, &["", "fill -8 -26 4 16 -25 8 air", "kill @e[type=item]", ""])
}

/// Blocks put on grass 2 east, west, south and north of the actor (each look at its own yaw).
const TURNED: [&str; 26] = [
    "chest", "ender_chest", "furnace", "lectern", "stonecutter_block", "end_portal_frame", "carved_pumpkin", "lit_pumpkin", "repeater", "comparator", "big_dripleaf",
    "pink_petals", "leaf_litter", "wildflowers", "oak_shelf", "vault", "dried_ghast", "copper_golem_statue", "skeleton_skull", "white_banner", "smoker", "trial_spawner",
    "creaking_heart", "scaffolding", "oak_sign", "bell",
];
/// Blocks put on a stone's top, its west face and under a ceiling stone two over it.
const HUNG: [&str; 9] = ["bell", "grindstone", "lantern", "soul_lantern", "pointed_dripstone", "skeleton_skull", "white_banner", "tube_coral_fan", "mangrove_propagule"];

fn facing() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 12.5 -26 6.5", "fill -3 -27 -3 3 -27 3 grass_block", "tp @s 0.5 -26 0.5 -90 0"]);
    for block in TURNED {
        lines.push(hold(block));
        lines.extend(["2 -1 0", "-2 -1 0", "0 -1 2", "0 -1 -2"].map(|at| format!("!place {at} up")));
        lines.extend(["fill -3 -26 -3 3 -24 3 air".into(), "kill @e[type=item]".into()]);
    }
    lines.extend(["fill -3 -27 -3 3 -27 3 stone".into(), "setblock 2 -26 0 stone".into()]);
    for block in HUNG {
        let ceiling = if block == "mangrove_propagule" { "mangrove_leaves [\"persistent_bit\"=true]" } else { "stone" };
        lines.extend([format!("setblock 2 -24 0 {ceiling}"), hold(block),"!place 2 0 0 up".into(), "setblock 2 -25 0 air".into(), "!place 2 0 0 west".into(), "setblock 1 -26 0 air".into(), "!place 2 2 0 down".into(), "setblock 2 -25 0 air".into(), "kill @e[type=item]".into()]);
    }
    with(lines, &["fill -3 -26 -3 3 -24 3 air", "kill @e[type=item]", ""])
}
