//! Mobs pushing each other apart, as BDS's cows did in the `crowd` drill
//! (research/captures/bds-wander-findings.md, "Crowds"): as a tick starts, every two mobs whose
//! boxes meet push each other apart across by the Java game's rule, from where they stand, and the
//! push joins their next move after the drag (`Body::jostle`). Two within 0.01 of one spot push
//! not at all, so mobs summoned onto one spot stay together until one walks. A player pushes mobs
//! as a mob does and is not pushed (its client moves it), and kinds the packs make unpushable
//! neither push nor are pushed: BDS's cow beside an armour stand or a shulker stood still
//! (research/captures/bds-jostle.txt).

use std::collections::HashMap;

use bevy_ecs::prelude::*;

use sapling_api::GameMode;

use super::{Creature, Dying};
use crate::body::Body;
use crate::components::{InLayer, Mode, Position, in_world};
use crate::entities;

/// Each of a pair pushes the other 0.05 by the Java game's rule and is pushed back as much, which
/// BDS's cows were moved by twice over.
const SHOVE: f32 = 0.1;
const TOGETHER: f32 = 0.01;
/// Two mobs' boxes this much wider meet: BDS's cows 0.995 apart pushed, 0.9 wide, and 1.561 apart
/// did not. A player's box and a mob's must overlap, the Java game's `pushEntities` rule.
// TODO: the player's gap is not drilled; any wider than a slime's touch (0.15, slimes.rs) and a
// slime could never reach a player who stands still.
const GRACE: f32 = 0.2;
const PLAYER_GRACE: f32 = 0.0;
/// Mobs are sorted into squares this wide, as wide as the widest mob that walks.
const SQUARE: f32 = 4.0;
const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];

/// The push across a mob at `from` takes from one at `to`, where their boxes meet.
fn push(from: &Body, to: &Body, grace: f32) -> Option<[f32; 2]> {
    let (dx, dz) = (to.at[0] - from.at[0], to.at[2] - from.at[2]);
    let reach = (from.size[0] + to.size[0]) / 2.0 + grace;
    let meet = dx.abs() < reach && dz.abs() < reach && from.at[1] < to.at[1] + to.size[1] && to.at[1] < from.at[1] + from.size[1];
    let apart = dx.abs().max(dz.abs());
    if !meet || apart < TOGETHER {
        return None;
    }
    let root = apart.sqrt();
    let scale = SHOVE * (1.0 / root).min(1.0) / root;
    Some([-dx * scale, -dz * scale])
}

/// Pushes apart every two mobs that crowd each other, before any of them moves.
pub(super) fn tick(world: &mut World) {
    // Not a rider or what is driven, which their seats and drivers move, nor a vex or the dragon.
    let bodies: Vec<(Entity, Entity, Body)> = world
        .query_filtered::<(Entity, &InLayer, &Body, &entities::Kind), (With<Creature>, Without<Dying>, Without<crate::riding::Riding>)>()
        .iter(world)
        .filter(|(.., kind)| !super::physics_of(&kind.0).passes && kind.0 != crate::dragon::DRAGON && !sapling_data::mob_traits(&kind.0).unpushable)
        .map(|(entity, layer, body, _)| (entity, layer.0, *body))
        .collect();
    let mut bodies: Vec<_> = bodies.into_iter().filter(|(entity, ..)| !crate::riding::is_driven(world, *entity)).collect();
    let mobs = bodies.len();
    bodies.extend(in_world(world).into_iter().filter_map(|player| {
        if world.get::<Mode>(player)?.0 == GameMode::Spectator || world.get::<crate::riding::Riding>(player).is_some() {
            return None;
        }
        Some((player, world.get::<InLayer>(player)?.0, Body::new(world.get::<Position>(player)?.feet, [0.0; 3], PLAYER_SIZE)))
    }));
    let square = |body: &Body| [(body.at[0] / SQUARE).floor() as i32, (body.at[2] / SQUARE).floor() as i32];
    let mut squares: HashMap<(Entity, [i32; 2]), Vec<usize>> = HashMap::new();
    for (index, (_, layer, body)) in bodies.iter().enumerate() {
        squares.entry((*layer, square(body))).or_default().push(index);
    }
    let mut pushes: Vec<(Entity, [f32; 2])> = Vec::new();
    for (index, (entity, layer, body)) in bodies.iter().enumerate() {
        let [x, z] = square(body);
        let near = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| [x + dx, z + dz])).filter_map(|at| squares.get(&(*layer, at))).flatten();
        for &other in near.filter(|other| **other > index) {
            let (other_entity, _, other_body) = &bodies[other];
            let grace = if other >= mobs { PLAYER_GRACE } else { GRACE };
            if let Some([px, pz]) = push(body, other_body, grace) {
                pushes.extend([(*entity, [px, pz]), (*other_entity, [-px, -pz])]);
            }
        }
    }
    let players: Vec<Entity> = bodies[mobs..].iter().map(|(player, ..)| *player).collect();
    for (entity, by) in pushes.into_iter().filter(|(entity, _)| !players.contains(entity)) {
        if let Some(mut body) = world.get_mut::<Body>(entity) {
            body.jostle(by);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crowding_cows_push_as_bdss_did() {
        // bds-crowd.txt: a cow 0.125 off two others on one spot was pushed 0.0707 more than its walk.
        let cow = |x: f32| Body::new([x, 0.0, 0.5], [0.0; 3], [0.9, 1.3]);
        let [px, _] = push(&cow(4.625), &cow(4.5), GRACE).unwrap();
        assert!((2.0 * px - 0.0707).abs() < 1e-4, "{px}");
        assert_eq!(push(&cow(4.5), &cow(4.505), GRACE), None, "on one spot");
        assert!(push(&cow(4.5), &cow(5.495), GRACE).is_some() && push(&cow(4.5), &cow(6.061), GRACE).is_none(), "0.995 apart and 1.561");
    }
}
