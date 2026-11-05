//! Igloos: the scattered feature's piece in snowy biomes (`RandomScatteredLargeFeature::
//! createStructureStart` 14835a7b0, piece vtable 14a927c90, postProcess 14836fd90), a port of
//! Java 1.12's template igloo. Only the start chunk post-processes it, but its templates are
//! placed unclipped, so they reach the chunks around the start through BDS's 3x3 block source.

use super::canvas::Canvas;
use super::spawn_areas;
use super::template::{Placement, Rotation, Template};
use super::BoundingBox;
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::random::Mt19937;

/// Snowy plains, snowy taiga, snowy slopes: the scattered start's biome for an igloo.
pub const IGLOO_BIOMES: &[u32] = &[12, 30, 184];

/// The igloo of the start chunk `start`, written where it reaches `canvas`'s chunk; `random` is
/// the start chunk's structure generator, freshly seeded for the scattered feature.
pub fn place(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32)) {
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let origin = (x0, canvas.floor_height(x0, z0) - 1, z0);
    // BDS files the top's unturned box: at the start's y 64 whole, and as its piece one below the
    // box's average ground (Java 1.12's `offsetToAverageGroundLevel(-1)`), not the template's corner.
    let top = |y| BoundingBox::new(x0, y, z0, x0 + 6, y + 4, z0 + 7);
    let grounds: Vec<i32> = (z0..=z0 + 7).flat_map(|z| (x0..=x0 + 6).map(move |x| (x, z))).map(|(x, z)| canvas.top(x, z, true).max(63)).collect();
    let average = grounds.iter().sum::<i32>() / grounds.len() as i32;
    spawn_areas::file(canvas, spawn_areas::IGLOO, top(64), [top(average - 1)]);
    let rotation = Rotation::from_index(random.next_int(4));
    let placement = Placement::new(rotation, BoundingBox::new(i32::MIN, 0, i32::MIN, i32::MAX, 319, i32::MAX));
    if random.next_float() < 0.5 {
        placement.place(canvas, Template::get("igloo/igloo_top_no_trapdoor"), origin);
        return;
    }
    let chests = placement.place(canvas, Template::get("igloo/igloo_top_trapdoor"), origin);
    chests.draw_seeds(canvas, random);
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
    // "chest", "Villager" and "Zombie Villager" markers all leave air (the chest sits below its marker).
    // Cleared first: the cell bars' update pass finds nothing to join there (BDS's bars beside the
    // two cell markers stay apart from them, though the cellar is dug into stone).
    for (index, _) in &bottom.markers {
        let (dx, dy, dz) = rotation.apply(bottom.blocks[*index].pos);
        canvas.set(at.0 + dx, at.1 + dy, at.2 + dz, Block::Air);
    }
    placement.place(canvas, bottom, at).draw_seeds(canvas, random);
    // The chest marker gives the chest below it its table and a seed (14836fd90).
    for (index, _) in bottom.markers.iter().filter(|(_, m)| m == "chest") {
        let (dx, dy, dz) = rotation.apply(bottom.blocks[*index].pos);
        let (x, y, z) = (at.0 + dx, at.1 + dy - 1, at.2 + dz);
        if canvas.get(x, y, z).name() == "chest" {
            canvas.set_entity(x, y, z, BlockEntity::Loot { table: "loot_tables/chests/igloo_chest.json", seed: random.next_int_unbounded() });
        }
    }
    // 14836f7f0 spawns the villager, then the zombie villager, mid-cell; BDS names v1 kinds, which it shows as their successors.
    for (marker, kind) in [("Villager", "minecraft:villager_v2"), ("Zombie Villager", "minecraft:zombie_villager_v2")] {
        for (index, _) in bottom.markers.iter().filter(|(_, m)| m == marker) {
            let (dx, dy, dz) = rotation.apply(bottom.blocks[*index].pos);
            let feet = [(at.0 + dx) as f32 + 0.5, (at.1 + dy) as f32, (at.2 + dz) as f32 + 0.5];
            canvas.add_actor_at(kind, "minecraft:entity_spawned", feet);
        }
    }
}
