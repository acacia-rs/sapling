//! A mob's projectile struck, by a player's hit (attacks.rs) or a shot that meets it (hits.rs), one
//! path for both: what its pack's `reflect_on_hurt` turns (a ghast's fireball, a breeze's charge, a blue
//! wither skull) flies off along the striker's look at 1 a tick, moved by that in the same tick, the
//! striker its owner (research/captures/bds-d3ghast.txt: the fireball's motion was the actor's unit
//! look, `OwnerEid` the actor's); what `destroy_on_hurt` breaks (a shulker's bullet) is gone. The
//! dragon's fireball turns by its own module (dragon/fireball.rs).

use acacia_proto::packets::{LevelEvent, LevelEventEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, NetId, Position};
use crate::dragon::cloud::item;
use crate::entities;

const CRITICAL: i32 = 15;

/// What a strike does to a mob's projectile.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum Strikable {
    Turns,
    Breaks,
}

/// A projectile turned this tick by the player or mob it names, whom its own tick takes for its owner.
#[derive(Component)]
pub(super) struct Turned(pub Entity);

/// A projectile broken this tick, which its own tick takes away.
#[derive(Component)]
pub(super) struct Broken;

/// Whether a strike does anything to `shot`.
pub(super) fn is_strikable(world: &World, shot: Entity) -> bool {
    world.get::<Strikable>(shot).is_some() || world.get::<crate::dragon::Fireball>(shot).is_some()
}

/// The way `by` looks, as a unit: a player's eyes, else `motion`'s way.
pub(crate) fn look_of(world: &World, by: Entity, motion: [f32; 3]) -> [f32; 3] {
    let along = match world.get::<Position>(by) {
        Some(position) => {
            let (yaw, pitch) = (position.yaw.to_radians(), position.pitch.to_radians());
            [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()]
        }
        None => motion,
    };
    let long = along.iter().map(|c| c * c).sum::<f32>().sqrt();
    if long < 1e-4 { [0.0; 3] } else { along.map(|c| c / long) }
}

/// `by` struck `shot`, which flew at `motion` if `by` shot it: whether it was one a strike turns or breaks.
pub(super) fn strike(world: &mut World, shot: Entity, by: Entity, motion: [f32; 3]) -> bool {
    if crate::dragon::reflect_fireball(world, shot, by, motion) {
        return true;
    }
    match world.get::<Strikable>(shot).copied() {
        None => false,
        Some(Strikable::Breaks) => {
            world.entity_mut(shot).insert(Broken);
            // Its pack's `crit_particle_on_hurt`, at once; it went the tick after (bds-d3shulker.txt).
            if let (Some(layer), Some(at)) = (world.get::<InLayer>(shot).map(|in_layer| in_layer.0), world.get::<Body>(shot).map(|body| body.at)) {
                let [x, y, z] = at;
                let sparks = LevelEvent { event: LevelEventEvent::ParticleCritical, position: Vec3f { x, y, z }, data: CRITICAL };
                entities::tell_viewers(world, shot, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&sparks)]);
            }
            true
        }
        Some(Strikable::Turns) => {
            let way = look_of(world, by, motion);
            if let Some(mut body) = world.get_mut::<Body>(shot) {
                body.motion = way;
            }
            world.entity_mut(shot).insert(Turned(by));
            tell_owner(world, shot, by);
            true
        }
    }
}

/// Viewers of `shot` are told `owner` owns it now.
pub(crate) fn tell_owner(world: &mut World, shot: Entity, owner: Entity) {
    let (Some(id), Some(owner)) = (world.get::<NetId>(shot).map(|id| id.0), world.get::<NetId>(owner).map(|id| id.0 as i64)) else { return };
    let told = SetEntityData { runtime_entity_id: id, metadata: vec![item(Key::OwnerEid, Plain::Long(owner))], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    if let (Some(layer), Some(body)) = (world.get::<InLayer>(shot).map(|in_layer| in_layer.0), world.get::<Body>(shot).copied()) {
        entities::tell_viewers(world, shot, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
    }
}

/// The new owner of a projectile turned this tick, taken off it.
pub(super) fn turned(world: &mut World, shot: Entity) -> Option<Entity> {
    world.entity_mut(shot).take::<Turned>().map(|turned| turned.0)
}
