//! A trader in its actor compound, under BDS's keys (bdsre: the actor load at 0x140d26a70 and
//! `MerchantRecipeList` at 0x1416404a0): `Variant`, `MarkVariant`, `SkinID`, `TradeTier`,
//! `TradeExperience` and `Offers` (`Recipes`, `TierExpRequirements`).
// TODO: a wandering trader's timer is not kept: one loaded again gets a new one.

use acacia_proto::nbt::{self, LittleEndian, Nbt, Str, Value};
use bevy_ecs::prelude::*;

use super::Trader;
use super::deals::{Deal, offers_compound};
use crate::components::{InLayer, Layer};
use crate::entities::Stored;

/// The trader a stored compound keeps; `None` for one that cannot be read.
pub(super) fn read(compound: &[u8]) -> Option<Trader> {
    let actor = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value;
    let int = |key: &str| if let Some(Value::Int(value)) = actor.get(key) { *value } else { 0 };
    let list = |key: &str| match actor.get("Offers").and_then(|offers| offers.get(key)) {
        Some(Value::List(list)) => list.items.clone(),
        _ => Vec::new(),
    };
    let tier_exp = |tier: &Value| match tier {
        Value::Compound(fields) => fields.iter().find_map(|(_, exp)| if let Value::Int(exp) = exp { Some(*exp) } else { None }),
        _ => None,
    };
    Some(Trader {
        profession: int("Variant"),
        mark: int("MarkVariant"),
        skin: int("SkinID"),
        tier: int("TradeTier"),
        experience: int("TradeExperience"),
        deals: list("Recipes").iter().filter_map(Deal::parse).collect(),
        tiers: list("TierExpRequirements").iter().filter_map(tier_exp).collect(),
        ..Trader::default()
    })
}

/// Writes the trader into the entity's stored compound, for its column's next save.
pub(super) fn keep(world: &mut World, entity: Entity) {
    let Some(trader) = world.get::<Trader>(entity) else { return };
    let Some(layer) = world.get::<InLayer>(entity).and_then(|layer| world.get::<Layer>(layer.0)) else { return };
    let Some(Value::Compound(mut fields)) = world.get::<Stored>(entity).and_then(|stored| nbt::read::<LittleEndian>(&mut &stored.compound[..]).ok()).map(|read| read.value) else { return };
    let mut set = vec![
        ("Variant", Value::Int(trader.profession)),
        ("MarkVariant", Value::Int(trader.mark)),
        ("SkinID", Value::Int(trader.skin)),
        ("TradeTier", Value::Int(trader.tier)),
        ("TradeExperience", Value::Int(trader.experience)),
    ];
    if !trader.deals.is_empty() {
        set.push(("Offers", offers_compound(&trader.deals, &trader.tiers, false, layer.world.registry())));
    }
    for (key, value) in set {
        match fields.iter_mut().find(|(name, _)| name == key) {
            Some((_, old)) => *old = value,
            None => fields.push((Str::from(key), value)),
        }
    }
    let mut out = bytes::BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields) });
    if let Some(mut stored) = world.get_mut::<Stored>(entity) {
        stored.compound = out.to_vec();
    }
}
