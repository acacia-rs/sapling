//! Water freezing and snow lying where it is cold, as BDS's `LevelChunk::tick` does it: a column
//! of each chunk around a player is picked `randomTickSpeed` times a tick and its top looked at
//! (docs/research/growth.md "Light").

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face, Weather};
use sapling_data::RuleValue;
use sapling_world::ChunkPos;

use super::super::{level, send};
use super::Picks;
use crate::components::InLayer;
use crate::view::View;

/// Columns this many chunks across from a player's are ticked.
const REACH: i32 = 4;
/// At most this many picks a column a tick, whatever the rule: a column's top is looked for
/// after every sixteenth.
const MOST_PICKS: i32 = 16;
/// One pick in this many freezes, and one in this many snows while it snows hard enough.
const ODDS: u32 = 16;
/// Block light under these lets water freeze (`Weather::shouldFreeze`) and snow lie
/// (`Weather::canPlaceTopSnow`).
const FREEZE_DIM: u8 = 10;
const SNOW_DIM: u8 = 12;
const ICE: &str = "minecraft:ice";
const SNOW: &str = "minecraft:snow_layer";

type Level = sapling_world::World;

/// The columns around players, each once, in an order that does not turn on the hash.
fn near_players(world: &mut World) -> Vec<(Entity, ChunkPos)> {
    let mut near = Vec::new();
    for (in_layer, view) in world.query::<(&InLayer, &View)>().iter(world) {
        let Some(center) = view.center() else { continue };
        near.extend((-REACH..=REACH).flat_map(|dx| (-REACH..=REACH).map(move |dz| (in_layer.0, ChunkPos::new(center.x + dx, center.z + dz)))));
    }
    near.sort_by_key(|(layer, pos)| (*layer, pos.x, pos.z));
    near.dedup();
    near
}

/// Whether a layer of snow can lie on the block `id`: leaves, or a full top that is not ice.
fn holds_snow(level: &Level, id: u32) -> bool {
    level.registry().get(id).is_some_and(|state| state.name.ends_with("leaves") || state.is_full_cube() && !matches!(state.name, ICE | "minecraft:frosted_ice" | "minecraft:packed_ice" | "minecraft:blue_ice" | "minecraft:barrier"))
}

fn is_still_water(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_water() && state.liquid_depth == 0)
}

fn block_light(level: &Level, [x, y, z]: BlockPos) -> u8 {
    level.light_at(x, y, z).map_or(15, |(_, block)| block)
}

/// What a pick of the block column `(x, z)` sets: ice on still water at its top that is not
/// water on all four sides, or a layer of snow over the top if `snows`; both where it is cold.
fn fallen(level: &Level, x: i32, z: i32, freezes: bool, snows: bool) -> Option<(BlockPos, u32)> {
    let top = level.surface(x, z)?;
    let (ground, over) = ([x, top, z], [x, top + 1, z]);
    let registry = level.registry();
    let is_shore = || [Face::North, Face::South, Face::West, Face::East].iter().any(|face| !is_still_water(level, face.neighbour(ground)));
    if freezes && is_still_water(level, ground) && level.is_cold(x, top, z) && block_light(level, ground) < FREEZE_DIM && is_shore() {
        return send::resolve(registry, &Block::new(ICE)).map(|ice| (ground, ice));
    }
    let is_air = level.in_height(top + 1) && level.block(x, top + 1, z) == registry.air_id();
    if snows && is_air && holds_snow(level, level.block(x, top, z)) && level.is_cold(x, top + 1, z) && block_light(level, over) < SNOW_DIM {
        return send::resolve(registry, &Block::new(SNOW)).map(|snow| (over, snow));
    }
    None
}

/// A tick's picks around every player.
pub(super) fn tick(world: &mut World, picks: &mut Picks) {
    let mut fell = Vec::new();
    for (layer, column) in near_players(world) {
        let Some(level) = level(world, layer).filter(|level| level.is_loaded(column)) else { continue };
        let rules = &level.level().rules;
        let Some((_, RuleValue::Int(speed))) = rules.get("randomTickSpeed") else { continue };
        let snowing = rules.on("doWeatherCycle") && crate::weather::of(world, layer) != Weather::Clear;
        for _ in 0..speed.clamp(0, MOST_PICKS) {
            let [x, z] = [column.x * 16 + picks.below(16) as i32, column.z * 16 + picks.below(16) as i32];
            let (freezes, snows) = (picks.below(ODDS) == 0, snowing && picks.below(ODDS) == 0);
            if freezes || snows {
                fell.extend(fallen(level, x, z, freezes, snows).map(|(pos, id)| (layer, pos, id)));
            }
        }
    }
    for (layer, pos, id) in fell {
        send::set(world, layer, pos, id);
    }
}
