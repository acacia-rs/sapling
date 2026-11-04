//! What a fire's tick does: BDS's `FireBlock` queued tick (147157ed0), its burn of a neighbour
//! (`checkBurn` 1471594b0) and its spread (docs/research/fire.md, "BDS's rule").

use sapling_api::{Block, BlockPos, Face};

use super::super::{send, with_property};

pub(super) const FIRE: &str = "minecraft:fire";
pub(super) const SOUL_FIRE: &str = "minecraft:soul_fire";
const TNT: &str = "minecraft:tnt";
const AGE: &str = "age";
const OLDEST: u32 = 15;
/// Fire over these burns on and does not age.
const ETERNAL: [&str; 4] = ["minecraft:netherrack", "minecraft:magma", "minecraft:soul_sand", "minecraft:soul_soil"];
/// Fire over these turns to soul fire on its tick (147159160).
const SOULED: [&str; 2] = ["minecraft:soul_sand", "minecraft:soul_soil"];
const FACES: [Face; 6] = [Face::East, Face::West, Face::Down, Face::Up, Face::North, Face::South];
/// One in this many is the chance a block beside, and over or under, a fire has to burn, times
/// what it burns by; 50 less where the biome is humid.
const BESIDE: u32 = 300;
const OVER: u32 = 250;
const HUMID_LESS: u32 = 50;
/// A biome rains this much and more for its fires to spread at half the odds.
const HUMID: f32 = 0.85;
/// Spread's odds go up by this for each step of the difficulty, easy to hard (14a8ea760).
const HARDSHIP: f32 = 7.0;

pub(super) type Level = sapling_world::World;

/// How readily fire catches by the block `name`, and how readily it burns away: the Java game's,
/// which are BDS's flame and burn odds for these.
pub(super) fn burns(name: &str) -> Option<[u32; 2]> {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    let tagged = |tag: &str| sapling_data::has_tag(name, tag);
    let ends = |ends: &[&str]| ends.iter().any(|end| bare.ends_with(end));
    let is_wooden = !["crimson", "warped", "nether_brick", "stone", "brick"].iter().any(|part| bare.contains(part));
    Some(match bare {
        _ if tagged("minecraft:logs_that_burn") || bare == "coal_block" => [5, 5],
        _ if is_wooden && (tagged("minecraft:planks") || tagged("minecraft:wooden_slabs")) => [5, 20],
        _ if is_wooden && ends(&["_fence", "_fence_gate", "_stairs"]) && sapling_data::has_tag(&name.replace("_fence_gate", "_planks").replace("_fence", "_planks").replace("_stairs", "_planks"), "minecraft:planks") => [5, 20],
        _ if ends(&["leaves", "_wool"]) || bare == "dried_kelp_block" => [30, 60],
        "bookshelf" | "lectern" => [30, 20],
        "tnt" => [15, 100],
        "vine" => [15, 100],
        "bamboo" | "scaffolding" => [60, 60],
        "hay_block" => [60, 20],
        _ if bare.ends_with("_carpet") && bare != "moss_carpet" => [60, 20],
        "short_grass" | "fern" | "tall_grass" | "large_fern" | "deadbush" | "poppy" | "dandelion" => [60, 100],
        _ => return None,
    })
}

