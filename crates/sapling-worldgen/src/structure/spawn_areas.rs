//! The spawn areas a chunk's post-process files for the structures BDS spawns mobs in by structure
//! (`addHardcodedSpawnAreas` 1438ced80): the start's whole box, then each piece meeting the chunk,
//! both cut to the chunk's columns. See docs/research/structure-spawns.md.

use super::BoundingBox;
use super::canvas::Canvas;
use crate::chunk::SpawnArea;

/// How far below a piece's top the spawner starts looking down, per structure (the start's vt[3]).
pub const FORTRESS: (&str, i32) = ("minecraft:fortress", 0);
pub const MONUMENT: (&str, i32) = ("minecraft:monument", 0);
pub const SWAMP_HUT: (&str, i32) = ("minecraft:swamp_hut", -3);
pub const OUTPOST: (&str, i32) = ("minecraft:pillager_outpost", -3);

pub fn file(canvas: &mut Canvas, (structure, y_offset): (&'static str, i32), whole: BoundingBox, pieces: impl IntoIterator<Item = BoundingBox>) {
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
