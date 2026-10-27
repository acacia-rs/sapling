//! Item cooldowns: the tick from which a player may use an item of a category again, told to the
//! client with ClientStartItemCooldown as BDS tells it (docs/research/item-uses.md, "Cooldowns").

use acacia_proto::packets::ClientStartItemCooldown;
use bevy_ecs::prelude::*;

use crate::components::{Outgoing, Tick};

/// The tick each cooling category may be used from.
#[derive(Component, Default)]
struct Cooldowns(Vec<(&'static str, u64)>);

/// Whether the player's `category` is still cooling down.
pub(crate) fn is_cooling(world: &World, player: Entity, category: &str) -> bool {
    let now = world.resource::<Tick>().0;
    world.get::<Cooldowns>(player).is_some_and(|cooled| cooled.0.iter().any(|(item, until)| *item == category && now < *until))
}

/// Tells the client its `category` cools down for `ticks`; with `start`, the server counts them too
/// (BDS tells a horn and chorus fruit again when the use ends, without starting anew).
pub(crate) fn tell(world: &mut World, player: Entity, category: &'static str, ticks: u64, start: bool) {
    if start {
        let until = world.resource::<Tick>().0 + ticks;
        if world.get::<Cooldowns>(player).is_none() {
            world.entity_mut(player).insert(Cooldowns::default());
        }
        if let Some(mut cooled) = world.get_mut::<Cooldowns>(player) {
            cooled.0.retain(|(item, _)| *item != category);
            cooled.0.push((category, until));
        }
    }
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.push(sapling_net::encode(&ClientStartItemCooldown { category: category.to_owned(), duration: ticks as i32 }));
    }
}
