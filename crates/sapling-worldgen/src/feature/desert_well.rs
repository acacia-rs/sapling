//! `minecraft:desert_well_feature` (`DesertWellFeature::place` 148295a70, vtable 14a928ad0): Java's well
//! without its step up, then two suspicious sands under the water (docs/research/worldgen-bds-features.md,
//! "Desert well").

use super::place::Ctx;
use super::trace;
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::terrain::MIN_Y;

const TABLE: &str = "loot_tables/entities/desert_well_brushable_block.json";

pub fn place(ctx: &mut Ctx, [x, mut y, z]: [i32; 3]) -> bool {
    let view = &mut *ctx.view;
    while view.block(x, y, z) == Block::Air && y > MIN_Y + 2 {
        y -= 1;
    }
    if !matches!(view.block(x, y, z), Block::Sand | Block::RedSand | Block::SuspiciousSand) {
        return false;
    }
    let ring = || (-2..=2).flat_map(|dx| (-2..=2).map(move |dz| (dx, dz)));
    if ring().any(|(dx, dz)| view.block(x + dx, y - 1, z + dz) == Block::Air && view.block(x + dx, y - 2, z + dz) == Block::Air) {
        return false;
    }
    trace::line(|| format!("  desert well at {x} {y} {z}"));
    let mut set = |dx: i32, dy: i32, dz: i32, block: Block| _ = view.set_block(x + dx, y + dy, z + dz, block);
    for dy in -2..=0 {
        ring().for_each(|(dx, dz)| set(dx, dy, dz, Block::Sandstone));
    }
    // The cross: water around the middle, sand under it (west, east, north, south).
    const CROSS: [(i32, i32); 5] = [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dy, block) in [(0, Block::Water), (-1, Block::Sand)] {
        CROSS.iter().for_each(|&(dx, dz)| set(dx, dy, dz, block));
    }
    ring().filter(|&(dx, dz)| dx.abs() == 2 || dz.abs() == 2).for_each(|(dx, dz)| set(dx, 1, dz, Block::Sandstone));
    [(2, 0), (-2, 0), (0, 2), (0, -2)].into_iter().for_each(|(dx, dz)| set(dx, 1, dz, Block::SandstoneSlab));
    (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| (dx, dz))).for_each(|(dx, dz)| set(dx, 4, dz, if (dx, dz) == (0, 0) { Block::Sandstone } else { Block::SandstoneSlab }));
    for dy in 1..=3 {
        [(-1, -1), (-1, 1), (1, -1), (1, 1)].into_iter().for_each(|(dx, dz)| set(dx, dy, dz, Block::Sandstone));
    }
    // Both picks are drawn before either sand is set: the origin, west, east, north, south.
    let first = CROSS[ctx.random.next_int(5) as usize];
    let second = CROSS[ctx.random.next_int(5) as usize];
    for ((dx, dz), dy) in [(first, -1), (second, -2)] {
        let at = [x + dx, y + dy, z + dz];
        view.set_block(at[0], at[1], at[2], Block::SuspiciousSand);
        // 148299240 seeds the table only when the cell holds the sand's block entity.
        if view.block(at[0], at[1], at[2]) == Block::SuspiciousSand {
            let seed = ctx.random.next_int_unbounded();
            view.set_block_entity(at, BlockEntity::Loot { table: TABLE, seed });
        }
    }
    true
}
