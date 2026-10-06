//! Experience orbs, as BDS's `ExperienceOrb` makes and moves them (docs/research/experience.md,
//! sections 1 to 3). The server moves them: the client only shows where they are. An orb falls,
//! slides to a stop, and from 21 to 120 ticks after it appears looks for the nearest player within
//! eight blocks every twenty, flies to it and is taken, worth its experience.
// TODO: orbs do not merge (BDS merges those whose ids agree mod 13, and spreads new experience
// over 13 orbs already in a cell), and are not saved.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::blocks::{self, Dice};
use crate::body::{Body, Physics};
use crate::sync::Told;
use crate::components::{InLayer, Layer, NetId, Position, Stage};
use crate::{experience, mobs};

/// How experience is split into orbs, largest first; above [`MANY`], into [`MANY_ORBS`] equal ones.
const SPLITS: [i32; 11] = [2477, 1237, 617, 307, 149, 73, 37, 17, 7, 3, 1];
const MANY: i32 = 12_000;
const MANY_ORBS: i32 = 13;
const LIFETIME: u32 = 6000;
/// Its box, wide and high (from memory of the game: unverified).
const SIZE: f32 = 0.25;
/// Players further than this are not followed.
const FOLLOW: f32 = 8.0;
const SEARCH_EVERY: u64 = 20;
/// An orb this close across to the player it follows (in eighths of the gap) is taken.
const TOUCH: f32 = 0.225;
/// Where on a player an orb flies to, above its feet.
const BODY: f32 = 0.9;

#[derive(Component)]
pub(crate) struct Orb {
    value: i32,
    body: Body,
    told: Told,
    age: u32,
    last_search: u64,
    target: Option<Entity>,
}

/// The orbs `points` of experience come as (BDS `ExperienceOrb::spawnOrbs`).
fn split(mut points: i32) -> Vec<i32> {
    if points > MANY {
        let each = points / MANY_ORBS;
        return (0..MANY_ORBS).map(|orb| if orb == MANY_ORBS - 1 { points - each * (MANY_ORBS - 1) } else { each }).collect();
    }
    let mut orbs = Vec::new();
    while points > 0 {
        let orb = SPLITS.into_iter().find(|split| *split <= points).unwrap_or(1);
        orbs.push(orb);
        points -= orb;
    }
    orbs
}

fn random(world: &mut World) -> f32 {
    world.get_resource_or_insert_with(Dice::default).fraction()
}

fn announce(id: u64, orb: &Orb) -> Vec<bytes::Bytes> {
    vec![sapling_net::encode(&sapling_data::add_experience_orb(id, orb.body.at, orb.body.motion, orb.value))]
}

/// Lets `points` of experience out at `at` as orbs, each thrown a little up and aside.
pub(crate) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], points: i32) {
    for value in split(points) {
        let mut throw = || random(world) * 0.2;
        let motion = [(throw() - 0.1) * 2.0, throw() * 2.0, (throw() - 0.1) * 2.0];
        let orb = Orb { value, body: Body::new(at, motion, [SIZE; 2]), told: Told::new(at), age: 0, last_search: 0, target: None };
        let Some(entity) = mobs::spawn_shown(world, layer, at, |id| Some(announce(id, &orb))) else { continue };
        world.entity_mut(entity).insert(orb);
    }
}

/// The nearest player of `layer` in the world within [`FOLLOW`] blocks of `at`.
fn nearest(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let mut players = world.query::<(Entity, &Stage, &InLayer, &Position)>();
    let distance = |feet: [f32; 3]| (0..3).map(|axis| (feet[axis] + if axis == 1 { sapling_data::EYE_HEIGHT } else { 0.0 } - at[axis]).powi(2)).sum::<f32>();
    let near = players.iter(world).filter(|(_, stage, in_layer, _)| **stage == Stage::InWorld && in_layer.0 == layer).map(|(player, .., position)| (player, distance(position.feet)));
    near.filter(|(_, distance)| *distance < FOLLOW * FOLLOW).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(player, _)| player)
}

/// One tick of the orb `entity`; false once it is gone.
fn step(world: &mut World, entity: Entity, orb: &mut Orb, layer: Entity, id: u64) -> bool {
    orb.age += 1;
    if orb.age >= LIFETIME {
        return false;
    }
    // Counted in the orb's own ticks, so that the first search is 21 to 120 ticks after it appeared.
    if u64::from(orb.age) > orb.last_search + SEARCH_EVERY + id % 100 {
        orb.last_search = u64::from(orb.age);
        orb.target = nearest(world, layer, orb.body.at);
    }
    let chest = orb.target.and_then(|player| world.get::<Position>(player)).map(|position| [position.feet[0], position.feet[1] + BODY, position.feet[2]]);
    if let (Some(player), Some(chest)) = (orb.target, chest) {
        let gap = [0, 1, 2].map(|axis| (chest[axis] - orb.body.at[axis]) / FOLLOW);
        let across = gap[0].hypot(gap[2]);
        if across <= TOUCH {
            orb.body.at = [0, 1, 2].map(|axis| orb.body.at[axis] + gap[axis]);
            taken(world, layer, orb, player);
            return false;
        }
        let pull = (1.0 - across).max(0.0).powi(2) * 0.1 / across;
        (0..3).for_each(|axis| orb.body.motion[axis] += gap[axis] * pull);
    }
    if let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) {
        orb.body.step(level, &Physics::ORB);
    }
    let packets = orb.told.update(id, orb.body.at, orb.body.motion, orb.body.on_ground);
    if !packets.is_empty() {
        blocks::to_viewers(world, layer, orb.body.at.map(|c| c.floor() as i32), &packets);
        mobs::reannounce(world, entity, announce(id, orb));
    }
    true
}


/// A player takes an orb: it hears it, and gets its experience.
fn taken(world: &mut World, layer: Entity, orb: &Orb, player: Entity) {
    let [x, y, z] = orb.body.at;
    let sound = LevelEvent { event: LevelEventEvent::SoundOrb, position: Vec3f { x, y, z }, data: 0 };
    blocks::to_viewers(world, layer, orb.body.at.map(|c| c.floor() as i32), &[sapling_net::encode(&sound)]);
    experience::give(world, player, orb.value);
}

/// Moves every orb a tick: falling, following and being taken.
pub(crate) fn tick(world: &mut World) {
    let mut orbs = world.query::<(Entity, &InLayer, &NetId, &Orb)>();
    let all: Vec<(Entity, Entity, u64)> = orbs.iter(world).map(|(entity, layer, id, _)| (entity, layer.0, id.0)).collect();
    for (entity, layer, id) in all {
        let Some(mut orb) = world.entity_mut(entity).take::<Orb>() else { continue };
        match step(world, entity, &mut orb, layer, id) {
            true => drop(world.entity_mut(entity).insert(orb)),
            false => mobs::remove(world, entity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experience_comes_as_the_games_orb_sizes() {
        assert_eq!(split(10), [7, 3]);
        assert_eq!(split(40), [37, 3]);
        assert_eq!(split(0), Vec::<i32>::new());
        let many = split(13_001);
        assert_eq!((many.len(), many[0], many.iter().sum::<i32>()), (13, 1000, 13_001));
    }
}