/// Whether lava lights fire by the block `name`: BDS's `mLavaFlammable` (Block+0xa6, read by
/// `LiquidBlock::randomTick` 148b5cd10), set apart from the flame odds by its registry 14849c7e0
/// (docs/research/fire.md, "Lava").
pub(super) fn lava_lights(name: &str) -> bool {
    const NEVER: &[&str] = &["coal_block", "hay_block", "poppy", "dandelion"];
    const ALWAYS: &[&str] = &[
        "sunflower", "rose_bush", "lilac", "peony", "short_dry_grass", "tall_dry_grass", "hanging_roots", "pale_hanging_moss", "noteblock", "trapped_chest",
        "daylight_detector", "daylight_detector_inverted", "red_mushroom_block", "brown_mushroom_block", "mushroom_stem", "chorus_plant", "chorus_flower",
        "standing_banner", "wall_banner",
    ];
    const WOOD_PARTS: &[&str] = &["_door", "trapdoor", "standing_sign", "wall_sign", "_button", "_pressure_plate"];
    const NOT_WOOD: &[&str] = &["iron", "stone", "copper", "gold", "blackstone", "weighted", "brick"];
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    let is_nether_wood = ["crimson_", "warped_"].iter().any(|kind| bare.contains(kind)) && !["_nylium", "_roots", "_fungus", "_wart_block"].iter().any(|end| bare.ends_with(end));
    let is_wooden_part = WOOD_PARTS.iter().any(|end| bare.ends_with(end)) && !NOT_WOOD.iter().any(|part| bare.contains(part));
    !NEVER.contains(&bare) && (burns(name).is_some() || ALWAYS.contains(&bare) || is_nether_wood || is_wooden_part)
}

pub(super) fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

pub(super) fn is_air(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air())
}

fn is_solid(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid())
}

/// The readiest of the six blocks around `cell` to catch (`isValidFireLocation` 147159330 asks
/// whether it is over 0).
fn catches_by(level: &Level, cell: BlockPos) -> u32 {
    FACES.iter().filter_map(|face| burns(name_at(level, face.neighbour(cell))?)).map(|[catches, _]| catches).max().unwrap_or(0)
}

pub(super) fn fire_of(level: &Level, age: u32) -> Option<u32> {
    let young = send::resolve(level.registry(), &Block::new(FIRE))?;
    with_property(level.registry(), young, AGE, &age.to_string())
}

/// Whether fire may be at `pos`: over a solid block, or by one that burns.
pub(crate) fn stands(level: &Level, [x, y, z]: BlockPos) -> bool {
    is_solid(level, [x, y - 1, z]) || catches_by(level, [x, y, z]) > 0
}

/// Whether rain falls on `cell` or a cell beside it, not up or down (`Weather::isRainingAt` 14119d6b0
/// over the five, with the biome warm enough for rain): open to the sky and not cold.
fn is_rained_on(level: &Level, rains: bool, [x, y, z]: BlockPos) -> bool {
    let open = |[x, y, z]: BlockPos| !level.is_cold(x, y, z) && !(y + 1..).take_while(|y| level.in_height(*y)).any(|y| is_solid(level, [x, y, z]));
    rains && [[x, y, z], [x + 1, y, z], [x - 1, y, z], [x, y, z - 1], [x, y, z + 1]].into_iter().any(open)
}

/// What a fire's tick does.
#[derive(Default)]
pub(super) struct Burnt {
    /// Whether the fire ticks again: it is still there, or the rule held it.
    pub(super) burns_on: bool,
    /// The fire itself, older: no one is told, as BDS told no one of its fires' ages.
    pub(super) aged: Option<u32>,
    pub(super) set: Vec<(BlockPos, u32)>,
    pub(super) primed: Vec<BlockPos>,
}

/// The world about a fire as its tick reads it.
pub(super) struct Around {
    /// Spread's odds for the difficulty, peaceful 0.
    pub(super) hardship: u32,
    pub(super) rains: bool,
    /// The world's `doFireTick`: off, the fire only waits for its next tick.
    pub(super) spreads: bool,
}

