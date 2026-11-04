//! Command block minecarts: an operator's editor gives one its command, and it runs that every
//! tick it is on a powered activator rail, as BDS's said `[!§r] cart` each tick
//! (research/captures/bds-t3cmd.txt).
// TODO: the command is not kept in the cart's saved actor, and its output is not tracked.

use bevy_ecs::prelude::*;

use crate::components::{InLayer, NetId};
use crate::entities::Kind;

pub(crate) const COMMAND_MINECART: &str = "minecraft:command_block_minecart";

#[derive(Component, Clone, Default)]
pub(super) struct CartCommand(String);

/// Gives the command block minecart `runtime_id` the command `line`.
pub(crate) fn set_command(world: &mut World, runtime_id: u64, line: &str) {
    let cart = world.query::<(Entity, &NetId, &Kind)>().iter(world).find(|(_, id, kind)| id.0 == runtime_id && kind.0 == COMMAND_MINECART).map(|(cart, ..)| cart);
    if let Some(cart) = cart {
        world.entity_mut(cart).insert(CartCommand(line.to_owned()));
    }
}

/// A tick of the cart on a powered activator rail.
pub(super) fn run(world: &mut World, cart: Entity, at: [f32; 3]) {
    let (Some(layer), Some(line)) = (world.get::<InLayer>(cart).map(|layer| layer.0), world.get::<CartCommand>(cart).map(|line| line.0.clone())) else { return };
    crate::blocks::run_for_cart(world, layer, at, &line);
}
