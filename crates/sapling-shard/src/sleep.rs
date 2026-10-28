//! Sleeping in a bed, as BDS does it (docs/research/beds-and-containers.md, sections 1 to 4):
//! a click on a bed at night or in a storm lies the player down in it and makes it its spawn,
//! unless a monster is near (rest.rs); once everyone in the world has slept for 100 ticks of a
//! night it is skipped and they all wake, each where stand.rs finds room.
// A sleeper who leaves the game leaves its bed's `occupied_bit`, which the next click clears.

mod rest;
mod spawn;
mod stand;

use std::collections::HashMap;

use acacia_proto::nbt::{Nbt, Value as NbtValue};
use acacia_proto::packets::{Animate, AnimateActionId, LevelEventGeneric, PlayerAction, SetEntityData};
use acacia_proto::types::{
    Action, EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, MetadataFlags2, Vec3f, Vec3i,
};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty, GameMode};
use sapling_data::EYE_HEIGHT;

pub(crate) use spawn::{BedSpawn, Own, set_anchor_spawn, sound_anchor_spent, spawn_of, spend_anchor, tell_lapsed};

use crate::blocks::{bed_head, set_network_only, with_property};
use crate::components::{InLayer, Layer, Mode, NetId, Outgoing, Position, Settings, in_world};
use crate::movement;
use crate::visibility::ToViewers;

const BED: &str = "minecraft:bed";
const SLEEPING_PERCENTAGE: &str = "playersSleepingPercentage";
const OCCUPIED: &str = "occupied_bit";
const DAY: i64 = 24_000;
/// How far from the head of a bed its sleeper's eyes may be, per axis.
const BED_REACH: [f32; 3] = [3.0, 4.0, 3.0];
/// How high in the bed's cell a sleeper's eyes lie (BDS `BedBlock::use_sleep`).
const LYING_HEIGHT: f32 = 0.90625;
/// How far over the floor of its cell a sleeper's feet are put when it gets up.
const STAND_UP_HEIGHT: f32 = 0.1;
/// A sleeper's yaw by its bed's `direction`.
const LYING_YAW: [f32; 4] = [0.0, 90.0, 180.0, -90.0];
/// Where a bed's head lies from its foot, x and z, by its `direction`.
const HEAD_AHEAD: [[i32; 2]; 4] = [[0, 1], [-1, 0], [0, -1], [1, 0]];
/// Ticks of sleep after which a sleeper counts for skipping the night.
const DEEP_SLEEP: u64 = 100;
/// The sleeping bit of `PlayerFlags`.
const SLEEPING_FLAG: i8 = 0x02;
/// The bit of `FlagsExtended` BDS set on a sleeper (acacia-proto names it `WANTS_TO_WAKE`).
const SLEEPING_EXTENDED: i64 = 1 << 12;
/// A player's box, wide and high, lying in a bed and standing.
pub(crate) const LYING_BOX: [f32; 2] = [0.2, 0.2];
const STANDING_BOX: [f32; 2] = [0.6, 1.8];
/// The `LevelEventGeneric` that counts the sleepers (9801). The game zigzags the id where
/// acacia-proto writes a plain varint, so it goes as 19602: BDS's bytes (bds-dims-travel.txt).
const SLEEPING_PLAYERS: u32 = 9801 << 1;

/// A player lying in a bed.
#[derive(Component)]
pub(crate) struct Sleeping {
    layer: Entity,
    head: BlockPos,
    foot: BlockPos,
    direction: usize,
    /// Where it stood before it lay down, which picks where it gets up (stand.rs).
    entered: [f32; 3],
    ticks: u64,
}

/// Tells a player why it did not lie down, or that its spawn is set, as BDS does: the game's own
/// grey line.
fn tell(world: &mut World, player: Entity, key: &str) {
    let packet = crate::chat::json_packet(vec![serde_json::json!({ "text": "§7" }), serde_json::json!({ "translate": key })]);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.push(packet);
    }
}

