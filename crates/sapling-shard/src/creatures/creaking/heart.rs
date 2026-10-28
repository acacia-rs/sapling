//! Creaking hearts and the creakings they call, as BDS's were in the `heart` drill
//! (research/captures/bds-heart.txt). A heart between two pale oak logs along its axis is `awake`
//! by night and `dormant` by day, else `uprooted`, a tick after a change beside it. An awake
//! natural heart, with mob spawning on and off peaceful, calls a creaking 18 ticks after it woke
//! and every 20 ticks while it has none: on top of a block up to 16 across and 8 up or down, shown
//! by a spawn particle there and heard `creaking_heart_spawn` there and at the heart a tick on.
//! While its heart is awake the creaking takes no harm from a hit, which sways it (entity property
//! 1 counting 1 to 6 a tick, then 0). When its heart goes, sleeps or is uprooted it crumbles:
//! property 0 is 3 and it stands; 47 ticks on particle event 16478 from its middle; a tick on its
//! `death` and property 0 is 4; a tick on it is gone, leaving nothing.
// TODO: hearts are known once a block beside them changes, not as their chunk loads; a hit's trail
// of `block.creaking_heart.trail` sounds to the heart and the resin it grows on the logs by it
// (BDS's third hit grew three clumps) are not done; a creaking strays from its heart without bound.

use std::collections::HashMap;

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, EntityPropertiesIntsItem, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty, SpawnReason};
use sapling_world::World as Level;

use super::{Creaking, KIND};
use crate::blocks::{self, Dice};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Settings, Tick};
use crate::{entities, late_sounds, mobs};

const HEART: &str = "minecraft:creaking_heart";
const LOGS: &[&str] = &["minecraft:pale_oak_log", "minecraft:stripped_pale_oak_log", "minecraft:pale_oak_wood", "minecraft:stripped_pale_oak_wood"];
const FIRST_CALL: u64 = 18;
const CALLS_EVERY: u64 = 20;
const TRIES: usize = 5;
const REACH: [i32; 2] = [16, 8];
/// What its spawn particle carries, as a spawner's (spawners/monster.rs): the creaking's width and
/// height rounded up.
const SPAWN_SIZE: i32 = 1 + (3 << 8);
const STATE: u32 = 0;
const SWAY: u32 = 1;
const SWAYS: i32 = 6;
const CRUMBLING: i32 = 3;
const CRUMBLED: i32 = 4;
const CRUMBLES: u64 = 47;
const CRUMBLE_EVENT: i64 = 16478;

#[derive(Resource, Default)]
pub(in crate::creatures) struct Hearts(HashMap<(Entity, BlockPos), Heart>);

#[derive(Default)]
pub(in crate::creatures) struct Heart {
    creaking: Option<Entity>,
    woke: Option<u64>,
}

/// A creaking's tie to the heart that called it.
#[derive(Clone, Copy)]
pub(in crate::creatures) struct Bound {
    heart: BlockPos,
    swaying: Option<i32>,
    crumbling: Option<u64>,
}

fn state_of(level: &Level, [x, y, z]: BlockPos) -> Option<(&'static str, &'static str)> {
    let state = level.registry().get(level.block(x, y, z))?;
    Some((state.name, state.properties))
}

fn property<'a>(properties: &'a str, key: &str) -> Option<&'a str> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value)
}

/// The block at `pos` changed or one beside it did: a heart there is looked after from now on.
pub(crate) fn noticed(world: &mut World, layer: Entity, pos: BlockPos) {
    let is_heart = world.get::<Layer>(layer).and_then(|layer| state_of(&layer.world, pos)).is_some_and(|(name, _)| name == HEART);
    if is_heart {
        world.get_resource_or_insert_with(Hearts::default).0.entry((layer, pos)).or_default();
    }
}

/// What a heart's `creaking_heart_state` should be: awake or dormant between two logs along its
/// axis by the time of day, else uprooted.
fn wanted(world: &World, layer: Entity, level: &Level, [x, y, z]: BlockPos, axis: &str) -> &'static str {
    let step = match axis {
        "x" => [1, 0, 0],
        "z" => [0, 0, 1],
        _ => [0, 1, 0],
    };
    let log = |sign: i32| {
        let at = [x + sign * step[0], y + sign * step[1], z + sign * step[2]];
        state_of(level, at).is_some_and(|(name, properties)| LOGS.contains(&name) && property(properties, "pillar_axis") == Some(axis))
    };
    match (log(1) && log(-1), crate::sleep::is_night(world, layer)) {
        (false, _) => "uprooted",
        (true, true) => "awake",
        (true, false) => "dormant",
    }
}

