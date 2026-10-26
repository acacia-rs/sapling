//! Experience orbs, as BDS's `ExperienceOrb` makes and moves them (docs/research/experience.md,
//! sections 1 to 3; the packets as recorded, `examples/bds_container_probe.rs`). The server moves
//! them: the client only shows where they are. An orb falls, slides to a stop, and from 21 to 120
//! ticks after it appears looks for the nearest player within eight blocks every twenty and flies
//! to it; a player it touches on the way, or before, takes it, worth its experience.
// They join up as BDS's do (merge.rs). Kept with their column (actors.rs), they come back new,
// worth what they were when saved.

mod merge;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::blocks::{self, Dice};
use crate::body::{Body, Physics};
use crate::components::{InLayer, NetId, Position, Stage};
use crate::{entities, experience, mobs};

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
/// A player takes an orb a touch no sooner than this many ticks after the last; the orb goes the
/// tick after (BDS took a furnace's two orbs 2 ticks apart, research/captures/containers-findings.md).
const TAKE_EVERY: u64 = 2;

#[derive(Component)]
pub(crate) struct Orb {
    value: i32,
    age: u32,
    last_search: u64,
    target: Option<Entity>,
}

/// What became of an orb in a tick.
enum Fate {
    Stays,
    /// Gone of age, or with its column.
    Gone,
    Taken,
}

/// The tick a player last took an orb.
#[derive(Component)]
pub(crate) struct LastOrb(u64);

const KIND: &str = "minecraft:xp_orb";

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

fn announce(id: u64, value: i32, body: &Body) -> Vec<bytes::Bytes> {
    vec![sapling_net::encode(&sapling_data::add_experience_orb(id, body.at, body.motion, value))]
}

/// Lets `points` of experience out at `at` as orbs, each thrown a little up and aside.
pub(crate) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], points: i32) {
    for value in split(points) {
        if !merge::shared(world, layer, at, value) {
            throw(world, layer, at, value);
        }
    }
}

/// Pays `points` to the player as orbs where it stands, at its eyes.
pub(crate) fn pay_player(world: &mut World, player: Entity, points: i32) {
    let (Some(position), Some(layer)) = (world.get::<Position>(player).copied(), world.get::<InLayer>(player).map(|layer| layer.0)) else { return };
    let [x, y, z] = position.feet;
    spawn(world, layer, [x, y + sapling_data::EYE_HEIGHT, z], points);
}

/// One orb worth `value`, thrown a little up and aside from `at`.
pub(crate) fn throw(world: &mut World, layer: Entity, at: [f32; 3], value: i32) -> Option<Entity> {
    let mut throw = || random(world) * 0.2;
    let body = Body::new(at, [(throw() - 0.1) * 2.0, throw() * 2.0, (throw() - 0.1) * 2.0], [SIZE; 2]);
    let entity = entities::spawn(world, layer, KIND, body, at, |id| Some(announce(id, value, &body)))?;
    world.entity_mut(entity).insert(Orb { value, age: 0, last_search: 0, target: None });
    let unique_id = entities::new_unique_id(world, entity)?;
    entities::keep(world, entity, layer, unique_id, sapling_world::new_orb_actor(unique_id, value, at, body.motion));
    Some(entity)
}

/// Makes an orb a world folder kept (its actor compound `compound`) lie in `layer` again; false
/// for one that does not say what it is worth.
pub(crate) fn from_storage(world: &mut World, layer: Entity, at: [f32; 3], unique_id: i64, compound: Vec<u8>) -> bool {
    let Some(value) = sapling_world::orb_value(&compound) else { return false };
    let body = Body::new(at, [0.0; 3], [SIZE; 2]);
    let Some(entity) = entities::spawn(world, layer, KIND, body, at, |id| Some(announce(id, value, &body))) else { return false };
    world.entity_mut(entity).insert(Orb { value, age: 0, last_search: 0, target: None });
    entities::keep(world, entity, layer, unique_id, compound);
    true
}

/// Whether `player` may take an orb this tick, and if so, that it has.
fn takes(world: &mut World, player: Entity) -> bool {
    let now = world.resource::<crate::components::Tick>().0;
    if world.get::<LastOrb>(player).is_some_and(|last| now < last.0 + TAKE_EVERY) {
        return false;
    }
    world.entity_mut(player).insert(LastOrb(now));
    true
}

/// The nearest player of `layer` in the world within [`FOLLOW`] blocks of `at`.
fn nearest(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let mut players = world.query::<(Entity, &Stage, &InLayer, &Position)>();
    let distance = |feet: [f32; 3]| (0..3).map(|axis| (feet[axis] + if axis == 1 { sapling_data::EYE_HEIGHT } else { 0.0 } - at[axis]).powi(2)).sum::<f32>();
    let near = players.iter(world).filter(|(_, stage, in_layer, _)| **stage == Stage::InWorld && in_layer.0 == layer).map(|(player, .., position)| (player, distance(position.feet)));
    near.filter(|(_, distance)| *distance < FOLLOW * FOLLOW).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(player, _)| player)
}

