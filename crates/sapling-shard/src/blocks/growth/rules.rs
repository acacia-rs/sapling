//! What a random tick does to each thing that grows (docs/research/growth.md).

use sapling_api::{Block, BlockPos, Face};

use super::super::{leaves, saplings, send, with_property};
use super::{Picks, cover};

const RIPE: u8 = 7;
/// The raw brightness crops, stems, bamboo and spreading grass need.
const GROWING_LIGHT: u8 = 9;
const RIPE_BUSH: u8 = 3;
const FARMLAND: &str = "minecraft:farmland";
const WET: &str = "moisturized_amount";
const WETTEST: u8 = 7;
/// Water this near, on its level or the one above, keeps farmland wet: the Java game's reach.
const WATER_REACH: i32 = 4;
const CROPS: [&str; 4] = ["minecraft:wheat", "minecraft:carrots", "minecraft:potatoes", "minecraft:beetroot"];
const STEMS: [(&str, &str); 2] = [("minecraft:melon_stem", "minecraft:melon_block"), ("minecraft:pumpkin_stem", "minecraft:pumpkin")];
const BUSH: &str = "minecraft:sweet_berry_bush";
/// What grows by its `age` to a last one, one random tick in so many: the Java game's odds, which
/// BDS's cocoa and wart kept.
const AGED: [(&str, u8, u32); 2] = [("minecraft:cocoa", 2, 5), ("minecraft:nether_wart", 3, 10)];
const PITCHER: &str = "minecraft:pitcher_crop";
const RIPE_PITCHER: u8 = 4;
/// From this growth a pitcher crop is two blocks high.
const TALL_PITCHER: u8 = 3;
const UPPER: &str = "upper_block_bit";
const COLUMNS: [&str; 2] = ["minecraft:reeds", "minecraft:cactus"];
const COLUMN_HEIGHT: i32 = 3;
const COLUMN_AGE: u8 = 15;
const BAMBOO: &str = "minecraft:bamboo";
/// BDS's bamboo stood thirteen high and growing when the drill ended; the Java game stops by 16.
const BAMBOO_HEIGHT: i32 = 14;
/// One random tick in this many grows bamboo: BDS's put on a segment every 43 ticks, where a
/// random tick came every 9.
const BAMBOO_ODDS: u32 = 5;
/// A fruit grows over these.
const FRUIT_GROUND: [&str; 5] = [FARMLAND, "minecraft:dirt", "minecraft:grass_block", "minecraft:coarse_dirt", "minecraft:podzol"];
const SIDES: [Face; 4] = [Face::North, Face::South, Face::West, Face::East];

type Level = sapling_world::World;

/// A block a random tick sets, and how.
pub(super) struct Change {
    pub pos: BlockPos,
    pub id: u32,
    pub how: How,
}

pub(super) enum How {
    Shown,
    /// Told to no one, where BDS told no one.
    Quiet,
    /// Broken to leave what it leaves, whatever `id` says.
    Breaks,
    /// Melted into water that then flows, whatever `id` says.
    Melts,
}

fn state_at(level: &Level, [x, y, z]: BlockPos) -> Option<(u32, &str, &str)> {
    let id = level.in_height(y).then(|| level.block(x, y, z))?;
    level.registry().get(id).map(|state| (id, state.name, state.properties))
}

pub(super) fn name_at(level: &Level, pos: BlockPos) -> Option<&str> {
    state_at(level, pos).map(|(_, name, _)| name)
}

pub(super) fn is_air(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air())
}

fn number(properties: &str, key: &str) -> Option<u8> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).and_then(|(_, value)| value.parse().ok())
}

pub(super) fn shown(pos: BlockPos, id: u32) -> Change {
    Change { pos, id, how: How::Shown }
}

/// Whether the block is one random ticks change, and so kept track of where
/// [`cover::is_kept`] says it is worth it.
pub(super) fn is_ticked(name: &str, properties: &str) -> bool {
    let growth = || number(properties, "growth");
    match name {
        FARMLAND | BAMBOO => true,
        _ if cover::is_ticked(name) || super::copper::is_ticked(name) => true,
        BUSH => growth().is_some_and(|growth| growth < RIPE_BUSH),
        PITCHER => number(properties, UPPER) == Some(0) && growth().is_some_and(|growth| growth < RIPE_PITCHER),
        _ if AGED.iter().any(|(aged, ..)| *aged == name) => number(properties, "age").zip(AGED.iter().find(|(aged, ..)| *aged == name)).is_some_and(|(age, (_, last, _))| age < *last),
        _ if CROPS.contains(&name) => growth().is_some_and(|growth| growth < RIPE),
        // A stem that bore its fruit points at it and is done.
        _ if STEMS.iter().any(|(stem, _)| *stem == name) => number(properties, "facing_direction") == Some(0),
        _ => COLUMNS.contains(&name) || leaves::is_marked(name, properties) || saplings::is_ticked(name.strip_prefix("minecraft:").unwrap_or(name), properties),
    }
}

