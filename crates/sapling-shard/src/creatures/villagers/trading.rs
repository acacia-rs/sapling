//! A trader's screen (research/captures/bds-trade2.txt): a player's use of a trader opens it with
//! `UpdateTrade` alone, the trader stands and says who it trades with; requests trade one offer at
//! a time (inventory/forge.rs, `Bench::Trade`), and each trade counts a use of the offer, gives the
//! trader its experience, a tier once it has enough, and the player orbs.

use acacia_proto::nbt::Nbt;
use acacia_proto::packets::UpdateTrade;
use acacia_proto::types::{
    MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, WindowID, WindowType,
};
use bevy_ecs::prelude::*;
use sapling_api::EntityId;

use super::deals::offers_compound;
use super::{Trader, int, jobs, tell};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Outgoing};
use crate::inventory::{Bargain, Inventory};
use crate::{containers, entities, orbs};

const TRADER_TITLE: &str = "entity.wandering_trader.name";
/// Where a trader's voice comes from, of its height.
const MOUTH: f32 = 0.85;

/// The trader whose screen a player has open, and the window its client knows it as.
#[derive(Component, Clone, Copy)]
pub(crate) struct Trading {
    trader: Entity,
    window: u8,
}

/// `UpdateTrade`'s tier and unique id are zigzag varints its fields do not encode as such.
fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

fn customer(player_id: i64) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key: Key::TradingPlayerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(player_id)) }
}

/// The trader says `sound` (`haggle`, `haggle.yes`) to those who see it.
fn say(world: &mut World, trader: Entity, sound: &str) {
    let (Some(id), Some(layer), Some(body), Some(kind)) =
        (world.get::<NetId>(trader).map(|id| id.0), world.get::<InLayer>(trader).map(|layer| layer.0), world.get::<Body>(trader).copied(), world.get::<entities::Kind>(trader).map(|kind| kind.0.clone()))
    else {
        return;
    };
    let said = super::super::hunting::sound(sound, id, super::shown_kind(&kind), &body, MOUTH);
    entities::tell_viewers(world, trader, layer, body.at.map(|c| c.floor() as i32), vec![said]);
}

/// A player's use of the mob `id`: a free trader with offers opens its screen. False for any other.
pub(crate) fn interact(world: &mut World, player: Entity, id: u64) -> bool {
    let Some(trader) = entities::find(world, EntityId(id)) else { return false };
    let free = world.get::<Trader>(trader).is_some_and(|t| !t.deals.is_empty() && t.customer.is_none_or(|customer| customer == player));
    let grown = world.get::<super::super::Creature>(trader).is_some() && !super::super::shown::is_baby(world, trader);
    if !free || !grown {
        return false;
    }
    containers::close(world, player);
    let Some(update) = screen(world, player, trader) else { return false };
    let Some(player_id) = world.get::<NetId>(player).map(|id| id.0 as i64) else { return false };
    let window = update.window_id.to_raw() as u8;
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&update);
    }
    world.entity_mut(player).insert(Trading { trader, window });
    if let Some(mut trader) = world.get_mut::<Trader>(trader) {
        trader.customer = Some(player);
    }
    tell(world, trader, vec![customer(player_id)]);
    say(world, trader, "haggle");
    true
}

/// The trading screen of `trader` for `player`, in the player's next window.
fn screen(world: &mut World, player: Entity, trader: Entity) -> Option<UpdateTrade> {
    let window = world.get_mut::<Inventory>(player)?.next_window();
    let player_id = world.get::<NetId>(player)?.0;
    let id = world.get::<NetId>(trader)?.0;
    let layer = world.get::<InLayer>(trader)?.0;
    let registry = world.get::<Layer>(layer)?.world.registry();
    let state = world.get::<Trader>(trader)?;
    let title = if state.leaves_at.is_some() { TRADER_TITLE } else { jobs::of(state.profession)?.title };
    Some(UpdateTrade {
        window_id: WindowID::from_raw(window.into()),
        window_type: WindowType::Trading,
        size: 0,
        trade_tier: zigzag(state.tier.into()) as u32,
        villager_unique_id: zigzag(id as i64),
        // BDS names the player by its runtime id here, unzigzagged (bds-trade.txt: 1).
        entity_unique_id: player_id,
        display_name: title.to_owned(),
        new_trading_ui: true,
        economic_trades: true,
        offers: Nbt { name: String::new(), value: offers_compound(&state.deals, &state.tiers, true, hero(world, player, state), registry) }.into(),
    })
}

