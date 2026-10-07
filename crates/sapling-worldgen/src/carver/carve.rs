//! Carving one segment into one chunk: the bounds cave `place` (148281d10) computes, and the
//! ellipsoid loop of `carveBlocks` (148283e70) with the 1.18 config 14a925940.

use crate::density::{AquiferCache, Density};
use crate::block::Block;
use crate::terrain::{ChunkShape, MIN_Y};

use super::cave::Segment;

/// Local block bounds of a segment inside the target chunk; y in BDS's 0-based local y.
struct Bounds {
    x: (i32, i32),
    y: (i32, i32),
    z: (i32, i32),
}

/// Whether a segment can reach the chunk at all: `place`'s two tests, in its float order.
pub fn reaches(segment: &Segment, chunk_x: i32, chunk_z: i32) -> bool {
    let (mid_x, mid_z) = ((chunk_x * 16 + 8) as f32, (chunk_z * 16 + 8) as f32);
    let (dx, dz) = (segment.center[0] - mid_x, segment.center[2] - mid_z);
    let remaining = segment.length - segment.step;
    let distance = (dz * dz + dx * dx) - (remaining * remaining) as f32;
    let reach = segment.thickness + 2.0 + 16.0;
    if reach * reach < distance {
        return false;
    }
    let margin = segment.horizontal_radius + segment.horizontal_radius + 16.0;
    let fx = segment.center[0].floor() as i32 as f32;
    let fz = segment.center[2].floor() as i32 as f32;
    !(mid_x - margin > fx || fx > mid_x + margin || mid_z - margin > fz || fz > mid_z + margin)
}

fn bounds(segment: &Segment, chunk_x: i32, chunk_z: i32, height_cap: i32) -> Bounds {
    let [x, y, z] = segment.center;
    let (hr, vr) = (segment.horizontal_radius, segment.vertical_radius);
    let horizontal = |center: f32, base: i32| {
        let low = ((center - hr).floor() as i32 - base - 1).max(0);
        let high = ((center + hr).floor() as i32 - base).min(15) + 1;
        (low, high)
    };
    let top = (y + vr).floor() as i32;
    let high = if height_cap <= top { height_cap } else { top.max(0) + 1 };
    let bottom = (y - vr).floor() as i32 - 1;
    let low = if bottom > high { high } else { bottom.max(1) };
    Bounds { x: horizontal(x, chunk_x * 16), y: (low, high), z: horizontal(z, chunk_z * 16) }
}

/// Solid cells inside the ellipsoid become air above the aquifer's fluid level and its fluid
/// below; aquifer barriers stay solid.
// TODO: BDS also turns sand above a carved cell into sandstone and regrows grass on the dirt below a carved grass block; neither changes the shape.
pub fn carve(shape: &mut ChunkShape, density: &Density, aquifer: &mut AquiferCache, segment: &Segment, chunk: (i32, i32), height_cap: i32) {
    let (chunk_x, chunk_z) = chunk;
    let bounds = bounds(segment, chunk_x, chunk_z, height_cap);
    let (hr, vr) = (segment.horizontal_radius, segment.vertical_radius);
    for lx in bounds.x.0..bounds.x.1 {
        let wx = chunk_x * 16 + lx;
        let dx = ((wx as f32 + 0.5) - segment.center[0]) / hr;
        let dx2 = dx * dx;
        for lz in bounds.z.0..bounds.z.1 {
            let wz = chunk_z * 16 + lz;
            let dz = ((wz as f32 + 0.5) - segment.center[2]) / hr;
            let dz2 = dz * dz;
            if !(1.0 > dx2 + dz2) {
                continue;
            }
            for local_y in (bounds.y.0 + 1..=bounds.y.1).rev() {
                let dy = (((local_y - 1) as f32 + 0.5) - segment.center[1]) / vr;
                if !(dy > segment.floor_level) || !(dy * dy + dx2 + dz2 < 1.0) {
                    continue;
                }
                carve_cell(shape, density, aquifer, [wx, local_y + MIN_Y, wz]);
            }
        }
    }
}

/// Empties one solid cell the way `Aquifer::computeSubstance` at density 0 fills it: nothing
/// behind a barrier, else air at or above the fluid level and the fluid below it.
pub fn carve_cell(shape: &mut ChunkShape, density: &Density, aquifer: &mut AquiferCache, [wx, y, wz]: [i32; 3]) {
    let (x, z) = (wx.rem_euclid(16) as usize, wz.rem_euclid(16) as usize);
    // BDS's block test (148281400) accepts every terrain block, so solid is enough.
    if !shape.block(x, y, z).is_solid() {
        return;
    }
    let answer = density.aquifer(aquifer, [wx, y, wz]);
    if answer.barrier > 0.0 {
        return;
    }
    let block = match answer.fluid {
        fluid if y >= fluid.level => Block::Air,
        fluid if fluid.lava => Block::Lava,
        _ => Block::Water,
    };
    shape.set_block(x, y, z, block);
}
