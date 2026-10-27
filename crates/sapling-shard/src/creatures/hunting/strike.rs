//! A hunter's hit: within reach of its prey it swings and hits it, heard a tick later; one that
//! shoots looses its shots (shooting.rs), and one that blows up lights its fuse (blast.rs).

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;

use super::super::{blast, shooting};
use super::{COOLDOWN, FLINGS, GOES_OUT, Hunter, IGNITES, MIDDLE, PLAYER, REACH, THROW, distance, on_difficulty, sound, take};
use crate::body::Body;
use crate::components::{Position, Tick};
use crate::{entities, late_sounds, vitals};

/// The kinds of fight whose hit is a bite within reach; the others hit their own way.
const BITES: &[&str] = &["swims", "flies"];

/// Its viewers see the mob swing its arm.
pub(in crate::creatures) fn swing(world: &mut World, mob: Entity, layer: Entity, id: u64, body: &Body) {
    let swing = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArmSwing, data: 0, fire_at_position: None };
    entities::tell_viewers(world, mob, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&swing)]);
}

/// After its move: a hunter whose reach its prey is in swings and hits it, heard a tick later.
pub(in crate::creatures) fn strike(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let hunts = sapling_data::mob_traits(kind).hunts.filter(|hunts| hunts.fights.is_none_or(|fights| BITES.contains(&fights)))?;
    let hunts = super::super::genes::hunts(world, entity, hunts);
    let harm = match hunts.upto {
        Some(most) => hunts.attack + world.get_resource_or_insert_with(crate::blocks::Dice::default).roll((most - hunts.attack) as u32 + 1) as f32,
        None => hunts.attack,
    };
    let (prey, ready) = world.get::<Hunter>(entity).map(|hunter| (hunter.prey, hunter.ready))?;
    if prey.is_none() && hunts.explodes.is_some() {
        blast::put_out(world, entity, layer, id, kind, body);
    }
    if let Some(every) = hunts.shoots.filter(|_| prey.is_some()) {
        return shooting::shoot(world, entity, layer, kind, body, hunts, every, prey?);
    }
    let prey = prey.filter(|_| now >= ready)?;
    let feet = world.get::<Position>(prey)?.feet;
    let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
    if hunts.explodes.is_some() {
        // Between their boxes, as the pack's `target_nearby_sensor` counts.
        match distance(feet, body.at) - (body.size[0] + PLAYER[0]) / 2.0 {
            gap if gap < IGNITES => blast::light(world, entity, layer, id, kind, body),
            gap if gap > GOES_OUT => blast::put_out(world, entity, layer, id, kind, body),
            _ => {}
        }
        return None;
    }
    let across = (body.size[0] + PLAYER[0]) / 2.0 + REACH;
    if dx.abs() >= across || dz.abs() >= across || feet[1] >= body.at[1] + body.size[1] || body.at[1] >= feet[1] + PLAYER[1] {
        return None;
    }
    world.get_mut::<Hunter>(entity)?.ready = now + hunts.every.map_or(COOLDOWN, u64::from);
    let harm = harm + hunts.varies.map_or(0.0, |most| world.get_resource_or_insert_with(crate::blocks::Dice::default).fraction() * (most - hunts.attack));
    let harm = on_difficulty(world, harm)?;
    let cell = body.at.map(|c| c.floor() as i32);
    swing(world, entity, layer, id, body);
    let far = dx.hypot(dz);
    let away = if far > 1e-4 { [dx / far, dz / far] } else { [0.0; 2] };
    let [along, up] = if hunts.upto.is_some() { FLINGS } else { [THROW; 2] };
    let bitten = vitals::bitten(world, prey, harm, kind, [away[0] * along, up, away[1] * along]);
    if let Some((effect, ticks)) = hunts.gives.filter(|_| bitten) {
        vitals::afflict(world, prey, effect, ticks);
    }
    if bitten {
        crate::enchants::thorns(world, prey, entity);
        super::super::owner_hurt(world, prey, entity);
    }
    if hunts.once {
        take(world, entity, layer, id, kind, body, None);
    }
    super::super::bees::stung(world, entity, prey, bitten);
    late_sounds::play_later(world, 1, layer, cell, sound("attack", id, kind, body, MIDDLE));
    Some(())
}
