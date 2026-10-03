//! Where plugin handlers meet the ECS: they run between ticks' systems with a [`ShardCtx`] over the
//! world, never seeing a bevy type.

use bevy_ecs::prelude::*;
use sapling_api::{Backend, Handler, PlayerId, ShardCtx};
use sapling_net::ConnId;

use crate::components::{Arrivals, Identity, Net, Players, Stage, Tick};

struct Ctx<'w> {
    world: &'w mut World,
}

impl Backend for Ctx<'_> {
    fn tick(&self) -> u64 {
        self.world.resource::<Tick>().0
    }

    fn player_count(&self) -> usize {
        self.world.resource::<Players>().in_world
    }

    fn kick(&mut self, player: PlayerId, message: &str) {
        self.world.resource::<Net>().0.kick(ConnId::new(player.0), message);
    }
}

/// Calls `on_join` for players whose client finished loading, then removes players whose
/// connection ended, with `on_quit` for those who had joined.
pub(crate) fn dispatch(world: &mut World, handlers: &mut [Box<dyn Handler>]) {
    let Arrivals { joined, left } = std::mem::take(&mut *world.resource_mut::<Arrivals>());
    for entity in joined {
        let Some(player) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        world.resource_mut::<Players>().in_world += 1;
        for handler in handlers.iter_mut() {
            handler.on_join(&mut ShardCtx::new(&mut Ctx { world }), &player);
        }
    }
    for entity in left {
        let had_joined = world.get::<Stage>(entity) == Some(&Stage::InWorld);
        let Some(player) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        world.despawn(entity);
        if had_joined {
            world.resource_mut::<Players>().in_world -= 1;
            for handler in handlers.iter_mut() {
                handler.on_quit(&mut ShardCtx::new(&mut Ctx { world }), &player);
            }
        }
    }
}
