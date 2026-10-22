//! The fight's side of the shard's other systems: a mob's death, a hit, a blast, and the API.

use bevy_ecs::prelude::*;
use sapling_world::END;

use super::{Crystal, Dragon, Fight, burst, crystals, death};
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::creatures::Health;

/// How far `eye` is from the nearest of the dragon's parts, tail and wings included; `None` for
/// another mob.
pub(crate) fn part_reach(world: &World, dragon: Entity, eye: [f32; 3]) -> Option<f32> {
    let boxes = super::parts::boxes(&world.get::<Dragon>(dragon)?.flight);
    let far = |(low, high): &([f32; 3], [f32; 3])| (0..3).map(|axis| (eye[axis].clamp(low[axis], high[axis]) - eye[axis]).powi(2)).sum::<f32>().sqrt();
    boxes.iter().map(far).min_by(f32::total_cmp)
}

/// Tells the dragon's viewers its health.
fn tell_health(world: &mut World, dragon: Entity, health: &Health) {
    let (Some(id), Some(layer), Some(at)) = (world.get::<NetId>(dragon).map(|id| id.0), world.get::<InLayer>(dragon).map(|l| l.0), world.get::<Body>(dragon).map(|b| b.at)) else { return };
    let told = sapling_net::encode(&sapling_data::mob_health_update(id, health.left(), health.max()));
    crate::entities::tell_viewers(world, dragon, layer, at.map(|c| c.floor() as i32), vec![told]);
}

/// The dragon's last flight ended at the portal: its death is shown and begins.
pub(super) fn killed_at_portal(world: &mut World, dragon: Entity) {
    super::fly::ended(world, dragon);
    dies(world, dragon);
}

/// The fight's dragon, hurt to death, begins to die; false for any other mob.
pub(crate) fn dies(world: &mut World, dragon: Entity) -> bool {
    let Some(layer) = world.get::<InLayer>(dragon).map(|in_layer| in_layer.0).filter(|_| world.get::<Dragon>(dragon).is_some()) else { return false };
    let first = !world.get::<Fight>(layer).is_some_and(|fight| fight.record.previously_killed);
    death::begin(world, dragon, first);
    let id = world.get::<NetId>(dragon).map_or(0, |id| id.0);
    crate::plugins::offer_in(world, layer, sapling_api::DragonKilledEvent::new(sapling_api::EntityId(id), first));
    true
}

/// What a hit of `harm` on the dragon's body takes (`hurtPart` 145997370), as BDS took a fist's 1
/// as 1.25 (research/captures/bds-dragon-hits.txt).
pub(crate) fn body_harm(harm: f32) -> f32 {
    harm.min(1.0) + harm * 0.25
}

/// A hit landed on the dragon, which had `before`: a hit to death leaves it 1 and sends it to the
/// portal to die; one sitting takes off once it has lost a quarter of its health since it sat.
pub(crate) fn hurt(world: &mut World, dragon: Entity, before: f32, health: &mut Health) {
    let Some(mut kept) = world.get_mut::<Dragon>(dragon) else { return };
    let flight = &mut kept.flight;
    if health.left() < 1.0 {
        health.mend(1.0 - health.left());
        if flight.group != super::flight::Group::Dying {
            super::flight::begin_dying(flight);
        }
        return;
    }
    if flight.sitting {
        flight.sat_harm += before - health.left();
        if flight.sat_harm > health.max() * 0.25 {
            flight.sat_harm = 0.0;
            super::flight::take_off(flight);
        }
    }
}

/// Whether a blast hurts this mob: the dragon only a block's (`hurtPart`'s cause test).
pub(crate) fn takes_blast(world: &World, mob: Entity, by_entity: bool) -> bool {
    !(by_entity && world.get::<Dragon>(mob).is_some())
}

/// A crystal blew up (`onCrystalDestroyed` 145998d00): the one it healed from costs its head 10,
/// and out of a sit it takes off after whoever broke it, else the nearest player within 64.
pub(super) fn crystal_destroyed(world: &mut World, layer: Entity, dragon: Entity, crystal: Entity, at: [f32; 3]) {
    let healing = world.get::<Dragon>(dragon).is_some_and(|kept| kept.flight.healer == Some(crystal));
    if healing {
        if let Some(mut health) = world.get::<Health>(dragon).copied() {
            let before = health.left();
            if health.hurt(10.0) {
                hurt(world, dragon, before, &mut health);
                *world.get_mut::<Health>(dragon).expect("read above") = health;
                tell_health(world, dragon, &health);
            }
        }
    }
    let attacker = nearest_player(world, layer, at, 64.0);
    if let Some(mut kept) = world.get_mut::<Dragon>(dragon).filter(|kept| !kept.flight.sitting) {
        kept.flight.prey = attacker;
        super::flight::take_off(&mut kept.flight);
    }
}

fn nearest_player(world: &World, layer: Entity, at: [f32; 3], reach: f32) -> Option<Entity> {
    let far = |feet: [f32; 3]| (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>();
    crate::components::in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer))
        .filter_map(|player| Some((player, far(world.get::<crate::components::Position>(player)?.feet))))
        .filter(|(_, d)| *d <= reach * reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(player, _)| player)
}

/// A blast of `power` at `at` reached the crystals near it: each blows up in turn.
pub(crate) fn blasted(world: &mut World, layer: Entity, at: [f32; 3], power: f32) {
    let reach = 2.0 * power;
    let near: Vec<Entity> = world
        .query_filtered::<(Entity, &InLayer, &Body), With<Crystal>>()
        .iter(world)
        .filter(|(_, in_layer, body)| in_layer.0 == layer && (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>().sqrt() < reach)
        .map(|(crystal, ..)| crystal)
        .collect();
    for crystal in near {
        if world.get::<Crystal>(crystal).is_some() {
            burst(world, crystal);
        }
    }
}

pub(crate) fn is_dying(world: &World, dragon: Entity) -> bool {
    death::is_dying(world, dragon)
}

/// The fight in `layer`'s realm's End, for the API; `None` before one began.
pub(crate) fn describe(world: &World, layer: Entity) -> Option<sapling_api::DragonFight> {
    let end = crate::dimensions::of(world, layer, END)?;
    let fight = world.get::<Fight>(end).filter(|fight| fight.record.spawned)?;
    let dragon = fight.dragon.filter(|dragon| world.get::<Dragon>(*dragon).is_some() && !death::is_dying(world, *dragon));
    let id = dragon.and_then(|dragon| world.get::<NetId>(dragon)).map(|id| sapling_api::EntityId(id.0));
    let health = dragon.and_then(|dragon| world.get::<Health>(dragon)).map(Health::left);
    let spikes = super::spikes(world, end);
    let on_spikes = |entity: &bevy_ecs::world::EntityRef<'_>| entity.contains::<Crystal>() && entity.get::<InLayer>().is_some_and(|in_layer| in_layer.0 == end) && entity.get::<Body>().is_some_and(|body| crystals::on_spike(&spikes, body.at));
    let crystals = world.iter_entities().filter(on_spikes).count();
    Some(sapling_api::DragonFight::new(id, health, crystals as u32, fight.record.previously_killed))
}