fn entity_data(id: u64, metadata: Vec<MetadataDictionaryItem>) -> SetEntityData {
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// What a sleeper's viewers and its own client are told: the flags, and the bed it lies in at
/// `head` and the eyes it lay down from (research/captures/bds-wake.txt).
fn lying_data(id: u64, lying: Option<(BlockPos, [f32; 3])>) -> SetEntityData {
    let flags = MetadataDictionaryItem { key: Key::PlayerFlags, r#type: Type::Byte, legacy_type: 0, value: Value::Default(Plain::Byte(if lying.is_some() { SLEEPING_FLAG } else { 0 })) };
    let bed = lying.map(|([x, y, z], _)| MetadataDictionaryItem { key: Key::PlayerBedPosition, r#type: Type::Vec3i, legacy_type: 6, value: Value::Default(Plain::Vec3i(Vec3i { x, y, z })) });
    let extended = MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(sapling_data::player_flags_extended().0 | if lying.is_some() { SLEEPING_EXTENDED } else { 0 })) };
    let entered = lying.map(|(_, [x, y, z])| MetadataDictionaryItem { key: Key::BedEnterPosition, r#type: Type::Vec3f, legacy_type: 8, value: Value::Default(Plain::Vec3f(Vec3f { x, y: y + EYE_HEIGHT, z })) });
    entity_data(id, [flags].into_iter().chain(bed).chain([extended]).chain(entered).collect())
}

/// A player's box, small while it lies in a bed (and dead: vitals/death.rs).
pub(crate) fn box_data(id: u64, lying: bool) -> SetEntityData {
    let [width, height] = if lying { LYING_BOX } else { STANDING_BOX };
    let size = |key, value| MetadataDictionaryItem { key, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(value)) };
    entity_data(id, vec![size(Key::BoundingboxWidth, width), size(Key::BoundingboxHeight, height)])
}

/// Players of the world `layer` that count for skipping its night: all but spectators.
fn counted(world: &mut World, layer: Entity) -> usize {
    let players = in_world(world);
    let counts = |player: &&Entity| world.get::<InLayer>(**player).is_some_and(|in_layer| in_layer.0 == layer) && world.get::<Mode>(**player).is_some_and(|mode| mode.0 != GameMode::Spectator);
    players.iter().filter(counts).count()
}

/// Sleepers that skip the night of the world `layer`: the world's `playersSleepingPercentage` of
/// those [`counted`], rounded up, at least one.
fn needed(world: &mut World, layer: Entity) -> usize {
    let percent = world.get::<Layer>(layer).and_then(|layer| match layer.world.level().rules.get(SLEEPING_PERCENTAGE) {
        Some((_, sapling_data::RuleValue::Int(percent))) => Some(percent.clamp(0, 100) as usize),
        _ => None,
    });
    (counted(world, layer) * percent.unwrap_or(100)).div_ceil(100).max(1)
}

/// Tells a world's players, and those in its other dimensions, how many of how many sleep (BDS
/// `updateSleepingPlayerList`, also sent on a dimension change).
pub(crate) fn tell_sleepers(world: &mut World, layer: Entity) {
    let mut sleepers = world.query::<&Sleeping>();
    let sleeping = sleepers.iter(world).filter(|sleeping| sleeping.layer == layer).count();
    let required = counted(world, layer).max(1);
    let int = |name: &str, value: usize| Nbt { name: name.to_owned(), value: NbtValue::Int(value as i32) };
    let nbt = vec![int("ableToSleep", 1), int("overworldPlayerCount", required), int("sleepingPlayerCount", sleeping)];
    let told = sapling_net::encode(&LevelEventGeneric { event_id: SLEEPING_PLAYERS, nbt });
    crate::dimensions::members(world, layer).into_iter().for_each(|member| crate::chat::send_to_layer(world, member, &told));
}

/// Shows a player's lying down in the bed at `head` from `entered` (its feet), or getting up, to
/// it and to those who see it.
fn show(world: &mut World, player: Entity, lying: Option<(BlockPos, [f32; 3])>) {
    if let Some(id) = world.get::<NetId>(player).map(|id| id.0) {
        tell_all(world, player, &lying_data(id, lying));
    }
}

/// Shows a player's box, lying or standing, to it and to those who see it.
fn show_box(world: &mut World, player: Entity, lying: bool) {
    if let Some(id) = world.get::<NetId>(player).map(|id| id.0) {
        tell_all(world, player, &box_data(id, lying));
    }
}

fn tell_all(world: &mut World, player: Entity, data: &SetEntityData) {
    world.resource_mut::<ToViewers>().push(player, data);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(data);
    }
}

fn set_occupied(world: &mut World, layer: Entity, head: BlockPos, occupied: bool) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let id = level.block(head[0], head[1], head[2]);
    if level.registry().get(id).is_some_and(|state| state.name == BED) {
        if let Some(new) = with_property(level.registry(), id, OCCUPIED, if occupied { "1" } else { "0" }).filter(|new| *new != id) {
            set_network_only(world, layer, head, new);
        }
    }
}

/// A click on the block `name` at `pos`: a bed lies the player down in it if it may (BDS
/// `BedBlock::use_sleep`, checks in its order). False for any other block.
pub(crate) fn lie_down(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let Some(head) = (name == BED).then(|| bed_head(world, layer, pos)).flatten() else { return name == BED };
    let occupied = world.get::<Layer>(layer).is_some_and(|level| {
        let id = level.world.block(head[0], head[1], head[2]);
        level.world.registry().get(id).and_then(|state| state.property(OCCUPIED)) == Some("1")
    });
    if occupied {
        let mut sleepers = world.query::<&Sleeping>();
        if sleepers.iter(world).any(|sleeping| sleeping.layer == layer && sleeping.head == head) {
            tell(world, player, "tile.bed.occupied");
            return true;
        }
        set_occupied(world, layer, head, false);
    }
    let Some(position) = world.get::<Position>(player).copied() else { return true };
    let eye = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    if (0..3).any(|axis| (eye[axis] - head[axis] as f32).abs() > BED_REACH[axis]) {
        tell(world, player, "tile.bed.tooFar");
        return true;
    }
    if world.get::<Sleeping>(player).is_some() {
        return true;
    }
    let direction = world.get::<Layer>(layer).and_then(|level| level.world.registry().get(level.world.block(head[0], head[1], head[2]))?.property("direction")?.parse::<usize>().ok()).unwrap_or(0) % 4;
    let foot = [head[0] - HEAD_AHEAD[direction][0], head[1], head[2] - HEAD_AHEAD[direction][1]];
    let roomy = world.get::<Layer>(layer).is_some_and(|level| stand::stand_up_cell(&level.world, foot, direction, position.feet).is_some());
    if !roomy {
        tell(world, player, "tile.bed.obstructed");
        return true;
    }
    spawn::set_spawn(world, player, head);
    if !rest::is_resting_time(world, layer) {
        tell(world, player, "tile.bed.noSleep");
        return true;
    }
    let watched = world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) && world.resource::<Settings>().0.difficulty != Difficulty::Peaceful;
    if watched && rest::monster_near(world, layer, head) {
        tell(world, player, "tile.bed.notSafe");
        return true;
    }
    let yaw = LYING_YAW[direction];
    if let Some(mut lying) = world.get_mut::<Position>(player) {
        (lying.yaw, lying.head_yaw, lying.pitch) = (yaw, yaw, 0.0);
    }
    world.entity_mut(player).insert(Sleeping { layer, head, foot, direction, entered: position.feet, ticks: 0 });
    show(world, player, Some((head, position.feet)));
    movement::correct_unmoving(world, player, [head[0] as f32 + 0.5, head[1] as f32 + LYING_HEIGHT - EYE_HEIGHT, head[2] as f32 + 0.5]);
    show_box(world, player, true);
    set_occupied(world, layer, head, true);
    tell_sleepers(world, layer);
    true
}

