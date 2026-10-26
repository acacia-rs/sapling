//! What crops, cane, berries, cocoa, vines, redstone and rails need to stand on, by BDS's
//! `mayPlace` / `canSurvive` of each (docs/research/standing-and-shapes.md, part A). Blocks
//! not named here need only what updates.rs asks.
// TODO: which blocks give a face's centre or edge support is approximated from collision
// boxes; the dirt and sand tags are Java's lists. Light is asked only on a neighbour's change
// ([`is_wrongly_lit`]), not on placing.

use sapling_api::BlockPos;
use sapling_world::{BlockRegistry, World};

const CROPS: &[&str] = &[
    "minecraft:wheat", "minecraft:carrots", "minecraft:potatoes", "minecraft:beetroot", "minecraft:torchflower_crop", "minecraft:pitcher_crop", "minecraft:melon_stem",
    "minecraft:pumpkin_stem",
];
/// What a sweet berry bush grows in (BDS `BushBlock` list 0x14ccd3328).
const VEGETATION: &[&str] = &[
    "minecraft:dirt", "minecraft:grass_block", "minecraft:podzol", "minecraft:coarse_dirt", "minecraft:mycelium", "minecraft:dirt_with_roots", "minecraft:moss_block",
    "minecraft:pale_moss_block", "minecraft:mud", "minecraft:muddy_mangrove_roots", "minecraft:farmland",
];
/// The `dirt` and `sand` tags sugar cane asks for, less farmland: Java's lists.
const CANE_GROUND: &[&str] = &[
    "minecraft:dirt", "minecraft:grass_block", "minecraft:podzol", "minecraft:coarse_dirt", "minecraft:mycelium", "minecraft:dirt_with_roots", "minecraft:moss_block",
    "minecraft:pale_moss_block", "minecraft:mud", "minecraft:muddy_mangrove_roots", "minecraft:sand", "minecraft:red_sand", "minecraft:suspicious_sand",
];
const REEDS: &str = "minecraft:reeds";
/// What lets redstone dust lie on it without a top to lie on.
const LIT_FROM_BELOW: &[&str] = &["minecraft:glowstone", "minecraft:redstone_lamp", "minecraft:lit_redstone_lamp"];
const DIODES: &[&str] = &["minecraft:unpowered_repeater", "minecraft:powered_repeater", "minecraft:unpowered_comparator", "minecraft:powered_comparator"];
const RAILS: &[&str] = &["minecraft:rail", "minecraft:golden_rail", "minecraft:detector_rail", "minecraft:activator_rail"];
const VINES: &[&str] = &["minecraft:cave_vines", "minecraft:cave_vines_body_with_berries", "minecraft:cave_vines_head_with_berries"];
/// A cocoa pod's `direction` names the side its log is on: 0 south, 1 west, 2 north, 3 east.
const COCOA_LOG: [[i32; 2]; 4] = [[0, 1], [-1, 0], [0, -1], [1, 0]];
/// A rising rail's `rail_direction` (2 to 5) and the side it rises towards.
const RISES_TO: [(&str, [i32; 2]); 4] = [("2", [1, 0]), ("3", [-1, 0]), ("4", [0, -1]), ("5", [0, 1])];

fn name(level: &World, [x, y, z]: BlockPos) -> Option<&'static str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z)).map(|state| state.name)).flatten()
}

fn below([x, y, z]: BlockPos) -> BlockPos {
    [x, y - 1, z]
}

fn property(registry: &BlockRegistry, id: u32, key: &str) -> Option<&'static str> {
    registry.get(id)?.property(key)
}

/// Whether the cell holds water or lava, in either layer.
fn is_wet(level: &World, [x, y, z]: BlockPos) -> bool {
    let registry = level.registry();
    level.in_height(y) && [level.block(x, y, z), level.liquid(x, y, z)].iter().any(|id| registry.get(*id).is_some_and(|state| state.is_liquid()))
}

/// The state a block put at `pos` takes from what is there: a sea pickle out of water is dead
/// (research/captures/bds-placing.txt).
pub(super) fn settled(level: &World, pos: BlockPos, id: u32) -> u32 {
    let registry = level.registry();
    let is_pickle = registry.get(id).is_some_and(|state| state.name == "minecraft:sea_pickle");
    let dead = if is_water(level, pos) { "0" } else { "1" };
    Some(id).filter(|_| is_pickle).and_then(|id| super::with_property(registry, id, "dead_bit", dead)).unwrap_or(id)
}

