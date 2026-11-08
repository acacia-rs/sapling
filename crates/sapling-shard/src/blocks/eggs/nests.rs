//! What mobs do with turtle eggs besides treading on them: a turtle lays them on sand, a zombie
//! goes for them and stomps them out (creatures/nesting.rs, creatures/stomping.rs).

use acacia_proto::packets::{LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::turtle::{COUNTS, EGG, SANDS, is_egg_at};
use super::{level, level_event, name_at, send};

const AIR: &str = "minecraft:air";

/// The cells within `across` and `up` of `at`, nearest first, that `fits`.
fn within(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32, fits: fn(&super::Level, BlockPos) -> bool) -> Vec<BlockPos> {
    let Some(level) = level(world, layer) else { return Vec::new() };
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let mut cells: Vec<BlockPos> = (-across..=across).flat_map(|dx| (-up..=up).flat_map(move |dy| (-across..=across).map(move |dz| [x + dx, y + dy, z + dz]))).collect();
    let away = |[cx, cy, cz]: &BlockPos| (cx - x).pow(2) + (cy - y).pow(2) + (cz - z).pow(2);
    cells.sort_by_key(away);
    cells.retain(|cell| fits(level, *cell));
    cells
}

fn nearest(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32, fits: fn(&super::Level, BlockPos) -> bool) -> Option<BlockPos> {
    within(world, layer, at, across, up, fits).into_iter().next()
}

fn is_egg(level: &super::Level, cell: BlockPos) -> bool {
    name_at(level, cell) == Some(EGG)
}

/// The nearest turtle egg within `across` and `up` of `at`.
pub(crate) fn egg_near(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32) -> Option<BlockPos> {
    nearest(world, layer, at, across, up, is_egg)
}

/// Every turtle egg within `across` and `up` of `at`, nearest first.
pub(crate) fn eggs_near(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32) -> Vec<BlockPos> {
    within(world, layer, at, across, up, is_egg)
}

fn is_nest(level: &super::Level, [x, y, z]: BlockPos) -> bool {
    name_at(level, [x, y, z]) == Some(AIR) && name_at(level, [x, y - 1, z]).is_some_and(|under| SANDS.contains(&under))
}

/// The nearest open cell on sand within `across` and `up` of `at`: where a turtle lays.
pub(crate) fn nest_near(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32) -> Option<BlockPos> {
    nearest(world, layer, at, across, up, is_nest)
}

fn is_spawn_site(level: &super::Level, pos: BlockPos) -> bool {
    name_at(level, pos) == Some(AIR) && super::frogspawn::afloat(level, pos)
}

/// The nearest open cell over water within `across` and `up` of `at`: where a frog lays.
pub(crate) fn spawn_site_near(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32) -> Option<BlockPos> {
    nearest(world, layer, at, across, up, is_spawn_site)
}

/// Frogspawn laid at `pos`, still open over water; false where it cannot be.
pub(crate) fn lay_spawn(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(id) = level(world, layer).filter(|level| is_spawn_site(level, pos)).and_then(|level| level.registry().find(super::frogspawn::SPAWN, "")) else { return false };
    send::set(world, layer, pos, id)
}

/// A turtle digging at `pos`: the sand under it flies from the cell's corner
/// (research/captures/bds-nesting.txt).
pub(crate) fn dig(world: &mut World, layer: Entity, [x, y, z]: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let sand = sapling_world::block_hash(level.registry(), level.block(x, y - 1, z)) as i32;
    let dust = level_event(LevelEventEvent::ParticleDestroy, [x, y, z], sand);
    send::to_viewers(world, layer, [x, y, z], &[dust]);
}

/// Lays `count` (1 to 4) fresh turtle eggs at `pos`, still open on sand (they sparkle as placed);
/// false where it cannot.
pub(crate) fn lay(world: &mut World, layer: Entity, pos: BlockPos, count: usize) -> bool {
    let Some(id) = clutch(world, layer, pos, count) else {
        if let Some(mut told) = world.get_resource_mut::<super::turtle::Foretold>() {
            told.0.retain(|nest| *nest != (layer, pos));
        }
        return false;
    };
    send::set(world, layer, pos, id)
}

/// The egg block `count` eggs laid at `pos` make, where a nest is open.
fn clutch(world: &World, layer: Entity, pos: BlockPos, count: usize) -> Option<u32> {
    let eggs = COUNTS.get(count.wrapping_sub(1)).filter(|_| level(world, layer).is_some_and(|level| is_nest(level, pos)))?;
    let block = Block::new(EGG).with(format!("cracked_state=no_cracks,turtle_egg_count={eggs}"));
    level(world, layer).and_then(|level| send::resolve(level.registry(), &block))
}

/// The sparkle of `count` eggs to be laid at `pos` the next tick, sent now: BDS's came a tick before
/// their UpdateBlock (bds-nesting.txt, bds-ao2nest.txt). Their [`lay`] then sparkles no more.
pub(crate) fn foretell(world: &mut World, layer: Entity, pos: BlockPos, count: usize) {
    let Some(sparkle) = clutch(world, layer, pos, count).zip(level(world, layer)).map(|(id, level)| level_event(LevelEventEvent::ParticleTurtleEgg, pos, sapling_world::block_hash(level.registry(), id) as i32)) else { return };
    send::to_viewers(world, layer, pos, &[sparkle]);
    world.get_resource_or_insert_with(super::turtle::Foretold::default).0.push((layer, pos));
}

/// A sound of a mob `kind` (`unique` its id) at the eggs at `pos`, from the cell's corner.
fn mob_sound(name: &str, pos: BlockPos, kind: &str, unique: i64) -> bytes::Bytes {
    let position = Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: unique, fire_at_position: None })
}

/// The plain egg BDS names in its stomping particles, whatever the eggs there.
fn plain_egg(level: &super::Level) -> i32 {
    level.registry().find(EGG, "cracked_state=no_cracks,turtle_egg_count=one_egg").map_or(0, |id| sapling_world::block_hash(level.registry(), id) as i32)
}

/// A tick of a mob of `kind` stomping the eggs at `pos`: they crumble when it `crumbles`, and it is `heard`
/// (research/captures/bds-stomping.txt).
pub(crate) fn stomped_at(world: &mut World, layer: Entity, pos: BlockPos, kind: &str, unique: i64, crumbles: bool, heard: bool) {
    let Some(level) = level(world, layer) else { return };
    let mut shown: Vec<_> = crumbles.then(|| level_event(LevelEventEvent::ParticleBreakingEgg, pos, plain_egg(level))).into_iter().collect();
    shown.extend(heard.then(|| mob_sound("block.turtle_egg.attack", pos, kind, unique)));
    send::to_viewers(world, layer, pos, &shown);
}

/// The eggs at `pos` are stomped out at once, all of them.
pub(crate) fn stomp_out(world: &mut World, layer: Entity, pos: BlockPos, kind: &str, unique: i64) {
    let Some(level) = level(world, layer).filter(|_| is_egg_at(world, layer, pos)) else { return };
    let (burst, air) = (level_event(LevelEventEvent::ParticleDestroyEgg, pos, plain_egg(level)), level.registry().air_id());
    send::to_viewers(world, layer, pos, &[burst]);
    if send::set(world, layer, pos, air) {
        send::to_viewers(world, layer, pos, &[mob_sound("block.turtle_egg.break", pos, kind, unique)]);
    }
}