/// How many random ticks the block takes for each one a crop takes: BDS's farmland dried a step
/// every 3 ticks where its wheat, cane and ores were ticked every 9.
pub(super) fn haste(name: &str) -> f32 {
    if name == FARMLAND { 3.0 } else { 1.0 }
}

/// How fast the crop `name` at `pos` grows: the Java game's reckoning, by the farmland under and
/// around it, wet or dry, and less in a field of its own kind that is not in rows.
fn speed(level: &Level, [x, y, z]: BlockPos, name: &str) -> f32 {
    let soil = |dx: i32, dz: i32| match state_at(level, [x + dx, y - 1, z + dz]) {
        Some((_, FARMLAND, properties)) => if number(properties, WET).is_some_and(|wet| wet > 0) { 3.0 } else { 1.0 },
        _ => 0.0,
    };
    let around: f32 = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| (dx, dz))).filter(|by| *by != (0, 0)).map(|(dx, dz)| soil(dx, dz) / 4.0).sum();
    let same = |dx: i32, dz: i32| name_at(level, [x + dx, y, z + dz]) == Some(name);
    let (along, across) = (same(-1, 0) || same(1, 0), same(0, -1) || same(0, 1));
    let cornered = same(-1, -1) || same(1, -1) || same(1, 1) || same(-1, 1);
    let speed = 1.0 + soil(0, 0) + around;
    if along && across || cornered { speed / 2.0 } else { speed }
}

/// Whether the raw brightness at `pos`, the sky dimmed by `darken`, lets a plant grow.
pub(super) fn is_lit(level: &Level, [x, y, z]: BlockPos, darken: u8) -> bool {
    level.brightness_at(x, y, z, darken).is_some_and(|light| light >= GROWING_LIGHT)
}

/// A crop's or stem's random tick grows it in light of 9 in its own cell (`CropBlock::randomTick`,
/// `StemBlock`'s).
fn crop_grows(level: &Level, pos: BlockPos, name: &str, darken: u8, picks: &mut Picks) -> bool {
    is_lit(level, pos, darken) && picks.below((25.0 / speed(level, pos, name)) as u32 + 1) == 0
}

/// How many of `name` stand in a run down from `pos`.
fn height(level: &Level, [x, y, z]: BlockPos, name: &str) -> i32 {
    (0..).take_while(|down| name_at(level, [x, y - down, z]) == Some(name)).count() as i32
}

fn is_watered(level: &Level, [x, y, z]: BlockPos) -> bool {
    let wet = |at: BlockPos| name_at(level, at).is_some_and(|name| name.ends_with("water"));
    (-WATER_REACH..=WATER_REACH).any(|dx| (-WATER_REACH..=WATER_REACH).any(|dz| wet([x + dx, y, z + dz]) || wet([x + dx, y + 1, z + dz])))
}

/// Bamboo `tall` high with its top at `top`: its leaves as BDS's wore them, thick from four up.
fn bamboo(level: &Level, [x, top, z]: BlockPos, tall: i32) -> Vec<Change> {
    let leaves: &[&str] = match tall {
        ..=2 => &["small_leaves"],
        3 => &["small_leaves", "small_leaves"],
        4 => &["large_leaves", "small_leaves", "no_leaves"],
        _ => &["large_leaves", "large_leaves", "small_leaves", "no_leaves"],
    };
    let thickness = if tall >= 4 { "thick" } else { "thin" };
    let state = |leaves: &str| send::resolve(level.registry(), &Block { name: BAMBOO.into(), properties: format!("bamboo_leaf_size={leaves},bamboo_stalk_thickness={thickness}") });
    let mut changes: Vec<Change> = leaves.iter().enumerate().filter_map(|(down, leaves)| Some(shown([x, top - down as i32, z], state(leaves)?))).collect();
    // The stalk under the leaves thickens once, the turn it reaches four.
    if tall == 4 {
        changes.extend(state("no_leaves").map(|id| shown([x, top - 3, z], id)));
    }
    changes
}

/// Whether what a random tick does to `name` is growth (offered to plugins): not farmland drying
/// or wetting, leaves decaying, grass dying nor ice melting; grass spreading is.
pub(super) fn grows(name: &str) -> bool {
    name == cover::DIRT || name != FARMLAND && !name.ends_with("leaves") && !cover::is_ticked(name) && !super::copper::is_ticked(name)
}

