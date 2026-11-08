//! Dolphins flocking (`minecraft:flocking`), as BDS's `FlockingSystem` (Windows 1446a0300, per leader
//! 1446a0cf0) and the `FollowFlockGoal` the component brings (Linux vtable 0xf2445b8; under gdb in
//! ab1flock it was built for each dolphin and ticked on three in four of their ticks). Each dolphin
//! draws once whether it is a loner (`loner_chance` 0.1) and its flock's size (`low_flock_limit` 4 to
//! `high_flock_limit` 8). One in no flock looks within `influence_radius` (6, a box) for a leader with
//! room and joins it, else starts a flock with a free dolphin it finds, which leads (141be2400). A
//! leader drops members gone out of that box and ends a flock left to itself (141be02a0), and each
//! tick works out each member's way (141be0f00): the leader's step less the member's own, plus
//! `separation_weight` 1.75 along the way from the members within `separation_threshold` 3, plus
//! `cohesion_weight` 1.85 towards the leader once over `innner_cohesion_threshold` 3.5 from it, plus
//! `goal_weight` 2 towards the leader's spot. A member swims along its way while its leader has a
//! spot (1430bd490); a leader swims about.
// Not ported: merging flocks (141be0900), block avoidance (`block_weight` 0 for dolphins), the doubled
// pace past `cohesion_threshold` 6.5 (sapling's dolphins swim at one pace) and how often a free one looks.

use bevy_ecs::prelude::*;

use super::Dolphin;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Tick};

const LONER: f32 = 0.1;
const LIMITS: [u32; 2] = [4, 8];
const INFLUENCE: f32 = 6.0;
/// Separation's weight and reach; cohesion's weight and the distance it starts at.
const SEPARATION: [f32; 2] = [1.75, 3.0];
const COHESION: [f32; 2] = [1.85, 3.5];
const GOAL: f32 = 2.0;
const LOOKS_EVERY: u64 = 20;
const STILL: f32 = 1e-4;

#[derive(Clone, Default)]
pub(super) struct Flock {
    drawn: bool,
    loner: bool,
    limit: usize,
    leader: Option<Entity>,
    /// A leader's members, itself first.
    members: Vec<Entity>,
    /// A member's way, worked out by its leader.
    way: Option<[f32; 3]>,
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let long = v.iter().map(|c| c * c).sum::<f32>().sqrt();
    if long < STILL { [0.0; 3] } else { v.map(|c| c / long) }
}

fn within(a: [f32; 3], b: [f32; 3], reach: f32) -> bool {
    (0..3).all(|axis| (a[axis] - b[axis]).abs() <= reach)
}

fn flock(world: &World, dolphin: Entity) -> Option<&Flock> {
    world.get::<Dolphin>(dolphin).map(|dolphin| &dolphin.flock)
}

fn edit(world: &mut World, dolphin: Entity, change: impl FnOnce(&mut Flock)) {
    if let Some(mut kept) = world.get_mut::<Dolphin>(dolphin) {
        change(&mut kept.flock);
    }
}

/// The way `dolphin` at `body` swims this tick as a member of a flock whose leader has a spot. A
/// free one looks for a flock; a leader keeps its flock and works out its members' ways.
pub(super) fn way(world: &mut World, dolphin: Entity, body: &Body) -> Option<[f32; 3]> {
    draw(world, dolphin);
    let kept = flock(world, dolphin)?.clone();
    match kept.leader {
        Some(leader) if leader == dolphin => {
            lead(world, dolphin, body);
            None
        }
        Some(leader) => world.get::<Dolphin>(leader).and_then(|leader| leader.spot).and(kept.way),
        None => {
            if !kept.loner && world.resource::<Tick>().0.is_multiple_of(LOOKS_EVERY) {
                join(world, dolphin, body);
            }
            None
        }
    }
}

fn draw(world: &mut World, dolphin: Entity) {
    if flock(world, dolphin).is_none_or(|kept| kept.drawn) {
        return;
    }
    let (loner, limit) = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (dice.fraction() < LONER, LIMITS[0] + dice.roll(LIMITS[1] - LIMITS[0] + 1))
    };
    edit(world, dolphin, |kept| (kept.drawn, kept.loner, kept.limit) = (true, loner, limit as usize));
}

/// The other dolphins of `dolphin`'s layer in the influence box about `at`.
fn near(world: &mut World, dolphin: Entity, at: [f32; 3]) -> Vec<Entity> {
    let layer = world.get::<InLayer>(dolphin).map(|on| on.0);
    world
        .query_filtered::<(Entity, &InLayer, &Body), With<Dolphin>>()
        .iter(world)
        .filter(|(other, on, body)| *other != dolphin && Some(on.0) == layer && within(body.at, at, INFLUENCE))
        .map(|(other, _, _)| other)
        .collect()
}

