//! The strafing dragon's fireball (`dragon_fireball`, research/dragon-extras-decompile.md 4.1-4.3,
//! research/captures/bds-dragon-survival.txt): shot from two over the dragon's feet and a block
//! towards its prey, at the middle of the prey's box, at 1.3 with BDS's spread; the dragon's box
//! pushes it off for its first ticks, then it flies straight, and where its point meets a block or
//! the box of a player or mob other than a dragon, grown by 0.3, it splashes, and the tick after
//! leaves a breath cloud (cloud.rs) and is gone. A player's hit does nothing to it (extras 5.1).
// TODO: a projectile that hits it sends it off along its shooter's look (extras 5.1), once players shoot.

use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;

use super::cloud::{self, item, with};
use super::random::Random;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, in_world};
use crate::sync::Told;
use crate::{entities, mobs};

const KIND: &str = "minecraft:dragon_fireball";
const SIZE: [f32; 2] = [0.31, 0.31];
const FLAGS: u64 = 0x3_0000_0000_0020;
const FLYING_FLAGS: u64 = 0x3_0004_0000_4000;
const ANCHOR: f32 = 2.0;
const OFFSET: f32 = 0.5;
const POWER: f32 = 1.3;
const SPREAD: f32 = 0.0075 * 10.0;
const GROWN: f32 = 0.3;
const PUSH: f32 = 0.05;
const TURNING: f32 = 0.2;
const PLAYER: [f32; 2] = [0.6, 1.8];
const PLAYER_MIDDLE: f32 = 0.9;
/// BDS's flies on until it meets something; here one that met nothing goes after this many ticks.
const FLIES: u32 = 600;
const STEP: f32 = 360.0 / 256.0;

#[derive(Component)]
struct Fireball {
    age: u32,
    pitch: f32,
    yaw: f32,
    met: Option<[f32; 3]>,
}

/// The pitch and yaw of a motion, as BDS turns a projectile (`shoot` 1424020c0).
fn facing([x, y, z]: [f32; 3]) -> [f32; 2] {
    [y.atan2(x.hypot(z)).to_degrees(), x.atan2(z).to_degrees()]
}

impl Fireball {
    /// A fifth of the way to facing its motion each tick (extras 4.2 step 6).
    fn turn(&mut self, motion: [f32; 3]) {
        let [pitch, yaw] = facing(motion);
        self.pitch += super::flight::wrap(pitch - self.pitch) * TURNING;
        self.yaw += super::flight::wrap(yaw - self.yaw) * TURNING;
    }
}

fn sound(name: &str, [x, y, z]: [f32; 3], kind: &str, unique: i64) -> Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: unique, fire_at_position: None })
}

fn turned(degrees: f32) -> f32 {
    (degrees / STEP).round() * STEP
}

