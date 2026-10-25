//! A rocket's burst, as BDS's `postNormalTick`: its viewers are told to show its stars
//! (`FireworkParticles`); one with n stars hurts every player and mob within 5 that it sees
//! (5 + 2n) times the root of its share of the way in, by `Fireworks`.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;

use super::{Rocket, item};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, in_world};
use crate::creatures::{Cause, Health};
use crate::{entities, mobs, vitals};

const HARM: f32 = 5.0;
const HARM_A_STAR: f32 = 2.0;
const REACH: f32 = 5.0;
const PLAYER_MIDDLE: f32 = 0.9;

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// What a rocket of `stars` costs one `far` from it.
fn harm_of(stars: usize, far: f32) -> f32 {
    (HARM + HARM_A_STAR * stars as f32) * ((REACH - far) / REACH).max(0.0).sqrt()
}

/// The middles of the players and mobs of `layer`, players first.
fn middles(world: &mut World, layer: Entity) -> Vec<(Entity, [f32; 3], bool)> {
    let players = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer)).filter_map(|player| world.get::<Position>(player).map(|at| (player, [at.feet[0], at.feet[1] + PLAYER_MIDDLE, at.feet[2]], true)));
    let mut all: Vec<(Entity, [f32; 3], bool)> = players.collect();
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), With<Health>>();
    all.extend(mobs.iter(world).filter(|(_, on, _)| on.0 == layer).map(|(mob, _, body)| (mob, [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]], false)));
    all
}

/// The rocket bursts and goes.
pub(super) fn burst(world: &mut World, rocket: Entity, layer: Entity) -> Option<()> {
    let (id, at) = (world.get::<NetId>(rocket)?.0, world.get::<Body>(rocket)?.at);
    let stars = world.get::<Rocket>(rocket).map(|kept| item::explosions(&kept.item).len())?;
    let show = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::FireworkParticles, data: 0, fire_at_position: None };
    entities::tell_viewers(world, rocket, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&show)]);
    if stars > 0 {
        let near: Vec<(Entity, f32, bool)> = middles(world, layer)
            .into_iter()
            .filter(|(_, middle, _)| distance(*middle, at) <= REACH && world.get::<Layer>(layer).is_some_and(|level| crate::creatures::sees(&level.world, at, *middle)))
            .map(|(victim, middle, is_player)| (victim, distance(middle, at), is_player))
            .collect();
        for (victim, far, is_player) in near {
            let harm = harm_of(stars, far);
            match is_player {
                true => drop(vitals::fireworked(world, victim, harm, at)),
                false => drop(crate::creatures::wound(world, victim, layer, harm, Cause::Fireworks)),
            }
        }
    }
    mobs::remove(world, rocket);
    None
}

#[cfg(test)]
mod tests {
    use super::harm_of;

    #[test]
    fn a_rocket_hurts_by_its_stars_and_the_root_of_the_share_of_the_way_in() {
        assert_eq!(harm_of(1, 0.0), 7.0);
        assert_eq!(harm_of(3, 0.0), 11.0);
        assert!((harm_of(2, 3.75) - 4.5).abs() < 1e-5);
        assert_eq!(harm_of(1, 5.0), 0.0);
    }
}
