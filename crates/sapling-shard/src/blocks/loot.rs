//! What a broken block leaves behind, by the game's loot tables (`sapling_data::block_loot`:
//! the Java game's, which Bedrock follows for the blocks checked). The tables speak Java's
//! block states; `state_holds` reads them off a Bedrock block.
// TODO: a row about a state this cannot read off the Bedrock block is skipped, stems leave
// nothing (their seed count is a formula the table file does not carry).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};
use sapling_data::{BlockLoot, Fortune, LootRow, StateTest, Tooling};

use crate::components::Layer;
use crate::{drops, inventory};

/// Java states that count from one where Bedrock's count from zero: Java's, Bedrock's.
const FROM_ONE: &[(&str, &str)] = &[("candles", "candles"), ("pickles", "cluster_count"), ("layers", "height"), ("flower_amount", "growth"), ("segment_amount", "growth")];
/// Bedrock's `growth` of a ripe crop.
const RIPE: i32 = 7;
const TILE_DROPS: &str = "doTileDrops";
const ENTITY_DROPS: &str = "doEntityDrops";
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

/// The shard's dice: xorshift, the same throws on every run.
#[derive(Resource)]
pub(crate) struct Dice(u64);

impl Default for Dice {
    fn default() -> Self {
        Self(0x9E37_79B9_7F4A_7C15)
    }
}

impl Dice {
    /// Dice of their own, throwing the same for the same seed.
    pub fn seeded(seed: u64) -> Self {
        // Xorshift never leaves 0.
        Self((seed ^ Self::default().0).max(1))
    }

    /// A number below `sides`.
    pub fn roll(&mut self, sides: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 32) as u32) % sides.max(1)
    }

    /// A number from 0 up to 1.
    pub fn fraction(&mut self) -> f32 {
        self.roll(1 << 20) as f32 / (1 << 20) as f32
    }

    /// A number from the normal spread about 0 (Box-Muller).
    pub fn gaussian(&mut self) -> f32 {
        let (u, v) = (1.0 - self.fraction(), self.fraction());
        (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
    }
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
        let wanted = tool_fits(row.tooling, by) && (by.by_entity || !row.by_entity) && (by.pickaxe || !row.pickaxe) && row.states.iter().all(|test| broken.holds(test));
        let chance = row.chances[usize::from(by.fortune).min(row.chances.len() - 1)];
        // An "otherwise" row is the other side of the throw the row before it lost.
        last_left = wanted && if row.otherwise { !last_left } else { roll(CHANCE_SIDES) < (chance * CHANCE_SIDES as f32) as u32 };
        if last_left {
            left.extend(some(row.item, count(row, by.fortune, roll)));
        }
    }
    left
}

/// The tags a broken block's entity gives the items it leaves, which a plugin's [`Item`] cannot
/// carry: each goes on the first spilled stack of its item.
#[derive(Default)]
pub(super) struct Tags(Vec<(&'static str, acacia_proto::nbt::Value)>);

impl Tags {
    /// Whether one goes on a stack of the item `name`.
    pub(super) fn tags(&self, name: &str) -> bool {
        self.0.iter().any(|(tagged, _)| *tagged == name)
    }

    fn put_on(&mut self, stack: &mut sapling_world::ItemStack) {
        if let Some(at) = self.0.iter().position(|(name, _)| *name == stack.name) {
            stack.tag = Some(self.0.remove(at).1);
        }
    }
}

/// Adds to `items`, left by the block at `pos`, what its entity kept (entities.rs); the tags
/// for [`spill_tagged`]. Call before the block goes.
pub(super) fn add_entity_loot(world: &World, layer: Entity, pos: BlockPos, items: &mut Vec<Item>) -> Tags {
    let Some(kept) = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)) else { return Tags::default() };
    super::entities::into_loot(&kept.nbt, items);
    Tags(super::entities::loot_tags(&kept.nbt))
}

/// [`of`] with the shard's dice.
pub(super) fn thrown(world: &mut World, block: &str, properties: &str, by: &Breaker) -> Vec<Item> {
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    of(block, properties, by, &mut |sides| dice.roll(sides))
}

/// What the block at `pos` would drop to `tool` (`/loot ... mine`); the block stays. Nothing from
/// air, a liquid or a cell outside the world.
pub(crate) fn mined(world: &mut World, layer: Entity, pos: BlockPos, tool: Option<&Item>) -> Vec<Item> {
    let Some(level) = super::level(world, layer).filter(|level| level.in_height(pos[1]) && level.is_loaded(super::send::column(pos))) else { return Vec::new() };
    let Some(state) = level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| !state.is_air() && !state.is_liquid()) else { return Vec::new() };
    let (name, properties, by) = (state.name.to_owned(), state.properties.to_owned(), super::mining::breaker_of(tool, &state.mining));
    thrown(world, &name, &properties, &by)
}

/// `items` as stacks; items the game lacks are left out.
fn stacks(world: &World, layer: Entity, items: &[Item]) -> Vec<sapling_world::ItemStack> {
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return Vec::new() };
    items.iter().filter(|item| item.count > 0).filter_map(|item| inventory::stored(item, registry)).collect()
}

/// Lays `items` where the block at `pos` was, tossed out of its cell, unless the world's
/// `doTileDrops` is off.
pub(super) fn spill(world: &mut World, layer: Entity, pos: BlockPos, items: &[Item]) {
    spill_tagged(world, layer, pos, items, &mut Tags::default());
}

