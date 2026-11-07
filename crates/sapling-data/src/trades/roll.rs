//! The offers a trader draws from a tier of its table, as BDS's `_generateTrades` (0x141bac370)
//! does: of each group's trades the trader's type offers, `select` by weight in the order drawn
//! (all of them where `select` is not below their count), then each item one of its alternatives
//! for the type, its count in its range, and what its functions make of it (docs/research/
//! villagers.md "Offers").
//! An explorer map comes without its `map_uuid`: the shard makes its map (villagers/deals.rs).

use acacia_proto::nbt::Value as Nbt;

use super::{Function, Item, Table, Trade};
use crate::enchant::{Draws, table};
use crate::items::Stack;

const EMERALD: &str = "minecraft:emerald";
/// The `Damage` of a wanted item that takes any of its kind, as BDS writes it.
pub const ANY_DAMAGE: i16 = i16::MAX;
/// The most a librarian's book costs.
const BOOK_PRICE_CAP: i32 = 64;
/// Each dye colour's RGB by its colour index (black 0 to white 15), BDS's table at 0x14a7716e4.
const DYES: [u32; 16] = [
    0x1D1D21, 0xB02E26, 0x5E7C16, 0x835432, 0x3C44AA, 0x8932B8, 0x169C9C, 0x9D9D97, 0x474F52, 0xF38BAA, 0x80C71F, 0xFED83D, 0x3AB3DA, 0xC74EBD, 0xF9801D, 0xF0F0F0,
];
/// The colour index `random_dye`'s draw of 16 lands on (BDS's tables at 0x14a805bb8 and 0x14a805bf8).
const DYE_DRAWS: [usize; 16] = [15, 7, 8, 0, 3, 1, 14, 11, 10, 2, 6, 12, 4, 5, 13, 9];

/// `random_dye` (BDS's 0x14752e880): one dye, a second below 0.3 of a draw and a third below 0.2
/// of another, their colours averaged as BDS's floats do; ARGB, opaque.
pub(super) fn random_dye(draws: &mut dyn Draws) -> u32 {
    let dye = |draws: &mut dyn Draws| {
        let rgb = DYES[DYE_DRAWS[draws.int(16) as usize]];
        [16, 8, 0].map(|shift| ((rgb >> shift) & 0xFF) as f32 / 255.0)
    };
    let mut sum = dye(draws);
    let mut dyes = 1;
    for odds in [0.3, 0.2] {
        if draws.float() < odds {
            let more = dye(draws);
            (0..3).for_each(|channel| sum[channel] += more[channel]);
            dyes += 1;
        }
    }
    if dyes > 1 {
        let share = 1.0 / dyes as f32;
        sum = sum.map(|channel| (channel * share).clamp(0.0, 1.0));
    }
    let [r, g, b] = sum.map(|channel| (channel * 255.0) as u32);
    0xFF00_0000 | r << 16 | g << 8 | b
}

/// One item of an offer.
#[derive(Debug, Clone, PartialEq)]
pub struct Ware {
    pub name: String,
    pub count: u8,
    pub damage: i16,
    pub tag: Option<Nbt>,
    /// How far demand moves its price (`priceMultiplierA`, `priceMultiplierB`).
    pub price_multiplier: f32,
}

/// One offer: what the trader wants, what it gives, and what a trade of it counts.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    pub buy_a: Ware,
    pub buy_b: Option<Ware>,
    pub sell: Ware,
    pub trader_exp: i32,
    pub max_uses: i32,
    pub reward_exp: bool,
}

fn between(draws: &mut dyn Draws, [low, high]: [i32; 2]) -> i32 {
    low + draws.int(high - low + 1)
}

fn offered(item: &Item, mark: i32) -> bool {
    item.marks.as_ref().is_none_or(|marks| marks.contains(&mark))
}

/// Whether every item of `trade` has an alternative a trader of type `mark` offers.
fn can_offer(trade: &Trade, mark: i32) -> bool {
    trade.wants.iter().chain(&trade.gives).all(|choice| choice.iter().any(|item| offered(item, mark)))
}

