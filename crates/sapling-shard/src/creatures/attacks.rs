//! A player's hit on a mob, as BDS answered one (docs/research/mob-damage.md, "Hits"), with
//! what the weapon's enchantments add (enchanted.rs) and a falling hitter's critical (blows.rs).
// TODO: a hit mob does not fight back; a hit on an armour stand (armour_stands/hits.rs) tires nobody.

use bevy_ecs::prelude::*;
use sapling_api::{EntityId, GameMode};
use sapling_data::enchant::{self, Enchant};

use super::Creature;
use super::{blows, harm, smash};
use super::health::{Cause, Health};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Mode, PerTick, Position, in_world_identity};
use crate::inventory::Inventory;
use crate::entities;

/// Hits looked at per player per tick.
const MAX_HITS_PER_TICK: usize = 4;
/// How far from a player's eyes the position (feet) of a mob it hits may be, as BDS measures
/// (docs/research/reach.md).
const REACH: f32 = 6.7;
const CREATIVE_REACH: f32 = 12.0;

/// A mob a player killed, the experience it leaves when it goes, and the Looting it was killed with.
#[derive(Component)]
pub(super) struct Slain(pub i32, pub u8);

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
    if super::duels::hit(world, player, id).is_some() {
        return Some(());
    }
    let (layer, from) = (world.get::<InLayer>(player)?.0, world.get::<Position>(player)?.feet);
    let eye = [from[0], from[1] + sapling_data::EYE_HEIGHT, from[2]];
    let reach = if world.get::<Mode>(player)?.0 == GameMode::Creative { CREATIVE_REACH } else { REACH };
    if entities::find(world, EntityId(id)).is_some_and(|knot| super::husbandry::undo_knot(world, knot)) {
        return Some(());
    }
    if let Some(crystal) = entities::find(world, EntityId(id)).filter(|entity| world.get::<crate::dragon::Crystal>(*entity).is_some()) {
        if world.get::<Body>(crystal).is_some_and(|body| distance(eye, body) <= reach) {
            crate::dragon::burst(world, crystal);
        }
        return Some(());
    }
    if let Some(vehicle) = entities::find(world, EntityId(id)).filter(|entity| world.get::<crate::vehicles::Vehicle>(*entity).is_some()) {
        if world.get::<Body>(vehicle).is_some_and(|body| distance(eye, body) <= reach) &&world.get::<Mode>(player)?.0 != GameMode::Spectator {
            let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone()));
            let damage = crate::vitals::hits_for(world, player, sapling_data::item_attack(held.as_deref()));
            crate::vehicles::hit(world, player, vehicle, damage);
        }
        return Some(());
    }
    let entity = entities::find(world, EntityId(id)).filter(|entity| world.get::<Creature>(*entity).is_some() && world.get::<super::Dying>(*entity).is_none() && !crate::dragon::is_dying(world, *entity))?;
    let (body, kind) = (*world.get::<Body>(entity)?, world.get::<entities::Kind>(entity)?.0.clone());
    let far = crate::dragon::part_reach(world, entity, eye).map_or(distance(eye, &body), |part| part.min(distance(eye, &body)));
    if world.get::<InLayer>(entity)?.0 != layer || world.get::<Mode>(player)?.0 == GameMode::Spectator || far > reach {
        return None;
    }
    let cell = body.at.map(|c| c.floor() as i32);
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone()));
    let weapon = crate::enchants::weapon(world, player);
    let smash = smash::smash(world, player, held.as_deref());
    // The enchantments' bonus goes in before strength and weakness (BDS 1429e56c0).
    let bonus = enchant::melee_bonus(weapon.as_ref(), Some(kind.as_str())) + smash::bonus(smash, weapon.as_ref());
    let blow = blows::blow(world, player, crate::vitals::hits_for(world, player, sapling_data::item_attack(held.as_deref()) + bonus));
    let mut packets = vec![sapling_net::encode(&blows::sound(blow, from))];
    if super::armour_stands::struck(world, entity, &kind, world.get::<Mode>(player)?.0 == GameMode::Creative, packets.clone()) || super::creaking::shrugs_off(world, entity) {
        return Some(());
    }
    let mut health = *world.get::<Health>(entity)?;
    let harm =if kind == crate::dragon::DRAGON { crate::dragon::body_harm(blow.harm) } else { blow.harm };
    let hitter = in_world_identity(world, player).map(sapling_api::Attacker::Player);
    let before = health.left();
    if crate::damage::ask(world, entity, harm, Cause::Attack, hitter).is_some_and(|harm| health.hurt(harm)) {
        if kind == crate::dragon::DRAGON {
            crate::dragon::hurt(world, entity, before, &mut health);
        }
        let flashed = if smash.is_some() { smash::MACE_SMASH } else { Cause::Attack.id() };
        packets.insert(0, harm::strike_as(id, &health, flashed, None));
        if let Some(sparks) = blows::sparks(blow, id) {
            packets.insert(1, sapling_net::encode(&sparks));
        }
        if let Some(smash) = smash {
            let at = packets.len() - 1;
            packets.splice(at..at, smash::shown(smash, from, body.at));
        }
        let (dx, dz) = (body.at[0] - from[0], body.at[2] - from[2]);
        let far = (dx * dx + dz * dz).sqrt();
        let away = if far > 1e-4 { [dx / far, dz / far] } else { [0.0; 2] };
        let mut thrown = world.get_mut::<Body>(entity)?;
        let speed = enchant::knockback(enchant::level(weapon.as_ref(), Enchant::Knockback), blow.sprinting, thrown.motion, away);
        // Knockback resistance takes its share off the throw (research/captures/bds-knockback.txt).
        let kept = 1.0 - sapling_data::mob_traits(&kind).resists;
        if kept > 0.0 {
            thrown.throw(speed.map(|c| c * kept));
        }
        *world.get_mut::<Health>(entity)? = health;
        if smash.is_some() {
            smash::follow(world, player, entity, body.at, weapon.as_ref());
        }
        super::enchanted::struck(world, entity, &kind, weapon.as_ref());
        crate::vitals::tire(world, player, crate::vitals::HIT_TIRES);
        super::strolling::startle(world, entity);
        super::hunting::provoke(world, entity, layer, id, &kind, &body, player);
        super::teleports::startle(world, entity);
        super::owner_hit(world, player, entity);
        super::villagers::hurt_standing(world, player, entity, layer, &kind, health.left() <= 0.0);
    }
    entities::tell_viewers(world, entity, layer, cell, packets);
    blows::wear(world, player);
    if health.left() <= 0.0 {
        slay(world, entity, &kind, enchant::level(weapon.as_ref(), Enchant::Looting));
        super::die(world, entity, layer, &kind, &body);
    }
    Some(())
}

