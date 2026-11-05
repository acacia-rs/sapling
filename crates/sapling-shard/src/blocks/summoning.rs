//! A wither built of blocks, as BDS's `SkullBlock::checkMobSpawn`: off peaceful, a wither skeleton
//! skull put down to finish three skulls over a T of soul sand or soul soil, the stem's sides
//! empty, turns the blocks to air and calls the wither up (creatures/wither) where BDS's matcher
//! (0x1415f2940) puts it: its match corner, the skull row's lowest end, plus 1 east, 2 down and 1
//! south in world axes whatever way the T stands, then (0.5, 0.05, 0.5). The T may stand or lie any
//! way, as BDS's matcher found one lying on the floor (bds-n2wither.txt).
// TODO: a T standing along x, or on its side (wither-lightning-fireworks.md).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Difficulty};

use super::{level, send};
use crate::components::Settings;

const SKULL: &str = "minecraft:wither_skeleton_skull";
const SOULS: [&str; 2] = ["minecraft:soul_sand", "minecraft:soul_soil"];
/// Where the wither stands from its match corner's cell.
const FROM_CORNER: [f32; 3] = [1.5, -1.95, 1.5];
const AXES: [[i32; 3]; 3] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
const SOULS_SHOWN: usize = 120;
const SOUL_PARTICLE: i32 = 15;

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
        built(world, layer, [0, 1, 2].map(|axis| pos[axis] - along[axis] * step), along, down).map(|cells| (cells, along, down))
    });
    let Some((cells, along, down)) = found else { return };
    let corner = [0, 1, 2].map(|axis| cells[..3].iter().map(|cell| cell[axis]).min().unwrap_or(0));
    let mut spot = [0, 1, 2].map(|axis| corner[axis] as f32 + FROM_CORNER[axis]);
    spot[0] += lying_east(along, down);
    let particles = souls(world, spot);
    crate::view::to_viewers(world, layer, spot.map(|c| c.floor() as i32), &particles);
    let air = Block::new("minecraft:air");
    for cell in &cells {
        send::put_block(world, layer, *cell, &air);
    }
    crate::creatures::build_wither(world, layer, spot);
}

/// The 120 particles BDS showed before the T went (bds-gxwither.txt): each at the wither's spot plus
/// a fraction across, and 2 below to 1.9 above, as the Java game's skull block placed them.
fn souls(world: &mut World, spot: [f32; 3]) -> Vec<bytes::Bytes> {
    let mut dice = world.get_resource_or_insert_with(super::Dice::default);
    (0..SOULS_SHOWN)
        .map(|_| {
            let [x, y, z] = [spot[0] + dice.fraction(), spot[1] - 2.0 + dice.fraction() * 3.9, spot[2] + dice.fraction()];
            sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleProjectileHit, position: Vec3f { x, y, z }, data: SOUL_PARTICLE })
        })
        .collect()
}

/// BDS's wither from a T lying on the floor stands a block further east than the corner rule, its
/// skulls along z whichever way its stem lies (bds-g8wither.txt) and along x with its stem north
/// (bds-s3wither.txt); a T on its side was not seen.
fn lying_east(along: [i32; 3], down: [i32; 3]) -> f32 {
    if along[1] == 0 && down[1] == 0 { 1.0 } else { 0.0 }
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