fn announcement(id: u64, owner: i64, at: [f32; 3], motion: [f32; 3], [pitch, yaw]: [f32; 2]) -> Vec<Bytes> {
    let options = AddEntityOptions { id, identifier: KIND, feet: at, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
    let mut added: AddEntity = sapling_data::add_entity(&options);
    added.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
    let items = vec![
        item(Key::Health, Plain::Int(1)),
        item(Key::OwnerEid, Plain::Long(owner)),
        item(Key::FireballPowerX, Plain::Float(0.0)),
        item(Key::FireballPowerY, Plain::Float(0.0)),
        item(Key::FireballPowerZ, Plain::Float(0.0)),
        item(Key::BoundingboxWidth, Plain::Float(SIZE[0])),
        item(Key::BoundingboxHeight, Plain::Float(SIZE[1])),
    ];
    with(&mut added.metadata, FLAGS, items);
    let again = SetEntityData { runtime_entity_id: id, metadata: added.metadata.clone(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    vec![sapling_net::encode(&AddEntity { head_yaw: 0.0, body_yaw: 0.0, ..added }), sapling_net::encode(&again)]
}

/// The dragon `dragon`, its feet at `from`, shoots at the player whose feet are at `prey`.
pub(super) fn shoot(world: &mut World, layer: Entity, dragon: Entity, from: [f32; 3], prey: [f32; 3], random: &mut Random) {
    let anchor = from[1] + ANCHOR;
    let (dx, dz) = (prey[0] - from[0], prey[2] - from[2]);
    let dy = prey[1] + PLAYER_MIDDLE - anchor - OFFSET;
    let across = dx.hypot(dz);
    if across < 1e-7 {
        return;
    }
    let at = [from[0] + dx / across, anchor + OFFSET, from[2] + dz / across];
    let length = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-4);
    let motion = [dx, dy, dz].map(|c| (c / length + random.next_gaussian() * SPREAD) * POWER);
    let turn = [turned(motion[1].atan2(motion[0].hypot(motion[2])).to_degrees()), turned(motion[0].atan2(motion[2]).to_degrees())];
    let Some(owner) = world.get::<NetId>(dragon).map(|id| id.0) else { return };
    let Some(fireball) = entities::spawn(world, layer, KIND, Body::new(at, motion, SIZE), at, |id| Some(announcement(id, owner as i64, at, motion, turn))) else { return };
    world.entity_mut(fireball).insert(Fireball { age: 0, pitch: turn[0], yaw: turn[1], met: None });
    if let Some(mut told) = world.get_mut::<Told>(fireball) {
        told.pitch(turn[0]);
        told.face(turn[1], 0.0);
    }
    let id = world.get::<NetId>(fireball).map_or(0, |id| id.0) as i64;
    let mouth = [from[0], anchor, from[2]];
    let sounds = vec![sound("disappeared", [from[0], from[1] + OFFSET, from[2]], KIND, id), sound("undefined", mouth, "", -1), sound("shoot", mouth, super::DRAGON, owner as i64)];
    entities::after_announcing(world, layer, at.map(|c| c.floor() as i32), sounds);
}

/// The dragon with its feet at `at` and its box `size` pushes the fireballs its box overlaps, as
/// any mob pushes (`Mob::pushActors`, `push` 142fa1d50, research/dragon-extras-decompile.md 5.3):
/// away from its feet across, the larger axis by 0.05. Called between its stored motion's move and
/// its own, where it fits bds-dragon-survival.txt's first fireball.
pub(super) fn pushed_by(world: &mut World, layer: Entity, at: [f32; 3], [width, height]: [f32; 2]) {
    let (low, high) = ([at[0] - width / 2.0, at[1], at[2] - width / 2.0], [at[0] + width / 2.0, at[1] + height, at[2] + width / 2.0]);
    let mut balls = world.query_filtered::<(&InLayer, &mut Body), With<Fireball>>();
    for (in_layer, mut ball) in balls.iter_mut(world) {
        let (ball_low, ball_high) = ball.bounds();
        let overlaps = (0..3).all(|axis| ball_low[axis] < high[axis] && ball_high[axis] > low[axis]);
        let (dx, dz) = (at[0] - ball.at[0], at[2] - ball.at[2]);
        let m = dx.abs().max(dz.abs());
        if in_layer.0 != layer || !overlaps || m < 0.1 {
            continue;
        }
        let inv = 1.0 / m.sqrt();
        let k = inv.min(1.0) * PUSH;
        (ball.motion[0], ball.motion[2]) = (ball.motion[0] - dx * inv * k, ball.motion[2] - dz * inv * k);
    }
}

/// Where along `from` to `from + by` (0..1) the point first enters a block with a box.
pub(super) fn meets_block(level: &sapling_world::World, from: [f32; 3], by: [f32; 3]) -> Option<f32> {
    const SAMPLES: u32 = 64;
    let inside = |t: f32| {
        let [x, y, z] = [0, 1, 2].map(|axis| (from[axis] + by[axis] * t).floor() as i32);
        level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid())
    };
    let first = (1..=SAMPLES).map(|i| i as f32 / SAMPLES as f32).find(|&t| inside(t))?;
    let (mut low, mut high) = (first - 1.0 / SAMPLES as f32, first);
    for _ in 0..20 {
        let middle = (low + high) / 2.0;
        if inside(middle) { high = middle } else { low = middle }
    }
    Some(high)
}

/// Where along the move the point enters a player's or a mob's box grown by 0.3 (the slab test);
/// the dragons are passed.
fn meets_body(world: &mut World, layer: Entity, from: [f32; 3], by: [f32; 3]) -> Option<f32> {
    let enters = |(low, high): ([f32; 3], [f32; 3])| {
        let (low, high) = (low.map(|c| c - GROWN), high.map(|c| c + GROWN));
        let (mut first, mut last) = (0.0f32, 1.0f32);
        for axis in 0..3 {
            if by[axis].abs() < 1e-9 {
                if from[axis] < low[axis] || from[axis] > high[axis] {
                    return None;
                }
                continue;
            }
            let (a, b) = ((low[axis] - from[axis]) / by[axis], (high[axis] - from[axis]) / by[axis]);
            (first, last) = (first.max(a.min(b)), last.min(a.max(b)));
        }
        (first <= last).then_some(first)
    };
    let player_box = |[x, y, z]: [f32; 3]| ([x - PLAYER[0] / 2.0, y, z - PLAYER[0] / 2.0], [x + PLAYER[0] / 2.0, y + PLAYER[1], z + PLAYER[0] / 2.0]);
    let spectating = |player: Entity| world.get::<crate::components::Mode>(player).is_some_and(|mode| mode.0 == sapling_api::GameMode::Spectator);
    let players = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer) && !spectating(*player));
    let mut boxes: Vec<_> = players.filter_map(|player| Some(player_box(world.get::<Position>(player)?.feet))).collect();
    let mut mobs = world.query_filtered::<(&InLayer, &Body), (With<crate::creatures::Creature>, Without<super::Dragon>)>();
    boxes.extend(mobs.iter(world).filter(|(in_layer, _)| in_layer.0 == layer).map(|(_, body)| body.bounds()));
    boxes.into_iter().filter_map(enters).min_by(f32::total_cmp)
}

