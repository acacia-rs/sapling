//! The End's drills for the portal probe (lines as shared/portal_drill.rs reads them, plus
//! `!box x0 y0 z0 x1 y1 z1`, the blocks in a box as the actor's client holds them, and `!top x z`,
//! the highest blocks of a column), and the column a drill prints whole.

use acacia_bot::Bot;
use acacia_world::{BlockAccess, BlockRegistry};

/// The spikes' centres, whatever the seed (worldgen-bds-end.md).
pub const SPIKES: [(i32, i32); 10] = [(42, 0), (33, 24), (12, 39), (-12, 39), (-33, 24), (-42, 0), (-33, -24), (-12, -39), (12, -39), (33, -24)];
/// End columns whose LevelChunk payload is printed: the platform's, the main island's and one of the void.
pub const SHOWN_COLUMNS: [(i32, i32); 3] = [(6, 0), (0, 0), (20, 0)];
/// The worldgen gateway the gateway drill enters (seed 20261018, tools: sapling-worldgen example end_gateways).
const GATEWAY: [i32; 3] = [1953, 66, 181];

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// Twelve frames round the 3x3 whose low corner is `(x, y, z)`, every eye in but the north side's
/// middle one; `inward` faces them in.
fn ring(x: i32, y: i32, z: i32, inward: bool) -> Vec<String> {
    let mut lines = vec![format!("fill {} {} {} {} {} {} air", x - 1, y, z - 1, x + 3, y + 2, z + 3)];
    let sides: [(i32, i32, i32, i32, [&str; 2]); 4] = [(0, -1, 1, 0, ["south", "north"]), (0, 3, 1, 0, ["north", "south"]), (-1, 0, 0, 1, ["east", "west"]), (3, 0, 0, 1, ["west", "east"])];
    for (ox, oz, dx, dz, facings) in sides {
        let facing = facings[usize::from(!inward)];
        for i in 0..3 {
            let (cx, cz) = (x + ox + dx * i, z + oz + dz * i);
            let eye = !(oz == -1 && i == 1);
            lines.push(format!("setblock {cx} {y} {cz} end_portal_frame [\"minecraft:cardinal_direction\"=\"{facing}\",\"end_portal_eye_bit\"={eye}]"));
        }
    }
    lines.push("replaceitem entity @s slot.weapon.mainhand 0 ender_eye 4".into());
    lines.push(format!("tp @s {}.5 {y} {}.5 #1500", x + 1, z - 3));
    lines.push(format!("!light {} {y} {} up 3000", x + 1, z - 1));
    lines.push(format!("!box {} {y} {} {} {y} {}", x - 1, z - 1, x + 3, z + 3));
    lines
}

fn platform_and_spikes() -> Vec<String> {
    let mut lines = vec!["!where".to_string(), "!box 97 47 -3 103 52 3".to_string()];
    lines.extend(SPIKES.map(|(x, z)| format!("!top {x} {z}")));
    lines
}

/// The lines of the End drill `name`, after the common start.
pub fn drill(name: &str) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    match name {
        // By command there and back, a death in the End.
        "end-travel" => {
            lines.extend(owned(&["execute in the_end run tp @s 100.5 49 0.5 #12000"]));
            lines.extend(platform_and_spikes());
            lines.extend(owned(&["execute in the_end run tp @s 0.5 90 0.5 #6000", "!where", "execute in overworld run tp @s 0.5 -60 0.5 #9000", "!where"]));
        }
        "end-death" => lines.extend(owned(&["execute in the_end run tp @s 100.5 49 0.5 #10000", "!where", "gamemode survival", "kill @s #12000", "!where", "gamemode creative"])),
        // Eyes into frames: one alone in survival, then a ring facing in and one facing out; through the
        // portal, and again after the platform was built on.
        "end-frame" => {
            lines.extend(owned(&["gamemode survival", "setblock 20 -60 5 end_portal_frame [\"minecraft:cardinal_direction\"=\"south\"]", "replaceitem entity @s slot.weapon.mainhand 0 ender_eye 2", "tp @s 20.5 -60 3.5 #1500"]));
            lines.extend(owned(&["!light 20 -60 5 up 2000", "!light 20 -60 5 up 2000", "!box 20 -60 5 20 -60 5", "gamemode creative"]));
            lines.extend(ring(30, -60, 10, false));
            lines.extend(ring(10, -60, 10, true));
            lines.extend(owned(&["tp @s 11.5 -60 11.5 #15000"]));
            lines.extend(platform_and_spikes());
            lines.extend(owned(&["setblock 100 49 0 stone", "setblock 101 50 1 stone", "setblock 99 48 -1 air", "setblock 100 46 0 stone", "execute in overworld run tp @s 11.5 -60 7.5 #9000", "tp @s 11.5 -60 11.5 #15000"]));
            lines.extend(platform_and_spikes());
        }
        "end-gateway" => {
            let [x, y, z] = GATEWAY;
            lines.push(format!("execute in the_end run tp @s {}.5 {} {}.5 #12000", x + 3, y, z));
            lines.push(format!("!box {} {} {} {} {} {}", x - 1, y - 3, z - 1, x + 1, y + 3, z + 1));
            lines.extend(owned(&["!where", &format!("tp @s {x}.5 {y} {z}.5 #8000"), "!where", "!wait 6000", "!where"]));
        }
        _ => return None,
    }
    Some(lines)
}

fn block_name(registry: &BlockRegistry, id: u32) -> String {
    registry.get(id).map_or("?".into(), |state| format!("{}[{}]", state.name.trim_start_matches("minecraft:"), state.properties))
}

/// Prints the blocks in a box that are not air, by layer from the bottom.
pub fn print_box(actor: &Bot, [x0, y0, z0, x1, y1, z1]: [i32; 6]) {
    let world = actor.world();
    let (Some(view), Some(registry)) = (world.and_then(|w| w.view()), world.and_then(|w| w.registry())) else { return println!("## box: no view") };
    for y in y0..=y1 {
        let mut cells = Vec::new();
        for z in z0..=z1 {
            for x in x0..=x1 {
                let id = view.block(x, y, z);
                if id != registry.air_id() {
                    cells.push(format!("{x} {z} {}", block_name(registry, id)));
                }
            }
        }
        println!("## box y {y}: {}", if cells.is_empty() { "all air".into() } else { cells.join("; ") });
    }
}

/// Prints the three highest blocks of a column that are not air.
pub fn print_top(actor: &Bot, x: i32, z: i32) {
    let world = actor.world();
    let (Some(view), Some(registry)) = (world.and_then(|w| w.view()), world.and_then(|w| w.registry())) else { return println!("## top: no view") };
    let found: Vec<String> = (0..256).rev().filter_map(|y| Some((y, view.block(x, y, z))).filter(|(_, id)| *id != registry.air_id())).take(3).map(|(y, id)| format!("{y} {}", block_name(registry, id))).collect();
    println!("## top {x} {z}: {}", found.join("; "));
}
