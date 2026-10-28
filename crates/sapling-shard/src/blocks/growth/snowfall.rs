//! Water freezing, snow lying where it is cold and rain filling cauldrons, as BDS's
//! `LevelChunk::tick` does it: a column
//! of each chunk around a player is picked `randomTickSpeed` times a tick and its top looked at
//! (docs/research/growth.md "Light").

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face, Weather};
use sapling_data::RuleValue;

use super::super::{level, send};
use super::Picks;
use crate::ticking::near_players;

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
/// A snow layer's `height` when it is a whole block, and the layers that is.
const FULL: u8 = 7;
const LAYERS: u8 = 8;
/// Biomes whose snow piles past one layer, and how far: `snow_accumulation`'s top times 8 in
/// BDS's vanilla biome definitions (behavior_packs/vanilla biomes.brarchive); one elsewhere.
const PILES: [(&str, u8); 15] = [
    ("cold_beach", 2),
    ("cold_taiga", 4),
    ("cold_taiga_hills", 4),
    ("deep_frozen_ocean", 2),
    ("extreme_hills", 2),
    ("extreme_hills_edge", 2),
    ("frozen_ocean", 2),
    ("frozen_river", 2),
    ("ice_mountains", 12),
    ("ice_plains", 8),
    ("ice_plains_spikes", 12),
    ("legacy_frozen_ocean", 2),
    ("stone_beach", 2),
    ("taiga", 2),
    ("taiga_hills", 2),
];

type Level = sapling_world::World;

/// Whether a layer of snow can lie on the block `id`: leaves, or a full top that is not ice.
pub(crate) fn holds_snow(level: &Level, id: u32) -> bool {
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
    if !snows {
        return None;
    }
    let lies = |pos: BlockPos| level.is_cold(pos[0], pos[1], pos[2]) && block_light(level, pos) < SNOW_DIM;
    if let Some(height) = snow_height(level, ground) {
        let depth = height + 1 + if snow_height(level, [x, top - 1, z]) == Some(FULL) { LAYERS } else { 0 };
        return (height < FULL && depth < most_layers(level, ground) && lies(ground)).then(|| registry.find(SNOW, &format!("covered_bit=0,height={}", height + 1)).map(|snow| (ground, snow))).flatten();
    }
    let is_air = level.in_height(top + 1) && level.block(x, top + 1, z) == registry.air_id();
    if is_air && holds_snow(level, level.block(x, top, z)) && lies(over) {
        return send::resolve(registry, &Block::new(SNOW)).map(|snow| (over, snow));
    }
    None
}

/// The `height` of the snow layer at `pos`, 0 for one layer.
fn snow_height(level: &Level, [x, y, z]: BlockPos) -> Option<u8> {
    let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().filter(|state| state.name == SNOW)?;
    state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == "height").and_then(|(_, value)| value.parse().ok())
}

/// How many layers snow piles to in the biome at `pos`: its `snow_accumulation`'s top times 8.
fn most_layers(level: &Level, [x, y, z]: BlockPos) -> u8 {
    let biome = level.biome(x, y, z);
    PILES.iter().find(|(name, _)| sapling_data::biome_id(name) == biome && biome.is_some()).map_or(1, |(_, layers)| *layers)
}

/// A tick's picks around every player.
pub(super) fn tick(world: &mut World, picks: &mut Picks) {
    let mut fell = Vec::new();
    for (layer, column) in near_players(world) {
        let Some(level) = level(world, layer).filter(|level| level.is_loaded(column)) else { continue };
        let rules = &level.level().rules;
        let Some((_, RuleValue::Int(speed))) = rules.get("randomTickSpeed") else { continue };
        let raining = crate::weather::of(world, layer) != Weather::Clear;
        let snowing = rules.on("doWeatherCycle") && raining;
        for _ in 0..speed.clamp(0, MOST_PICKS) {
            let [x, z] = [column.x * 16 + picks.below(16) as i32, column.z * 16 + picks.below(16) as i32];
            // TODO: how often BDS's picks fill a cauldron is not measured; here every pick on one does.
            if let Some(filled) = level.surface(x, z).filter(|_| raining).and_then(|top| Some((top, super::super::cauldrons::rained_in(level, [x, top, z])?))) {
                fell.push((layer, [x, filled.0, z], filled.1));
            }
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
