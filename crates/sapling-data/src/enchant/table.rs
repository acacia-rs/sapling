//! The enchanting table: the bookshelves that count, the three offers it draws for an item from
//! the player's seed, and what taking one does (BDS's rules: docs/research/workstations.md,
//! "Enchanting table").

use super::kinds::{self, Activation, KINDS};
use super::list::{all, with_all};
use super::Draws;
use crate::gear::{BOOK, ENCHANTED_BOOK, Made, gear};
use crate::items::Stack;

pub const LAPIS: &str = "minecraft:lapis_lazuli";
const SHELF: &str = "minecraft:bookshelf";
const MAX_SHELVES: u32 = 15;
/// The words an offer's name is drawn from (BDS's table, each with its trailing space).
const WORDS: [&str; 55] = [
    "the ", "elder ", "scrolls ", "klaatu ", "berata ", "niktu ", "xyzzy ", "bless ", "curse ", "light ", "darkness ", "fire ", "air ", "earth ",
    "water ", "hot ", "dry ", "cold ", "wet ", "ignite ", "snuff ", "embiggen ", "twist ", "shorten ", "stretch ", "fiddle ", "destroy ", "imbue ",
    "galvanize ", "enchant ", "free ", "limited ", "range ", "of ", "towards ", "inside ", "sphere ", "cube ", "self ", "other ", "ball ",
    "mental ", "physical ", "grow ", "shrink ", "demon ", "elemental ", "spirit ", "animal ", "creature ", "beast ", "humanoid ", "undead ",
    "fresh ", "stale ",
];

/// One of the table's offers: the level it wants, and the enchantments it gives.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    pub cost: u8,
    /// The item's `EnchantSlot` bits.
    pub slots: u32,
    pub enchants: Vec<(i16, u8)>,
    /// What the screen shows of it, in the enchanting table's script.
    pub name: String,
}

/// Each place a bookshelf counts, beside the table (at 0, 0, 0), with the column between, both
/// cells of which must let it through; a shelf off a corner looks through the corner.
pub fn shelf_spots() -> impl Iterator<Item = ([i32; 3], [[i32; 3]; 2])> {
    let ring = (-2..=2).flat_map(|dx: i32| (-2..=2).map(move |dz: i32| (dx, dz))).filter(|(dx, dz)| dx.abs() == 2 || dz.abs() == 2);
    ring.flat_map(|(dx, dz)| (0..=1).map(move |dy| ([dx, dy, dz], [0, 1].map(|gap_y| [dx.signum(), gap_y, dz.signum()]))))
}

pub fn is_shelf(block: &str) -> bool {
    block == SHELF
}

/// Whether a block between a shelf and the table lets it count: on BDS air, a carpet and a snow
/// layer do; a torch, short grass, glass and stone do not (research/captures/bds-enchants.txt).
pub fn lets_through(block: &str) -> bool {
    matches!(block, "minecraft:air" | "minecraft:snow_layer") || block.ends_with("_carpet")
}

/// The lapis the offer at `index` takes.
pub fn lapis(index: usize) -> u8 {
    index as u8 + 1
}

/// The levels the offer at `index` takes: its place, not its cost.
pub fn levels_taken(index: usize) -> i32 {
    index as i32 + 1
}

/// The level each of the three offers wants with `shelves` around the table.
pub fn costs(shelves: u32, draws: &mut dyn Draws) -> [u8; 3] {
    let shelves = shelves.min(MAX_SHELVES) as i32;
    let base = draws.int(8) + 1 + shelves / 2 + draws.int(shelves + 1);
    [(base / 3).max(1), base * 2 / 3 + 1, base.max(shelves * 2)].map(|cost| cost as u8)
}

/// The power an offer of `cost` has on an item of `enchantability`.
fn power(cost: u8, enchantability: u8, draws: &mut dyn Draws) -> i32 {
    let quarter = i32::from(enchantability) / 4 + 1;
    let level = i32::from(cost) + draws.int(quarter) + draws.int(quarter) + 1;
    let spread = draws.float() * 0.3 - 0.15 + 1.0;
    ((spread * level as f32 + 0.5) as i32).max(1)
}

/// Every enchantment that can be drawn for an item of `slots` at `power`, at its highest level
/// whose least power that reaches: none of [`UNFOUND`], and treasures only when `treasure`.
fn reachable(slots: u32, power: i32, treasure: bool) -> Vec<(i16, u8)> {
    let level = |kind: &kinds::Kind| (1..=kind.max).rev().find(|level| kind.min_power(*level) <= power);
    let fits = |id: usize, kind: &kinds::Kind| (treasure || !kind.treasure) && !UNFOUND.contains(&(id as i16)) && kind.goes_on(slots);
    KINDS.iter().enumerate().filter(|(id, kind)| fits(*id, kind)).filter_map(|(id, kind)| Some((id as i16, level(kind)?))).collect()
}

