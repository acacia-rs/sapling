//! Water freezing, snow lying where it is cold and rain filling cauldrons, as BDS's
//! `LevelChunk::tick` does it: a column
//! of each chunk that ticks (a player's tick region or a kept column) is picked `randomTickSpeed` times a tick and its top looked at
//! (docs/research/growth.md "Light").

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face};
use sapling_data::RuleValue;

use super::super::{level, send};
use super::Picks;
use crate::ticking::ticked_columns;

/// One pick in this many freezes.
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
    let registry = level.registry();
    // A thin layer of snow has no collision, so `surface` would look past it and never pile it.
    let top = level.top_where(x, z, |id| registry.get(id).is_some_and(|state| state.is_solid() || state.is_liquid() || state.name == SNOW))?;
    let (ground, over) = ([x, top, z], [x, top + 1, z]);
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

/// The sides of the die whose 0 lays snow where `downfall` (the rain's level times the biome's
/// downfall) falls: `256 - min(512 x downfall, 240)` in `LevelChunk::tick`, 16 from 0.47 up.
fn snow_sides(downfall: f32) -> u32 {
    (256 - ((downfall * 512.0) as i32).min(240)) as u32
}

/// Whether rain or snow falling at `downfall` fills a cauldron it falls on:
/// `CauldronBlock::handlePrecipitation` (141a46850) leaves it while `downfall < nextFloat() * 0.5`.
fn fills(downfall: f32, picks: &mut Picks) -> bool {
    downfall >= picks.below(1 << 24) as f32 / (1 << 24) as f32 * 0.5
}

/// A tick's picks in every ticked column (`ticked_columns`): `randomTickSpeed` columns a chunk, as `LevelChunk::tick`
/// (1409f98a0) loops.
pub(super) fn tick(world: &mut World, picks: &mut Picks) {
    let mut fell = Vec::new();
    for (layer, column) in ticked_columns(world) {
        let Some(level) = level(world, layer).filter(|level| level.is_loaded(column)) else { continue };
        let rules = &level.level().rules;
        let Some((_, RuleValue::Int(speed))) = rules.get("randomTickSpeed") else { continue };
        let rain = crate::weather::falling(world, layer).rain;
        let lays = rules.on("doWeatherCycle");
        for _ in 0..speed.max(0) {
            let [x, z] = [column.x * 16 + picks.below(16) as i32, column.z * 16 + picks.below(16) as i32];
            let freezes = picks.below(ODDS) == 0;
            // Nothing else is drawn on a dry pick that does not freeze: skipping it keeps high rules cheap.
            if !freezes && rain <= 0.0 {
                continue;
            }
            let top = level.surface(x, z);
            let downfall = top.and_then(|top| level.biome(x, top + 1, z)).map_or(0.0, |biome| rain * sapling_data::biome_downfall(biome));
            let snows = downfall > 0.0 && picks.below(snow_sides(downfall)) == 0 && lays;
            if freezes || snows {
                fell.extend(fallen(level, x, z, freezes, snows).map(|(pos, id)| (layer, pos, id)));
            }
            if let Some((top, filled)) = top.filter(|_| downfall > 0.0).and_then(|top| Some((top, super::super::cauldrons::fallen_in(level, [x, top, z])?))) {
                if fills(downfall, picks) {
                    fell.push((layer, [x, top, z], filled));
                }
            }
        }
    }
    for (layer, pos, id) in fell {
        send::set(world, layer, pos, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snow_lies_one_pick_in_sixteen_from_downfall_047_and_more_rarely_under() {
        assert_eq!([snow_sides(1.0), snow_sides(0.47), snow_sides(0.4), snow_sides(0.01)], [16, 16, 52, 251]);
    }

    #[test]
    fn rain_fills_a_cauldron_at_twice_its_downfall() {
        let mut picks = Picks::default();
        let share = |downfall: f32, picks: &mut Picks| (0..10_000).filter(|_| fills(downfall, picks)).count();
        let [plains, light, full] = [share(0.4, &mut picks), share(0.1, &mut picks), share(0.5, &mut picks)];
        assert!((7_700..8_300).contains(&plains) && (1_800..2_200).contains(&light) && full == 10_000, "{plains} {light} {full}");
    }
}
