//! A trader's offers as it keeps them: each offer of its table with the tier it opens at, how
//! often it was traded since the last restock and the demand that moves its price, as BDS's
//! `MerchantRecipe` writes them (docs/research/villagers.md, "Offers").

use acacia_proto::nbt::{List, Str, Value};
use sapling_data::trades::{Offer, Table, Ware};
use sapling_world::{BlockRegistry, ItemStack};

use crate::blocks::Dice;
use crate::inventory::Bargain;

const COMPOUND: u8 = 10;
/// The recipe id a screen names its first offer by; BDS counts on from where its other recipes
/// end (research/captures/bds-trade.txt: 4769).
const FIRST_NET_ID: u32 = 4769;

/// One offer of a trader.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Deal {
    pub offer: Offer,
    pub tier: i32,
    pub uses: i32,
    pub demand: i32,
}

/// Draws for a trade table from the shard's dice.
struct Draws<'a>(&'a mut Dice);

impl sapling_data::enchant::Draws for Draws<'_> {
    fn int(&mut self, bound: i32) -> i32 {
        if bound < 1 { 0 } else { self.0.roll(bound as u32) as i32 }
    }

    fn float(&mut self) -> f32 {
        self.0.fraction()
    }
}

/// Every tier's offers of `table` for a trader of type `mark`: BDS lists the tiers a trader has
/// not reached yet as well, locked (bds-trade.txt).
pub(super) fn rolled(table: &Table, mark: i32, dice: &mut Dice) -> Vec<Deal> {
    let mut draws = Draws(dice);
    let tiers = (0..table.tiers.len()).flat_map(|tier| sapling_data::trades::roll(table, tier, mark, &mut draws).into_iter().map(move |offer| (tier, offer)));
    tiers.map(|(tier, offer)| Deal { offer, tier: tier as i32, uses: 0, demand: 0 }).collect()
}

impl Deal {
    pub fn sold_out(&self) -> bool {
        self.offer.max_uses >= 0 && self.uses >= self.offer.max_uses
    }

    /// What `ware` costs now: its count raised by demand, from one to a stack.
    // TODO: BDS's pricing was not decompiled; this is the Java game's `getModifiedCostCount`.
    fn price(&self, ware: &Ware) -> u8 {
        let base = i32::from(ware.count);
        let raised = ((base * self.demand) as f32 * ware.price_multiplier).floor().max(0.0) as i32;
        (base + raised).clamp(1, i32::from(sapling_data::max_stack(&ware.name))) as u8
    }

    /// Restocked: none used, and demand moved by how much of it sold.
    // TODO: BDS's demand rule was not decompiled; this is the Java game's `updateDemand`.
    pub fn restock(&mut self) {
        self.demand += self.uses - (self.offer.max_uses - self.uses);
        self.uses = 0;
    }

    /// The offer as a trading screen's request pays for it, named by its place.
    pub fn bargain(&self, at: usize, registry: &BlockRegistry) -> Bargain {
        let priced = |ware: &Ware| ItemStack { count: self.price(ware), ..stack(ware, registry) };
        Bargain { net_id: net_id(at), wants: [Some(priced(&self.offer.buy_a)), self.offer.buy_b.as_ref().map(priced)], gives: stack(&self.offer.sell, registry), sold_out: self.sold_out() }
    }

    /// The offer's compound, as the trading screen (`net_id` its recipe id) or the actor keeps it.
    pub fn compound(&self, net_id: Option<u32>, registry: &BlockRegistry) -> Value {
        let item = |ware: &Ware| {
            let mut fields = stack(ware, registry).fields(registry);
            fields.push(("WasPickedUp".into(), Value::Byte(0)));
            Value::Compound(fields)
        };
        let int = |key: &str, value: i32| (Str::from(key), Value::Int(value));
        let mut fields = vec![(Str::from("buyA"), item(&self.offer.buy_a))];
        fields.extend(self.offer.buy_b.as_ref().map(|ware| (Str::from("buyB"), item(ware))));
        fields.extend([
            int("buyCountA", self.price(&self.offer.buy_a).into()),
            int("buyCountB", self.offer.buy_b.as_ref().map_or(0, |ware| self.price(ware).into())),
            int("demand", self.demand),
            int("maxUses", self.offer.max_uses),
        ]);
        fields.extend(net_id.map(|id| int("netId", id as i32)));
        fields.extend([
            ("priceMultiplierA".into(), Value::Float(self.offer.buy_a.price_multiplier)),
            ("priceMultiplierB".into(), Value::Float(self.offer.buy_b.as_ref().map_or(0.0, |ware| ware.price_multiplier))),
            ("rewardExp".into(), Value::Byte(self.offer.reward_exp.into())),
            ("sell".into(), item(&self.offer.sell)),
            int("tier", self.tier),
            int("traderExp", self.offer.trader_exp),
            int("uses", self.uses),
        ]);
        Value::Compound(fields)
    }

    /// An offer as an actor kept it (`Offers.Recipes`); `None` for one missing what it needs.
    pub fn parse(stored: &Value) -> Option<Self> {
        let int = |key: &str| match stored.get(key) {
            Some(Value::Int(value)) => Some(*value),
            Some(Value::Byte(value)) => Some(i32::from(*value)),
            _ => None,
        };
        let float = |key: &str| if let Some(Value::Float(value)) = stored.get(key) { *value } else { 0.0 };
        let ware = |key: &str, multiplier: f32| {
            let stack = ItemStack::parse(stored.get(key)?)?;
            Some(Ware { name: stack.name, count: stack.count, damage: stack.damage, tag: stack.tag, price_multiplier: multiplier })
        };
        let offer = Offer {
            buy_a: ware("buyA", float("priceMultiplierA"))?,
            buy_b: ware("buyB", float("priceMultiplierB")),
            sell: ware("sell", 0.0)?,
            trader_exp: int("traderExp").unwrap_or(0),
            max_uses: int("maxUses").unwrap_or(0),
            reward_exp: int("rewardExp").unwrap_or(1) != 0,
        };
        Some(Self { offer, tier: int("tier").unwrap_or(0), uses: int("uses").unwrap_or(0), demand: int("demand").unwrap_or(0) })
    }
}

/// The recipe id of the offer at `at`.
pub(super) fn net_id(at: usize) -> u32 {
    FIRST_NET_ID + at as u32
}

/// The offer list and each tier's experience, as `UpdateTrade` and an actor's `Offers` carry them
/// (BDS's `MerchantRecipeList` tag: `Recipes`, `TierExpRequirements`).
pub(super) fn offers_compound(deals: &[Deal], tiers: &[i32], on_screen: bool, registry: &BlockRegistry) -> Value {
    let recipes = deals.iter().enumerate().map(|(at, deal)| deal.compound(on_screen.then(|| net_id(at)), registry)).collect();
    let needs = tiers.iter().enumerate().map(|(tier, exp)| Value::Compound(vec![(Str::from(tier.to_string()), Value::Int(*exp))])).collect();
    Value::Compound(vec![
        ("Recipes".into(), Value::List(List { tag: COMPOUND, items: recipes })),
        ("TierExpRequirements".into(), Value::List(List { tag: COMPOUND, items: needs })),
    ])
}

/// A ware as a stack: a block item with its block's first state.
fn stack(ware: &Ware, registry: &BlockRegistry) -> ItemStack {
    let block = registry.states_of(&ware.name).next().map(|(id, _)| registry.network_hash(id));
    ItemStack { name: ware.name.clone(), count: ware.count, damage: ware.damage, block, tag: ware.tag.clone() }
}