/// Gets a sleeper up: its client is told to wake, the bed is free again and it stands in the
/// middle of the cell stand.rs picks, a little above its floor, its prediction corrected as
/// BDS's `Player::stopSleepInBed` puts it (research/captures/bds-wake.txt).
fn wake(world: &mut World, player: Entity) {
    get_up(world, player, None);
}

/// Teleports a player's feet to `feet` as `/tp` does, waking it first if it sleeps: BDS's `/tp`
/// got a sleeper up where it sent it (research/captures/bds-wake.txt).
pub(crate) fn teleport_waking(world: &mut World, player: Entity, feet: [f32; 3]) {
    if !get_up(world, player, Some(feet)) {
        movement::teleport(world, player, feet);
    }
}

/// [`wake`], or a wake by a teleport to `sent_to`; false for a player not asleep.
fn get_up(world: &mut World, player: Entity, sent_to: Option<[f32; 3]>) -> bool {
    let Some(sleeping) = world.entity_mut(player).take::<Sleeping>() else { return false };
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return true };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&Animate { action_id: AnimateActionId::WakeUp, runtime_entity_id: id, data: 0.0, has_swing_source: false, swing_source: None });
    }
    // A correction a tick after the teleport came in one BDS run of two: not the wake's own.
    match sent_to {
        Some(feet) => movement::teleport(world, player, feet),
        None => {
            let cell = world.get::<Layer>(sleeping.layer).and_then(|level| stand::stand_up_cell(&level.world, sleeping.foot, sleeping.direction, sleeping.entered));
            // None: BDS stands it on the bed.
            let [x, y, z] = cell.unwrap_or([sleeping.foot[0], sleeping.foot[1] + 1, sleeping.foot[2]]).map(|c| c as f32);
            movement::correct(world, player, [x + 0.5, y + STAND_UP_HEIGHT, z + 0.5]);
        }
    }
    show(world, player, None);
    set_occupied(world, sleeping.layer, sleeping.head, false);
    show_box(world, player, false);
    tell_sleepers(world, sleeping.layer);
    true
}

