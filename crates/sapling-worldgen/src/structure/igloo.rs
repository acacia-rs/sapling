//! Igloos: the scattered feature's piece in snowy biomes (`RandomScatteredLargeFeature::
//! createStructureStart` 14835a7b0, piece vtable 14a927c90, postProcess 14836fd90), a port of
//! Java 1.12's template igloo. Only the start chunk post-processes it, but its templates are
//! placed unclipped, so they reach the chunks around the start through BDS's 3x3 block source.

use super::canvas::Canvas;
use super::template::{Placement, Rotation, Template};
use super::BoundingBox;
use crate::block::Block;
use crate::random::Mt19937;

/// Snowy plains, snowy taiga, snowy slopes: the scattered start's biome for an igloo.
pub const IGLOO_BIOMES: &[u32] = &[12, 30, 184];

/// The igloo of the start chunk `start`, written where it reaches `canvas`'s chunk; `random` is
/// the start chunk's structure generator, freshly seeded for the scattered feature.
pub fn place(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32)) {
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let origin = (x0, canvas.floor_height(x0, z0) - 1, z0);
    let rotation = Rotation::from_index(random.next_int(4));
    let placement = Placement::new(rotation, BoundingBox::new(i32::MIN, 0, i32::MIN, i32::MAX, 319, i32::MAX));
    if random.next_float() < 0.5 {
        placement.place(canvas, Template::get("igloo/igloo_top_no_trapdoor"), origin);
        return;
    }
    placement.place(canvas, Template::get("igloo/igloo_top_trapdoor"), origin);
    let connected = |a: (i32, i32, i32), b: (i32, i32, i32)| {
        let (a, b) = (rotation.apply(a), rotation.apply(b));
        (origin.0 + a.0 - b.0, origin.1 + a.1 - b.1, origin.2 + a.2 - b.2)
    };
    let depth = random.next_int(8) + 4;
    let middle = Template::get("igloo/igloo_middle");
    for j in 0..depth {
        placement.place(canvas, middle, connected((3, -1 - j * 3, 5), (1, 2, 1)));
    }
    let bottom = Template::get("igloo/igloo_bottom");
    let at = connected((3, -1 - depth * 3, 5), (3, 5, 7));
    placement.place(canvas, bottom, at);
    // "chest", "Villager" and "Zombie Villager" markers all leave air (the chest sits below its marker).
    for (index, _) in &bottom.markers {
        let (dx, dy, dz) = rotation.apply(bottom.blocks[*index].pos);
        canvas.set(at.0 + dx, at.1 + dy, at.2 + dz, Block::Air);
    }
}
