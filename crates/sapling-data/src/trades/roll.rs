//! The offers a trader draws from a tier of its table: of each group, `select` trades by weight
//! (all of a group of -1), each item one of its alternatives for the trader's type, its count in
//! its range, and what its functions make of it.
// TODO: BDS's draw was not decompiled: trades are drawn without putting back, alternatives each as
// likely, and the offers keep the table's order.

use acacia_proto::nbt::Value as Nbt;

use super::{Function, Item, Table, Trade};
use crate::enchant::{Draws, table};
use crate::items::Stack;

const EMERALD: &str = "minecraft:emerald";
/// The `Damage` of a wanted item that takes any of its kind, as BDS writes it.
pub const ANY_DAMAGE: i16 = i16::MAX;
/// The most a librarian's book costs.
const BOOK_PRICE_CAP: i32 = 64;
/// The colours a dyed piece of leather is given (each dye's colour, white to black).
// TODO: BDS's `random_dye` was not decompiled; one dye's colour, each as likely.
const DYES: [u32; 16] = [
    0xF9FFFE, 0xF9801D, 0xC74EBD, 0x3AB3DA, 0xFED83D, 0x80C71F, 0xF38BAA, 0x474F52, 0x9D9D97, 0x169C9C, 0x8932B8, 0x3C44AA, 0x835432, 0x5E7C16, 0xB02E26, 0x1D1D21,
];

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
    item.marks.as_ref().is_none_or(|marks| marks.contains(&mark)) && !item.functions.contains(&Function::ExplorationMap)
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
            Function::EnchantWithLevels(levels) => {
                let cost = between(draws, levels.map(i32::from)) as u8;
                let stack = Stack { name: &ware.name, count: ware.count, damage: ware.damage, block: None, tag: None };
                if let Some(made) = table::with_levels(&stack, cost, draws) {
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
            Function::RandomDye => {
                let colour = DYES[draws.int(DYES.len() as i32) as usize] | 0xFF00_0000;
                ware.tag = Some(Nbt::Compound(vec![("customColor".into(), Nbt::Int(colour as i32))]));
            }
            Function::ExplorationMap => return None,
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

/// Draws `select` of `trades` by weight, without putting one back; their places in the group.
fn pick(trades: &[&Trade], select: i32, draws: &mut dyn Draws) -> Vec<usize> {
    let mut left: Vec<usize> = (0..trades.len()).collect();
    if select < 0 || select as usize >= trades.len() {
        return left;
    }
    let mut chosen = Vec::new();
    for _ in 0..select {
        let mut roll = draws.int(left.iter().map(|at| trades[*at].weight as i32).sum());
        let at = left.iter().position(|at| {
            roll -= trades[*at].weight as i32;
            roll < 0
        });
        chosen.push(left.remove(at.unwrap_or(left.len() - 1)));
    }
    chosen.sort_unstable();
    chosen
}

/// The offers a trader of type `mark` (`MarkVariant`) makes at `tier` (0 the first) of `table`.
pub fn roll(table: &Table, tier: usize, mark: i32, draws: &mut dyn Draws) -> Vec<Offer> {
    let Some(tier) = table.tiers.get(tier) else { return Vec::new() };
    let mut offers = Vec::new();
    for group in &tier.groups {
        let trades: Vec<&Trade> = group.trades.iter().filter(|trade| can_offer(trade, mark)).collect();
        for at in pick(&trades, group.select, draws) {
            offers.extend(make(trades[at], mark, draws));
        }
    }
    offers
}
