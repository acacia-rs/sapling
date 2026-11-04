//! A wither built of blocks, as BDS's `SkullBlock::checkMobSpawn`: off peaceful, a wither skeleton
//! skull put down to finish three skulls over a T of soul sand or soul soil, the stem's sides
//! empty, turns the blocks to air and calls the wither up over the stem's foot (creatures/wither).
//! The T may stand or lie any way, as BDS's matcher found one lying on the floor (bds-n2wither.txt).
// TODO: BDS put the wither one east of an upright T's foot and 4 east and 2 under a lying one's
// (its match corner is not the foot); 120 soul particles are not shown.

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Difficulty};

use super::{level, send};
use crate::components::Settings;

const SKULL: &str = "minecraft:wither_skeleton_skull";
const SOULS: [&str; 2] = ["minecraft:soul_sand", "minecraft:soul_soil"];
/// The wither stands this far over the stem's foot.
const RISES: f32 = 0.05;
const AXES: [[i32; 3]; 3] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];

fn name_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<&'static str> {
    let level = level(world, layer)?;
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

/// The T's cells, its stem's foot last, if one whose top row starts at `left`, runs along `along`
/// and has its stem towards `down` stands there.
fn built(world: &World, layer: Entity, left: BlockPos, along: [i32; 3], down: [i32; 3]) -> Option<Vec<BlockPos>> {
    let at = |step: i32, depth: i32| [0, 1, 2].map(|axis| left[axis] + along[axis] * step + down[axis] * depth);
    let is = |cell: BlockPos, names: &[&str]| name_at(world, layer, cell).is_some_and(|name| names.contains(&name));
    let skulls: Vec<BlockPos> = (0..3).map(|step| at(step, 0)).collect();
    let arms: Vec<BlockPos> = (0..3).map(|step| at(step, 1)).collect();
    let empty = [at(0, 2), at(2, 2)].iter().all(|cell| is(*cell, &["minecraft:air"]));
    let stands = skulls.iter().all(|cell| is(*cell, &[SKULL])) && arms.iter().all(|cell| is(*cell, &SOULS)) && is(at(1, 2), &SOULS) && empty;
    stands.then(|| [skulls, arms, vec![at(1, 2)]].concat())
}

/// Every way a T may lie: its row's axis and its stem's way across that axis.
fn ways() -> impl Iterator<Item = ([i32; 3], [i32; 3])> {
    let downs = AXES.into_iter().flat_map(|axis| [axis, axis.map(|v| -v)]);
    AXES.into_iter().flat_map(move |along| downs.clone().filter(move |down| (0..3).all(|i| along[i] * down[i] == 0)).map(move |down| (along, down)))
}

/// A block was put at `pos`: a skull that finishes a T calls the wither up.
pub(super) fn placed(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    if name != SKULL || world.resource::<Settings>().0.difficulty == Difficulty::Peaceful {
        return;
    }
    let found = ways().flat_map(|(along, down)| (0..3).map(move |step| (along, down, step))).find_map(|(along, down, step)| {
        built(world, layer, [0, 1, 2].map(|axis| pos[axis] - along[axis] * step), along, down)
    });
    let Some(cells) = found else { return };
    let air = Block::new("minecraft:air");
    for cell in &cells {
        send::put_block(world, layer, *cell, &air);
    }
    let foot = cells[cells.len() - 1];
    crate::creatures::build_wither(world, layer, [foot[0] as f32 + 0.5, foot[1] as f32 + RISES, foot[2] as f32 + 0.5]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_t_may_lie_twelve_ways() {
        let ways: Vec<_> = ways().collect();
        assert_eq!(ways.len(), 12);
        assert!(ways.contains(&([0, 0, 1], [-1, 0, 0])) && ways.contains(&([1, 0, 0], [0, -1, 0])));
    }
}
