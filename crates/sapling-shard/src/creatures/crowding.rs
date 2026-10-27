//! Mobs pushing each other apart, as BDS's cows did in the `crowd` drill
//! (research/captures/bds-wander-findings.md, "Crowds"): as a tick starts, every two mobs whose
//! boxes meet push each other apart across by the Java game's rule, from where they stand, and the
//! push joins their next move after the drag (`Body::jostle`). Two within 0.01 of one spot push
//! not at all, so mobs summoned onto one spot stay together until one walks.
// TODO: players neither push mobs nor are pushed by them; kinds the packs make unpushable
// (`minecraft:pushable`) are pushed all the same.

use std::collections::HashMap;

use bevy_ecs::prelude::*;

use super::{Creature, Dying};
use crate::body::Body;
use crate::components::InLayer;
use crate::entities;

/// Each of a pair pushes the other 0.05 by the Java game's rule and is pushed back as much, which
/// BDS's cows were moved by twice over.
const SHOVE: f32 = 0.1;
const TOGETHER: f32 = 0.01;
/// Boxes this much wider meet: BDS's cows 0.995 apart pushed, 0.9 wide, and 1.561 apart did not.
const GRACE: f32 = 0.2;
/// Mobs are sorted into squares this wide, as wide as the widest mob that walks.
const SQUARE: f32 = 4.0;

/// The push across a mob at `from` takes from one at `to`, where their boxes meet.
fn push(from: &Body, to: &Body) -> Option<[f32; 2]> {
    let (dx, dz) = (to.at[0] - from.at[0], to.at[2] - from.at[2]);
    let reach = (from.size[0] + to.size[0]) / 2.0 + GRACE;
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
        .filter(|(.., kind)| !super::physics_of(&kind.0).passes && kind.0 != crate::dragon::DRAGON)
        .map(|(entity, layer, body, _)| (entity, layer.0, *body))
        .collect();
    let bodies: Vec<_> = bodies.into_iter().filter(|(entity, ..)| !crate::riding::is_driven(world, *entity)).collect();
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
            if let Some([px, pz]) = push(body, other_body) {
                pushes.extend([(*entity, [px, pz]), (*other_entity, [-px, -pz])]);
            }
        }
    }
    for (entity, by) in pushes {
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
        let [px, _] = push(&cow(4.625), &cow(4.5)).unwrap();
        assert!((2.0 * px - 0.0707).abs() < 1e-4, "{px}");
        assert_eq!(push(&cow(4.5), &cow(4.505)), None, "on one spot");
        assert!(push(&cow(4.5), &cow(5.495)).is_some() && push(&cow(4.5), &cow(6.061)).is_none(), "0.995 apart and 1.561");
    }
}
