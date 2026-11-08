//! What a hit its awake heart shrugs off sets off, as BDS's in the `j3creak` drill
//! (research/captures/bds-j3creak.txt): a tick on, 2 or 3 resin clumps grow on the pale oak logs
//! by the heart, each heard `place`; a tick after that, and every [`EVERY`] ticks for [`SOUNDS`]
//! sounds, `block.creaking_heart.trail` sounds along the line from the creaking's middle to the
//! heart's. A hit while a trail is under way sets off nothing.
// TODO: BDS put no clump in water; Bedrock's resin_clump has no waterlogged state, so water is skipped.

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::heart::LOGS;
use crate::blocks::{self, Dice};
use crate::body::Body;
use crate::components::Layer;

const EVERY: u64 = 10;
const SOUNDS: u64 = 10;
/// Ticks from a hit until another sets a trail off again: its last sound's, and a step.
pub(super) const LASTS: u64 = 2 + EVERY * SOUNDS;
/// How far along to the heart the first sound is, and each next one further.
const FIRST: f32 = 0.2;
const STEP: f64 = 0.08;
/// The clumps a hit grows, least and most, and how far through the logs it looks: steps and cells
/// (the Java game's `spreadResin`).
const CLUMPS: [u32; 2] = [2, 3];
const DEPTH: usize = 2;
const CELLS: usize = 64;
const CLUMP: &str = "minecraft:resin_clump";
const FACES: &str = "multi_face_direction_bits";
/// Each way, and the bit of a clump's face that lies that way (down, up, south, west, north, east).
const WAYS: [([i32; 3], u32); 6] = [([0, -1, 0], 1), ([0, 1, 0], 2), ([0, 0, 1], 4), ([-1, 0, 0], 8), ([0, 0, -1], 16), ([1, 0, 0], 32)];

/// A tick of a trail set off `since` ticks ago by a hit on `creaking`, of the heart at `heart`.
pub(super) fn tick(world: &mut World, layer: Entity, creaking: Entity, heart: BlockPos, since: u64) {
    if since == 1 {
        let count = CLUMPS[0] + world.get_resource_or_insert_with(Dice::default).roll(CLUMPS[1] - CLUMPS[0] + 1);
        for _ in 0..count {
            grow_resin(world, layer, heart);
        }
    } else if since >= 2 && (since - 2).is_multiple_of(EVERY) && (since - 2) / EVERY < SOUNDS {
        sound(world, layer, creaking, heart, (since - 2) / EVERY);
    }
}

/// The trail's `step`th sound, at `0.2 + 0.08 step` of the way from the creaking's middle to the
/// heart's, carrying `1000 (2 f - 1)` of that.
fn sound(world: &mut World, layer: Entity, creaking: Entity, heart: BlockPos, step: u64) {
    let Some(body) = world.get::<Body>(creaking).copied() else { return };
    // f32 sums of BDS's exact data: -440 and 679, not -439 and 680 (bds-j3creak.txt).
    let along = FIRST + (STEP * step as f64) as f32;
    let from = [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]];
    let [x, y, z]: [f32; 3] = std::array::from_fn(|axis| from[axis] + along * (heart[axis] as f32 + 0.5 - from[axis]));
    let data = ((2.0 * along - 1.0) * 1000.0) as i32;
    blocks::to_viewers(world, layer, heart, &[blocks::noise("block.creaking_heart.trail", Vec3f { x, y, z }, data)]);
}

/// One clump on the first open side of a log met going out from the heart through its logs, sides
/// tried in a random order: the clump faces the log.
fn grow_resin(world: &mut World, layer: Entity, heart: BlockPos) -> Option<()> {
    let (mut queue, mut seen) = (std::collections::VecDeque::from([(heart, 0)]), vec![heart]);
    let mut found = None;
    while let Some((cell, depth)) = queue.pop_front() {
        let ways = shuffled(world);
        let level = &world.get::<Layer>(layer)?.world;
        let step = |way: [i32; 3]| -> BlockPos { std::array::from_fn(|axis| cell[axis] + way[axis]) };
        if cell != heart && is_log(level, cell) {
            found = ways.iter().find_map(|&(way, _)| {
                let toward = WAYS.iter().find(|(back, _)| (0..3).all(|axis| back[axis] == -way[axis]))?.1;
                let faces = faces_at(level, step(way))?;
                (faces & toward == 0).then_some((step(way), faces | toward))
            });
            if found.is_some() {
                break;
            }
        }
        if depth < DEPTH {
            let next: Vec<BlockPos> = shuffled(world).into_iter().map(|(way, _)| step(way)).collect();
            let level = &world.get::<Layer>(layer)?.world;
            for at in next {
                if is_log(level, at) && !seen.contains(&at) && seen.len() < CELLS {
                    seen.push(at);
                    queue.push_back((at, depth + 1));
                }
            }
        }
    }
    let (at, faces) = found?;
    let level = &world.get::<Layer>(layer)?.world;
    let id = blocks::with_property(level.registry(), level.block_id(CLUMP)?, FACES, &faces.to_string())?;
    let placed = blocks::noise("place", Vec3f { x: at[0] as f32, y: at[1] as f32, z: at[2] as f32 }, sapling_world::block_hash(level.registry(), id) as i32);
    blocks::set(world, layer, at, id).then(|| blocks::to_viewers(world, layer, at, &[placed]))
}

/// The six ways, in a random order.
fn shuffled(world: &mut World) -> [([i32; 3], u32); 6] {
    let mut ways = WAYS;
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    for at in (1..ways.len()).rev() {
        ways.swap(at, dice.roll(at as u32 + 1) as usize);
    }
    ways
}

fn is_log(level: &sapling_world::World, [x, y, z]: BlockPos) -> bool {
    level.registry().get(level.block(x, y, z)).is_some_and(|state| LOGS.contains(&state.name))
}

/// The face bits a clump at `at` has, 0 in air; none where neither is.
fn faces_at(level: &sapling_world::World, [x, y, z]: BlockPos) -> Option<u32> {
    let state = level.registry().get(level.block(x, y, z))?;
    match state.name {
        "minecraft:air" => Some(0),
        CLUMP => state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == FACES)?.1.parse().ok(),
        _ => None,
    }
}
