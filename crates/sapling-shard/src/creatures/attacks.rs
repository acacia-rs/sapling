//! A player's hit on a mob, as BDS answered one (docs/research/mob-damage.md, "Hits").
// TODO: no critical hit, enchantment or wear of the item; a hit mob neither flees nor
// fights back; a hit on an armour stand, which has no health here, tires nobody; plugins are not asked; `REACH` is the nearest whole number to what BDS allowed.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{EntityId, GameMode};

use super::Creature;
use super::harm;
use super::health::{Cause, Health};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Mode, PerTick, Position, in_world_identity};
use crate::inventory::Inventory;
use crate::entities;

/// Hits looked at per player per tick.
const MAX_HITS_PER_TICK: usize = 4;
/// How far from a player's eyes the nearest point of a mob it hits may be.
const REACH: f32 = 3.0;
/// The speed a hit throws a mob at, away from the hitter and up; what it moved at across counts half.
const THROW: f32 = 0.4;
const KEPT: f32 = 0.5;
/// A player's sounds come from the middle of its box.
const HALF_PLAYER: f32 = 0.9;

/// A mob a player killed, and the experience it leaves when it goes.
#[derive(Component)]
pub(super) struct Slain(pub i32);

/// This tick's hits, each a player and the entity it named.
pub(crate) fn attacked(world: &mut World, hits: Vec<(Entity, u64)>) {
    let mut seen = PerTick::new(MAX_HITS_PER_TICK);
    for (player, id) in hits {
        if seen.allow(player) && in_world_identity(world, player).is_some() {
            hit(world, player, id);
        }
    }
}

fn hit(world: &mut World, player: Entity, id: u64) -> Option<()> {
    let (layer, from) = (world.get::<InLayer>(player)?.0, world.get::<Position>(player)?.feet);
    let entity = entities::find(world, EntityId(id)).filter(|entity| world.get::<Creature>(*entity).is_some() && world.get::<super::Dying>(*entity).is_none())?;
    let (body, kind) = (*world.get::<Body>(entity)?, world.get::<entities::Kind>(entity)?.0.clone());
    let mut health = *world.get::<Health>(entity)?;
    let eye = [from[0], from[1] + sapling_data::EYE_HEIGHT, from[2]];
    if world.get::<InLayer>(entity)?.0 != layer || world.get::<Mode>(player)?.0 == GameMode::Spectator || distance(eye, &body) > REACH {
        return None;
    }
    let cell = body.at.map(|c| c.floor() as i32);
    let [x, y, z] = from;
    let swing = LevelSoundEvent {
        sound_id: "attack.strong".into(),
        position: Vec3f { x, y: y + HALF_PLAYER, z },
        extra_data: -1,
        entity_type: "minecraft:player".into(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: -1,
        fire_at_position: None,
    };
    let mut packets = vec![sapling_net::encode(&swing)];
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone()));
    if health.hurt(sapling_data::item_attack(held.as_deref())) {
        packets.insert(0, harm::strike(id, &health, Cause::Attack, None));
        let (dx, dz) = (body.at[0] - from[0], body.at[2] - from[2]);
        let far = (dx * dx + dz * dz).sqrt();
        let away = if far > 1e-4 { [dx / far, dz / far] } else { [0.0; 2] };
        let mut thrown = world.get_mut::<Body>(entity)?;
        let speed = [thrown.motion[0] * KEPT + away[0] * THROW, THROW, thrown.motion[2] * KEPT + away[1] * THROW];
        thrown.throw(speed);
        *world.get_mut::<Health>(entity)? = health;
        crate::vitals::tire(world, player, crate::vitals::HIT_TIRES);
    }
    entities::tell_viewers(world, entity, layer, cell, packets);
    if health.left() <= 0.0 {
        let is_baby = world.get::<Creature>(entity).is_some_and(Creature::is_baby);
        let points = sapling_data::mob_traits(&kind).experience.filter(|_| !is_baby).map_or(0, |(least, most)| {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            least + (dice.fraction() * (most - least) as f32) as i32
        });
        world.entity_mut(entity).insert(Slain(points));
        super::die(world, entity, layer, &kind, &body);
    }
    Some(())
}

/// From `eye` to the nearest point of the body's box.
fn distance(eye: [f32; 3], body: &Body) -> f32 {
    let (low, high) = body.bounds();
    (0..3).map(|axis| (eye[axis].clamp(low[axis], high[axis]) - eye[axis]).powi(2)).sum::<f32>().sqrt()
}

/// Lets out what a mob a player killed leaves as it goes: its experience, where it lies.
pub(super) fn leave_experience(world: &mut World, entity: Entity) {
    let (Some(points), Some(layer), Some(at)) = (world.get::<Slain>(entity).map(|slain| slain.0), world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).map(|body| body.at)) else { return };
    if points > 0 && super::leaves_loot(world, layer) {
        crate::orbs::spawn(world, layer, at, points);
    }
}
