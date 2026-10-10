//! The reads drill of bds_command_probe: what a comparator reads of blocks that are not chests
//! (docs/research/redstone.md, "Comparators").

const COMPARATOR: &str = "unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=false]";

/// A comparator and dust east of the cell ~5 ~ ~`z`, on stone.
fn rig(lines: &mut Vec<String>, z: i32) {
    lines.extend([format!("setblock ~6 ~ ~{z} {COMPARATOR}"), format!("setblock ~7 ~ ~{z} redstone_wire")]);
}

/// Blocks put behind a comparator in each of their states, then containers of other sizes
/// holding one full stack, half a stack, and a thing that does not stack.
pub fn reads() -> Vec<String> {
    let mut lines = vec!["fill ~2 ~-1 ~4 ~22 ~-1 ~30 stone".to_owned(), "fill ~2 ~ ~4 ~22 ~4 ~30 air".to_owned()];
    let states: [(&str, &str, std::ops::RangeInclusive<i32>, &str); 5] = [
        ("cake", "\"bite_counter\"", 0..=6, ""),
        ("cauldron", "\"fill_level\"", 0..=6, ",\"cauldron_liquid\"=\"water\""),
        ("composter", "\"composter_fill_level\"", 0..=8, ""),
        ("respawn_anchor", "\"respawn_anchor_charge\"", 0..=4, ""),
        ("beehive", "\"honey_level\"", 0..=5, ""),
    ];
    let mut z = 5;
    for (block, state, values, rest) in states {
        rig(&mut lines, z);
        lines.extend(values.map(|value| format!("setblock ~5 ~ ~{z} {block} [{state}={value}{rest}]")));
        z += 2;
    }
    rig(&mut lines, z);
    lines.extend(["false", "true"].map(|eye| format!("setblock ~5 ~ ~{z} end_portal_frame [\"end_portal_eye_bit\"={eye}]")));
    for container in ["hopper", "furnace", "dispenser", "barrel", "brewing_stand", "undyed_shulker_box", "trapped_chest"] {
        z += 2;
        rig(&mut lines, z);
        lines.push(format!("setblock ~5 ~ ~{z} {container}"));
        lines.extend(["stone 64", "stone 32", "diamond_sword 1", "air"].map(|item| format!("replaceitem block ~5 ~ ~{z} slot.container 0 {item}")));
    }
    lines
}