/// One of `choice`'s alternatives for `mark`, made, and the price a librarian's book asks; one
/// `wanted` without a data value takes any.
fn ware(choice: &[Item], mark: i32, wanted: bool, draws: &mut dyn Draws) -> Option<(Ware, Option<i32>)> {
    let alternatives: Vec<&Item> = choice.iter().filter(|item| offered(item, mark)).collect();
    let item = alternatives.get(draws.int(alternatives.len() as i32) as usize)?;
    let count = between(draws, item.count.map(i32::from)) as u8;
    let mut ware = Ware { name: item.name.clone(), count, damage: item.data.unwrap_or(if wanted { ANY_DAMAGE } else { 0 }), tag: None, price_multiplier: item.price_multiplier };
    let mut price = None;
    for function in &item.functions {
        match *function {
            Function::EnchantWithLevels(levels, treasure) => {
                let cost = between(draws, levels.map(i32::from)) as u8;
                let stack = Stack { name: &ware.name, count: ware.count, damage: ware.damage, block: None, tag: None };
                if let Some(made) = table::with_levels(&stack, cost, treasure, draws) {
                    (ware.name, ware.tag) = (made.name, made.tag);
                }
            }
            Function::EnchantBook { base, random, per_level, per_level_random } => {
                let (id, level, treasure) = table::for_trade(draws);
                let level_i = i32::from(level);
                let cost = base + draws.int(random + level_i * per_level_random) + level_i * per_level;
                price = Some((if treasure { cost * 2 } else { cost }).min(BOOK_PRICE_CAP));
                let made = table::book(&[(id, level)]);
                (ware.name, ware.tag) = (made.name, made.tag);
            }
            Function::RandomAux(values) => ware.damage = between(draws, values.map(i32::from)) as i16,
            Function::RandomDye => ware.tag = Some(Nbt::Compound(vec![("customColor".into(), Nbt::Int(random_dye(draws) as i32))])),
            Function::ExplorationMap(damage) => {
                ware.damage = damage;
                ware.tag = Some(Nbt::Compound(vec![("map_display_players".into(), Nbt::Byte(1)), ("map_regenerate".into(), Nbt::Byte(1))]));
            }
        }
    }
    Some((ware, price))
}

fn make(trade: &Trade, mark: i32, draws: &mut dyn Draws) -> Option<Offer> {
    let mut gives = Vec::new();
    let mut price = None;
    for choice in &trade.gives {
        let (made, asked) = ware(choice, mark, false, draws)?;
        price = price.or(asked);
        gives.push(made);
    }
    let mut wants = Vec::new();
    for choice in &trade.wants {
        wants.push(ware(choice, mark, true, draws)?.0);
    }
    // A book's price is the emeralds it wants.
    if let (Some(price), Some(emeralds)) = (price, wants.iter_mut().find(|ware| ware.name == EMERALD)) {
        emeralds.count = price.clamp(1, 64) as u8;
    }
    let mut wants = wants.into_iter();
    Some(Offer { buy_a: wants.next()?, buy_b: wants.next(), sell: gives.into_iter().next()?, trader_exp: trade.trader_exp, max_uses: trade.max_uses, reward_exp: trade.reward_exp })
}

/// Draws `select` of `trades` by weight, without putting one back, as BDS does: the `n`th draw
/// rolls over the trades from place `n` on and swaps the one it lands on into place `n`. Their
/// places in the group, in the order drawn; all for a `select` not between 0 and the group's size.
/// Only the trades the trader's type `offers` are drawn from: BDS's plains cartographer drew its two
/// tier 1 and two tier 3 trades from the three each its type offers (bds-an1maps.txt), where a draw
/// over all eight would have found two such one time in nine.
pub(super) fn pick(trades: &[Trade], select: i32, draws: &mut dyn Draws, offers: &dyn Fn(&Trade) -> bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..trades.len()).filter(|at| offers(&trades[*at])).collect();
    if select <= 0 || select as usize >= order.len() {
        return order;
    }
    for n in 0..select as usize {
        let mut roll = draws.int(order[n..].iter().map(|at| trades[*at].weight as i32).sum());
        let Some(landed) = order[n..].iter().position(|at| {
            roll -= trades[*at].weight as i32;
            roll < 0
        }) else {
            break;
        };
        order.swap(n, n + landed);
    }
    order.truncate(select as usize);
    order
}

/// The offers a trader of type `mark` (`MarkVariant`) makes at `tier` (0 the first) of `table`.
pub fn roll(table: &Table, tier: usize, mark: i32, draws: &mut dyn Draws) -> Vec<Offer> {
    let Some(tier) = table.tiers.get(tier) else { return Vec::new() };
    let mut offers = Vec::new();
    for group in &tier.groups {
        for at in pick(&group.trades, group.select, draws, &|trade| can_offer(trade, mark)) {
            offers.extend(make(&group.trades[at], mark, draws));
        }
    }
    offers
}
