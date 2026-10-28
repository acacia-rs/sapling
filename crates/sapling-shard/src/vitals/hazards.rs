//! What else hurts a player where it stands: a cactus it touches, a ripe sweet berry bush it moves
//! through, the void under the world. Each cost BDS's player 1, 1 and 4 a hit, a hit each time
//! its guard let one in (research/captures/bds-thorny.txt, bds-void.txt), with the cause
//! `contact`, `contact` and `void`. A glider that flies into a wall is hurt too.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::body::Body;
use crate::components::{NetId, Position};
use crate::creatures::touches;

const CACTUS: &[&str] = &["minecraft:cactus"];
const BUSH: &str = "minecraft:sweet_berry_bush";
const GROWTH: &str = "growth";
pub(super) const PRICKS: f32 = 1.0;
pub(super) const VOID_HARM: f32 = 4.0;
/// The void hurts this far under the bottom of the world (BDS `OutOfWorldSystem` 142f1f2f0).
const VOID_DEPTH: f32 = 16.0;
/// A player in a bush that moved no more than this across this tick is not pricked (BDS 1471f0d30).
const STIRS: f32 = 0.003;
const WALL_HARM: f32 = 10.0;
const WALL_SPARED: f32 = 3.0;
/// Flying into a wall for more than this is heard as a big fall.
const BIG_WALL: f32 = 4.0;

/// Whether the player's box touches a cactus, and whether it is in a bush with berries on it.
pub(super) fn thorns(level: &Level, body: &Body) -> [bool; 2] {
    [touches(level, body, CACTUS), in_ripe_bush(level, body)]
}

/// Whether the player is pricked by the thorns it is among, having moved `moved` across.
pub(super) fn pricks([cactus, bush]: [bool; 2], moved: [f32; 2]) -> bool {
    cactus || bush && (moved[0].abs() > STIRS || moved[1].abs() > STIRS)
}

fn in_ripe_bush(level: &Level, body: &Body) -> bool {
    let registry = level.registry();
    let (low, high) = body.bounds();
    let cells = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
    let ripe = |[x, y, z]: [i32; 3]| {
        let state = level.in_height(y).then(|| registry.get(level.block(x, y, z))).flatten();
        state.is_some_and(|state| state.name == BUSH && state.property(GROWTH).is_some_and(|growth| growth != "0"))
    };
    cells(0).any(|x| cells(1).any(|y| cells(2).any(|z| ripe([x, y, z]))))
}

/// A glider's speed across as its last input stepped it.
#[derive(Component)]
pub(super) struct Glided(f32);

/// What flying into a wall costs a gliding player this tick: the speed across it lost since the
/// last, times ten, less three, where that is more than nought (BDS 148430e20, decompile only).
pub(super) fn into_wall(world: &mut World, player: Entity) -> Option<f32> {
    let speed = crate::gliding::velocity(world, player).map(|[x, _, z]| x.hypot(z));
    let before = world.get::<Glided>(player).map(|glided| glided.0);
    match speed {
        Some(speed) => drop(world.entity_mut(player).insert(Glided(speed))),
        None => drop(world.entity_mut(player).remove::<Glided>()),
    }
    let harm = (before? - speed?) * WALL_HARM - WALL_SPARED;
    (harm > 0.0).then_some(harm)
}

/// What the player's viewers hear of it flying into a wall for `harm`, a tick later, as a landing's.
pub(super) fn sound_wall(world: &mut World, player: Entity, layer: Entity, harm: f32) {
    let (Some(id), Some([x, y, z])) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet)) else { return };
    let name = if harm > BIG_WALL { "fall.big" } else { "fall.small" };
    let sound = LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    crate::late_sounds::play_later_but(world, 1, layer, [x, y, z].map(|c| c.floor() as i32), Some(player), sapling_net::encode(&sound));
}

/// Whether feet at height `y` are in the void.
pub(super) fn in_void(level: &Level, y: f32) -> bool {
    y < level.heights().start as f32 - VOID_DEPTH
}
