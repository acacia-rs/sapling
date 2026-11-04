//! A jukebox's song, as `JukeboxBlockActor::tick` (1457fcab0) plays it: ticks played count up while
//! a disc is in, and every 20th tick, once they are past the song's length (seconds at 0.05 a tick,
//! in f32: disc 11's 71 s ended at 72, research/captures/bds-e8juke.txt), the record is finished and
//! the jukebox gives no more power. BDS shows nothing at the end; the hopper's take shows it.

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{FINISHED, TICKS, disc, held};
use crate::blocks::{fields, redstone};
use crate::components::Layer;

/// Ticks between the checks of a song's end.
const CHECK: u64 = 20;
const SECONDS_PER_TICK: f32 = 0.05;

/// Ticks played by each jukebox whose song plays.
#[derive(Resource, Default)]
struct Songs(HashMap<(Entity, BlockPos), u64>);

/// Starts the song of the jukebox at `pos` from its beginning, or forgets it (`false`).
pub(super) fn playing(world: &mut World, layer: Entity, pos: BlockPos, plays: bool) {
    let mut songs = world.get_resource_or_insert_with(Songs::default);
    if plays {
        songs.0.insert((layer, pos), 0);
    } else {
        songs.0.remove(&(layer, pos));
    }
}

/// Goes on with the song of a jukebox loaded from storage, from the ticks its entity kept.
pub(super) fn resume(world: &mut World, layer: Entity, pos: BlockPos) {
    let played = match fields::get(world, layer, pos, TICKS) {
        Some(Value::Long(ticks)) => ticks.max(0) as u64,
        _ => 0,
    };
    world.get_resource_or_insert_with(Songs::default).0.entry((layer, pos)).or_insert(played);
}

/// Plays every song a tick on, ending those past their length.
pub(crate) fn tick(world: &mut World) {
    let Some(mut songs) = world.get_resource_mut::<Songs>() else { return };
    let checked: Vec<((Entity, BlockPos), u64)> = songs.0.iter_mut().filter_map(|(key, played)| {
        *played += 1;
        (*played % CHECK == 0).then_some((*key, *played))
    }).collect();
    for ((layer, pos), played) in checked {
        let length = held(world, layer, pos).and_then(|held| disc(&held.name)).map(|(.., seconds)| seconds);
        let Some(length) = length.filter(|_| super::is_jukebox(world, layer, pos)) else {
            world.resource_mut::<Songs>().0.remove(&(layer, pos));
            continue;
        };
        let over = played as f32 * SECONDS_PER_TICK > length;
        write_quietly(world, layer, pos, played, over);
        if over {
            world.resource_mut::<Songs>().0.remove(&(layer, pos));
            redstone::emit(world, layer, pos, 0);
        }
    }
}

/// Keeps the ticks played, and the record finished, in the entity without showing it.
fn write_quietly(world: &mut World, layer: Entity, pos: BlockPos, played: u64, over: bool) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    let Some(Value::Compound(mut entity)) = level.world.block_entity(pos).map(|entity| entity.nbt.clone()) else { return };
    entity.retain(|(key, _)| key != TICKS && !(over && key == FINISHED));
    entity.push((TICKS.into(), Value::Long(played as i64)));
    if over {
        entity.push((FINISHED.into(), Value::Byte(1)));
    }
    _ = level.world.set_block_entity(pos, &Value::Compound(entity));
}
