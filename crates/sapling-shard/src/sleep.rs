//! Sleeping in a bed, as BDS does it (docs/research/beds-and-containers.md, sections 1 to 4):
//! a click on a bed at night lies the player down in it and makes it its spawn; once everyone
//! in the world has slept for 100 ticks the night is skipped and they all wake.
// TODO: no monsters keep anyone awake (mobs are statues), no weather, no stand-up search
// (a sleeper wakes where it lay down), and the spawn is not used: nobody dies or respawns.
// A sleeper who leaves the game leaves its bed's `occupied_bit`, which the next click clears.

use std::collections::HashMap;

use acacia_proto::nbt::{Nbt, Value as NbtValue};
use acacia_proto::packets::{Animate, AnimateActionId, LevelEventGeneric, PlayerAction, SetEntityData, SetSpawnPosition, SetSpawnPositionSpawnType};
use acacia_proto::types::{
    Action, BlockCoordinates, EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type,
    MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, Vec3i,
};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};
use sapling_data::EYE_HEIGHT;

use crate::blocks::{bed_head, set, with_property};
use crate::components::{InLayer, Layer, Mode, NetId, Outgoing, PacketQueues, Position, in_world};
use crate::movement;
use crate::visibility::ToViewers;

const BED: &str = "minecraft:bed";
const OCCUPIED: &str = "occupied_bit";
/// The ticks of a clear day in which one may sleep: the sky is dark enough (BDS `skyDarken` 4).
const NIGHT: std::ops::RangeInclusive<i64> = 12_541..=23_459;
const DAY: i64 = 24_000;
/// How far from the head of a bed its sleeper's eyes may be, per axis.
const BED_REACH: [f32; 3] = [3.0, 4.0, 3.0];
/// How high in the bed's cell a sleeper's eyes lie (BDS `BedBlock::use_sleep`).
const LYING_HEIGHT: f32 = 0.90625;
/// A sleeper's yaw by its bed's `direction`.
const LYING_YAW: [f32; 4] = [0.0, 90.0, 180.0, -90.0];
/// Ticks of sleep after which a sleeper counts for skipping the night.
const DEEP_SLEEP: u64 = 100;
/// The sleeping bit of `PlayerFlags`.
const SLEEPING_FLAG: i8 = 0x02;
/// The `LevelEventGeneric` that counts the sleepers.
const SLEEPING_PLAYERS: u32 = 9801;

/// A player lying in a bed.
#[derive(Component)]
pub(crate) struct Sleeping {
    layer: Entity,
    head: BlockPos,
    /// Where it stood before it lay down, and gets up again.
    entered: [f32; 3],
    ticks: u64,
}

/// The bed a player last slept in, its spawn while it stays.
#[derive(Component, PartialEq)]
pub(crate) struct BedSpawn(BlockPos);

fn coordinates([x, y, z]: BlockPos) -> BlockCoordinates {
    BlockCoordinates { x, y, z }
}

/// Tells a player why it did not lie down, as BDS does: the game's own grey line.
fn tell(world: &mut World, player: Entity, key: &str) {
    let packet = crate::chat::json_packet(vec![serde_json::json!({ "text": "§7" }), serde_json::json!({ "translate": format!("tile.bed.{key}") })]);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.push(packet);
    }
}

/// What a sleeper's viewers and its own client are told: the flag, and the bed it lies in.
fn lying_data(id: u64, head: Option<BlockPos>) -> SetEntityData {
    let flags = MetadataDictionaryItem { key: Key::PlayerFlags, r#type: Type::Byte, legacy_type: 0, value: Value::Default(Plain::Byte(if head.is_some() { SLEEPING_FLAG } else { 0 })) };
    let bed = head.map(|[x, y, z]| MetadataDictionaryItem { key: Key::PlayerBedPosition, r#type: Type::Vec3i, legacy_type: 6, value: Value::Default(Plain::Vec3i(Vec3i { x, y, z })) });
    SetEntityData { runtime_entity_id: id, metadata: [flags].into_iter().chain(bed).collect(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// Players of the world `layer` that count for skipping its night: all but spectators.
fn counted(world: &mut World, layer: Entity) -> usize {
    let players = in_world(world);
    let counts = |player: &&Entity| world.get::<InLayer>(**player).is_some_and(|in_layer| in_layer.0 == layer) && world.get::<Mode>(**player).is_some_and(|mode| mode.0 != GameMode::Spectator);
    players.iter().filter(counts).count()
}

/// Tells a world's players how many of how many sleep (BDS `updateSleepingPlayerList`).
fn tell_sleepers(world: &mut World, layer: Entity) {
    let mut sleepers = world.query::<&Sleeping>();
    let sleeping = sleepers.iter(world).filter(|sleeping| sleeping.layer == layer).count();
    let required = counted(world, layer).max(1);
    let int = |name: &str, value: usize| Nbt { name: name.to_owned(), value: NbtValue::Int(value as i32) };
    let nbt = vec![int("ableToSleep", 1), int("overworldPlayerCount", required), int("sleepingPlayerCount", sleeping)];
    crate::chat::send_to_layer(world, layer, &sapling_net::encode(&LevelEventGeneric { event_id: SLEEPING_PLAYERS, nbt }));
}

/// Shows a player's lying down or getting up to it and to those who see it.
fn show(world: &mut World, player: Entity, head: Option<BlockPos>) {
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return };
    let data = lying_data(id, head);
    world.resource_mut::<ToViewers>().push(player, &data);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&data);
    }
}

fn set_occupied(world: &mut World, layer: Entity, head: BlockPos, occupied: bool) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let id = level.block(head[0], head[1], head[2]);
    if level.registry().get(id).is_some_and(|state| state.name == BED) {
        if let Some(new) = with_property(level.registry(), id, OCCUPIED, if occupied { "1" } else { "0" }).filter(|new| *new != id) {
            set(world, layer, head, new);
        }
    }
}

fn is_night(world: &World, layer: Entity) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| NIGHT.contains(&layer.world.level().time.rem_euclid(DAY)))
}