/// A free dolphin joins a leader near with room, else leads a new flock with a free one near.
fn join(world: &mut World, dolphin: Entity, body: &Body) {
    let others = near(world, dolphin, body.at);
    if let Some(leader) = others.iter().copied().find(|other| flock(world, *other).is_some_and(|kept| kept.leader == Some(*other) && kept.members.len() <= kept.limit)) {
        edit(world, leader, |kept| kept.members.push(dolphin));
        edit(world, dolphin, |kept| kept.leader = Some(leader));
        return;
    }
    let limit = flock(world, dolphin).map_or(0, |kept| kept.limit);
    if let Some(free) = others.iter().copied().find(|other| flock(world, *other).is_some_and(|kept| kept.leader.is_none() && !kept.loner)) {
        edit(world, free, |kept| (kept.leader, kept.members, kept.limit) = (Some(free), vec![free, dolphin], limit));
        edit(world, dolphin, |kept| kept.leader = Some(free));
    }
}

/// A leader at `body` drops members out of its box, ends a flock left to itself, and works out its
/// members' ways.
fn lead(world: &mut World, leader: Entity, body: &Body) {
    let members = flock(world, leader).map(|kept| kept.members.clone()).unwrap_or_default();
    let bodies: Vec<(Entity, Body)> = members.iter().filter_map(|member| Some((*member, *world.get::<Body>(*member)?))).collect();
    let (kept, gone): (Vec<_>, Vec<_>) = bodies.into_iter().partition(|(member, at)| *member == leader || within(at.at, body.at, INFLUENCE));
    for (member, _) in &gone {
        edit(world, *member, |flock| (flock.leader, flock.way) = (None, None));
    }
    if kept.len() < 2 {
        edit(world, leader, |flock| (flock.leader, flock.members, flock.way) = (None, Vec::new(), None));
        return;
    }
    edit(world, leader, |flock| flock.members = kept.iter().map(|(member, _)| *member).collect());
    let spot = world.get::<Dolphin>(leader).and_then(|dolphin| dolphin.spot);
    for (member, at) in kept.iter().filter(|(member, _)| *member != leader) {
        let way = steer(at, body, spot, &kept);
        edit(world, *member, |flock| flock.way = Some(way));
    }
}

/// A member at `at`'s way, by its `leader`, the leader's `spot` and the flock's bodies.
fn steer(at: &Body, leader: &Body, spot: Option<[f32; 3]>, flock: &[(Entity, Body)]) -> [f32; 3] {
    let close = |other: &[f32; 3]| *other != at.at && (0..3).map(|axis| (at.at[axis] - other[axis]).powi(2)).sum::<f32>() < SEPARATION[1] * SEPARATION[1];
    let apart = unit(flock.iter().map(|(_, other)| other.at).filter(close).fold([0.0; 3], |sum, other| std::array::from_fn(|axis| sum[axis] + at.at[axis] - other[axis])));
    let off: [f32; 3] = std::array::from_fn(|axis| leader.at[axis] - at.at[axis]);
    let together = if off.iter().map(|c| c * c).sum::<f32>().sqrt() > COHESION[1] { unit(off) } else { [0.0; 3] };
    let goal = spot.map_or([0.0; 3], |spot| unit(std::array::from_fn(|axis| spot[axis] - at.at[axis])));
    unit(std::array::from_fn(|axis| leader.motion[axis] - at.motion[axis] + apart[axis] * SEPARATION[0] + together[axis] * COHESION[0] + goal[axis] * GOAL))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_far_from_its_leader_heads_back_and_one_crowded_moves_apart() {
        let leader = Body::new([0.0, 60.0, 0.0], [0.0; 3], [0.9, 0.6]);
        let far = Body::new([5.0, 60.0, 0.0], [0.0; 3], [0.9, 0.6]);
        let flock = [(Entity::PLACEHOLDER, leader), (Entity::PLACEHOLDER, far)];
        assert!(steer(&far, &leader, None, &flock)[0] < -0.99, "back towards the leader");
        let crowded = Body::new([1.0, 60.0, 0.0], [0.0; 3], [0.9, 0.6]);
        let flock = [(Entity::PLACEHOLDER, leader), (Entity::PLACEHOLDER, crowded)];
        assert!(steer(&crowded, &leader, None, &flock)[0] > 0.99, "away from the leader 1 off");
        let way = steer(&crowded, &leader, Some([1.0, 60.0, 10.0]), &flock);
        assert!(way[2] > way[0] && way[0] > 0.0, "the leader's spot outweighs the crowding: {way:?}");
    }
}
