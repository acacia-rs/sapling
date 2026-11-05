//! The spawn areas a chunk's post-process files for the structures BDS spawns mobs in by structure
//! (`addHardcodedSpawnAreas` 1438ced80, and the data-driven ones): the start's whole box, then each
//! piece meeting the chunk, both cut to the chunk's columns. See docs/research/structure-spawns.md.

use super::BoundingBox;
use super::canvas::Canvas;
use crate::chunk::SpawnArea;

/// How far below a piece's top the spawner starts looking down, per hardcoded structure (the start's
/// vt[3]); None for data-driven ones.
pub type Kind = (&'static str, Option<i32>);

pub const FORTRESS: Kind = ("minecraft:fortress", Some(0));
pub const MONUMENT: Kind = ("minecraft:monument", Some(0));
pub const SWAMP_HUT: Kind = ("minecraft:swamp_hut", Some(-3));
pub const OUTPOST: Kind = ("minecraft:pillager_outpost", Some(-3));
pub const DESERT_PYRAMID: Kind = ("minecraft:desert_pyramid", Some(0));
pub const JUNGLE_PYRAMID: Kind = ("minecraft:jungle_pyramid", Some(0));
pub const IGLOO: Kind = ("minecraft:igloo", Some(0));

pub fn file(canvas: &mut Canvas, (structure, y_offset): Kind, whole: BoundingBox, pieces: impl IntoIterator<Item = BoundingBox>) {
    let column = canvas.bb;
    let cut = |b: BoundingBox| {
        let meets = b.x1 >= column.x0 && b.x0 <= column.x1 && b.z1 >= column.z0 && b.z0 <= column.z1;
        meets.then(|| [b.x0.max(column.x0), b.y0, b.z0.max(column.z0), b.x1.min(column.x1), b.y1, b.z1.min(column.z1)])
    };
    let Some(bounds) = cut(whole) else { return };
    let area = |bounds, whole| SpawnArea { structure, bounds, whole, y_offset };
    canvas.add_spawn_area(area(bounds, true));
    for bounds in pieces.into_iter().filter_map(cut) {
        canvas.add_spawn_area(area(bounds, false));
    }
}