fn is_water(level: &World, [x, y, z]: BlockPos) -> bool {
    let registry = level.registry();
    [level.block(x, y, z), level.liquid(x, y, z)].iter().any(|id| registry.get(*id).is_some_and(|state| state.is_water() || state.name == "minecraft:frosted_ice"))
}

/// Blocks whose box covers the middle of their top but which hold no dust on BDS
/// (research/captures/bds-conductors.txt), by name ending.
const NO_CENTRE: &[&str] = &["leaves", "shulker_box", "anvil", "_wall", "_fence", "glass_pane", "iron_bars", "grindstone"];
/// Lower blocks that hold dust on BDS.
const LOW_CENTRE: &[&str] = &["minecraft:soul_sand", "minecraft:mud"];

/// Whether the top of the block at `pos` holds something up: under the middle of the face
/// (BDS support "centre"), or across all of it ("edge").
fn has_top(level: &World, [x, y, z]: BlockPos, whole: bool) -> bool {
    let Some(state) = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten() else { return false };
    if !whole && (LOW_CENTRE.contains(&state.name) || NO_CENTRE.iter().any(|end| state.name.ends_with(end))) {
        return LOW_CENTRE.contains(&state.name);
    }
    let covers = |min: f32, max: f32| if whole { min <= 0.0 && max >= 1.0 } else { min <= 0.5 && max >= 0.5 };
    state.boxes.iter().any(|shape| shape.max[1] >= 1.0 && covers(shape.min[0], shape.max[0]) && covers(shape.min[2], shape.max[2]))
}

fn cane_may_stand(level: &World, pos: BlockPos) -> bool {
    let ground = below(pos);
    if name(level, ground) == Some(REEDS) {
        return true;
    }
    let watered = [[0, -1], [1, 0], [0, 1], [-1, 0]].iter().any(|[dx, dz]| is_water(level, [ground[0] + dx, ground[1], ground[2] + dz]));
    name(level, ground).is_some_and(|ground| CANE_GROUND.contains(&ground)) && !is_wet(level, pos) && !is_wet(level, [pos[0], pos[1] + 1, pos[2]]) && watered
}

/// Whether the block `id` may stand at `pos` by its own ground rule; true for a block without
/// one. Before a place the cell still holds what the block replaces, which counts as its own:
/// cane does not go into water.
pub(super) fn may_stand(level: &World, pos: BlockPos, id: u32) -> bool {
    let registry = level.registry();
    let Some(block) = registry.get(id).map(|state| state.name) else { return true };
    let under = name(level, below(pos));
    match block {
        _ if CROPS.contains(&block) => under == Some("minecraft:farmland"),
        "minecraft:nether_wart" => under == Some("minecraft:soul_sand"),
        "minecraft:fire" | "minecraft:soul_fire" => super::fire::stands(level, pos),
        "minecraft:sweet_berry_bush" => under.is_some_and(|under| VEGETATION.contains(&under)) && !is_wet(level, pos),
        REEDS => cane_may_stand(level, pos),
        "minecraft:redstone_wire" => !is_wet(level, pos) && (has_top(level, below(pos), false) || under.is_some_and(|under| LIT_FROM_BELOW.contains(&under))),
        _ if DIODES.contains(&block) => has_top(level, below(pos), false),
        _ if RAILS.contains(&block) => {
            let rises = property(registry, id, "rail_direction").and_then(|shape| RISES_TO.iter().find(|(rising, _)| *rising == shape));
            has_top(level, below(pos), false) && rises.is_none_or(|(_, [dx, dz])| has_top(level, [pos[0] + dx, pos[1] - 1, pos[2] + dz], false))
        }
        "minecraft:cocoa" => {
            let side = property(registry, id, "direction").and_then(|direction| COCOA_LOG.get(direction.parse::<usize>().ok()?));
            side.and_then(|[dx, dz]| name(level, [pos[0] + dx, pos[1], pos[2] + dz])).is_some_and(|log| log.contains("jungle_log") || log.contains("jungle_wood"))
        }
        // No bamboo on stone (research/captures/bds-placing.txt); what it does stand on is the
        // Java game's list.
        "minecraft:bamboo" | "minecraft:bamboo_sapling" => under.is_some_and(|under| BAMBOO_GROUND.contains(&under)),
        // Nor these on stone (bds-placing2.txt); the ground they take is the bush's here.
        "minecraft:pink_petals" | "minecraft:big_dripleaf" | "minecraft:mangrove_propagule" => under.is_some_and(|under| VEGETATION.contains(&under)),
        _ if VINES.contains(&block) => {
            let above = [pos[0], pos[1] + 1, pos[2]];
            !is_wet(level, pos) && (name(level, above).is_some_and(|above| VINES.contains(&above)) || level.in_height(above[1]) && has_bottom(level, above))
        }
        _ => true,
    }
}

