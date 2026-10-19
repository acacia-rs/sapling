//! Player actions offered to plugins before the game acts on them: moves, which
//! `movement::player_input` queues because systems have no plugins, item use and entity clicks.

use bevy_ecs::prelude::*;
use sapling_api::{EntityAttackEvent, EntityId, EntityInteractEvent, ItemUseEvent, MoveEvent, Player, Slot, Target};

use crate::components::{NetId, Position, in_world, in_world_identity};
use crate::inventory::{self, Inventory};
use crate::movement::{self, Moves};
use crate::plugins::Plugins;
use crate::{entities, worlds};

/// Offers this tick's moves; a cancelled one puts the player back where it was.
pub(crate) fn dispatch_moves(world: &mut World, plugins: &mut Plugins) {
    for (entity, from, to) in world.resource_mut::<Moves>().take() {
        let Some(player) = in_world_identity(world, entity) else { continue };
        worlds::focus_on(world, entity);
        if plugins.offer(world, MoveEvent::new(player, api_position(&from), api_position(&to))).is_cancelled() {
            if let Some(mut position) = world.get_mut::<Position>(entity) {
                *position = from;
            }
            movement::teleport(world, entity, from.feet);
        }
    }
}

fn api_position(position: &Position) -> sapling_api::Position {
    sapling_api::Position { feet: position.feet, yaw: position.yaw, pitch: position.pitch }
}

/// Whether plugins let `player` use the item in its hand on nothing.
pub(crate) fn may_use_item(world: &mut World, plugins: &mut Plugins, entity: Entity, player: &Player) -> bool {
    let held = world.get::<Inventory>(entity).map(|inventory| inventory.selected);
    let item = held.and_then(|slot| inventory::item_at(world, entity, Slot::Main(slot)));
    let cancelled = plugins.offer(world, ItemUseEvent::new(player.clone(), item)).is_cancelled();
    if cancelled {
        // The client began the use on its own screen.
        crate::vitals::eating::stop(world, entity);
    }
    !cancelled
}

/// The hits plugins let through, of this tick's (a player, the runtime id it hit).
pub(crate) fn allowed_hits(world: &mut World, plugins: &mut Plugins, hits: Vec<(Entity, u64)>) -> Vec<(Entity, u64)> {
    hits.into_iter().filter(|&(entity, id)| offer_click(world, plugins, entity, id, EntityAttackEvent::new).is_some_and(|event| !event.is_cancelled())).collect()
}

/// Offers this tick's uses of players and entities; nothing in the game answers them yet.
pub(crate) fn interacted(world: &mut World, plugins: &mut Plugins, uses: Vec<(Entity, u64)>) {
    for (entity, id) in uses {
        offer_click(world, plugins, entity, id, EntityInteractEvent::new);
    }
}

/// `None` where `id` names nothing the player could have clicked.
fn offer_click<E: sapling_api::Event>(world: &mut World, plugins: &mut Plugins, entity: Entity, id: u64, event: fn(Player, Target) -> E) -> Option<E> {
    let player = in_world_identity(world, entity)?;
    let target = target(world, entity, id)?;
    worlds::focus_on(world, entity);
    Some(plugins.offer(world, event(player, target)))
}

fn target(world: &World, clicker: Entity, id: u64) -> Option<Target> {
    let other = in_world(world).into_iter().find(|player| *player != clicker && world.get::<NetId>(*player).is_some_and(|net| net.0 == id));
    match other {
        Some(other) => in_world_identity(world, other).map(Target::Player),
        None => entities::find(world, EntityId(id)).map(|_| Target::Entity(EntityId(id))),
    }
}
