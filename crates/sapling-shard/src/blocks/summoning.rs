//! A wither built of blocks, as BDS's `SkullBlock::checkMobSpawn`: off peaceful, a wither skeleton
//! skull put down to finish three skulls over a T of soul sand or soul soil, the stem's sides
//! empty, turns the blocks to air and calls the wither up over the stem's foot (creatures/wither).
// TODO: the T is looked for upright along x and z; BDS's matcher also tries it lying down. Its
// 120 soul particles are not shown.

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Difficulty};

use super::{level, send};
use crate::components::Settings;

const SKULL: &str = "minecraft:wither_skeleton_skull";
const SOULS: [&str; 2] = ["minecraft:soul_sand", "minecraft:soul_soil"];
/// The wither stands this far over the stem's foot.
const RISES: f32 = 0.05;

fn name_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<&'static str> {
    let level = level(world, layer)?;
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

/// The T's cells, its stem's foot last, if one whose top row starts at `left` and runs along `along` stands there.
fn built(world: &World, layer: Entity, left: BlockPos, along: [i32; 3]) -> Option<Vec<BlockPos>> {
    let at = |step: i32, down: i32| [left[0] + along[0] * step, left[1] - down, left[2] + along[2] * step];
    let is = |cell: BlockPos, names: &[&str]| name_at(world, layer, cell).is_some_and(|name| names.contains(&name));
    let skulls: Vec<BlockPos> = (0..3).map(|step| at(step, 0)).collect();
    let arms: Vec<BlockPos> = (0..3).map(|step| at(step, 1)).collect();
    let empty = [at(0, 2), at(2, 2)].iter().all(|cell| is(*cell, &["minecraft:air"]));
    let stands = skulls.iter().all(|cell| is(*cell, &[SKULL])) && arms.iter().all(|cell| is(*cell, &SOULS)) && is(at(1, 2), &SOULS) && empty;
    stands.then(|| [skulls, arms, vec![at(1, 2)]].concat())
}

/// A block was put at `pos`: a skull that finishes a T calls the wither up.
pub(super) fn placed(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    if name != SKULL || world.resource::<Settings>().0.difficulty == Difficulty::Peaceful {
        return;
    }
    let tries = [[1, 0, 0], [0, 0, 1]].into_iter().flat_map(|along| (0..3).map(move |step| (along, step)));
    let found = tries.into_iter().find_map(|(along, step)| built(world, layer, [pos[0] - along[0] * step, pos[1], pos[2] - along[2] * step], along));
    let Some(cells) = found else { return };
    let air = Block::new("minecraft:air");
    for cell in &cells {
        send::put_block(world, layer, *cell, &air);
    }
    let foot = cells[cells.len() - 1];
    crate::creatures::build_wither(world, layer, [foot[0] as f32 + 0.5, foot[1] as f32 + RISES, foot[2] as f32 + 0.5]);
}