const BAMBOO_GROUND: &[&str] = &[
    "minecraft:grass_block", "minecraft:dirt", "minecraft:coarse_dirt", "minecraft:podzol", "minecraft:mycelium", "minecraft:dirt_with_roots", "minecraft:moss_block", "minecraft:mud",
    "minecraft:sand", "minecraft:red_sand", "minecraft:gravel", "minecraft:bamboo", "minecraft:bamboo_sapling",
];

/// Whether the block `id` at `pos` has the wrong light to stay when a neighbour changes, its
/// brightness read with no night in it: a crop under 8 (`CropBlock::canSurvive`; open sky over
/// it always gives that), a mushroom from 13 up off mycelium and podzol (`MushroomBlock`'s).
pub(super) fn is_wrongly_lit(level: &World, [x, y, z]: BlockPos, id: u32) -> bool {
    const DARK_KILLS: &[&str] = &["minecraft:wheat", "minecraft:carrots", "minecraft:potatoes", "minecraft:beetroot", "minecraft:torchflower_crop"];
    const MUSHROOMS: &[&str] = &["minecraft:brown_mushroom", "minecraft:red_mushroom"];
    const MUSHROOM_GROUND: &[&str] = &["minecraft:mycelium", "minecraft:podzol"];
    let Some(name) = level.registry().get(id).map(|state| state.name) else { return false };
    let light = || level.brightness_at(x, y, z, 0).unwrap_or(0);
    match name {
        _ if DARK_KILLS.contains(&name) => level.light_at(x, y, z).is_some() && light() < 8,
        _ if MUSHROOMS.contains(&name) => !self::name(level, [x, y - 1, z]).is_some_and(|under| MUSHROOM_GROUND.contains(&under)) && light() >= 13,
        _ => false,
    }
}

/// Whether [`may_stand`] has a rule for the block `name`, which then replaces updates.rs's.
pub(super) fn has_rule(name: &str) -> bool {
    const OTHERS: &[&str] = &["minecraft:bamboo", "minecraft:bamboo_sapling", "minecraft:pink_petals", "minecraft:big_dripleaf", "minecraft:mangrove_propagule", "minecraft:nether_wart", "minecraft:sweet_berry_bush", REEDS, "minecraft:redstone_wire", "minecraft:cocoa", "minecraft:trip_wire"];
    [CROPS, DIODES, RAILS, VINES, OTHERS].iter().any(|list| list.contains(&name))
}

/// Whether the bottom of the block at `pos` holds something hanging under its middle.
fn has_bottom(level: &World, [x, y, z]: BlockPos) -> bool {
    let Some(state) = level.registry().get(level.block(x, y, z)) else { return false };
    state.boxes.iter().any(|shape| shape.min[1] <= 0.0 && (shape.min[0]..=shape.max[0]).contains(&0.5) && (shape.min[2]..=shape.max[2]).contains(&0.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_block_named_is_in_the_registry() {
        let registry = BlockRegistry::vanilla();
        let lists = [CROPS, VEGETATION, CANE_GROUND, LIT_FROM_BELOW, DIODES, RAILS, VINES, BAMBOO_GROUND, &[REEDS, "minecraft:farmland", "minecraft:soul_sand", "minecraft:frosted_ice"]];
        for block in lists.concat() {
            assert!(registry.states_of(block).next().is_some(), "{block}");
        }
    }
}
