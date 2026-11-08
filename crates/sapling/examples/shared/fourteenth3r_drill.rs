//! The fourteenth wave's group R drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's
//! floor (stone at y -27). `r3cart`: a minecart run down a powered-rail runway off the floor's east
//! edge, falling free (off a rail: the 0.4 cap and the air's drag). `r3copper`: a copper chest put by hand
//! beside an exposed one (do stages pair?), then opened by a click without a sneak. `r3light`: plants
//! that ask for light put by hand in the open and in a shut room. `r3myc`: mycelium in the dark under
//! blocks of several kinds.

use super::creatures::{floor, waits, with};

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "ContainerOpen", "LevelEvent", "LevelSoundEvent", "CommandOutput"];
const MOVES: &[&str] = &["AddEntity", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "RemoveEntity", "SetEntityData", "LevelEvent", "LevelSoundEvent", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "r3cart" => (cart(), MOVES),
        "r3copper" => (copper(), BLOCKS),
        "r3light" => (light(), BLOCKS),
        "r3myc" => (mycelium(), BLOCKS),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

fn tidy(lines: Vec<String>) -> Vec<String> {
    with(lines, &["", "fill -8 -26 -8 16 -16 8 air", "kill @e[type=!player]", "tp @s 0.5 -26 0.5 -90 0", ""])
}

/// Seventeen powered rails east along z 0 on redstone blocks, a stone at their west end to kick a
/// standing cart off; the cart leaves the rails' east end into the open over the floor's edge.
fn cart() -> Vec<String> {
    let mut lines = with(floor(), &[
        "tp @s -3.5 -26 4.5 -90 0",
        "fill 0 -27 0 16 -27 0 redstone_block",
        "fill 0 -26 0 16 -26 0 golden_rail [\"rail_direction\"=1,\"rail_data_bit\"=true]",
        "setblock -1 -26 0 stone",
        "",
        "summon minecart 0.5 -26 0.5",
    ]);
    waits(&mut lines, 6);
    with(tidy(lines), &["kill @e[type=minecart]", "fill 0 -27 0 16 -27 0 stone", "fill -1 -26 0 -1 -26 0 air", ""])
}

/// A copper chest put west-facing, an exposed one put beside it; then the copper one clicked with an
/// empty hand, not sneaking.
fn copper() -> Vec<String> {
    let lines = with(floor(), &[
        "gamemode survival @s",
        &hold("copper_chest 1"),
        "!place 2 -1 0 up",
        &hold("exposed_copper_chest 1"),
        "!place 2 -1 1 up",
        "",
        "replaceitem entity @s slot.weapon.mainhand 0 air",
        "!click 2 0 0 up",
        "",
        "",
        "gamemode creative @s",
    ]);
    tidy(lines)
}

/// Plants that ask for light put by hand at noon in the open, then in a shut stone room: a mushroom
/// and a sapling on stone and dirt, wheat seeds on farmland.
fn light() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "setblock 2 -27 1 dirt", "setblock 2 -27 -1 farmland", ""]);
    let plant = |lines: &mut Vec<String>| {
        for (item, at) in [("brown_mushroom", "2 -1 0"), ("brown_mushroom", "2 -1 1"), ("oak_sapling", "2 -1 1"), ("wheat_seeds", "2 -1 -1")] {
            lines.extend([hold(&format!("{item} 1")), format!("!place {at} up")]);
        }
        lines.extend(["", "fill 2 -26 -1 2 -26 1 air", ""].map(String::from));
    };
    plant(&mut lines);
    lines.extend(["fill -2 -27 -3 4 -23 3 stone hollow", "setblock 2 -27 1 dirt", "setblock 2 -27 -1 farmland", ""].map(String::from));
    plant(&mut lines);
    with(tidy(lines), &["fill -2 -27 -3 4 -27 3 stone", "time set 1000"])
}

/// What over mycelium kills it in the dark: a shut stone box, mycelium along its floor under a block of
/// each kind (the first under air; water between solid blocks), random ticks fast.
const OVER_MYCELIUM: &[&str] = &[
    "air", "stone", "glass", "oak_leaves", "oak_slab", "brown_mushroom_block", "mushroom_stem", "oak_planks", "oak_fence", "sand", "water", "dirt",
];

fn mycelium() -> Vec<String> {
    let last = 1 + OVER_MYCELIUM.len() as i32;
    let mut lines = with(floor(), &["tp @s 0.5 -26 4.5 -90 0"]);
    lines.extend([format!("fill 1 -28 -1 {} -24 1 stone hollow", last + 1), format!("fill 2 -28 0 {last} -28 0 mycelium")]);
    for (x, block) in (2..).zip(OVER_MYCELIUM) {
        lines.push(format!("setblock {x} -27 0 {block}"));
    }
    lines.extend(["", "gamerule randomtickspeed 4096"].map(String::from));
    waits(&mut lines, 6);
    lines.push("gamerule randomtickspeed 1".into());
    for x in 2..=last {
        lines.push(format!("testforblock {x} -28 0 mycelium"));
    }
    with(tidy(lines), &[&format!("fill 1 -28 -1 {} -24 1 air", last + 1), &format!("fill 1 -27 -1 {} -27 1 stone", last + 1)])
}