/// What a random tick makes of the block at `pos`.
pub(super) fn ticked(level: &Level, pos: BlockPos, darken: u8, picks: &mut Picks) -> Vec<Change> {
    let Some((id, name, properties)) = state_at(level, pos) else { return Vec::new() };
    let registry = level.registry();
    let [x, y, z] = pos;
    let above = [x, y + 1, z];
    let with = |key: &str, value: u8| with_property(registry, id, key, &value.to_string());
    let growth = number(properties, "growth").unwrap_or(0);
    match name {
        BUSH if growth < RIPE_BUSH && picks.below(5) == 0 => with("growth", growth + 1).map(|id| shown(pos, id)).into_iter().collect(),
        _ if AGED.iter().any(|(aged, ..)| *aged == name) => {
            let (age, odds) = (number(properties, "age").unwrap_or(0), AGED.iter().find(|(aged, ..)| *aged == name).map_or(1, |(.., odds)| *odds));
            with("age", age + 1).filter(|_| picks.below(odds) == 0).map(|id| shown(pos, id)).into_iter().collect()
        }
        // The upper half comes with the third stage, as BDS's did, and grows with the lower.
        PITCHER if growth < RIPE_PITCHER && crop_grows(level, pos, name, darken, picks) => {
            let lower = with("growth", growth + 1);
            let upper = lower.filter(|_| growth + 1 >= TALL_PITCHER && (is_air(level, above) || name_at(level, above) == Some(PITCHER))).and_then(|lower| with_property(registry, lower, UPPER, "1"));
            lower.map(|id| shown(pos, id)).into_iter().chain(upper.map(|id| shown(above, id))).collect()
        }
        _ if CROPS.contains(&name) && growth < RIPE && crop_grows(level, pos, name, darken, picks) => with("growth", growth + 1).map(|id| shown(pos, id)).into_iter().collect(),
        _ if STEMS.iter().any(|(stem, _)| *stem == name) && crop_grows(level, pos, name, darken, picks) => {
            if growth < RIPE {
                return with("growth", growth + 1).map(|id| shown(pos, id)).into_iter().collect();
            }
            let side = SIDES[picks.below(4) as usize];
            let (beside, fruit) = (side.neighbour(pos), STEMS.iter().find(|(stem, _)| *stem == name).map(|(_, fruit)| *fruit));
            let is_free = is_air(level, beside) && name_at(level, Face::Down.neighbour(beside)).is_some_and(|ground| FRUIT_GROUND.contains(&ground));
            let fruit = fruit.filter(|_| is_free).and_then(|fruit| send::resolve(registry, &Block::new(fruit)));
            let Some((fruit, turned)) = fruit.zip(with("facing_direction", side as u8)) else { return Vec::new() };
            vec![shown(beside, fruit), shown(pos, turned)]
        }
        _ if COLUMNS.contains(&name) && is_air(level, above) && height(level, pos, name) < COLUMN_HEIGHT => {
            let age = number(properties, "age").unwrap_or(0);
            let (older, young) = (with("age", age + 1), with("age", 0));
            match (age < COLUMN_AGE, older, young) {
                (true, Some(older), _) => vec![Change { pos, id: older, how: How::Quiet }],
                (false, _, Some(young)) => vec![shown(above, young), Change { pos, id: young, how: How::Quiet }],
                _ => Vec::new(),
            }
        }
        BAMBOO if is_air(level, above) && is_lit(level, above, darken) && picks.below(BAMBOO_ODDS) == 0 => {
            let tall = height(level, pos, name) + 1;
            if tall > BAMBOO_HEIGHT { Vec::new() } else { bamboo(level, above, tall) }
        }
        _ if cover::is_ticked(name) => cover::ticked(level, pos, name, darken, picks),
        _ if super::copper::is_ticked(name) => super::copper::ticked(level, pos, picks),
        // Marked leaves drop on their first tick with no wood near, as BDS's did; held, they rest.
        _ if leaves::is_marked(name, properties) => match leaves::hangs(level, pos) {
            true => with_property(registry, id, "update_bit", "0").map(|id| shown(pos, id)).into_iter().collect(),
            false => vec![Change { pos, id, how: How::Breaks }],
        },
        FARMLAND => {
            let wet = number(properties, WET).unwrap_or(0);
            let is_sown = name_at(level, above).is_some_and(|over| CROPS.contains(&over) || over.ends_with("_crop") || STEMS.iter().any(|(stem, _)| *stem == over));
            let now = match (is_watered(level, pos), wet) {
                (true, WETTEST) => None,
                (true, _) => with(WET, WETTEST),
                (false, 1..) => with(WET, wet - 1),
                (false, 0) if !is_sown => send::resolve(registry, &Block::new(cover::DIRT)),
                (false, 0) => None,
            };
            now.map(|id| shown(pos, id)).into_iter().collect()
        }
        _ => Vec::new(),
    }
}
