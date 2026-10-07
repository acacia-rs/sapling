//! A placed ruined portal's ageing pass (14835d070 after placement; Java's block age, lava and
//! gold processors done as one sweep over the box, x then y then z): stone bricks crack, moss
//! over or turn to stairs, slabs and walls moss over, obsidian cries, lava hardens, gold goes.

use super::BoundingBox;
use super::canvas::Canvas;
use super::template::named;
use crate::block::Block;
use crate::random::Mt19937;

/// Blocks aged like full stone bricks (lists 14ccd25f0 and 14ccda7d0, plus stone and oak slabs).
const FULL: &[&str] = &[
    "stone_bricks",
    "chiseled_stone_bricks",
    "cracked_stone_bricks",
    "mossy_stone_bricks",
    "smooth_stone_double_slab",
    "sandstone_double_slab",
    "petrified_oak_double_slab",
    "cobblestone_double_slab",
    "brick_double_slab",
    "stone_brick_double_slab",
    "quartz_double_slab",
    "nether_brick_double_slab",
    "oak_double_slab",
    "stone",
];
/// A wall's default state, as BDS sets the mossy one.
const WALL_ALONE: &str =
    "wall_connection_type_east=none,wall_connection_type_north=none,wall_connection_type_south=none,wall_connection_type_west=none,wall_post_bit=0";
const SLABS: &[&str] =
    &["smooth_stone_slab", "sandstone_slab", "petrified_oak_slab", "cobblestone_slab", "brick_slab", "stone_brick_slab", "quartz_slab", "nether_brick_slab", "oak_slab"];

fn state(block: Block, key: &str) -> &'static str {
    block.states().split(',').filter_map(|p| p.split_once('=')).find(|(k, _)| *k == key).map_or("0", |(_, v)| v)
}

fn stairs(name: &str, upside_down: &str, weirdo: &str) -> Block {
    named(name, &format!("minecraft:corner=none,upside_down_bit={upside_down},weirdo_direction={weirdo}"), false)
}

/// Stairs facing a random way, upside down on a coin flip (14835feb0).
fn random_stairs(random: &mut Mt19937, name: &str) -> Block {
    let weirdo = random.next_int(4);
    let upside_down = random.next_float() < 0.5;
    stairs(name, if upside_down { "1" } else { "0" }, &weirdo.to_string())
}

pub fn age(canvas: &mut Canvas, random: &mut Mt19937, b: &BoundingBox, mossiness: f32, cold: bool, sea_floor: bool) {
    let plain = |name: &str| named(name, "", false);
    let slab = |name: &str| named(name, "minecraft:vertical_half=bottom", false);
    for x in b.x0..=b.x1 {
        for y in b.y0..=b.y1 {
            for z in b.z0..=b.z1 {
                let block = canvas.get(x, y, z);
                let name = block.name();
                let aged = if FULL.contains(&name) {
                    if random.next_float() < 0.5 {
                        continue;
                    }
                    let normal = [plain("cracked_stone_bricks"), random_stairs(random, "stone_brick_stairs")];
                    let mossy = [plain("mossy_stone_bricks"), random_stairs(random, "mossy_stone_brick_stairs")];
                    let f = random.next_float();
                    let i = random.next_int(2) as usize;
                    if mossiness <= f { normal[i] } else { mossy[i] }
                } else if name == "stone_brick_stairs" {
                    let (weirdo, upside_down) = (state(block, "weirdo_direction"), state(block, "upside_down_bit"));
                    if random.next_float() < 0.5 {
                        continue;
                    }
                    let normal = [slab("normal_stone_slab"), slab("stone_brick_slab")];
                    let mossy = [stairs("mossy_stone_brick_stairs", upside_down, weirdo), slab("mossy_stone_brick_slab")];
                    let f = random.next_float();
                    let i = random.next_int(2) as usize;
                    if mossiness <= f { normal[i] } else { mossy[i] }
                } else if SLABS.contains(&name) {
                    if random.next_float() >= mossiness {
                        continue;
                    }
                    slab("mossy_stone_brick_slab")
                } else if name.ends_with("_wall") && block.states().contains("wall_post_bit") {
                    if random.next_float() >= mossiness {
                        continue;
                    }
                    named("mossy_stone_brick_wall", WALL_ALONE, false)
                } else if name == "netherrack" {
                    if cold || random.next_float() >= 0.1 {
                        continue;
                    }
                    plain("magma")
                } else if name == "obsidian" {
                    if random.next_float() >= 0.2 {
                        continue;
                    }
                    plain("crying_obsidian")
                } else if name == "lava" || name == "flowing_lava" {
                    match (sea_floor, cold) {
                        (false, false) if random.next_float() < 0.2 => plain("magma"),
                        (false, false) => continue,
                        (false, true) => plain("netherrack"),
                        (true, _) => plain("magma"),
                    }
                } else if name == "gold_block" {
                    if random.next_float() >= 0.3 {
                        continue;
                    }
                    let around = [(-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1), (0, 1, 0)];
                    let lava = around.iter().any(|(dx, dy, dz)| matches!(canvas.get(x + dx, y + dy, z + dz).name(), "lava" | "flowing_lava"));
                    if lava { Block::Lava } else { Block::Air }
                } else {
                    continue;
                };
                canvas.set(x, y, z, aged);
            }
        }
    }
}