/// Each player takes an orb its pickup area touches (`drops::within_pickup`), from its own side
/// as BDS's `Player::aiStep` does (docs/research/assumptions-checked.md 3), before the orbs move.
pub(crate) fn touch(world: &mut World) {
    let mut players = world.query::<(Entity, &Stage, &InLayer, &Position)>();
    let standing: Vec<(Entity, Entity, [f32; 3])> = players.iter(world).filter(|(_, stage, ..)| **stage == Stage::InWorld).map(|(player, _, layer, position)| (player, layer.0, position.feet)).collect();
    for (player, layer, feet) in standing {
        let mut orbs = world.query::<(Entity, &InLayer, &Body, &Orb)>();
        let touched = orbs.iter(world).find(|(_, in_layer, body, _)| in_layer.0 == layer && crate::drops::within_pickup(feet, body.at, SIZE));
        let Some((orb, at, value)) = touched.map(|(orb, _, body, kept)| (orb, body.at, kept.value)) else { continue };
        if takes(world, player) {
            world.entity_mut(orb).remove::<Orb>();
            taken(world, layer, value, at, player);
            entities::leave(world, orb);
        }
    }
}

/// One tick of the orb `entity`.
fn step(world: &mut World, entity: Entity, orb: &mut Orb, layer: Entity, id: u64) -> Fate {
    orb.age += 1;
    let Some(mut body) = world.get::<Body>(entity).copied().filter(|_| orb.age < LIFETIME) else { return Fate::Gone };
    merge::take_in_kin(world, entity, orb, layer, id, body.at);
    // Counted in the orb's own ticks, so that the first search is 21 to 120 ticks after it appeared.
    if u64::from(orb.age) > orb.last_search + SEARCH_EVERY + id % 100 {
        orb.last_search = u64::from(orb.age);
        orb.target = nearest(world, layer, body.at);
    }
    let chest = orb.target.and_then(|player| world.get::<Position>(player)).map(|position| [position.feet[0], position.feet[1] + BODY, position.feet[2]]);
    if let (Some(player), Some(chest)) = (orb.target, chest) {
        let gap = [0, 1, 2].map(|axis| (chest[axis] - body.at[axis]) / FOLLOW);
        let across = gap[0].hypot(gap[2]);
        if across <= TOUCH && takes(world, player) {
            taken(world, layer, orb.value, [0, 1, 2].map(|axis| body.at[axis] + gap[axis]), player);
            return Fate::Taken;
        }
        let pull = (1.0 - across).max(0.0).powi(2) * 0.1 / across;
        (0..3).for_each(|axis| body.motion[axis] += gap[axis] * pull);
        world.entity_mut(entity).insert(body);
    }
    let Some((body, packets)) = entities::step(world, entity, layer, id, &Physics::ORB, 0.0) else { return Fate::Gone };
    if !packets.is_empty() {
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), packets);
        mobs::reannounce(world, entity, announce(id, orb.value, &body));
    }
    Fate::Stays
}

/// A player takes an orb worth `value` at `at`: it hears it, the orb mends what it has on with
/// Mending first, and it gets the rest as experience.
fn taken(world: &mut World, layer: Entity, value: i32, at: [f32; 3], player: Entity) {
    let [x, y, z] = at;
    let sound = LevelEvent { event: LevelEventEvent::SoundOrb, position: Vec3f { x, y, z }, data: 0 };
    blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[sapling_net::encode(&sound)]);
    let left = crate::enchants::mend(world, player, value);
    if left > 0 {
        experience::give(world, player, left);
    }
}

/// Moves every orb a tick: falling, following and being taken.
pub(crate) fn tick(world: &mut World) {
    let mut orbs = world.query::<(Entity, &InLayer, &NetId, &Orb)>();
    let all: Vec<(Entity, Entity, u64)> = orbs.iter(world).map(|(entity, layer, id, _)| (entity, layer.0, id.0)).collect();
    for (entity, layer, id) in all {
        // One taken in by another earlier this tick is gone.
        let Some(mut orb) = world.get_entity_mut(entity).ok().and_then(|mut kept| kept.take::<Orb>()) else { continue };
        match step(world, entity, &mut orb, layer, id) {
            Fate::Stays => drop(world.entity_mut(entity).insert(orb)),
            Fate::Gone => mobs::remove(world, entity),
            Fate::Taken => entities::leave(world, entity),
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
