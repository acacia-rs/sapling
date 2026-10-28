//! Sculk catalysts (docs/research/sculk.md, "Catalyst"): a mob or player dying within 10 blocks
//! makes the nearest bloom and gives it its experience, which it no longer drops, as charge
//! where it died. Each tick the catalyst's `SculkSpreader` moves the charge (the generator's own
//! port, `sapling_worldgen::sculk`, run over the live world), laying veins, turning blocks to sculk
//! and growing sensors and shriekers.
// TODO: cursors are not saved with the catalyst, and blocks the generator never places read as bedrock.

use std::collections::BTreeMap;

use acacia_proto::nbt::{Nbt, Value as NbtValue};
use acacia_proto::packets::LevelEventGeneric;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_worldgen::Block;
use sapling_worldgen::random::Mt19937;
use sapling_worldgen::sculk::{self, Cells, Cursor, SculkWorld, Sound, Spreader};

use super::super::queued::{self, Job};
use super::super::{Dice, level, send, with_property};

pub(super) const CATALYST: &str = "minecraft:sculk_catalyst";
const SHRIEKER: &str = "minecraft:sculk_shrieker";
/// `SculkCatalystBlockActor::getRange` (140214560).
const RANGE: i32 = 10;
/// Ticks a catalyst blooms (148b435b0).
const BLOOMING: u64 = 8;
/// `LevelEventGeneric`s 2036 bloom, 2037 charge and 2038 charge pop, zigzagged (sleep.rs).
const BLOOM: u32 = 2036 << 1;
const CHARGE: u32 = 2037 << 1;
const CHARGE_POP: u32 = 2038 << 1;

#[derive(Resource)]
pub(crate) struct Catalysts {
    cursors: BTreeMap<(Entity, BlockPos), Vec<Cursor>>,
    /// The level's generator, which BDS's spreader draws from.
    random: Mt19937,
}

fn catalysts(world: &mut World) -> Mut<'_, Catalysts> {
    if !world.contains_resource::<Catalysts>() {
        let seed = world.get_resource_or_insert_with(Dice::default).roll(u32::MAX);
        world.insert_resource(Catalysts { cursors: BTreeMap::new(), random: Mt19937::new(seed) });
    }
    world.resource_mut::<Catalysts>()
}

fn is_catalyst(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    level(world, layer).filter(|level| level.in_height(y)).and_then(|level| level.registry().get(level.block(x, y, z))).is_some_and(|state| state.name == CATALYST)
}

/// The catalyst whose centre is nearest `at` within its range, BDS's game-event reach (sculk.md S1).
fn nearest(world: &World, layer: Entity, at: [f32; 3]) -> Option<BlockPos> {
    let [cx, cy, cz] = at.map(|c| c.floor() as i32);
    let level = level(world, layer)?;
    let cells = (-RANGE..=RANGE).flat_map(|dx| (-RANGE..=RANGE).flat_map(move |dy| (-RANGE..=RANGE).map(move |dz| [cx + dx, cy + dy, cz + dz])));
    let distance = |pos: BlockPos| (0..3).map(|axis| (pos[axis] as f32 + 0.5 - at[axis]).powi(2)).sum::<f32>() as u64;
    cells
        .filter(|&pos| level.is_loaded(send::column(pos)) && is_catalyst(world, layer, pos))
        .map(|pos| (distance(pos), pos))
        .filter(|(distance, _)| *distance <= (RANGE * RANGE) as u64)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, pos)| pos)
}

/// Something with experience to give died with its feet at `at` (144879f70): the nearest catalyst
/// in range blooms and, if `reward` offers experience, takes it as charge on the cell it died in.
/// Whether a catalyst took the death; the dead then drop no experience.
pub(crate) fn mob_died(world: &mut World, layer: Entity, at: [f32; 3], reward: impl FnOnce(&mut World) -> Option<i32>) -> bool {
    let Some(catalyst) = nearest(world, layer, at) else { return false };
    if let Some(points) = reward(world).filter(|points| *points > 0) {
        let cell = at.map(|c| c.floor() as i32);
        sculk::add_cursors(catalysts(world).cursors.entry((layer, catalyst)).or_default(), cell, points);
    }
    bloom(world, layer, catalyst);
    true
}

fn generic(event_id: u32, tags: Vec<Nbt>) -> bytes::Bytes {
    sapling_net::encode(&LevelEventGeneric { event_id, nbt: tags })
}

