//! The dragon's parts (research/dragon-ai-decompile.md 3 step 10, 5 and 7): its body, wings, neck
//! and head round where it is and its yaw. Out of a sit and past its hurt time (the 10 ticks of a
//! hit's guard) its wings throw the players under them and hurt them for 5, its head and neck bite
//! for 10; head, neck and body break the blocks they are in that are not of the End's hard ones,
//! while mobGriefing is on, each showing its dust once.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::flight::Flight;
use super::random::Random;
use crate::components::{InLayer, Layer, Mode, Position, in_world};

const HALF_HEIGHT: f32 = 2.0;
const WINGS_HARM: f32 = 5.0;
const BITE_HARM: f32 = 10.0;
const THROW: f32 = 0.4;
const DUST: i32 = 65;
const GRIEFING: &str = "mobGriefing";
/// Blocks it passes without breaking or being slowed.
const PASSED: &[&str] = &["minecraft:air", "minecraft:fire", "minecraft:soul_fire", "minecraft:structure_void"];
/// Blocks it cannot break.
const HARD: &[&str] = &[
    "bedrock", "obsidian", "crying_obsidian", "respawn_anchor", "end_stone", "end_portal_frame", "end_portal", "end_gateway", "iron_bars", "reinforced_deepslate", "barrier", "allow", "deny",
    "border_block", "command_block", "repeating_command_block", "chain_command_block", "structure_block", "jigsaw",
];
const PLAYER: [f32; 2] = [0.6, 1.8];

type Box3 = ([f32; 3], [f32; 3]);

/// A part's box round its middle, `size` wide and high.
fn around([x, y, z]: [f32; 3], [w, h]: [f32; 2]) -> Box3 {
    ([x - w / 2.0, y - h / 2.0, z - w / 2.0], [x + w / 2.0, y + h / 2.0, z + w / 2.0])
}

fn grown((min, max): Box3, [gx, gy, gz]: [f32; 3]) -> Box3 {
    ([min[0] - gx, min[1] - gy, min[2] - gz], [max[0] + gx, max[1] + gy, max[2] + gz])
}

struct Parts {
    head: Box3,
    neck: Box3,
    body_box: Box3,
    wings: [Box3; 2],
    tails: [Box3; 3],
}

fn parts(flight: &Flight) -> Parts {
    let [x, y, z] = flight.motion.at;
    let (yaw, now) = (flight.motion.yaw, [flight.motion.yaw, y]);
    let back = |n: usize| flight.trail.back(n, now);
    let (f13, f12) = ((back(5)[1] - back(10)[1]) * 10.0).to_radians().sin_cos();
    let (s, c) = yaw.to_radians().sin_cos();
    let (sh, ch) = (yaw.to_radians() - flight.motion.yaw_speed * 0.01).sin_cos();
    let lift = if flight.sitting { -1.0 } else { back(5)[1] - back(0)[1] };
    let reach = |r: f32| [x + sh * r * f12, y + f13 * r + HALF_HEIGHT + lift, z - ch * r * f12];
    let body = [x + s * 0.5, y + HALF_HEIGHT, z - c * 0.5];
    let wing = |side: f32| around([x + side * c * 4.5, y + 2.0 + HALF_HEIGHT, z + side * s * 4.5], [4.0, 2.0]);
    // BDS reads the trail 4, 6 and 8 back where Java reads 12, 14 and 16.
    let tail = |k: usize| {
        let [then_yaw, then_y] = back(4 + 2 * k);
        let g = yaw.to_radians() + super::flight::wrap(then_yaw - back(5)[0]).to_radians();
        let w = 2.0 * (k as f32 + 1.0);
        around([x - (s * 1.5 + g.sin() * w) * f12, then_y - back(5)[1] + y - (w + 1.5) * f13 + 1.5, z + (c * 1.5 + g.cos() * w) * f12], [2.0, 2.0])
    };
    Parts { head: around(reach(6.5), [1.0, 1.0]), neck: around(reach(5.5), [3.0, 3.0]), body_box: around(body, [5.0, 3.0]), wings: [wing(1.0), wing(-1.0)], tails: [tail(0), tail(1), tail(2)] }
}

/// The middle of its head's box.
pub(super) fn head_middle(flight: &Flight) -> [f32; 3] {
    let (low, high) = parts(flight).head;
    std::array::from_fn(|axis| (low[axis] + high[axis]) / 2.0)
}

/// Every part's box, where a player's hit can reach it.
pub(crate) fn boxes(flight: &Flight) -> Vec<Box3> {
    let parts = parts(flight);
    [parts.head, parts.neck, parts.body_box].into_iter().chain(parts.wings).chain(parts.tails).collect()
}

