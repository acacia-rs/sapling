//! The wandering trader: its table, the two trader llamas it comes with (bds-trade2.txt: at its
//! feet half a block up, with a `plop`), and the time it leaves at: its pack's `minecraft:timer`
//! of 2400 or 3600 seconds (or its `minecraft:start_despawn` fired), after which it goes once no
//! player is within 24 blocks, taking its llamas with it (`remove_child_entities`; bds-k2trader.txt:
//! each told `RemoveLeash` 0 the tick after the event, then all three removed the tick after that).
//! Its llamas tame themselves once off its lead (husbandry/leads.rs).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::SpawnReason;

use super::Trader;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Position, Tick, in_world};
use crate::creatures::{Raised, husbandry};
use crate::entities::Stored;
use crate::mobs;

pub(super) const TABLE: &str = "wandering_trader";
const LLAMA: &str = "minecraft:trader_llama";
const LLAMAS: usize = 2;
const STAYS: [u64; 2] = [48_000, 72_000];
const NO_PLAYER_WITHIN: f32 = 24.0;
pub(super) const START_DESPAWN: &str = "minecraft:start_despawn";
const TIME_STAMP: &str = "TimeStamp";

/// A trader that let its llamas go: it and they are removed the tick after.
#[derive(Component)]
struct Departing(Vec<Entity>);

/// Sets the trader's time to leave: the level tick a kept one's `TimeStamp` names (BDS keeps its
/// timer's end so, research/captures/bds-f14nkeep-dump.txt: 48672 for one summoned 672 ticks in),
/// else rolled; a `new` one comes with its llamas on its leads (its pack's `should_leash`;
/// bds-trade2.txt: their `LeadHolderEid` the trader's from their announcement).
pub(super) fn arrive(world: &mut World, entity: Entity, layer: Entity, trader: &mut Trader, feet: [f32; 3], new: bool) {
    let now = world.resource::<Tick>().0;
    let kept = (!new).then(|| world.get::<Stored>(entity).and_then(|stored| sapling_world::actor_field(&stored.compound, TIME_STAMP))).flatten();
    let age = world.get::<Layer>(layer).map_or(0, |level| level.world.level().age);
    trader.leaves_at = Some(match kept {
        Some(Value::Long(stamp)) => now + u64::try_from(stamp - age).unwrap_or(0),
        _ => now + STAYS[world.get_resource_or_insert_with(Dice::default).roll(STAYS.len() as u32) as usize],
    });
    if new {
        for _ in 0..LLAMAS {
            let llama = super::super::spawn(world, layer, LLAMA, [feet[0], feet[1] + 0.5, feet[2]], SpawnReason::Summoned).and_then(|id| crate::entities::find(world, id));
            if let Some(llama) = llama {
                husbandry::tie_lead(world, llama, entity);
                world.entity_mut(llama).insert(husbandry::TamedWhenLoose);
            }
        }
    }
}

/// The level tick the trader leaves at, as its `TimeStamp` keeps it.
pub(super) fn time_stamp(world: &World, layer: Entity, trader: &Trader) -> Option<(&'static str, Value)> {
    let (now, age) = (world.resource::<Tick>().0, world.get::<Layer>(layer)?.world.level().age);
    Some((TIME_STAMP, Value::Long(age + trader.leaves_at?.saturating_sub(now) as i64)))
}

/// `minecraft:start_despawn` fired on the trader: its time is up from the next tick.
pub(super) fn start_despawn(world: &mut World, entity: Entity) {
    let now = world.resource::<Tick>().0;
    if let Some(mut trader) = world.get_mut::<Trader>(entity) {
        trader.leaves_at = Some(now + 1);
    }
}

/// A trader whose time is up goes once no player is near (BDS's despawn filter: also one that
/// sold out of everything), checked each tick: it lets its llamas go, and the tick after they and
/// it are removed. True once it went.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, now: u64) -> bool {
    if let Some(Departing(llamas)) = world.entity_mut(entity).take::<Departing>() {
        for llama in llamas {
            mobs::remove(world, llama);
        }
        mobs::remove(world, entity);
        return true;
    }
    let Some(trader) = world.get::<Trader>(entity) else { return false };
    let sold_out = trader.deals.iter().all(super::Deal::sold_out);
    let due = trader.leaves_at.is_some_and(|at| now >= at) || sold_out;
    if !due || trader.customer.is_some() {
        return false;
    }
    let Some(at) = world.get::<Body>(entity).map(|body| body.at) else { return false };
    let near = |player: &Entity| {
        world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer)
            && world.get::<Position>(*player).is_some_and(|position| (0..3).map(|axis| (position.feet[axis] - at[axis]).powi(2)).sum::<f32>().sqrt() <= NO_PLAYER_WITHIN)
    };
    if in_world(world).iter().any(near) {
        return false;
    }
    let llamas: Vec<Entity> = world.query::<(Entity, &Raised)>().iter(world).filter(|(_, raised)| raised.holder == Some(entity)).map(|(llama, _)| llama).collect();
    for llama in &llamas {
        husbandry::release_lead(world, *llama);
    }
    world.entity_mut(entity).insert(Departing(llamas));
    false
}