/// [`spill`], `tags` going on the stacks of their items.
pub(super) fn spill_tagged(world: &mut World, layer: Entity, pos: BlockPos, items: &[Item], tags: &mut Tags) {
    if !crate::rules::on(world, layer, TILE_DROPS) {
        return;
    }
    for mut stack in stacks(world, layer, items) {
        tags.put_on(&mut stack);
        drops::spill(world, layer, stack, pos);
    }
}

/// Tosses `items` out of `centre`, where an entity that broke up was, unless the world's
/// `doEntityDrops` is off.
pub(super) fn let_go(world: &mut World, layer: Entity, centre: [f32; 3], items: &[Item]) {
    if !crate::rules::on(world, layer, ENTITY_DROPS) {
        return;
    }
    for stack in stacks(world, layer, items) {
        drops::let_go(world, layer, stack, centre);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAND: Breaker = Breaker { by_entity: true, ..Breaker::UNAIDED };

    /// What a block leaves when every throw comes up `thrown` (capped below the sides).
    fn left_of(block: &str, properties: &str, by: Breaker, thrown: u32) -> Vec<(String, u8)> {
        of(&format!("minecraft:{block}"), properties, &by, &mut |sides| thrown.min(sides - 1)).into_iter().map(|item| (item.name, item.count)).collect()
    }

    fn left(block: &str, by: Breaker, thrown: u32) -> Vec<(String, u8)> {
        left_of(block, "", by, thrown)
    }

    fn named(item: &str, count: u8) -> Vec<(String, u8)> {
        vec![(format!("minecraft:{item}"), count)]
    }

    #[test]
    fn a_block_leaves_itself_or_what_its_table_names() {
        assert_eq!([left("dirt", HAND, 0), left("stone", HAND, 0), left("oak_planks", HAND, 0)], [named("dirt", 1), named("cobblestone", 1), named("oak_planks", 1)]);
        assert_eq!([left("deepslate_diamond_ore", HAND, 0), left("lit_redstone_ore", HAND, u32::MAX), left("reeds", HAND, 0)], [named("diamond", 1), named("redstone", 5), named("sugar_cane", 1)]);
        assert_eq!([left("glass", HAND, 0), left("blue_ice", HAND, 0), left("mob_spawner", HAND, 0)], [vec![], vec![], vec![]]);
        assert_eq!([left("oak_slab", HAND, 0), left("oak_double_slab", HAND, 0)], [named("oak_slab", 1), named("oak_slab", 2)]);
        assert!(left("stone", Breaker { harvests: false, ..HAND }, 0).is_empty(), "nothing from a block the tool does not harvest");
        assert!(of("mod:lamp", "", &HAND, &mut |_| 0).is_empty(), "no item stands for it");
        let signs = [left("standing_sign", HAND, 0), left("wall_sign", HAND, 0), left("spruce_standing_sign", HAND, 0), left("darkoak_wall_sign", HAND, 0)];
        assert_eq!(signs, [named("oak_sign", 1), named("oak_sign", 1), named("spruce_sign", 1), named("dark_oak_sign", 1)]);
        assert_eq!([left("standing_banner", HAND, 0), left("wall_banner", HAND, 0)], [named("banner", 1), named("banner", 1)]);
    }

    #[test]
    fn chance_enchantments_and_shears_change_what_is_left() {
        assert_eq!([left("gravel", HAND, 0), left("gravel", HAND, u32::MAX)], [named("flint", 1), named("gravel", 1)]);
        assert_eq!([left("short_grass", HAND, 0), left("short_grass", HAND, u32::MAX), left("oak_leaves", HAND, u32::MAX)], [named("wheat_seeds", 1), vec![], vec![]]);
        assert_eq!(left("oak_leaves", HAND, 0), [named("apple", 1), named("oak_sapling", 1), named("stick", 1)].concat());
        let silk = Breaker { silk_touch: true, ..HAND };
        assert_eq!([left("stone", silk, 0), left("glass", silk, 0), left("diamond_ore", silk, 0)], [named("stone", 1), named("glass", 1), named("diamond_ore", 1)]);
        assert_eq!(left("oak_leaves", Breaker { shears: true, ..HAND }, u32::MAX), named("oak_leaves", 1));
        let fortune = Breaker { fortune: 3, ..HAND };
        assert_eq!([left("diamond_ore", fortune, u32::MAX), left("diamond_ore", fortune, 0), left("stone", fortune, u32::MAX)], [named("diamond", 4), named("diamond", 1), named("cobblestone", 1)]);
    }

    #[test]
    fn the_tables_states_are_read_off_bedrock_blocks() {
        assert_eq!(left_of("wheat", "growth=3", HAND, 0), named("wheat_seeds", 1));
        assert_eq!(left_of("wheat", "growth=7", HAND, 0), [named("wheat", 1), named("wheat_seeds", 4)].concat(), "every try of the binomial succeeds");
        assert_eq!([left_of("beetroot", "growth=7", HAND, u32::MAX), left_of("beetroot", "growth=4", HAND, 0)], [[named("beetroot", 1), named("beetroot_seeds", 1)].concat(), named("beetroot_seeds", 1)]);
        assert_eq!([left_of("snow_layer", "covered_bit=0,height=2", HAND, 0), left_of("snow_layer", "covered_bit=0,height=2", Breaker::UNAIDED, 0)], [named("snowball", 3), vec![]]);
    }
}
