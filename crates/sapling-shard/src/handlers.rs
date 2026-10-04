//! Where plugin handlers meet the ECS: they run after a tick's systems with a [`ShardCtx`] over
//! the world, never seeing a bevy type.

use bevy_ecs::prelude::*;
use sapling_api::{Backend, Handler, Player, PlayerId, ShardCtx};
use sapling_net::ConnId;

use crate::components::{Arrivals, Identity, Net, Outgoing, Players, Position, Stage, Tick, in_world};
use crate::{chat, roster};

pub(crate) struct Ctx<'w> {
    pub world: &'w mut World,
}

impl Ctx<'_> {
    /// The entity of a player who is in the world.
    fn entity(&self, player: PlayerId) -> Option<Entity> {
        let entity = self.world.resource::<Players>().entity(player)?;
        (self.world.get::<Stage>(entity) == Some(&Stage::InWorld)).then_some(entity)
    }
}

impl Backend for Ctx<'_> {
    fn tick(&self) -> u64 {
        self.world.resource::<Tick>().0
    }

    fn player_count(&self) -> usize {
        self.world.resource::<Players>().in_world
    }

    fn players(&self) -> Vec<Player> {
        in_world(self.world).into_iter().filter_map(|entity| self.world.get::<Identity>(entity).map(|i| i.0.clone())).collect()
    }

    fn position(&self, player: PlayerId) -> Option<sapling_api::Position> {
        let position = self.world.get::<Position>(self.entity(player)?)?;
        Some(sapling_api::Position { feet: position.feet, yaw: position.yaw, pitch: position.pitch })
    }

    fn kick(&mut self, player: PlayerId, message: &str) {
        self.world.resource::<Net>().0.kick(ConnId::new(player.0), message);
    }

    fn broadcast(&mut self, message: &str) {
        chat::send_to_all(self.world, &chat::raw_packet(message));
    }

    fn send_message(&mut self, player: PlayerId, message: &str) {
        if let Some(mut out) = self.entity(player).and_then(|entity| self.world.get_mut::<Outgoing>(entity)) {
            out.0.push(chat::raw_packet(message));
        }
    }
}

/// Lists players whose client finished loading and calls `on_join` for them, runs the chat, then
/// removes players whose connection ended, with `on_quit` for those who had joined.
pub(crate) fn dispatch(world: &mut World, handlers: &mut [Box<dyn Handler>]) {
    let Arrivals { joined, left } = std::mem::take(&mut *world.resource_mut::<Arrivals>());
    for entity in joined {
        let Some(player) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        world.resource_mut::<Players>().in_world += 1;
        roster::arrive(world, entity);
        for handler in handlers.iter_mut() {
            handler.on_join(&mut ShardCtx::new(&mut Ctx { world }), &player);
        }
    }
    chat::dispatch(world, handlers);
    for entity in left {
        let had_joined = world.get::<Stage>(entity) == Some(&Stage::InWorld);
        let Some(player) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        if had_joined {
            roster::depart(world, entity);
        }
        world.despawn(entity);
        if had_joined {
            world.resource_mut::<Players>().in_world -= 1;
            for handler in handlers.iter_mut() {
                handler.on_quit(&mut ShardCtx::new(&mut Ctx { world }), &player);
            }
        }
    }
}