/// A tick of the fire at `pos` (BDS 147157ed0); `roll(n)` draws below `n`, `chance()` in [0, 1).
pub(super) fn burnt(level: &Level, pos: BlockPos, around: &Around, roll: &mut dyn FnMut(u32) -> u32, chance: &mut dyn FnMut() -> f32) -> Burnt {
    let [x, y, z] = pos;
    let (mut done, air) = (Burnt::default(), level.registry().air_id());
    let Some(state) = level.registry().get(level.block(x, y, z)).filter(|state| state.name == FIRE) else { return done };
    let under = [x, y - 1, z];
    let under_name = name_at(level, under);
    if let Some(soul) = under_name.filter(|name| SOULED.contains(name)).and_then(|_| send::resolve(level.registry(), &Block::new(SOUL_FIRE))) {
        done.set.push((pos, soul));
        return done;
    }
    let (stands, is_fed) = (is_solid(level, under), catches_by(level, pos) > 0);
    if !(stands || is_fed) {
        done.set.push((pos, air));
        return done;
    }
    done.burns_on = true;
    if !around.spreads {
        return done;
    }
    let eternal = under_name.is_some_and(|under| ETERNAL.contains(&under));
    if !eternal && is_rained_on(level, around.rains, pos) {
        return Burnt { set: vec![(pos, air)], ..Burnt::default() };
    }
    let was = state_age(state.properties);
    let age = if was < OLDEST && !eternal { (was + roll(3) / 2).min(OLDEST) } else { was };
    done.aged = fire_of(level, age).filter(|_| age != was);
    if !eternal {
        if !is_fed {
            if age > 3 || !stands {
                done.set.push((pos, air));
            }
            return done;
        }
        if age == OLDEST && under_name.and_then(burns).is_none() && roll(4) == 0 {
            done.set.push((pos, air));
            return done;
        }
    }
    let humid = level.biome(x, y, z).is_some_and(|biome| sapling_data::biome_downfall(biome) > HUMID);
    let less = if humid { HUMID_LESS } else { 0 };
    for face in FACES {
        let cell = face.neighbour(pos);
        let Some(name) = name_at(level, cell) else { continue };
        let Some([_, burns]) = burns(name) else { continue };
        if roll(if matches!(face, Face::Up | Face::Down) { OVER - less } else { BESIDE - less }) >= burns {
            continue;
        }
        let catches = roll(age + 10) < 5 && !is_rained_on(level, around.rains, cell);
        let spread_age = (age + roll(5) / 4).min(OLDEST);
        match fire_of(level, spread_age).filter(|_| catches && catches_by(level, cell) > 0) {
            _ if name == TNT => done.primed.push(cell),
            Some(fire) => done.set.push((cell, fire)),
            None => done.set.push((cell, air)),
        }
    }
    for [dx, dy, dz] in (-1..=1).flat_map(|dx| (-1..=4).flat_map(move |dy| (-1..=1).map(move |dz| [dx, dy, dz]))).filter(|by| *by != [0; 3]) {
        let cell = [x + dx, y + dy, z + dz];
        let catches = if is_air(level, cell) { catches_by(level, cell) } else { 0 };
        if catches == 0 || done.set.iter().any(|(at, _)| *at == cell) {
            continue;
        }
        let rate = if dy < 2 { 100.0 } else { 100.0 * dy as f32 };
        let odds = (catches as f32 + 40.0 + HARDSHIP * around.hardship as f32) / (age + 30) as f32;
        let odds = if humid { odds / 2.0 } else { odds };
        if odds > 0.0 && rate * chance() <= odds && !is_rained_on(level, around.rains, cell)
            && let Some(fire) = fire_of(level, (age + roll(5) / 4).min(OLDEST))
        {
            done.set.push((cell, fire));
        }
    }
    done
}

fn state_age(properties: &str) -> u32 {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == AGE).and_then(|(_, age)| age.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_and_wool_burn_and_stone_does_not() {
        let burning = ["minecraft:oak_planks", "minecraft:oak_log", "minecraft:white_wool", "minecraft:oak_leaves", "minecraft:stone", "minecraft:crimson_planks"].map(burns);
        assert_eq!(burning, [Some([5, 20]), Some([5, 5]), Some([30, 60]), Some([30, 60]), None, None]);
    }

    #[test]
    fn lava_lights_by_bds_lava_flag_not_the_flame_odds() {
        let lit = ["oak_planks", "crimson_planks", "stripped_warped_stem", "spruce_door", "wooden_pressure_plate", "standing_sign", "noteblock", "rose_bush"];
        let unlit = ["coal_block", "hay_block", "dandelion", "iron_door", "stone_button", "crimson_nylium", "stone"];
        assert!(lit.iter().all(|name| lava_lights(&format!("minecraft:{name}"))), "{lit:?}");
        assert!(!unlit.iter().any(|name| lava_lights(&format!("minecraft:{name}"))), "{unlit:?}");
    }
}
