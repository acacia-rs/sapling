//! Villagers and wandering traders (docs/research/villagers.md): a villager's profession, type and
//! skin and the job site it works at (jobs.rs), the offers it trades (deals.rs) on its trading
//! screen (trading.rs), kept in its actor compound (storage.rs); the wandering trader's offers,
//! llamas and the timer it leaves by (wandering.rs).
// TODO: no gossip, reputation or Hero of the Village discounts, raids, sleeping or schedules but
// work; villager breeding (husbandry) is to arm its baby with `arm(.., Some("minecraft:entity_born"))`.

mod deals;
mod jobs;
mod storage;
mod trading;
mod wandering;

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain,
};
use bevy_ecs::prelude::*;
use bytes::Bytes;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::{entities, mobs};
use deals::Deal;

pub(crate) use trading::{bargains, interact, stop_trading, traded};

const VILLAGER: &str = "minecraft:villager_v2";
/// What `/summon villager` names it by; shown as [`VILLAGER`].
const OLD_VILLAGER: &str = "minecraft:villager";
const WANDERING_TRADER: &str = "minecraft:wandering_trader";
/// The kind a trader of `kind` is shown as.
fn shown_kind(kind: &str) -> &str {
    if kind == OLD_VILLAGER { VILLAGER } else { kind }
}

/// A villager's skins (`SkinID`), from its behaviour pack.
const SKINS: u32 = 6;

/// A villager or wandering trader: what it is shown as, what it offers and how far it got.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct Trader {
    /// `Variant`: the profession (jobs.rs).
    profession: i32,
    /// `MarkVariant`: the biome type it comes from.
    mark: i32,
    skin: i32,
    tier: i32,
    experience: i32,
    deals: Vec<Deal>,
    /// The experience each tier takes; empty for a trader without a table.
    tiers: Vec<i32>,
    job: Option<[i32; 3]>,
    /// The work period (jobs.rs) it last restocked in.
    restocked: Option<i64>,
    next_look: u64,
    /// A wandering trader's tick to leave at.
    leaves_at: Option<u64>,
    customer: Option<Entity>,
}

impl Trader {
    /// The highest tier it can reach (`MaxTradeTier`).
    fn max_tier(&self) -> i32 {
        self.tiers.len().saturating_sub(1) as i32
    }

    /// Its offers and tiers drawn afresh from `table` (none without one), from the first tier.
    fn stock(&mut self, table: Option<&str>, dice: &mut Dice) {
        let table = table.and_then(sapling_data::trades::trade_table);
        self.deals = table.map(|table| deals::rolled(table, self.mark, dice)).unwrap_or_default();
        self.tiers = table.map(sapling_data::trades::Table::exp_needed).unwrap_or_default();
        (self.tier, self.experience) = (0, 0);
    }
}

fn int(key: Key, value: i32) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(value)) }
}

/// The trader `id`'s metadata of what it is and how far it got, as BDS follows its announcement.
fn shown(id: u64, trader: &Trader) -> SetEntityData {
    let metadata = vec![
        int(Key::Variant, trader.profession),
        int(Key::MarkVariant, trader.mark),
        int(Key::SkinId, trader.skin),
        int(Key::TradeTier, trader.tier),
        int(Key::MaxTradeTier, trader.max_tier()),
        int(Key::TradeExperience, trader.experience),
    ];
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// `announcement` with the trader's metadata after it, for a mob that is one.
pub(super) fn dressed(world: &World, entity: Entity, mut announcement: Vec<Bytes>) -> Vec<Bytes> {
    if let (Some(trader), Some(id)) = (world.get::<Trader>(entity), world.get::<NetId>(entity)) {
        announcement.push(sapling_net::encode(&shown(id.0, trader)));
    }
    announcement
}

/// Tells the trader's viewers, and later ones, what `changed` of its metadata, and keeps it.
fn tell(world: &mut World, entity: Entity, changed: Vec<MetadataDictionaryItem>) {
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(entity).map(|id| id.0), world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).copied()) else { return };
    let data = SetEntityData { runtime_entity_id: id, metadata: changed, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&data)]);
    storage::keep(world, entity);
    // A trader's announcement ends in its metadata (`dressed`): that is told afresh.
    if let Some(mut shown) = mobs::announced(world, entity) {
        shown.pop();
        let shown = dressed(world, entity, shown);
        mobs::reannounce(world, entity, shown);
    }
}

/// Makes a mob of `kind` that trades a trader: as its actor compound keeps it when `loaded`, or
/// new, as the spawn `event` makes it where it names one.
pub(super) fn arm(world: &mut World, entity: Entity, layer: Entity, kind: &str, loaded: bool, event: Option<&str>) {
    let villager = kind == VILLAGER || kind == OLD_VILLAGER;
    if !villager && kind != WANDERING_TRADER {
        return;
    }
    let now = world.resource::<Tick>().0;
    let feet = world.get::<Body>(entity).map_or([0.0; 3], |body| body.at);
    let kept = world.get::<entities::Stored>(entity).filter(|_| loaded).and_then(|stored| storage::read(&stored.compound));
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    let mut trader = kept.clone().unwrap_or_default();
    if kept.is_none() {
        trader.mark = world.get::<Layer>(layer).map_or(0, |layer| jobs::mark_at(&layer.world, feet));
        trader.skin = dice.roll(SKINS) as i32;
        match villager {
            true => jobs::take_up(&mut trader, jobs::first_profession(event, &mut dice), &mut dice),
            false => trader.stock(Some(wandering::TABLE), &mut dice),
        }
    } else if trader.deals.is_empty() {
        let table = if villager { jobs::of(trader.profession).map(|profession| profession.table) } else { Some(wandering::TABLE) };
        trader.stock(table, &mut dice);
    }
    trader.next_look = now + 1 + u64::from(dice.roll(jobs::LOOK_SPREAD));
    world.insert_resource(dice);
    if kind == WANDERING_TRADER {
        wandering::arrive(world, layer, &mut trader, feet, kept.is_none());
    }
    world.entity_mut(entity).insert(trader);
    storage::keep(world, entity);
    let shown = mobs::announced(world, entity).map(|announcement| dressed(world, entity, announcement));
    if let Some(shown) = shown {
        mobs::reannounce(world, entity, shown);
    }
}

/// A trader's tick: a villager looks after its job (jobs.rs), a wandering trader may leave.
/// True for one that went.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let now = world.resource::<Tick>().0;
    let Some((wanders, next_look)) = world.get::<Trader>(entity).map(|trader| (trader.leaves_at.is_some(), trader.next_look)) else { return false };
    if wanders {
        return wandering::tick(world, entity, layer, now);
    }
    if now >= next_look && !super::shown::is_baby(world, entity) {
        jobs::look(world, entity, layer, now);
    }
    false
}

/// Before the trader's move: one with a customer stands still, a villager in working hours goes
/// to its job site. False when it is free to stroll.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let Some(served) = world.get::<Trader>(entity).map(|trader| trader.customer.is_some()) else { return false };
    if served && trading::keeps_customer(world, entity) {
        super::legs::rest(world, entity);
        return true;
    }
    jobs::work(world, entity, layer)
}