fn int(name: &str, value: i32) -> Nbt {
    Nbt { name: name.to_owned(), value: NbtValue::Int(value) }
}

/// `SculkCatalystBlock::bloom` (148b435b0): `bloom` set for 8 ticks, and the bloom particles.
fn bloom(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos;
    let Some(bloomed) = level(world, layer).and_then(|level| with_property(level.registry(), level.block(x, y, z), "bloom", "1")) else { return };
    send::set(world, layer, pos, bloomed);
    queued::after(world, layer, pos, BLOOMING, Job::Sculk);
    let told = generic(BLOOM, vec![int("originX", x), int("originY", y), int("originZ", z)]);
    send::to_viewers(world, layer, pos, &[told]);
}

/// A catalyst's bloom is over.
pub(super) fn wilt(world: &mut World, layer: Entity, [x, y, z]: BlockPos) {
    let Some(wilted) = level(world, layer).and_then(|level| with_property(level.registry(), level.block(x, y, z), "bloom", "0")) else { return };
    send::set(world, layer, [x, y, z], wilted);
}

/// Every catalyst's cursors take a round (`SculkCatalystBlockActor::tick` 144879bc0). A catalyst
/// gone from a loaded column takes its cursors with it.
pub(super) fn tick(world: &mut World) {
    let Some(mut catalysts) = world.remove_resource::<Catalysts>() else { return };
    let mut spent = Vec::new();
    for (&(layer, pos), cursors) in catalysts.cursors.iter_mut() {
        let loaded = level(world, layer).is_some_and(|level| level.is_loaded(send::column(pos)));
        if loaded && !is_catalyst(world, layer, pos) {
            cursors.clear();
        }
        if loaded && !cursors.is_empty() {
            sculk::update_cursors(&mut Live { world: &mut *world, layer }, &Spreader::LEVEL, cursors, pos, &mut catalysts.random, true);
        }
        if cursors.is_empty() {
            spent.push((layer, pos));
        }
    }
    spent.iter().for_each(|key| _ = catalysts.cursors.remove(key));
    world.insert_resource(catalysts);
}

/// The live world as the spreader sees it.
struct Live<'w> {
    world: &'w mut World,
    layer: Entity,
}

impl Live<'_> {
    fn tell(&mut self, pos: BlockPos, packet: bytes::Bytes) {
        send::to_viewers(self.world, self.layer, pos, &[packet]);
    }
}

impl Cells for Live<'_> {
    fn block(&self, x: i32, y: i32, z: i32) -> Block {
        let Some(level) = level(self.world, self.layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z]))) else { return Block::Air };
        level.generator_block(x, y, z).or_else(|| Block::from_name("bedrock")).unwrap_or(Block::Stone)
    }

    fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) -> bool {
        let Some(level) = level(self.world, self.layer) else { return false };
        let id = level.generator_id(block);
        // The generator's shrieker can summon; a catalyst's cannot (148b41ca1).
        let id = if level.registry().get(id).is_some_and(|state| state.name == SHRIEKER) { with_property(level.registry(), id, "can_summon", "0").unwrap_or(id) } else { id };
        send::set(self.world, self.layer, [x, y, z], id)
    }
}

impl SculkWorld for Live<'_> {
    fn sound(&mut self, at: [i32; 3], sound: Sound) {
        let name = match sound {
            Sound::Spread => "block.sculk.spread",
            Sound::SensorPlaced => "block.sculk_sensor.place",
            Sound::ShriekerPlaced => "block.sculk_shrieker.place",
        };
        let [x, y, z] = at.map(|c| c as f32);
        self.tell(at, send::noise(name, Vec3f { x, y, z }, -1));
    }

    fn charge(&mut self, at: [i32; 3], charge: i32, facings: i32) {
        let short = |name: &str, value: i32| Nbt { name: name.to_owned(), value: NbtValue::Short(value as i16) };
        let [x, y, z] = at;
        self.tell(at, generic(CHARGE, vec![int("x", x), int("y", y), int("z", z), short("charge", charge), short("facing", facings)]));
    }

    fn charge_pop(&mut self, at: [i32; 3]) {
        let [x, y, z] = at;
        self.tell(at, generic(CHARGE_POP, vec![int("x", x), int("y", y), int("z", z)]));
    }

    fn ticking(&self, [x, y, z]: [i32; 3]) -> bool {
        level(self.world, self.layer).is_some_and(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))
    }
}
