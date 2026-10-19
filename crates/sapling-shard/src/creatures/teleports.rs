//! An enderman's teleports (docs/research/mob-kinds.md, "The enderman"), as BDS's were seen to
//! (research/captures/bds-ender-findings.md): on its first tick, the tick after a hit, and towards
//! prey that has been more than 16 off for 30 ticks.
// TODO: none while calm (BDS's came some 5 seconds apart in one run and not at all in another),
// none from beside its prey or when looked at, and water neither hurts it nor sends it off.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};
use crate::sync::Told;

/// Half its `random_teleport_cube`.
const CUBE: f32 = 16.0;
/// Prey further off than this for this many ticks it teleports towards, landing this much nearer
/// give or take the jitter, as the Java game's `teleportTowards`.
const FAR: f32 = 16.0;
const FAR_FOR: u64 = 30;
const JITTER: f32 = 4.0;
/// Places a random teleport tries before it is given up, as the Java game's on a hit.
const TRIES: usize = 64;
const LIQUIDS: &[&str] = &["minecraft:water", "minecraft:flowing_water", "minecraft:lava", "minecraft:flowing_lava"];

#[derive(Component)]
pub(super) struct Blinks {
    /// The tick from which a random teleport is due.
    due: Option<u64>,
    /// The tick its prey went further off than [`FAR`].
    far_since: Option<u64>,
}

/// Has the mob teleport, from its first tick, if its kind does.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if sapling_data::mob_traits(kind).teleports {
        let now = world.resource::<Tick>().0;
        world.entity_mut(entity).insert(Blinks { due: Some(now), far_since: None });
    }
}

/// It was hit: it teleports away the tick after.
pub(super) fn startle(world: &mut World, entity: Entity) {
    let now = world.resource::<Tick>().0;
    if let Some(mut blinks) = world.get_mut::<Blinks>(entity) {
        blinks.due = Some(now + 1);
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// Before its move: the mob teleports somewhere at random if that is due, or towards its prey,
/// whose feet are at `prey`, if that has been far long enough.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, prey: Option<[f32; 3]>) {
    let now = world.resource::<Tick>().0;
    let (Some(body), Some(blinks)) = (world.get::<Body>(entity).copied(), world.get::<Blinks>(entity)) else { return };
    let far = prey.filter(|feet| distance(*feet, body.at) > FAR);
    let far_since = far.map(|_| blinks.far_since.unwrap_or(now));
    let towards = far.filter(|_| far_since.is_some_and(|since| now >= since + FAR_FOR));
    let random = blinks.due.is_some_and(|due| now >= due);
    world.get_resource_or_insert_with(Dice::default);
    let to = world.resource_scope(|world, mut dice: Mut<Dice>| {
        let level = &world.get::<Layer>(layer)?.world;
        match (towards, random) {
            (Some(feet), _) => landing(level, &body, aimed(&mut dice, body.at, feet)),
            (None, true) => (0..TRIES).find_map(|_| landing(level, &body, scattered(&mut dice, body.at))),
            (None, false) => None,
        }
    });
    let mut blinks = world.get_mut::<Blinks>(entity).expect("a mob that teleports, above");
    blinks.far_since = far_since.filter(|_| towards.is_none() || to.is_none());
    if random {
        blinks.due = None;
    }
    if let Some(to) = to {
        jump(world, entity, layer, body.at, to);
    }
}

/// Somewhere in the cube about `at`.
fn scattered(dice: &mut Dice, at: [f32; 3]) -> [f32; 3] {
    let mut off = || (dice.fraction() * 2.0 - 1.0) * CUBE;
    [at[0] + off(), at[1] + off(), at[2] + off()]
}

/// [`FAR`] along the way from `at` to `feet`, give or take.
fn aimed(dice: &mut Dice, at: [f32; 3], feet: [f32; 3]) -> [f32; 3] {
    let long = distance(at, feet).max(1e-4);
    std::array::from_fn(|axis| at[axis] + (feet[axis] - at[axis]) / long * FAR + (dice.fraction() * 2.0 - 1.0) * JITTER)
}

/// Where a body teleported to `at` stands: down onto the first block below with a collision
/// box, if its own box is free there and in no liquid.
fn landing(level: &Level, body: &Body, at: [f32; 3]) -> Option<[f32; 3]> {
    let [x, z] = [at[0].floor() as i32, at[2].floor() as i32];
    let blocks = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty());
    let mut y = at[1].floor() as i32;
    while level.in_height(y - 1) && !blocks(y - 1) {
        y -= 1;
    }
    let moved = Body::new([at[0], y as f32, at[2]], [0.0; 3], body.size);
    let loaded = level.is_loaded(sapling_world::ChunkPos::of_block(x, z));
    (loaded && blocks(y - 1) && moved.is_free(level) && !super::harm::touches(level, &moved, LIQUIDS)).then_some(moved.at)
}

/// Puts the mob at `to`, stopped, heard where it left and where it came.
fn jump(world: &mut World, entity: Entity, layer: Entity, from: [f32; 3], to: [f32; 3]) {
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        (body.at, body.motion) = (to, [0.0; 3]);
    }
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.jump();
    }
    for [x, y, z] in [from, to] {
        let heard = sapling_net::encode(&LevelEvent { event: LevelEventEvent::SoundEndermanTeleport, position: Vec3f { x, y, z }, data: 0 });
        crate::view::to_viewers(world, layer, [x, y, z].map(|c| c.floor() as i32), &[heard]);
    }
}