/// Every tick: each heart takes the state its logs and the time give it, an awake natural one
/// calls its creaking, and the creaking of one gone, asleep or uprooted crumbles.
pub(in crate::creatures) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let known: Vec<(Entity, BlockPos)> = world.get_resource::<Hearts>().map_or_else(Vec::new, |hearts| hearts.0.keys().copied().collect());
    for (layer, pos) in known {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        let found = state_of(level, pos).filter(|(name, _)| *name == HEART).map(|(_, properties)| properties);
        let Some(properties) = found else {
            let gone = world.resource_mut::<Hearts>().0.remove(&(layer, pos));
            if let Some(creaking) = gone.and_then(|heart| heart.creaking) {
                crumble(world, creaking);
            }
            continue;
        };
        let (axis, natural) = (property(properties, "pillar_axis").unwrap_or("y"), property(properties, "natural") == Some("1"));
        let state = wanted(world, layer, level, pos, axis);
        if property(properties, "creaking_heart_state") != Some(state) {
            let id = level.block(pos[0], pos[1], pos[2]);
            if let Some(id) = blocks::with_property(level.registry(), id, "creaking_heart_state", state) {
                blocks::set(world, layer, pos, id);
            }
        }
        let mut hearts = world.resource_mut::<Hearts>();
        let heart = hearts.0.entry((layer, pos)).or_default();
        let woke = heart.woke;
        heart.woke = (state == "awake").then(|| woke.unwrap_or(now));
        let (creaking, woke) = (heart.creaking, heart.woke);
        match (creaking, woke) {
            (Some(creaking), None) => {
                world.resource_mut::<Hearts>().0.entry((layer, pos)).or_default().creaking = None;
                crumble(world, creaking);
            }
            (None, Some(woke)) if natural && now >= woke + FIRST_CALL && (now - woke - FIRST_CALL).is_multiple_of(CALLS_EVERY) => call(world, layer, pos),
            _ => {}
        }
    }
    sway_and_crumble(world, now);
}

/// Whether a creaking may stand where its feet are `[x, y, z]`: on a block's top, not leaves,
/// with its height of room.
fn room_at(level: &Level, [x, y, z]: BlockPos) -> bool {
    let registry = level.registry();
    let empty = |y: i32| registry.get(level.block(x, y, z)).is_none_or(|state| state.boxes.is_empty());
    let floor = registry.get(level.block(x, y - 1, z)).is_some_and(|state| state.is_full_cube() && !state.name.ends_with("leaves"));
    floor && (0..3).all(|up| empty(y + up))
}

/// Calls a creaking for the heart at `pos`, where there is room near it.
fn call(world: &mut World, layer: Entity, heart: BlockPos) {
    let rules_allow = world.resource::<Settings>().0.difficulty != Difficulty::Peaceful && crate::rules::on(world, layer, "doMobSpawning");
    if !rules_allow {
        return;
    }
    let tries: Vec<BlockPos> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let [across, up] = REACH;
        let mut off = |reach: i32| dice.roll(2 * reach as u32 + 1) as i32 - reach;
        (0..TRIES).map(|_| [heart[0] + off(across), heart[1] + off(up), heart[2] + off(across)]).collect()
    };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let found = tries.into_iter().find_map(|[x, y, z]| (y - REACH[1]..=y + REACH[1]).rev().map(|y| [x, y, z]).find(|at| room_at(level, *at)));
    let Some([x, y, z]) = found else { return };
    let Some(id) = crate::creatures::spawn(world, layer, KIND, [x as f32 + 0.5, y as f32, z as f32 + 0.5], SpawnReason::Spawner) else { return };
    let Some(creaking) = entities::find(world, id) else { return };
    if let Some(mut tie) = world.get_mut::<Creaking>(creaking) {
        tie.bound = Some(Bound { heart, swaying: None, crumbling: None });
    }
    world.resource_mut::<Hearts>().0.entry((layer, heart)).or_default().creaking = Some(creaking);
    let corner = |[x, y, z]: BlockPos| Vec3f { x: x as f32, y: y as f32, z: z as f32 };
    blocks::to_viewers(world, layer, [x, y, z], &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleSpawn, position: corner([x, y, z]), data: SPAWN_SIZE })]);
    let state = world.get::<Layer>(layer).map_or(0, |layer| {
        let level = &layer.world;
        level.registry().network_hash(level.block(heart[0], heart[1], heart[2]))
    });
    let sound = |position, extra_data, entity_type: &str, entity_unique_id| {
        let sound = LevelSoundEvent { sound_id: "creaking_heart_spawn".into(), position, extra_data, entity_type: entity_type.into(), is_baby_mob: false, is_global: false, entity_unique_id, fire_at_position: None };
        sapling_net::encode(&sound)
    };
    late_sounds::play_later(world, 1, layer, [x, y, z], sound(corner([x, y, z]), -1, KIND, id.0 as i64));
    late_sounds::play_later(world, 1, layer, heart, sound(corner(heart), state as i32, "", -1));
}

