//! Mobs spawned through the plugin API: bodies (body.rs) that fall and collide and do nothing
//! else, shown as BDS announced their kind (`sapling_data::add_entity`); and the entity side of
//! the handlers' context.
// TODO: no AI, no damage, not saved; their physics are the Java game's (`Physics::MOB`).

use bevy_ecs::prelude::*;
use bytes::Bytes;
use acacia_proto::types::MetadataFlags1;
use sapling_api::{EntityBackend, EntityId, EntityInfo};
use sapling_data::AddEntityOptions;

use crate::body::{Body, Physics};
use crate::components::{InLayer, Layer, NetId};
use crate::handlers::Ctx;
use crate::{blocks, entities, mobs, worlds};

/// A mob the API spawned; its [`Body`] and [`entities::Kind`] are beside it.
#[derive(Component)]
pub(crate) struct Creature;

fn announcement(id: u64, kind: &str, body: &Body) -> Vec<Bytes> {
    let options = AddEntityOptions {
        id,
        identifier: kind,
        feet: body.at,
        yaw: 0.0,
        pitch: 0.0,
        variant: 0,
        mark_variant: 0,
        color: 0,
        baby: false,
        flags: MetadataFlags1(0),
        health: None,
        name: "",
    };
    vec![sapling_net::encode(&sapling_data::add_entity(&options))]
}

/// A mob of `kind` with its feet at `feet` in `layer`: one BDS was recorded announcing, in a
/// loaded column.
fn spawn(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3]) -> Option<EntityId> {
    let size = sapling_data::mob_size(kind)?;
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    if !(level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))) {
        return None;
    }
    let body = Body::new(feet, [0.0; 3], size);
    let entity = entities::spawn(world, layer, kind, body, feet, |id| Some(announcement(id, kind, &body)))?;
    world.entity_mut(entity).insert(Creature);
    Some(EntityId(world.get::<NetId>(entity)?.0))
}

/// Moves every spawned mob a tick.
pub(crate) fn tick(world: &mut World) {
    let all: Vec<(Entity, Entity, u64, String)> =
        world.query_filtered::<(Entity, &InLayer, &NetId, &entities::Kind), With<Creature>>().iter(world).map(|(entity, layer, id, kind)| (entity, layer.0, id.0, kind.0.clone())).collect();
    for (entity, layer, id, kind) in all {
        let Some((body, packets)) = entities::step(world, entity, layer, id, &Physics::MOB, 0.0) else { continue };
        if !packets.is_empty() {
            blocks::to_viewers(world, layer, body.at.map(|c| c.floor() as i32), &packets);
            mobs::reannounce(world, entity, announcement(id, &kind, &body));
        }
    }
}

impl EntityBackend for Ctx<'_> {
    fn spawn_entity(&mut self, kind: &str, feet: [f32; 3]) -> Option<EntityId> {
        let layer = worlds::current(self.world);
        spawn(self.world, layer, kind, feet)
    }

    fn entity(&self, id: EntityId) -> Option<EntityInfo> {
        entities::describe(self.world, id)
    }

    fn entities(&self) -> Vec<EntityInfo> {
        entities::listed(self.world, worlds::current(self.world))
    }

    fn move_entity(&mut self, id: EntityId, feet: [f32; 3]) -> bool {
        entities::change(self.world, id, |body| body.at = feet)
    }

    fn set_velocity(&mut self, id: EntityId, velocity: [f32; 3]) -> bool {
        entities::change(self.world, id, |body| body.motion = velocity)
    }

    fn remove_entity(&mut self, id: EntityId) -> bool {
        entities::remove(self.world, id)
    }
}
