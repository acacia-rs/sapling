//! What a broken block leaves behind, by the game's loot tables (`sapling_data::block_loot`:
//! the Java game's, which Bedrock follows for the blocks checked). The tables speak Java's
//! block states; `state_holds` reads them off a Bedrock block. Putting it in the world: spill.rs.
// TODO: a row about a state this cannot read off the Bedrock block is skipped, stems leave
// nothing (their seed count is a formula the table file does not carry).

use sapling_api::Item;
use sapling_data::{BlockLoot, Fortune, LootRow, StateTest, Tooling};

mod dice;
mod spill;
#[cfg(test)]
mod tests;

pub(crate) use dice::Dice;
pub(crate) use spill::mined;
pub(super) use spill::{Tags, add_entity_loot, let_go, spill, spill_tagged, thrown};

/// Java states that count from one where Bedrock's count from zero: Java's, Bedrock's.
const FROM_ONE: &[(&str, &str)] = &[("candles", "candles"), ("pickles", "cluster_count"), ("layers", "height"), ("flower_amount", "growth"), ("segment_amount", "growth")];
/// Bedrock's `growth` of a ripe crop.
const RIPE: i32 = 7;
/// Chances are thrown out of this many.
const CHANCE_SIDES: u32 = 1_000_000;

/// What breaks a block, as far as its drop depends on it.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Breaker {
    /// The tool makes the block drop at all (`Mining::can_harvest`).
    pub harvests: bool,
    pub silk_touch: bool,
    pub shears: bool,
    pub pickaxe: bool,
    /// The Fortune level.
    pub fortune: u8,
    /// A player, not a block losing its hold.
    pub by_entity: bool,
}

impl Breaker {
    /// A block that breaks on its own, having lost its hold.
    pub const UNAIDED: Breaker = Breaker { harvests: true, silk_touch: false, shears: false, pickaxe: false, fortune: 0, by_entity: false };
}

fn some(name: &str, count: u32) -> Vec<Item> {
    let fits = count > 0 && sapling_data::is_item(name);
    fits.then(|| Item::new(name, count.min(64) as u8)).into_iter().collect()
}

/// A Bedrock block as the tables' conditions see it.
struct Broken<'a> {
    /// Without the namespace.
    name: &'a str,
    properties: &'a str,
    java: &'a str,
    is_double_slab: bool,
}

impl Broken<'_> {
    fn number(&self, key: &str) -> Option<i32> {
        self.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key)?.1.parse().ok()
    }

    /// The Java state `key` of this block, where it can be told: a crop's `age` from Bedrock's
    /// `growth`, a slab's `type` from the block being the double one. A two-cell block drops
    /// once from whichever cell was broken (doors.rs takes the other out), so it is the half
    /// the table wants.
    fn java_state(&self, key: &str) -> Option<String> {
        let counted = |bedrock: &str| self.number(bedrock).map(|from_zero| (from_zero + 1).to_string());
        match key {
            "type" => Some(if self.is_double_slab { "double" } else { "single" }.to_owned()),
            "half" => Some("lower".to_owned()),
            "part" => Some("head".to_owned()),
            "unstable" => Some("false".to_owned()),
            // A shot pot shatters (shot_blocks.rs); broken any other way it is whole.
            "cracked" => Some("false".to_owned()),
            "berries" => Some(self.name.contains("with_berries").to_string()),
            "age" => match (self.number("growth"), self.number("age"), sapling_data::ripe_age(self.java)) {
                (Some(growth), _, Some(ripe)) => Some(if growth >= RIPE { i32::from(ripe) } else { growth * i32::from(ripe) / RIPE }),
                (None, age, _) => age,
                (Some(_), _, None) => None,
            }
            .map(|age| age.to_string()),
            _ => FROM_ONE.iter().find(|(java, _)| *java == key).and_then(|(_, bedrock)| counted(bedrock)),
        }
    }

    fn holds(&self, test: &StateTest) -> bool {
        self.java_state(test.key).is_some_and(|value| test.values.contains(&value.as_str()) != test.negated)
    }
}

fn tool_fits(wanted: Tooling, by: &Breaker) -> bool {
    match wanted {
        Tooling::Any => true,
        Tooling::SilkTouch => by.silk_touch,
        Tooling::NoSilkTouch => !by.silk_touch,
        Tooling::Shears => by.shears,
        Tooling::NoShears => !by.shears,
        Tooling::ShearsOrSilkTouch => by.shears || by.silk_touch,
        Tooling::NoShearsNoSilkTouch => !by.shears && !by.silk_touch,
    }
}

/// How many a row leaves, by the game's formulas for Fortune.
fn count(row: &LootRow, fortune: u8, roll: &mut dyn FnMut(u32) -> u32) -> u32 {
    let (least, level) = (u32::from(row.min), u32::from(fortune));
    let plain = |roll: &mut dyn FnMut(u32) -> u32| least + roll(u32::from(row.max.saturating_sub(row.min)) + 1);
    let count = match row.fortune {
        Fortune::None => plain(&mut *roll),
        Fortune::OreDrops => plain(&mut *roll) * if level > 0 { roll(level + 2).max(1) } else { 1 },
        Fortune::UniformBonus(times) => plain(&mut *roll) + roll(u32::from(times) * level + 1),
        Fortune::Binomial { extra, p } => least + (0..u32::from(extra) + level).filter(|_| roll(CHANCE_SIDES) < (p * CHANCE_SIDES as f32) as u32).count() as u32,
    };
    count.clamp(row.limit.0.into(), row.limit.1.into())
}

/// What the block `block` (namespaced) with `properties` leaves when `by` breaks it; nothing
/// for a block no item stands for. `roll(n)` throws a number below `n`.
pub(super) fn of(block: &str, properties: &str, by: &Breaker, roll: &mut dyn FnMut(u32) -> u32) -> Vec<Item> {
    // A shulker box drops itself with what it holds (containers::spill).
    if !by.harvests || crate::containers::is_shulker_box(block) {
        return Vec::new();
    }
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    let (java, is_double_slab) = sapling_data::java_block(name);
    let broken = Broken { name, properties, java, is_double_slab };
    let rows = match sapling_data::block_loot(java) {
        BlockLoot::Nothing => return Vec::new(),
        // A Bedrock block of another name than its item (`reeds`) has the Java id's item.
        BlockLoot::Itself => return [block, &format!("minecraft:{java}")].iter().map(|item| some(item, 1)).find(|left| !left.is_empty()).unwrap_or_default(),
        BlockLoot::Rows(rows) => rows,
    };
    let (mut left, mut last_left) = (Vec::new(), false);
    for row in rows {
        let wanted = tool_fits(row.tooling, by) && (by.by_entity || !row.by_entity) && (by.pickaxe || !row.pickaxe) && !(by.pickaxe && row.no_pickaxe) && row.states.iter().all(|test| broken.holds(test));
        let chance = row.chances[usize::from(by.fortune).min(row.chances.len() - 1)];
        // An "otherwise" row is the other side of the throw the row before it lost.
        last_left = wanted && if row.otherwise { !last_left } else { roll(CHANCE_SIDES) < (chance * CHANCE_SIDES as f32) as u32 };
        if last_left {
            left.extend(some(row.item, count(row, by.fortune, roll)));
        }
    }
    left
}