fn properties(id: u64, index: u32, value: i32) -> bytes::Bytes {
    let properties = EntityProperties { ints: vec![EntityPropertiesIntsItem { index, value }], floats: Vec::new() };
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: Vec::new(), properties, tick: 0 })
}

fn tell(world: &mut World, creaking: Entity, packets: Vec<bytes::Bytes>) {
    let (Some(layer), Some(body)) = (world.get::<InLayer>(creaking).map(|layer| layer.0), world.get::<Body>(creaking).copied()) else { return };
    entities::tell_viewers(world, creaking, layer, body.at.map(|c| c.floor() as i32), packets);
}

/// Its heart is lost to `creaking`: it starts to crumble.
fn crumble(world: &mut World, creaking: Entity) {
    let now = world.resource::<Tick>().0;
    let Some(id) = world.get::<NetId>(creaking).map(|id| id.0) else { return };
    let started = {
        let Some(mut tie) = world.get_mut::<Creaking>(creaking) else { return };
        let Some(bound) = tie.bound.as_mut() else { return };
        bound.crumbling.replace(now).is_none()
    };
    if started {
        tell(world, creaking, vec![properties(id, STATE, CRUMBLING)]);
    }
}

/// A sway's next step, and the crumbling's.
fn sway_and_crumble(world: &mut World, now: u64) {
    let bound: Vec<(Entity, u64, Bound)> = world.query::<(Entity, &NetId, &Creaking)>().iter(world).filter_map(|(entity, id, tie)| tie.bound.map(|bound| (entity, id.0, bound))).collect();
    for (creaking, id, bound) in bound {
        if let Some(sway) = bound.swaying {
            let next = (sway < SWAYS).then_some(sway + 1);
            tell(world, creaking, vec![properties(id, SWAY, next.unwrap_or(0))]);
            set_sway(world, creaking, next);
        }
        let Some(since) = bound.crumbling else { continue };
        let Some(body) = world.get::<Body>(creaking).copied() else { continue };
        if now == since + CRUMBLES {
            let position = Vec3f { x: body.at[0], y: body.at[1] + body.size[1] / 2.0, z: body.at[2] };
            tell(world, creaking, vec![sapling_net::encode(&LevelEvent { event: LevelEventEvent::Unknown(CRUMBLE_EVENT), position, data: 0 })]);
        } else if now == since + CRUMBLES + 1 {
            tell(world, creaking, vec![crate::creatures::hunting::sound("death", id, KIND, &body, 0.0), properties(id, STATE, CRUMBLED)]);
        } else if now > since + CRUMBLES + 1 {
            mobs::remove(world, creaking);
        }
    }
}

fn set_sway(world: &mut World, creaking: Entity, sway: Option<i32>) {
    if let Some(mut tie) = world.get_mut::<Creaking>(creaking)
        && let Some(bound) = tie.bound.as_mut()
    {
        bound.swaying = sway;
    }
}

/// A hit on `creaking`: false unless its heart is awake, when it sways and takes no harm.
pub(in crate::creatures) fn shrugs_off(world: &mut World, creaking: Entity) -> bool {
    let Some(bound) = world.get::<Creaking>(creaking).and_then(|tie| tie.bound) else { return false };
    let layer = world.get::<InLayer>(creaking).map(|layer| layer.0);
    let awake = layer.and_then(|layer| world.get_resource::<Hearts>().and_then(|hearts| hearts.0.get(&(layer, bound.heart))).map(|heart| heart.woke.is_some()));
    if bound.crumbling.is_some() || awake != Some(true) {
        return false;
    }
    if bound.swaying.is_none() {
        let Some(id) = world.get::<NetId>(creaking).map(|id| id.0) else { return true };
        tell(world, creaking, vec![properties(id, SWAY, 1)]);
        set_sway(world, creaking, Some(1));
    }
    true
}

/// Whether `creaking` is crumbling, and stands.
pub(in crate::creatures) fn is_crumbling(world: &World, creaking: Entity) -> bool {
    world.get::<Creaking>(creaking).and_then(|tie| tie.bound).is_some_and(|bound| bound.crumbling.is_some())
}
