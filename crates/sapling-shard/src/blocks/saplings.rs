//! Saplings, mangrove propagules and azaleas growing into trees, by random tick and bone meal, as
//! BDS's `SaplingBlock`, `MangrovePropaguleBlock` and `AzaleaBlock` do (docs/research/saplings.md).
//! The tree is worldgen's own feature, placed over the live world (`World::place_feature`).
// TODO: a hanging propagule neither ages nor takes bone meal.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{Dice, level, send, with_property};

/// What a species grows: alone, alone with a beehive, and from four saplings in a square.
struct Species {
    sapling: &'static str,
    single: Option<&'static str>,
    beehive: Option<&'static str>,
    square: Option<&'static str>,
}

const fn species(sapling: &'static str, single: Option<&'static str>, beehive: Option<&'static str>, square: Option<&'static str>) -> Species {
    Species { sapling, single, beehive, square }
}

const SPECIES: [Species; 9] = [
    species("oak_sapling", Some("random_oak_tree_from_sapling"), Some("random_oak_tree_with_beehive_from_sapling"), None),
    species("spruce_sapling", Some("select_standing_spruce_tree"), None, Some("mega_spruce_tree")),
    species("birch_sapling", Some("birch_tree"), Some("birch_tree_with_beehive"), None),
    species("jungle_sapling", Some("select_undecorated_jungle_tree"), None, Some("mega_jungle_tree")),
    species("acacia_sapling", Some("savanna_tree"), None, None),
    species("dark_oak_sapling", None, None, Some("select_roofed_tree")),
    species("cherry_sapling", Some("cherry_tree"), Some("cherry_tree_with_beehive"), None),
    species("pale_oak_sapling", None, None, Some("pale_oak_tree")),
    species("poplar_sapling", Some("select_poplar_tree"), None, None),
];
const PROPAGULE: &str = "mangrove_propagule";
const AZALEAS: [&str; 2] = ["azalea", "flowering_azalea"];
const AGE: &str = "age_bit";
/// A random tick's chance, as one in this many.
pub(super) const TICK_ODDS: u32 = 7;
/// The raw brightness over a sapling a random tick needs to age or grow it; a propagule needs none.
const GROWING_LIGHT: u8 = 9;
const BONE_MEAL: f32 = 0.45;
const BEEHIVE: f32 = 0.05;
/// A beehive comes only with one of these within two blocks across at the sapling's height.
const FLOWERS: [&str; 26] = [
    "cherry_leaves", "pink_petals", "blue_orchid", "allium", "azure_bluet", "oxeye_daisy", "cornflower", "lily_of_the_valley", "dandelion", "wither_rose", "torchflower", "open_eyeblossom",
    "closed_eyeblossom", "sunflower", "rose_bush", "pitcher_plant", "chorus_flower", "spore_blossom", "poppy", "red_tulip", "orange_tulip", "white_tulip", "pink_tulip", "wildflowers", "peony", "lilac",
];
/// The cells of a square the sapling may be in, by the corner it is at, in BDS's order.
const SQUARE: [(i32, i32); 4] = [(0, 0), (0, -1), (-1, 0), (-1, -1)];

fn name_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<&'static str> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    level.registry().get(level.block(x, y, z)).map(|state| state.name.strip_prefix("minecraft:").unwrap_or(state.name))
}

fn property(world: &World, layer: Entity, [x, y, z]: BlockPos, key: &str) -> Option<u8> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    let properties = level.registry().get(level.block(x, y, z))?.properties;
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key)?.1.parse().ok()
}

fn of(name: &str) -> Option<&'static Species> {
    SPECIES.iter().find(|species| species.sapling == name)
}

/// Whether a random tick can grow the block `name` (unprefixed) with `properties`.
pub(super) fn is_ticked(name: &str, properties: &str) -> bool {
    of(name).is_some() || name == PROPAGULE && !properties.contains("hanging=1")
}

/// The corner of the square of four saplings like the one at `pos` that it is part of.
fn square(world: &World, layer: Entity, [x, y, z]: BlockPos, sapling: &str) -> Option<BlockPos> {
    let is = |dx: i32, dz: i32| name_at(world, layer, [x + dx, y, z + dz]) == Some(sapling);
    SQUARE.iter().find(|(dx, dz)| is(*dx, *dz) && is(dx + 1, *dz) && is(*dx, dz + 1) && is(dx + 1, dz + 1)).map(|(dx, dz)| [x + dx, y, z + dz])
}

fn near_flower(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    (-2..=2).any(|dx| (-2..=2).any(|dz| name_at(world, layer, [x + dx, y, z + dz]).is_some_and(|name| FLOWERS.contains(&name))))
}

