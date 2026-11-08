//! The wandering trader: its table, the two trader llamas it comes with (bds-trade2.txt: at its
//! feet half a block up, with a `plop`), and the time it leaves at: its pack's `minecraft:timer`
//! of 2400 or 3600 seconds (or its `minecraft:start_despawn` fired), after which it goes once no
//! player is within 24 blocks, taking its llamas with it (`remove_child_entities`; bds-k2trader.txt:
//! each told `RemoveLeash` 0, then all three removed in that tick). Its llamas tame themselves once
//! off its lead (husbandry/leads.rs).

use bevy_ecs::prelude::*;
use sapling_api::SpawnReason;

use super::Trader;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::creatures::{Raised, husbandry};
use crate::mobs;

pub(super) const TABLE: &str = "wandering_trader";
const LLAMA: &str = "minecraft:trader_llama";
const LLAMAS: usize = 2;
const STAYS: [u64; 2] = [48_000, 72_000];
const NO_PLAYER_WITHIN: f32 = 24.0;
/// Ticks between a trader's looks for players once its time is up.
const LOOK_EVERY: u64 = 20;
pub(super) const START_DESPAWN: &str = "minecraft:start_despawn";

/// Sets the trader's time to leave; a `new` one comes with its llamas on its leads (its pack's
/// `should_leash`; bds-trade2.txt: their `LeadHolderEid` the trader's from their announcement).
pub(super) fn arrive(world: &mut World, entity: Entity, layer: Entity, trader: &mut Trader, feet: [f32; 3], new: bool) {
    let stays = STAYS[world.get_resource_or_insert_with(Dice::default).roll(STAYS.len() as u32) as usize];
    trader.leaves_at = Some(world.resource::<Tick>().0 + stays);
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

/// `minecraft:start_despawn` fired on the trader: its time is up now.
pub(super) fn start_despawn(world: &mut World, entity: Entity) {
    let now = world.resource::<Tick>().0;
    if let Some(mut trader) = world.get_mut::<Trader>(entity) {
        trader.leaves_at = Some(now);
    }
}

/// A trader whose time is up goes once no player is near (BDS's despawn filter: also one that
/// sold out of everything). True once it went.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, now: u64) -> bool {
    let Some(trader) = world.get::<Trader>(entity) else { return false };
    let sold_out = trader.deals.iter().all(super::Deal::sold_out);
    let due = trader.leaves_at.is_some_and(|at| now >= at) || sold_out;
    if !due || now % LOOK_EVERY != 0 || trader.customer.is_some() {
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
    for llama in llamas {
        mobs::remove(world, llama);
    }
    mobs::remove(world, entity);
    true
}
