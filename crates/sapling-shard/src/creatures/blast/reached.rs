//! What a blast does to the mobs, vehicles and primed TNT it reaches (bds-blasts.txt, bds-c1blast.txt).

use acacia_proto::packets::SetEntityMotion;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::{EYES, HARM, KNOCK, distance, seen};
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::creatures::harm;
use crate::creatures::health::{Cause, Health};
use crate::entities;

/// What a blast reaching `reach` costs a mob or vehicle it meets by `impact`.
pub(super) fn harm_by(impact: f32, reach: f32) -> f32 {
    (impact * impact + impact) / 2.0 * HARM * reach + 1.0
}

/// How hard a blast at `at` that reaches `reach` hits a mob at `feet`, `size` wide and high:
/// BDS measures to its feet and takes half its width off (`Explosion::explode`).
pub(in crate::creatures) fn impact(world: &World, layer: Entity, at: [f32; 3], feet: [f32; 3], size: [f32; 2], reach: f32) -> Option<f32> {
    let far = (distance(feet, at) - size[0] / 2.0).max(0.0);
    (far <= reach).then(|| (1.0 - far / reach) * seen(world, layer, at, feet, size))
}

/// The way from `from` to `to`, `long` long.
pub(in crate::creatures) fn towards(from: [f32; 3], to: [f32; 3], long: f32) -> [f32; 3] {
    let far = distance(from, to);
    if far < 1e-4 { [0.0; 3] } else { std::array::from_fn(|axis| (to[axis] - from[axis]) / far * long) }
}

/// A mob in a blast at `at`: hurt by at least 1 however hidden, thrown off as by a hit from the
/// blast and, unless no hit throws it, towards its eyes by the blast's impact (bds-blasts.txt).
pub(super) fn blast_mob(world: &mut World, layer: Entity, mob: Entity, id: u64, its: &Body, at: [f32; 3], reach: f32) {
    let Some(impact) = impact(world, layer, at, its.at, its.size, reach) else { return };
    let harm = harm_by(impact, reach);
    let hurt = crate::damage::ask(world, mob, harm, Cause::EntityExplosion, None).and_then(|harm| {
        let mut health = world.get::<Health>(mob).copied()?;
        // Resistance that takes all of it still hurts and throws (the drill's pigs had it).
        let resisted = !health.is_guarded() && health.left() > 0.0;
        (health.hurt(harm) || resisted).then(|| {
            *world.get_mut::<Health>(mob).expect("read above") = health;
            entities::tell_viewers(world, mob, layer, its.at.map(|c| c.floor() as i32), vec![harm::strike(id, &health, Cause::EntityExplosion, None)]);
        })
    });
    let kind = world.get::<entities::Kind>(mob).map(|kind| kind.0.clone()).unwrap_or_default();
    if sapling_data::mob_traits(&kind).steadfast {
        return;
    }
    let knock = match hurt {
        Some(()) => {
            let [x, _, z] = towards([at[0], 0.0, at[2]], [its.at[0], 0.0, its.at[2]], KNOCK);
            [x, KNOCK, z]
        }
        None => [0.0; 3],
    };
    let push = towards(at, [its.at[0], its.at[1] + its.size[1] * EYES, its.at[2]], impact);
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.throw(std::array::from_fn(|axis| knock[axis] + push[axis]));
    }
}

/// Primed TNT in a blast at `at` that reaches `reach` is pushed away by its impact.
pub(super) fn throw_primed(world: &mut World, layer: Entity, at: [f32; 3], reach: f32) {
    let primed: Vec<(Entity, Body)> = world.query_filtered::<(Entity, &InLayer, &Body), With<crate::tnt::Primed>>().iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).map(|(entity, _, body)| (entity, *body)).collect();
    for (entity, its) in primed {
        if let Some(impact) = impact(world, layer, at, its.at, its.size, reach) {
            let push = towards(at, its.at, impact);
            if let Some(mut body) = world.get_mut::<Body>(entity) {
                body.motion = std::array::from_fn(|axis| body.motion[axis] + push[axis]);
            }
        }
    }
}

/// Falling blocks in a blast at `at` that reaches `reach` are pushed by its impact towards their
/// middle, the new motion told at once (bds-o1blast.txt: a layer of 34 fit, the layer above as well
/// with its height 0.74 under its last told).
pub(super) fn throw_falling(world: &mut World, layer: Entity, at: [f32; 3], reach: f32) {
    let falling: Vec<(Entity, u64, Body)> = world
        .query_filtered::<(Entity, &InLayer, &NetId, &Body), With<crate::blocks::FallingBlock>>()
        .iter(world)
        .filter(|(_, in_layer, ..)| in_layer.0 == layer)
        .map(|(entity, _, id, body)| (entity, id.0, *body))
        .collect();
    for (entity, id, its) in falling {
        let Some(impact) = impact(world, layer, at, its.at, its.size, reach) else { continue };
        let push = towards(at, [its.at[0], its.at[1] + its.size[1] / 2.0, its.at[2]], impact);
        // BDS told, and next moved by, the pushed motion after a tick's gravity and drag.
        let falls = crate::body::Physics::FALLING_BLOCK;
        let motion: [f32; 3] = std::array::from_fn(|axis| (its.motion[axis] + push[axis] - if axis == 1 { falls.gravity } else { 0.0 }) * falls.air_drag);
        if let Some(mut body) = world.get_mut::<Body>(entity) {
            body.motion = motion;
        }
        let told = SetEntityMotion { runtime_entity_id: id, velocity: Vec3f { x: motion[0], y: motion[1], z: motion[2] }, tick: 0 };
        entities::tell_viewers(world, entity, layer, its.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
    }
}
