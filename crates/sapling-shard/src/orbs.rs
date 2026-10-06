//! Experience orbs, as BDS's `ExperienceOrb` makes and moves them (docs/research/experience.md,
//! sections 1 to 3; the packets as recorded, `examples/bds_container_probe.rs`). The server moves
//! them: the client only shows where they are. An orb falls, slides to a stop, and from 21 to 120
//! ticks after it appears looks for the nearest player within eight blocks every twenty and flies
//! to it; a player it touches on the way, or before, takes it, worth its experience.
// TODO: orbs do not merge (BDS merges those whose ids agree mod 13, and spreads new experience
// over 13 orbs already in a cell), collide only with the block they fall into, and are not saved.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, MoveEntityDelta};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::blocks::{self, Dice};
use crate::components::{InLayer, Layer, NetId, Position, Stage};
use crate::{experience, mobs};

/// How experience is split into orbs, largest first; above [`MANY`], into [`MANY_ORBS`] equal ones.
const SPLITS: [i32; 11] = [2477, 1237, 617, 307, 149, 73, 37, 17, 7, 3, 1];
const MANY: i32 = 12_000;
const MANY_ORBS: i32 = 13;
const LIFETIME: u32 = 6000;
const GRAVITY: f32 = 0.04;
const AIR_FRICTION: f32 = 0.98;
const GROUND_FRICTION: f32 = 0.588;
const BOUNCE: f32 = -0.5;
/// Players further than this are not followed.
const FOLLOW: f32 = 8.0;
const SEARCH_EVERY: u64 = 20;
/// An orb this close across to the player it follows (in eighths of the gap) is taken.
const TOUCH: f32 = 0.225;
/// Where on a player an orb flies to, above its feet.
const BODY: f32 = 0.9;
/// How far from a player's feet an orb is touched: across, and from below to above.
const TOUCH_ACROSS: f32 = 1.55;
const TOUCH_BELOW: f32 = -1.0;
const TOUCH_ABOVE: f32 = 2.3;
/// An orb is not taken by touch before it is this old: BDS took one let out at the player
/// after 7 ticks (one recording).
const TOUCH_AFTER: u32 = 7;

