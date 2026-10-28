//! The rest of what the world does offered to plugins (sapling_api's world_notice.rs): liquid
//! drying up, sounds told to viewers, entities going and worlds closing.

use std::borrow::Cow;

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::{Packet, RawPacket};
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use bytes::Bytes;
use sapling_api::{Block, BlockPos, EntityDespawnEvent, EntityId, LiquidDecayEvent, SoundEvent, WorldCloseEvent};

use crate::components::{InLayer, Layer, NetId};
use crate::entities::Kind;
use crate::plugins;
use crate::worlds::Named;

/// Entities gone since the last [`offer_gone`]: id, kind, world.
#[derive(Resource, Default)]
pub(crate) struct Gone(Vec<(u64, String, Option<Entity>)>);

fn liquid(name: &str, depth: Option<u8>) -> Block {
    depth.map_or_else(Block::air, |depth| Block::new(name).with(format!("liquid_depth={depth}")))
}

/// Whether plugins let the flowing `name` at `pos` go from `depth` to `now` (`None`: dry).
pub(crate) fn may_decay(world: &mut World, layer: Entity, pos: BlockPos, name: &str, depth: u8, now: Option<u8>) -> bool {
    !plugins::stopped_in(world, layer, |_| LiquidDecayEvent::new(pos, liquid(name, Some(depth)), liquid(name, now)))
}

/// `packets` less the sounds plugins silenced.
pub(crate) fn heard<'a>(world: &mut World, layer: Entity, packets: &'a [Bytes]) -> Cow<'a, [Bytes]> {
    if !plugins::listening::<SoundEvent>(world) {
        return Cow::Borrowed(packets);
    }
    let sound = |packet: &Bytes| RawPacket::parse(packet.clone()).ok().filter(|raw| raw.id == LevelSoundEvent::ID).and_then(|raw| raw.decode::<LevelSoundEvent>().ok());
    let mut kept = Vec::with_capacity(packets.len());
    for packet in packets {
        let silenced = sound(packet).is_some_and(|sound| {
            let at = [sound.position.x, sound.position.y, sound.position.z];
            plugins::stopped_in(world, layer, |_| SoundEvent::new(at, sound.sound_id))
        });
        if !silenced {
            kept.push(packet.clone());
        }
    }
    Cow::Owned(kept)
}

/// `Kind`'s remove hook: notes the entity for [`offer_gone`].
pub(crate) fn note_gone(mut world: DeferredWorld, context: HookContext) {
    let entity = world.entity(context.entity);
    let (Some(id), Some(kind)) = (entity.get::<NetId>(), entity.get::<Kind>()) else { return };
    let gone = (id.0, kind.0.clone(), entity.get::<InLayer>().map(|layer| layer.0));
    world.commands().queue(move |world: &mut World| world.get_resource_or_insert_with(Gone::default).0.push(gone));
}

/// Offers the entities gone since the last call, each in its world while that still is.
pub(crate) fn offer_gone(world: &mut World) {
    let Some(gone) = world.get_resource_mut::<Gone>().map(|mut gone| std::mem::take(&mut gone.0)) else { return };
    if !plugins::listening::<EntityDespawnEvent>(world) {
        return;
    }
    for (id, kind, layer) in gone {
        let event = EntityDespawnEvent::new(EntityId(id), kind);
        match layer.filter(|layer| world.get::<Layer>(*layer).is_some()) {
            Some(layer) => plugins::offer_in(world, layer, event),
            None => plugins::offer(world, event),
        };
    }
}

/// Offers that the world `layer` is closing.
pub(crate) fn closing(world: &mut World, layer: Entity) {
    if let Some(name) = world.get::<Named>(layer).map(|name| name.0.clone()).filter(|_| plugins::listening::<WorldCloseEvent>(world)) {
        plugins::offer_in(world, layer, WorldCloseEvent::new(name));
    }
}

/// Offers that every world of the shard is closing.
pub(crate) fn closing_all(world: &mut World) {
    let layers: Vec<Entity> = world.query_filtered::<Entity, (With<Layer>, With<Named>)>().iter(world).collect();
    layers.into_iter().for_each(|layer| closing(world, layer));
}
