//! Command block minecarts: an operator's editor gives one its command, and it runs that every
//! tick it is on a powered activator rail, as BDS's said `[!§r] cart` each tick
//! (research/captures/bds-t3cmd.txt). The cart's saved actor keeps the command and its last run,
//! output tracked, as BDS's (sapling_world::commanded_actor).

use bevy_ecs::prelude::*;
use sapling_world::CartCommand;

use crate::components::{InLayer, NetId, Tick};
use crate::entities::{self, Kind, Stored};

pub(crate) const COMMAND_MINECART: &str = "minecraft:command_block_minecart";

#[derive(Component, Clone, Default)]
pub(super) struct Command(CartCommand);

/// Gives a new command block minecart the command its stored actor keeps, or none.
pub(super) fn recall(world: &mut World, cart: Entity, kind: &str) {
    if kind != COMMAND_MINECART {
        return;
    }
    let kept = world.get::<Stored>(cart).and_then(|stored| sapling_world::actor_command(&stored.compound)).unwrap_or_default();
    keep(world, cart, kept);
}

fn keep(world: &mut World, cart: Entity, command: CartCommand) {
    entities::restore(world, cart, |compound, _| sapling_world::commanded_actor(compound, &command).ok());
    world.entity_mut(cart).insert(Command(command));
}

/// Gives the command block minecart `runtime_id` the command `line`.
pub(crate) fn set_command(world: &mut World, runtime_id: u64, line: &str) {
    let cart = world.query::<(Entity, &NetId, &Kind)>().iter(world).find(|(_, id, kind)| id.0 == runtime_id && kind.0 == COMMAND_MINECART).map(|(cart, ..)| cart);
    let Some(cart) = cart else { return };
    let kept = world.get::<Command>(cart).map(|command| command.0.clone()).unwrap_or_default();
    keep(world, cart, CartCommand { command: line.to_owned(), ..kept });
}

/// A tick of the cart on a powered activator rail.
pub(super) fn run(world: &mut World, cart: Entity, at: [f32; 3]) {
    let (Some(layer), Some(kept)) = (world.get::<InLayer>(cart).map(|layer| layer.0), world.get::<Command>(cart).map(|command| command.0.clone())) else { return };
    let Some((success_count, last_output, parameters)) = crate::blocks::run_for_cart(world, layer, at, &kept.command) else { return };
    let last_execution = world.resource::<Tick>().0 as i64;
    keep(world, cart, CartCommand { last_execution, success_count, last_output, parameters, ..kept });
}
