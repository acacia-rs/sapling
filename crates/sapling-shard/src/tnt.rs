//! Primed TNT, as BDS's burnt and burst (docs/research/block-uses.md, "TNT"): lit by flint and
//! steel, a signal or a blast, the block becomes an entity that hops, burns its fuse and bursts.
// TODO: the `MOVING` flag BDS turned on and off is not told; fire and burning arrows light none;
// one in water still breaks blocks; a blast throws none; none is kept in the world folder; the
// fuse of one a blast lit is the Java game's, not measured.

use std::f32::consts::TAU;

use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent};
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{Block, BlockPos};
use sapling_data::AddEntityOptions;

use crate::blocks::{self, Dice};
use crate::body::{Body, Physics};
use crate::components::{InLayer, Layer, NetId};
use crate::creatures::{self, Wreck};
use crate::{entities, mobs};

pub(crate) const KIND: &str = "minecraft:tnt";
const SIZE: f32 = 0.98;
/// Off, a primed TNT goes out at the end of its fuse ("whether TNT explodes after activation").
const EXPLODES: &str = "tntExplodes";
const DROP_DECAY: &str = "tntExplosionDropDecay";
/// The wire position is this far above the bottom of its box.
const WIRE_OFFSET: f32 = 0.49;
pub(crate) const FUSE: i32 = 80;
/// The fuse of one a blast lit: from this many ticks, to this many more.
const SHORT_FUSE: [u32; 2] = [10, 20];
const POWER: f32 = 4.0;
/// It starts upwards at this speed, and across at this one, any way.
const HOP: f32 = 0.2;
const DRIFT: f32 = 0.02;
/// How far up its box it bursts.
const HEART: f32 = 0.0625 * SIZE;

/// A primed TNT's own part: the ticks left of its fuse.
#[derive(Component)]
struct Primed(i32);

fn wire([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y + WIRE_OFFSET, z]
}

fn announcement(id: u64, body: &Body) -> Vec<Bytes> {
    let options = AddEntityOptions { id, identifier: KIND, feet: wire(body.at), yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1::IGNITED, health: None, name: "" };
    let [x, y, z] = body.motion;
    vec![sapling_net::encode(&AddEntity { velocity: Vec3f { x, y, z }, ..sapling_data::add_entity(&options) })]
}

/// Primes a TNT with the bottom of its box at `feet`, to burst in `fuse` ticks.
pub(crate) fn prime(world: &mut World, layer: Entity, feet: [f32; 3], fuse: i32) -> Option<Entity> {
    let angle = world.get_resource_or_insert_with(Dice::default).fraction() * TAU;
    let body = Body::new(feet, [-angle.sin() * DRIFT, HOP, -angle.cos() * DRIFT], [SIZE, SIZE]);
    let entity = entities::spawn(world, layer, KIND, body, wire(feet), |id| Some(announcement(id, &body)))?;
    world.entity_mut(entity).insert(Primed(fuse));
    Some(entity)
}

fn is_tnt(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.name == KIND)
}

fn set_off(world: &mut World, layer: Entity, pos: BlockPos, fuse: i32) -> bool {
    if !is_tnt(world, layer, pos) {
        return false;
    }
    blocks::update_block(world, layer, pos, &Block::new("minecraft:air"));
    let [x, y, z] = pos.map(|c| c as f32);
    prime(world, layer, [x + 0.5, y, z + 0.5], fuse);
    true
}

/// Lights the TNT block at `pos`, with the sound of it; false for any other block.
pub(crate) fn light(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    if !is_tnt(world, layer, pos) {
        return false;
    }
    let [x, y, z] = pos.map(|c| c as f32);
    let lit = LevelEvent { event: LevelEventEvent::SoundIgnite, position: Vec3f { x: x + 0.5, y, z: z + 0.5 }, data: 0 };
    blocks::to_viewers(world, layer, pos, &[sapling_net::encode(&lit)]);
    set_off(world, layer, pos, FUSE)
}

/// A blast reached the block at `pos`: TNT is primed on a short fuse; false for any other block.
pub(crate) fn catch(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let [least, more] = SHORT_FUSE;
    let fuse = least + world.get_resource_or_insert_with(Dice::default).roll(more);
    set_off(world, layer, pos, fuse as i32)
}

/// A tick of every primed TNT: it moves, its fuse burns, and the tick after its end it bursts.
pub(crate) fn burn(world: &mut World) {
    let primed: Vec<(Entity, Entity, u64, i32)> = world.query::<(Entity, &InLayer, &NetId, &Primed)>().iter(world).map(|(entity, layer, id, primed)| (entity, layer.0, id.0, primed.0 - 1)).collect();
    for (entity, layer, id, left) in primed {
        let Some((body, mut packets)) = entities::step(world, entity, layer, id, &Physics::TNT, WIRE_OFFSET) else { continue };
        if left < 0 && !crate::rules::on(world, layer, EXPLODES) {
            mobs::remove(world, entity);
            continue;
        }
        if left < 0 {
            let [x, y, z] = body.at;
            let wreck = if crate::rules::on(world, layer, DROP_DECAY) { Wreck::Decays } else { Wreck::Leaves };
            creatures::explode(world, Some(entity), layer, [x, y + HEART, z], POWER, wreck);
            continue;
        }
        world.entity_mut(entity).insert(Primed(left));
        if !packets.is_empty() {
            mobs::reannounce(world, entity, announcement(id, &body));
        }
        packets.insert(0, sapling_net::encode(&sapling_data::mob_fuse(id, KIND, left)));
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), packets);
    }
}