#[derive(Component)]
pub(crate) struct Orb {
    value: i32,
    at: [f32; 3],
    motion: [f32; 3],
    age: u32,
    last_search: u64,
    target: Option<Entity>,
    on_ground: bool,
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

/// A number from 0 to 1 off the shard's dice.
fn random(world: &mut World) -> f32 {
    world.get_resource_or_insert_with(Dice::default).roll(1 << 20) as f32 / (1 << 20) as f32
}

fn announce(id: u64, orb: &Orb) -> Vec<bytes::Bytes> {
    vec![sapling_net::encode(&sapling_data::add_experience_orb(id, orb.at, orb.motion, orb.value))]
}

/// Lets `points` of experience out at `at` as orbs, each thrown a little up and aside.
pub(crate) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], points: i32) {
    for value in split(points) {
        let mut throw = || random(world) * 0.2;
        let motion = [(throw() - 0.1) * 2.0, throw() * 2.0, (throw() - 0.1) * 2.0];
        let orb = Orb { value, at, motion, age: 0, last_search: 0, target: None, on_ground: false };
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

/// A player of `layer` whose reach touches the orb at `at`: its box grown by one block across and
/// half a block up and down (Java's pickup box; BDS took an orb 1.1 blocks off in 7 ticks).
fn touching(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let mut players = world.query::<(Entity, &Stage, &InLayer, &Position)>();
    let touches = |feet: [f32; 3]| (at[0] - feet[0]).abs() <= TOUCH_ACROSS && (at[2] - feet[2]).abs() <= TOUCH_ACROSS && (TOUCH_BELOW..=TOUCH_ABOVE).contains(&(at[1] - feet[1]));
    let mut near = players.iter(world).filter(|(_, stage, in_layer, position)| **stage == Stage::InWorld && in_layer.0 == layer && touches(position.feet));
    near.next().map(|(player, ..)| player)
}

/// One tick of the orb `entity`; false once it is gone.
fn step(world: &mut World, entity: Entity, orb: &mut Orb, layer: Entity, id: u64) -> bool {
    orb.age += 1;
    if orb.age >= LIFETIME {
        return false;
    }
    if let Some(player) = touching(world, layer, orb.at).filter(|_| orb.age >= TOUCH_AFTER) {
        taken(world, layer, orb, player);
        return false;
    }
    orb.motion[1] -= GRAVITY;
    // Counted in the orb's own ticks, so that the first search is 21 to 120 ticks after it appeared.
    if u64::from(orb.age) > orb.last_search + SEARCH_EVERY + id % 100 {
        orb.last_search = u64::from(orb.age);
        orb.target = nearest(world, layer, orb.at);
    }
    let body = orb.target.and_then(|player| world.get::<Position>(player)).map(|position| [position.feet[0], position.feet[1] + BODY, position.feet[2]]);
    if let (Some(player), Some(body)) = (orb.target, body) {
        let gap = [0, 1, 2].map(|axis| (body[axis] - orb.at[axis]) / FOLLOW);
        let across = gap[0].hypot(gap[2]);
        if across <= TOUCH {
            orb.at = [0, 1, 2].map(|axis| orb.at[axis] + gap[axis]);
            taken(world, layer, orb, player);
            return false;
        }
        let pull = (1.0 - across).max(0.0).powi(2) * 0.1 / across;
        (0..3).for_each(|axis| orb.motion[axis] += gap[axis] * pull);
    }
    let before = orb.at;
    fall(world, layer, orb);
    if before != orb.at {
        // As BDS moves them: the axes that changed, every tick it moves.
        let changed = |axis: usize| (before[axis] != orb.at[axis]).then_some(orb.at[axis]);
        let moved = MoveEntityDelta {
            runtime_entity_id: id,
            x: changed(0),
            y: changed(1),
            z: changed(2),
            rot_x: None,
            rot_y: None,
            rot_z: None,
            on_ground: orb.on_ground,
            force_move: false,
            force_move_local_entity: false,
            force_completion: false,
            ticks: 0,
        };
        blocks::to_viewers(world, layer, orb.at.map(|c| c.floor() as i32), &[sapling_net::encode(&moved)]);
        mobs::reannounce(world, entity, announce(id, orb));
    }
    true
}

/// Moves the orb by its motion, onto the block it would fall into, and slows it down.
fn fall(world: &World, layer: Entity, orb: &mut Orb) {
    let next = [0, 1, 2].map(|axis| orb.at[axis] + orb.motion[axis]);
    let level = world.get::<Layer>(layer).map(|layer| &layer.world);
    let [x, y, z] = next.map(|c| c.floor() as i32);
    let is_solid = level.is_some_and(|level| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid()));
    orb.on_ground = is_solid && orb.motion[1] < 0.0;
    orb.at = if orb.on_ground { [next[0], (y + 1) as f32, next[2]] } else { next };
    let friction = if orb.on_ground { GROUND_FRICTION } else { AIR_FRICTION };
    orb.motion = [orb.motion[0] * friction, orb.motion[1] * if orb.on_ground { BOUNCE } else { AIR_FRICTION }, orb.motion[2] * friction];
}

/// A player takes an orb: it hears it, and gets its experience.
fn taken(world: &mut World, layer: Entity, orb: &Orb, player: Entity) {
    let [x, y, z] = orb.at;
    let sound = LevelEvent { event: LevelEventEvent::SoundOrb, position: Vec3f { x, y, z }, data: 0 };
    blocks::to_viewers(world, layer, orb.at.map(|c| c.floor() as i32), &[sapling_net::encode(&sound)]);
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
