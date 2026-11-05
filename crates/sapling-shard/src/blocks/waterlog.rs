//! Blocks that keep water in layer 1 beside themselves, by BDS's per-block liquid rule
//! (`sapling_data::liquid_detection`) and `LevelChunk::setBlock` (docs/research/waterlogging.md).
// TODO: blocks that keep flowing water or other media when placed (`isPreservingMediumWhenPlaced`) are not looked at.

use sapling_data::{LiquidReaction, liquid_detection};
use sapling_world::BlockRegistry;

const WATER: &str = "minecraft:water";
const SNOW: &str = "minecraft:snow_layer";

/// Whether the block `id` takes the water a bucket pours into it.
pub(super) fn holds_water(registry: &BlockRegistry, id: u32) -> bool {
    registry.get(id).is_some_and(|state| !state.is_air() && liquid_detection(state.name).can_contain)
}

/// What water running into the block `id` does to it.
pub(super) fn reaction(registry: &BlockRegistry, id: u32) -> LiquidReaction {
    registry.get(id).map_or(LiquidReaction::Blocking, |state| liquid_detection(state.name).reaction)
}

fn is_water_source(registry: &BlockRegistry, id: u32) -> bool {
    registry.get(id).is_some_and(|state| state.name == WATER && state.liquid_depth == 0)
}

/// Whether a snow layer written over the block `id` keeps it in layer 1: the small plants BDS's freeze
/// pass snows over (sapling-worldgen block/layers.rs), and `Weather::canPlaceTopSnow`'s test (14119d880).
pub(crate) fn snow_keeps(registry: &BlockRegistry, id: u32) -> bool {
    registry.get(id).and_then(|state| sapling_worldgen::Block::from_name(state.name)).is_some_and(|plant| plant.snowed().is_some())
}

/// What the two layers of a cell hold once `new` is written over `old` and `held` (layer 1):
/// a block that can keep water placed into a source keeps it beside itself; any other block
/// throws it away; air lets the water, or the plant under snow, that was beside a block take the
/// cell; a snow layer keeps a small plant it falls on.
pub(super) fn written(registry: &BlockRegistry, new: u32, old: u32, held: u32) -> (u32, u32) {
    let air = registry.air_id();
    let Some(state) = registry.get(new) else { return (new, held) };
    if state.is_air() {
        return if held != air { (held, air) } else { (air, air) };
    }
    if state.name == SNOW {
        return (new, if snow_keeps(registry, old) { old } else { held });
    }
    if !liquid_detection(state.name).can_contain || state.is_liquid() {
        return (new, air);
    }
    let kept = [old, held].into_iter().find(|id| is_water_source(registry, *id));
    (new, kept.unwrap_or(air))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stairs_keep_a_source_stone_does_not_and_a_break_leaves_the_water() {
        let registry = BlockRegistry::vanilla();
        let id = |name: &str| registry.states_of(name).next().unwrap().0;
        let flowing = registry.find(WATER, "liquid_depth=3").unwrap();
        let (air, water, stairs, stone) = (registry.air_id(), id(WATER), id("minecraft:oak_stairs"), id("minecraft:stone"));
        assert_eq!(written(&registry, stairs, water, air), (stairs, water));
        assert_eq!(written(&registry, stairs, flowing, air), (stairs, air), "flowing water is not kept");
        assert_eq!(written(&registry, stone, water, air), (stone, air));
        assert_eq!(written(&registry, air, stairs, water), (water, air));
        assert_eq!(written(&registry, stone, stairs, water), (stone, air));
    }

    #[test]
    fn snow_keeps_a_small_plant_under_it_and_gives_it_back_when_cleared() {
        let registry = BlockRegistry::vanilla();
        let id = |name: &str| registry.states_of(name).next().unwrap().0;
        let (air, snow, grass, poppy, wheat) = (registry.air_id(), id(SNOW), id("minecraft:short_grass"), id("minecraft:poppy"), id("minecraft:wheat"));
        assert_eq!(written(&registry, snow, grass, air), (snow, grass));
        assert_eq!(written(&registry, snow, poppy, air), (snow, poppy));
        assert_eq!(written(&registry, snow, wheat, air), (snow, air), "a crop is no small plant");
        assert_eq!(written(&registry, air, snow, grass), (grass, air));
    }
}
