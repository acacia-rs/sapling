//! What crops, cane, berries, cocoa, vines, redstone and rails need to stand on, by BDS's
//! `mayPlace` / `canSurvive` of each (docs/research/standing-and-shapes.md, part A). Blocks
//! not named here need only what updates.rs asks.
// TODO: the dirt and sand tags are Java's lists.

use sapling_api::{BlockPos, Face};
use sapling_worldgen::Support;
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
/// Flowers and grass that keep to [`VEGETATION`]: a poppy and short grass on stone broke at a neighbour's
/// change (research/captures/bds-p2fall.txt); the rest are the Java game's same `BushBlock`s.
const BUSHES: &[&str] = &[
    "minecraft:poppy", "minecraft:dandelion", "minecraft:blue_orchid", "minecraft:allium", "minecraft:azure_bluet", "minecraft:red_tulip", "minecraft:orange_tulip",
    "minecraft:white_tulip", "minecraft:pink_tulip", "minecraft:oxeye_daisy", "minecraft:cornflower", "minecraft:lily_of_the_valley", "minecraft:short_grass", "minecraft:fern",
];
/// The `dirt` and `sand` tags sugar cane asks for, less farmland: Java's lists.
const CANE_GROUND: &[&str] = &[
    "minecraft:dirt", "minecraft:grass_block", "minecraft:podzol", "minecraft:coarse_dirt", "minecraft:mycelium", "minecraft:dirt_with_roots", "minecraft:moss_block",
    "minecraft:pale_moss_block", "minecraft:mud", "minecraft:muddy_mangrove_roots", "minecraft:sand", "minecraft:red_sand", "minecraft:suspicious_sand",
];
const REEDS: &str = "minecraft:reeds";
const KELP: &str = "minecraft:kelp";
const TWISTING: &str = "minecraft:twisting_vines";
const WEEPING: &str = "minecraft:weeping_vines";
const SMALL_DRIPLEAF: &str = "minecraft:small_dripleaf_block";
const DRIPSTONE: &str = "minecraft:pointed_dripstone";
/// What a small dripleaf stands on dry; in water, the dirt list too (1466059d0, worldgen-bds-caves.md).
const DRIPLEAF_GROUND: &[&str] = &["minecraft:clay", "minecraft:moss_block"];
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
/// Blocks whose top is low or open in the middle but holds dust, rails and diodes on BDS (a hopper's,
/// research/captures/bds-p2place.txt).
const LOW_CENTRE: &[&str] = &["minecraft:soul_sand", "minecraft:mud", "minecraft:hopper"];

/// Whether the top of the block at `pos` holds something up: under the middle of the face
/// (BDS support "centre"), or across all of it ("edge").
pub(super) fn has_top(level: &World, [x, y, z]: BlockPos, whole: bool) -> bool {
    let Some(state) = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten() else { return false };
    if !whole && (LOW_CENTRE.contains(&state.name) || NO_CENTRE.iter().any(|end| state.name.ends_with(end))) {
        return LOW_CENTRE.contains(&state.name);
    }
    let covers = |min: f32, max: f32| if whole { min <= 0.0 && max >= 1.0 } else { min <= 0.5 && max >= 0.5 };
    state.boxes.iter().any(|shape| shape.max[1] >= 1.0 && covers(shape.min[0], shape.max[0]) && covers(shape.min[2], shape.max[2]))
}

/// Whether the top of the block at `pos` gives every kind of support in `kinds` (BDS
/// `canProvideSupport`): dust wants centre and edge, a rail edge, a diode any (bds-p3ground.txt).
/// A shulker box's holds no dust (bds-conductors.txt).
fn top_gives(level: &World, pos: BlockPos, kinds: &[Support]) -> bool {
    face_gives(level, pos, Face::Up, kinds)
}

fn face_gives(level: &World, [x, y, z]: BlockPos, face: Face, kinds: &[Support]) -> bool {
    let Some(state) = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten() else { return false };
    let shulker_centre = state.name.ends_with("shulker_box") && kinds.contains(&Support::Centre);
    !shulker_centre && kinds.iter().all(|kind| sapling_worldgen::provides_support(state.name, state.properties, face as usize, *kind))
}

/// Whether the block at `pos` has a whole face `face`, centre and edge: what pointed dripstone and
/// weeping vines need (research/captures/bds-u3plants.txt: a top slab, glass, upside-down stairs and
/// a hopper hold standing dripstone; a bottom slab, glass and a composter hold the hanging kinds).
pub(super) fn whole_face(level: &World, pos: BlockPos, face: Face) -> bool {
    face_gives(level, pos, face, &[Support::Centre, Support::Edge])
}