fn fly(world: &mut World, fireball: Entity, layer: Entity, id: u64) -> Option<()> {
    let mut body = *world.get::<Body>(fireball)?;
    let (age, yaw, met) = world.get::<Fireball>(fireball).map(|ball| (ball.age, ball.yaw, ball.met))?;
    world.get_mut::<Fireball>(fireball)?.age += 1;
    let cell = body.at.map(|c| c.floor() as i32);
    if let Some(at) = met {
        cloud::spawn(world, layer, [at[0].floor() + 0.5, at[1] + 0.5, at[2].floor() + 0.5], yaw, cloud::FIREBALL);
        mobs::remove(world, fireball);
        crate::blocks::to_viewers(world, layer, cell, &[sound("explode", at, "", -1)]);
        return None;
    }
    if age >= FLIES {
        mobs::remove(world, fireball);
        return None;
    }
    let mut told = Vec::new();
    match age {
        1 => {
            let mut metadata = vec![item(Key::Flags, Plain::Long(0))];
            with(&mut metadata, FLYING_FLAGS, Vec::new());
            told.push(sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }));
        }
        2 => told.push(sound("reappeared", [body.at[0], body.at[1] + SIZE[1] / 2.0, body.at[2]], KIND, id as i64)),
        _ => {}
    }
    let block = meets_block(&world.get::<Layer>(layer)?.world, body.at, body.motion);
    let hit = [block, meets_body(world, layer, body.at, body.motion)].into_iter().flatten().min_by(f32::total_cmp);
    let to = hit.map_or(1.0, |t| t);
    // A face it meets is a whole block's: the search's last bits are dropped.
    let snapped = |c: f32| if (c - c.round()).abs() < 1e-4 && hit == block && hit.is_some() { c.round() } else { c };
    body.at = std::array::from_fn(|axis| snapped(body.at[axis] + body.motion[axis] * to));
    *world.get_mut::<Body>(fireball)? = body;
    // The move onto what it meets is not told (bds-dragon-survival.txt).
    if hit.is_none() {
        let [pitch, yaw] = {
            let mut ball = world.get_mut::<Fireball>(fireball)?;
            ball.turn(body.motion);
            [ball.pitch, ball.yaw]
        };
        let mut kept = world.get_mut::<Told>(fireball)?;
        kept.pitch(pitch);
        kept.face(yaw, 0.0);
        told.extend(kept.update(id, body.at, body.motion, false));
    }
    if hit.is_some() {
        world.get_mut::<Fireball>(fireball)?.met = Some(body.at);
        let [x, y, z] = body.at;
        told.push(sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleSplash, position: Vec3f { x, y, z }, data: cloud::FIREBALL.colour }));
    }
    entities::tell_viewers(world, fireball, layer, cell, told);
    Some(())
}

/// Moves every dragon fireball a tick.
pub(super) fn tick(world: &mut World) {
    let fireballs: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Fireball>>().iter(world).map(|(ball, layer, id)| (ball, layer.0, id.0)).collect();
    for (fireball, layer, id) in fireballs {
        fly(world, fireball, layer, id);
    }
}