/// Makes the bed at `head` a player's spawn, telling it when that is news.
fn set_spawn(world: &mut World, player: Entity, head: BlockPos) {
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet.map(|c| c.floor() as i32)) else { return };
    let is_new = world.get::<BedSpawn>(player) != Some(&BedSpawn(head));
    let packet = SetSpawnPosition { spawn_type: SetSpawnPositionSpawnType::Player, player_position: coordinates(feet), dimension: 0, world_position: coordinates(head) };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&packet);
    }
    world.entity_mut(player).insert(BedSpawn(head));
    if is_new {
        tell(world, player, "respawnSet");
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
            tell(world, player, "occupied");
            return true;
        }
        set_occupied(world, layer, head, false);
    }
    let Some(position) = world.get::<Position>(player).copied() else { return true };
    let eye = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    if (0..3).any(|axis| (eye[axis] - head[axis] as f32).abs() > BED_REACH[axis]) {
        tell(world, player, "tooFar");
        return true;
    }
    if world.get::<Sleeping>(player).is_some() {
        return true;
    }
    set_spawn(world, player, head);
    if !is_night(world, layer) {
        tell(world, player, "noSleep");
        return true;
    }
    let direction = world.get::<Layer>(layer).and_then(|level| level.world.registry().get(level.world.block(head[0], head[1], head[2]))?.property("direction")?.parse::<usize>().ok());
    let yaw = direction.and_then(|direction| LYING_YAW.get(direction)).copied().unwrap_or(position.yaw);
    if let Some(mut lying) = world.get_mut::<Position>(player) {
        lying.feet = [head[0] as f32 + 0.5, head[1] as f32 + LYING_HEIGHT - EYE_HEIGHT, head[2] as f32 + 0.5];
        (lying.yaw, lying.head_yaw, lying.pitch) = (yaw, yaw, 0.0);
    }
    world.entity_mut(player).insert(Sleeping { layer, head, entered: position.feet, ticks: 0 });
    set_occupied(world, layer, head, true);
    show(world, player, Some(head));
    tell_sleepers(world, layer);
    true
}

/// Gets a sleeper up: its client is told to wake, the bed is free again and it stands where it
/// lay down from.
fn wake(world: &mut World, player: Entity) {
    let Some(sleeping) = world.entity_mut(player).take::<Sleeping>() else { return };
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&Animate { action_id: AnimateActionId::WakeUp, runtime_entity_id: id, data: 0.0, has_swing_source: false, swing_source: None });
    }
    set_occupied(world, sleeping.layer, sleeping.head, false);
    show(world, player, None);
    movement::teleport(world, player, sleeping.entered);
    tell_sleepers(world, sleeping.layer);
}

/// Whether the bed a sleeper lies in is still there.
fn bed_is_there(world: &World, sleeping: &Sleeping) -> bool {
    let [x, y, z] = sleeping.head;
    world.get::<Layer>(sleeping.layer).is_some_and(|layer| layer.world.registry().get(layer.world.block(x, y, z)).is_some_and(|state| state.name == BED))
}

/// Gets up the players whose client left its bed, whose bed went or whose night ended; counts
/// the others' sleep, and skips the night of a world whose players have all slept long enough.
pub(crate) fn tick(world: &mut World) {
    let left_bed = world.resource_mut::<PacketQueues>().take::<PlayerAction>();
    for (player, _) in left_bed.into_iter().filter(|(_, action)| action.action == Action::StopSleeping) {
        wake(world, player);
    }
    let mut sleepers = world.query::<(Entity, &mut Sleeping)>();
    let mut awake = Vec::new();
    let mut deep: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let night: HashMap<Entity, bool> = world.query::<(Entity, &Layer)>().iter(world).map(|(layer, _)| (layer, is_night(world, layer))).collect();
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
        if sleepers.len() < counted(world, layer).max(1) {
            continue;
        }
        let mut all = world.query::<(Entity, &Sleeping)>();
        let everyone: Vec<Entity> = all.iter(world).filter(|(_, sleeping)| sleeping.layer == layer).map(|(player, _)| player).collect();
        everyone.into_iter().for_each(|player| wake(world, player));
        let time = world.get::<Layer>(layer).map_or(0, |layer| layer.world.level().time);
        crate::clock::set(world, layer, (time + DAY) / DAY * DAY);
    }
}