/// Draws one of `pool` out by weight, and keeps it if it goes with what was chosen.
fn take(pool: &mut Vec<(i16, u8)>, chosen: &mut Vec<(i16, u8)>, draws: &mut dyn Draws) {
    let weight = |id: i16| kinds::kind_of(id).map_or(0, kinds::Kind::weight) as i32;
    let mut roll = draws.int(pool.iter().map(|(id, _)| weight(*id)).sum());
    let at = pool.iter().position(|(id, _)| {
        roll -= weight(*id);
        roll < 0
    });
    let picked = pool.remove(at.unwrap_or(pool.len() - 1));
    let activation = |id: i16| kinds::kind_of(id).map(|kind| kind.activation);
    // Only an enchantment acting the same way can clash (BDS's `ItemEnchants::canEnchant`).
    if chosen.iter().all(|(have, _)| activation(*have) != activation(picked.0) || kinds::go_together(*have, picked.0)) {
        chosen.push(picked);
    }
}

/// The enchantments an offer of `cost` gives an item of `slots` and `enchantability`
/// (`EnchantUtils::selectEnchantments`; the table allows no treasure).
fn chosen(slots: u32, enchantability: u8, cost: u8, treasure: bool, draws: &mut dyn Draws) -> Vec<(i16, u8)> {
    let mut power = power(cost, enchantability, draws);
    let mut pool = reachable(slots, power, treasure);
    let mut chosen = Vec::new();
    if pool.is_empty() {
        return chosen;
    }
    let initial = pool.len();
    take(&mut pool, &mut chosen, draws);
    let mut picks = 1;
    while draws.int(50) <= power && picks < initial {
        take(&mut pool, &mut chosen, draws);
        (power, picks) = (power * 4 / 5 + 1, picks + 1);
    }
    chosen
}

/// An offer's name: three or four of the table's words, each with its space; BDS keeps the last.
fn name(draws: &mut dyn Draws) -> String {
    let words = draws.int(2) + 3;
    (0..words).map(|_| WORDS[draws.int(WORDS.len() as i32) as usize]).collect()
}

/// The three offers for `item` with `shelves` around the table, drawn from the player's seed
/// (`draws` seeded by it); none for an item the table does not enchant.
pub fn offers(shelves: u32, item: &Stack<'_>, draws: &mut dyn Draws) -> Vec<Offer> {
    let Some(gear) = gear(item.name).filter(|gear| gear.enchantability > 0 && all(item.tag).is_empty()) else { return Vec::new() };
    let costs = costs(shelves, draws);
    let offer = |cost: u8, draws: &mut dyn Draws| {
        let enchants = chosen(gear.slots, gear.enchantability, cost, false, draws);
        Offer { cost, slots: gear.slots, enchants, name: name(draws) }
    };
    costs.into_iter().map(|cost| offer(cost, draws)).collect()
}

/// The item an offer makes of `item`: a book becomes an enchanted book.
pub fn enchanted(item: &Stack<'_>, offer: &Offer) -> Option<Made> {
    let name = if item.name == BOOK { ENCHANTED_BOOK } else { item.name };
    Some(Made { name: name.to_owned(), tag: with_all(item.tag.cloned(), &offer.enchants) }).filter(|_| !offer.enchants.is_empty())
}

/// `item` enchanted by a loot or trade `enchant_with_levels` of `cost` (BDS 14752bf20 calls
/// `EnchantUtils::randomlyEnchant` 141b62a20): the table's draw, treasures in it when `treasure`.
// BDS draws the enchantments from a thread's own random, not the loot's; sapling's shard dice are its own anyway.
pub fn with_levels(item: &Stack<'_>, cost: u8, treasure: bool, draws: &mut dyn Draws) -> Option<Made> {
    let gear = gear(item.name).filter(|gear| gear.enchantability > 0)?;
    let enchants = chosen(gear.slots, gear.enchantability, cost, treasure, draws);
    enchanted(item, &Offer { cost, slots: gear.slots, enchants, name: String::new() })
}

/// Enchantments never drawn, nor sold by a librarian: Soul Speed, Swift Sneak, Wind Burst (their
/// vtables' slot 0x80 answers false, 141b5be40).
const UNFOUND: [i16; 3] = [36, 37, 38];

/// A librarian's book: an enchantment other than a curse at any of its levels, and whether it is a
/// treasure, which doubles its price.
pub fn for_trade(draws: &mut dyn Draws) -> (i16, u8, bool) {
    let sold: Vec<(i16, &kinds::Kind)> = KINDS.iter().enumerate().map(|(id, kind)| (id as i16, kind)).filter(|(id, kind)| !kind.curse && !UNFOUND.contains(id)).collect();
    let (id, kind) = sold[draws.int(sold.len() as i32) as usize];
    (id, draws.int(i32::from(kind.max)) as u8 + 1, kind.treasure)
}

/// An enchanted book of `levels`, as a librarian sells it.
pub fn book(levels: &[(i16, u8)]) -> Made {
    Made { name: ENCHANTED_BOOK.to_owned(), tag: with_all(None, levels) }
}

/// An offer's enchantments in the three lists the screen shows: worn, held, on the item itself.
pub fn by_activation(enchants: &[(i16, u8)]) -> [Vec<(i16, u8)>; 3] {
    let of = |activation: Activation| enchants.iter().filter(|(id, _)| kinds::kind_of(*id).is_some_and(|kind| kind.activation == activation)).copied().collect();
    [of(Activation::Equipped), of(Activation::Held), of(Activation::Itself)]
}