fn overlaps((a_min, a_max): Box3, (b_min, b_max): Box3) -> bool {
    (0..3).all(|axis| a_min[axis] < b_max[axis] && a_max[axis] > b_min[axis])
}

/// The players of `layer` it may hurt, with their feet.
fn players(world: &World, layer: Entity) -> Vec<(Entity, [f32; 3])> {
    let mortal = |player: &Entity| world.get::<Mode>(*player).is_some_and(|mode| matches!(mode.0, GameMode::Survival | GameMode::Adventure)) && world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer);
    in_world(world).into_iter().filter(mortal).filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).collect()
}

fn player_box([x, y, z]: [f32; 3]) -> Box3 {
    ([x - PLAYER[0] / 2.0, y, z - PLAYER[0] / 2.0], [x + PLAYER[0] / 2.0, y + PLAYER[1], z + PLAYER[0] / 2.0])
}

/// What its parts do this tick; whether a block slowed it.
pub(super) fn touch(world: &mut World, layer: Entity, dragon: Entity, flight: &Flight, random: &mut Random) -> bool {
    let parts = parts(flight);
    let hurt_lately = world.get::<crate::creatures::Health>(dragon).is_some_and(crate::creatures::Health::is_guarded);
    if !flight.sitting && !hurt_lately {
        // A wing reaches 4 across and from 4 under it to its top.
        let knocks = |wing: &Box3| {
            let (mut low, mut high) = grown(*wing, [4.0, 2.0, 4.0]);
            (low[1], high[1]) = (low[1] - 2.0, high[1] - 2.0);
            (low, high)
        };
        // Both hurt on the world's difficulty and throw as any hit, 0.4 away and up: BDS's easy took
        // 3.5 for a wing and 6 for a bite (bds-dragon-survival.txt), not Java's throw by distance.
        for (player, feet) in players(world, layer) {
            let body = player_box(feet);
            let (dx, dz) = (feet[0] - flight.motion.at[0], feet[2] - flight.motion.at[2]);
            let far = dx.hypot(dz).max(1e-4);
            let throw = [dx / far * THROW, THROW, dz / far * THROW];
            let hits = [(parts.wings.iter().any(|wing| overlaps(knocks(wing), body)), WINGS_HARM), ([parts.head, parts.neck].iter().any(|bite| overlaps(grown(*bite, [1.0; 3]), body)), BITE_HARM)];
            for (_, harm) in hits.into_iter().filter(|(hit, _)| *hit) {
                if let Some(harm) = crate::creatures::on_difficulty(world, harm) {
                    crate::vitals::bitten(world, player, harm, super::DRAGON, throw);
                }
            }
        }
    }
    let griefs = world.get::<Layer>(layer).is_some_and(|level| level.world.level().rules.on(GRIEFING));
    [parts.head, parts.neck, parts.body_box].iter().any(|part| walls(world, layer, *part, griefs, random))
}

/// Breaks what it may of the blocks in `part`; whether one it could not break was there.
fn walls(world: &mut World, layer: Entity, (min, max): Box3, griefs: bool, random: &mut Random) -> bool {
    let (lo, hi) = (min.map(|c| c.floor() as i32), max.map(|c| c.floor() as i32));
    let (mut blocked, mut broke) = (false, false);
    for x in lo[0]..=hi[0] {
        for y in lo[1]..=hi[1] {
            for z in lo[2]..=hi[2] {
                let Some(level) = world.get::<Layer>(layer).map(|level| &level.world).filter(|level| level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))) else { continue };
                let Some(state) = level.registry().get(level.block(x, y, z)) else { continue };
                if PASSED.contains(&state.name) || state.name.starts_with("minecraft:light_block") {
                    continue;
                }
                let bare = state.name.trim_start_matches("minecraft:");
                if !griefs || HARD.contains(&bare) || bare.ends_with("copper_bars") {
                    blocked = true;
                    continue;
                }
                let air = level.registry().air_id();
                broke |= crate::blocks::set(world, layer, [x, y, z], air);
            }
        }
    }
    if broke {
        let at = [0, 1, 2].map(|axis| min[axis] + (max[axis] - min[axis]) * random.next_float());
        let dust = LevelEvent { event: LevelEventEvent::ParticleProjectileHit, position: Vec3f { x: at[0], y: at[1], z: at[2] }, data: DUST };
        crate::blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[sapling_net::encode(&dust)]);
    }
    blocked
}