/// Whether the block at `pos` is solid-blocking or a top slab: what twisting vines and kelp stand
/// on; upside-down stairs and a hopper, whole on top, held neither (bds-u3plants.txt).
fn solid_top(level: &World, pos: BlockPos) -> bool {
    name(level, pos).is_some_and(|under| sapling_worldgen::is_solid_blocking(under) || under.ends_with("_slab") && whole_face(level, pos, Face::Up))
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
        // Its water taken away, it breaks at once (research/captures/bds-eggs.txt).
        "minecraft:frog_spawn" => is_water(level, below(pos)),
        "minecraft:fire" | "minecraft:soul_fire" => super::fire::stands(level, pos),
        "minecraft:sweet_berry_bush" => under.is_some_and(|under| VEGETATION.contains(&under)) && !is_wet(level, pos),
        _ if BUSHES.contains(&block) => under.is_some_and(|under| VEGETATION.contains(&under)),
        REEDS => cane_may_stand(level, pos),
        "minecraft:redstone_wire" => !is_wet(level, pos) && (top_gives(level, below(pos), &[Support::Centre, Support::Edge]) || under.is_some_and(|under| LIT_FROM_BELOW.contains(&under))),
        _ if DIODES.contains(&block) => top_gives(level, below(pos), &[Support::Any]),
        _ if RAILS.contains(&block) => {
            let rises = property(registry, id, "rail_direction").and_then(|shape| RISES_TO.iter().find(|(rising, _)| *rising == shape));
            // The cell risen to, at the rail's own height (`BaseRailBlock` 142cd5660).
            top_gives(level, below(pos), &[Support::Edge]) && rises.is_none_or(|(_, [dx, dz])| top_gives(level, [pos[0] + dx, pos[1], pos[2] + dz], &[Support::Edge]))
        }
        // `CarpetBlock::canSurvive`: anything but air; set over air it stays till a neighbour changes (bds-p3carpet.txt).
        _ if block.ends_with("carpet") => under.is_some_and(|under| under != "minecraft:air"),
        SMALL_DRIPLEAF if property(registry, id, "upper_block_bit") == Some("1") => under == Some(SMALL_DRIPLEAF),
        SMALL_DRIPLEAF => under.is_some_and(|under| DRIPLEAF_GROUND.contains(&under) || is_water(level, pos) && VEGETATION.contains(&under)),
        _ if block.ends_with("_hanging_sign") && property(registry, id, "hanging") == Some("1") => {
            let above = [pos[0], pos[1] + 1, pos[2]];
            level.in_height(above[1]) && has_bottom(level, above)
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
        // On glass it went at a neighbour's change, on stone it stayed (research/captures/bds-plantpop.txt).
        // On leaves too it went (bds-u3plants.txt).
        KELP => under == Some(KELP) || solid_top(level, below(pos)) && under.is_some_and(|under| !under.ends_with("glass") && !under.ends_with("leaves") && under != "minecraft:magma"),
        // Water run in beside a nether vine broke it (bds-plantpop2.txt), as it does cave vines.
        TWISTING => !is_wet(level, pos) && (under == Some(TWISTING) || solid_top(level, below(pos))),
        WEEPING => {
            let above = [pos[0], pos[1] + 1, pos[2]];
            !is_wet(level, pos) && (name(level, above) == Some(WEEPING) || whole_face(level, above, Face::Down))
        }
        // On more of it or a whole top (`PointedDripstoneBlock::canSurvive` 14660c830; bds-i2drip.txt).
        DRIPSTONE if property(registry, id, "hanging") == Some("0") => {
            let ground = below(pos);
            let on_more = level.in_height(ground[1]) && property(registry, level.block(ground[0], ground[1], ground[2]), "hanging") == Some("0") && under == Some(DRIPSTONE);
            on_more || whole_face(level, ground, Face::Up)
        }
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

/// Whether the block `id` at `pos` has the wrong light to be put or to stay when a neighbour
/// changes, its brightness read with no night in it: a mushroom from 13 up off mycelium and podzol
/// (`MushroomBlock`'s; BDS's put by hand at noon on stone and dirt was refused, in a shut room
/// put, research/captures/bds-r3light.txt). Crops never: `CropBlock::neighborChanged` asks only
/// the ground (research/captures/bds-lightgrow.txt).
pub(super) fn is_wrongly_lit(level: &World, [x, y, z]: BlockPos, id: u32) -> bool {
    const MUSHROOMS: &[&str] = &["minecraft:brown_mushroom", "minecraft:red_mushroom"];
    const MUSHROOM_GROUND: &[&str] = &["minecraft:mycelium", "minecraft:podzol"];
    let Some(name) = level.registry().get(id).map(|state| state.name) else { return false };
    let light = || level.brightness_at(x, y, z, 0).unwrap_or(0);
    match name {
        _ if MUSHROOMS.contains(&name) => !self::name(level, [x, y - 1, z]).is_some_and(|under| MUSHROOM_GROUND.contains(&under)) && light() >= 13,
        _ => false,
    }
}

/// Whether [`may_stand`] has a rule for the block `name`, which then replaces updates.rs's.
pub(super) fn has_rule(name: &str) -> bool {
    const OTHERS: &[&str] = &["minecraft:bamboo", "minecraft:bamboo_sapling", "minecraft:pink_petals", "minecraft:big_dripleaf", "minecraft:mangrove_propagule", "minecraft:nether_wart", "minecraft:sweet_berry_bush", REEDS, "minecraft:redstone_wire", "minecraft:cocoa", "minecraft:trip_wire", "minecraft:frog_spawn", KELP, TWISTING, WEEPING, SMALL_DRIPLEAF];
    [CROPS, DIODES, RAILS, VINES, BUSHES, OTHERS].iter().any(|list| list.contains(&name)) || name.ends_with("carpet")
}

/// Whether the bottom of the block at `pos` holds something hanging under its middle.
fn has_bottom(level: &World, [x, y, z]: BlockPos) -> bool {
    let Some(state) = level.registry().get(level.block(x, y, z)) else { return false };
    state.boxes.iter().any(|shape| shape.min[1] <= 0.0 && (shape.min[0]..=shape.max[0]).contains(&0.5) && (shape.min[2]..=shape.max[2]).contains(&0.5))
}

#[cfg(test)]
mod tests;