/// The amplifier of the player's Hero of the Village, which a villager, not a wandering trader,
/// sells cheaper for.
fn hero(world: &World, player: Entity, trader: &Trader) -> Option<u8> {
    let (amplifier, _) = crate::vitals::effects::on(world, player, sapling_api::Effect::HeroOfTheVillage)?;
    trader.leaves_at.is_none().then_some(amplifier)
}

/// The offers of the trader whose screen the player has open, as its requests pay for them.
pub(crate) fn bargains(world: &World, player: Entity) -> Option<Vec<Bargain>> {
    let trader = world.get::<Trading>(player)?.trader;
    let registry = world.get::<Layer>(world.get::<InLayer>(trader)?.0)?.world.registry();
    let state = world.get::<Trader>(trader)?;
    let hero = hero(world, player, state);
    Some(state.deals.iter().enumerate().map(|(at, deal)| deal.bargain(at, hero, registry)).collect())
}

/// What a trade of the offer at `index` leaves: a use of it, the trader's experience (and the
/// next tier at the experience it takes), the player's orbs.
pub(crate) fn traded(world: &mut World, player: Entity, index: usize) {
    let Some(trader) = world.get::<Trading>(player).map(|trading| trading.trader) else { return };
    let Some(mut state) = world.get_mut::<Trader>(trader) else { return };
    let Some(deal) = state.deals.get_mut(index) else { return };
    deal.uses += 1;
    let (exp, rewarded) = (deal.offer.trader_exp, deal.offer.reward_exp);
    state.experience += exp;
    let before = state.tier;
    while state.tier < state.max_tier() && state.tiers.get(state.tier as usize + 1).is_some_and(|needs| state.experience >= *needs) {
        state.tier += 1;
    }
    let mut changed = vec![int(Key::TradeExperience, state.experience)];
    if state.tier != before {
        changed.insert(0, int(Key::TradeTier, state.tier));
    }
    tell(world, trader, changed);
    say(world, trader, "haggle.yes");
    super::standing::traded(world, player, trader);
    if rewarded {
        // TODO: BDS's reward was not decompiled; this is the Java game's 3 to 6 points.
        let points = 3 + world.get_resource_or_insert_with(Dice::default).roll(4) as i32;
        let (Some(layer), Some(body)) = (world.get::<InLayer>(trader).map(|layer| layer.0), world.get::<Body>(trader).copied()) else { return };
        orbs::spawn(world, layer, body.at, points);
    }
}

/// Ends a player's trading at the window `window` its client closed, if that is the one.
pub(crate) fn stop_trading(world: &mut World, player: Entity, window: i64) {
    let Some(trading) = world.get::<Trading>(player).copied().filter(|trading| i64::from(trading.window) == window) else { return };
    world.entity_mut(player).remove::<Trading>();
    let freed = world.get_mut::<Trader>(trading.trader).is_some_and(|mut trader| trader.customer.take().is_some());
    if freed {
        tell(world, trading.trader, vec![customer(0)]);
    }
}

/// Whether the trader's customer still has its screen open; one gone or done is let go.
pub(super) fn keeps_customer(world: &mut World, trader: Entity) -> bool {
    let Some(buyer) = world.get::<Trader>(trader).and_then(|state| state.customer) else { return false };
    if world.get::<Trading>(buyer).is_some_and(|trading| trading.trader == trader) {
        return true;
    }
    if let Some(mut state) = world.get_mut::<Trader>(trader) {
        state.customer = None;
    }
    tell(world, trader, vec![customer(0)]);
    false
}