/// Sets `age_bit` on a sapling that has none: true where it did.
fn aged(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let Some(id) = level(world, layer).map(|level| level.block(x, y, z)) else { return false };
    let Some(older) = level(world, layer).and_then(|level| with_property(level.registry(), id, AGE, "1")).filter(|older| *older != id) else { return false };
    send::set(world, layer, [x, y, z], older)
}

/// Places `feature` at `origin` over `cleared` (set to air first; all given back the first one's
/// block if the tree does not grow) and shows the tree.
fn place(world: &mut World, layer: Entity, feature: &str, origin: BlockPos, cleared: &[BlockPos]) -> bool {
    let Some(here) = level(world, layer) else { return false };
    let (air, saved) = (here.registry().air_id(), cleared.first().map_or(0, |[x, y, z]| here.block(*x, *y, *z)));
    cleared.iter().for_each(|cell| _ = send::set_quietly(world, layer, *cell, air));
    let seed = world.get_resource_or_insert_with(Dice::default).roll(u32::MAX);
    let tree = level(world, layer).and_then(|level| level.place_feature(&format!("minecraft:{feature}_feature"), origin, seed));
    let Some(tree) = tree.filter(|tree| !tree.is_empty()) else {
        cleared.iter().for_each(|cell| _ = send::set_quietly(world, layer, *cell, saved));
        return false;
    };
    send::set_many(world, layer, &tree);
    cleared.iter().filter(|cell| !tree.iter().any(|(at, _)| at == *cell)).for_each(|cell| send::show(world, layer, *cell));
    true
}

/// A sapling's tree (`SaplingBlock::_growTree`): from its square where the species has one, with
/// a beehive now and then by flowers, else alone.
fn grow(world: &mut World, layer: Entity, pos: BlockPos, species: &Species) -> bool {
    let [x, y, z] = pos;
    if let Some((corner, feature)) = square(world, layer, pos, species.sapling).zip(species.square) {
        let [cx, _, cz] = corner;
        return place(world, layer, feature, corner, &[[cx, y, cz], [cx, y, cz + 1], [cx + 1, y, cz], [cx + 1, y, cz + 1]]);
    }
    let roll = world.get_resource_or_insert_with(Dice::default).fraction();
    let feature = match species.beehive {
        Some(beehive) if roll < BEEHIVE && near_flower(world, layer, pos) => beehive,
        _ => match species.single {
            Some(single) => single,
            None => return false,
        },
    };
    place(world, layer, feature, pos, &[[x, y, z]])
}

/// A mangrove propagule's tree: tall more often than not, with a bee nest one time in a hundred.
fn grow_mangrove(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let (short, nest) = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (dice.fraction() >= 0.85, dice.fraction() < 0.01)
    };
    let feature = format!("{}mangrove_tree{}", if short { "" } else { "tall_" }, if nest { "_with_beenest" } else { "" });
    place(world, layer, &feature, pos, &[pos])
}

/// A random tick that came up for the block at `pos` (one in [`TICK_ODDS`] of those it is
/// picked for): a sapling with light enough over it ages, or grows if it already had.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos;
    match name_at(world, layer, pos) {
        Some(PROPAGULE) => _ = grow_mangrove(world, layer, pos),
        Some(name) => {
            let lit = crate::light::brightness(world, layer, [x, y + 1, z]).is_some_and(|light| light >= GROWING_LIGHT);
            if let Some(species) = of(name).filter(|_| lit && !aged(world, layer, pos)) {
                grow(world, layer, pos, species);
            }
        }
        None => {}
    }
}

/// Bone meal on the block at `pos` (`onFertilized`): `None` for what is no sapling, else whether
/// it took the bone meal. In creative it grows at once; else it ages, or grows if it had,
/// at BDS's odds.
pub(super) fn fertilized(world: &mut World, layer: Entity, pos: BlockPos, creative: bool) -> Option<bool> {
    let name = name_at(world, layer, pos)?;
    let lucky = creative || world.get_resource_or_insert_with(Dice::default).fraction() < BONE_MEAL;
    if name == PROPAGULE {
        if property(world, layer, pos, "hanging") == Some(1) {
            return Some(false);
        }
        if lucky {
            grow_mangrove(world, layer, pos);
        }
        return Some(true);
    }
    if AZALEAS.contains(&name) {
        if lucky {
            place(world, layer, "azalea_tree", pos, &[]);
        }
        return Some(true);
    }
    let species = of(name)?;
    if species.single.is_none() && square(world, layer, pos, name).is_none() {
        return Some(false);
    }
    if creative {
        grow(world, layer, pos, species);
    } else if lucky && !aged(world, layer, pos) {
        grow(world, layer, pos, species);
    }
    Some(true)
}