/// The mob of `kind` was killed by a player with Looting at `looting`: it is [`Slain`], for the
/// experience its kind leaves.
pub(super) fn slay(world: &mut World, entity: Entity, kind: &str, looting: u8) {
    let points = experience_reward(world, entity, kind);
    world.entity_mut(entity).insert(Slain(points, looting));
}

/// The experience the mob of `kind` is worth, rolled: none for a baby.
pub(super) fn experience_reward(world: &mut World, entity: Entity, kind: &str) -> i32 {
    let is_baby = world.get::<super::Raised>(entity).is_some_and(super::Raised::is_baby);
    super::slimes::experience(world, entity, kind).or(sapling_data::mob_traits(kind).experience).filter(|_| !is_baby).map_or(0, |(least, most)| {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        least + (dice.fraction() * (most - least) as f32) as i32
    })
}

/// From `eye` to the body's position (its feet).
fn distance(eye: [f32; 3], body: &Body) -> f32 {
    (0..3).map(|axis| (body.at[axis] - eye[axis]).powi(2)).sum::<f32>().sqrt()
}

/// Lets out what a mob a player killed leaves as it goes: its experience, where it lies.
pub(super) fn leave_experience(world: &mut World, entity: Entity) {
    let (Some(points), Some(layer), Some(at)) = (world.get::<Slain>(entity).map(|slain| slain.0), world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).map(|body| body.at)) else { return };
    if points > 0 && super::leaves_loot(world, layer) {
        crate::orbs::spawn(world, layer, at, points);
    }
}