/// Whether the bed a sleeper lies in is still there.
fn bed_is_there(world: &World, sleeping: &Sleeping) -> bool {
    let [x, y, z] = sleeping.head;
    world.get::<Layer>(sleeping.layer).is_some_and(|layer| layer.world.registry().get(layer.world.block(x, y, z)).is_some_and(|state| state.name == BED))
}

/// Gets up the players whose client left its bed, whose bed went or whose night ended; counts
/// the others' sleep, and skips the night of a world whose players have all slept long enough.
pub(crate) fn tick(world: &mut World, actions: &[(Entity, PlayerAction)]) {
    for (player, _) in actions.iter().filter(|(_, action)| action.action == Action::StopSleeping) {
        wake(world, *player);
    }
    let mut sleepers = world.query::<(Entity, &mut Sleeping)>();
    let mut awake = Vec::new();
    let mut deep: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let night: HashMap<Entity, bool> = world.query::<(Entity, &Layer)>().iter(world).map(|(layer, _)| (layer, rest::is_resting_time(world, layer))).collect();
    let still: Vec<(Entity, bool)> = sleepers.iter(world).map(|(player, sleeping)| (player, bed_is_there(world, sleeping) && night.get(&sleeping.layer) == Some(&true))).collect();
    for (player, stays) in still {
        let Ok((_, mut sleeping)) = sleepers.get_mut(world, player) else { continue };
        if !stays {
            awake.push(player);
            continue;
        }
        sleeping.ticks += 1;
        if sleeping.ticks >= DEEP_SLEEP {
            deep.entry(sleeping.layer).or_default().push(player);
        }
    }
    awake.into_iter().for_each(|player| wake(world, player));
    for (layer, sleepers) in deep {
        if sleepers.len() < needed(world, layer) || !rest::is_night(world, layer) {
            continue;
        }
        let mut all = world.query::<(Entity, &Sleeping)>();
        let everyone: Vec<Entity> = all.iter(world).filter(|(_, sleeping)| sleeping.layer == layer).map(|(player, _)| player).collect();
        everyone.into_iter().for_each(|player| wake(world, player));
        let time = world.get::<Layer>(layer).map_or(0, |layer| layer.world.level().time);
        crate::clock::set(world, layer, (time + DAY) / DAY * DAY);
        crate::weather::stop(world, layer);
    }
}
