//! Where plugin handlers meet the ECS: they run after a tick's systems with a [`ShardCtx`] over
//! the world, never seeing a bevy type. What a ctx call does: backend.rs.
//!
//! [`ShardCtx`]: sapling_api::ShardCtx

mod backend;

use bevy_ecs::prelude::*;
use sapling_api::PlayerId;

use crate::components::{Arrivals, Identity, Layer, Outgoing, Players, Stage};
use crate::plugins::Plugins;
use crate::{blocks, campfires, chat, commands, containers, creatures, drops, entities, forms, furnaces, hoppers, inventory, late_sounds, orbs, roster, sleep, worlds};

pub(crate) struct Ctx<'w> {
    pub world: &'w mut World,
}

impl Ctx<'_> {
    /// The entity of a player who is in the world.
    pub(crate) fn entity(&self, player: PlayerId) -> Option<Entity> {
        let entity = self.world.resource::<Players>().entity(player)?;
        (self.world.get::<Stage>(entity) == Some(&Stage::InWorld)).then_some(entity)
    }

    /// The world in scope (`worlds::current`): the calls that name no world act on it.
    fn layer(&self) -> &Layer {
        self.world.get::<Layer>(worlds::current(self.world)).expect("the world in scope exists")
    }

    fn out(&mut self, player: PlayerId) -> Option<Mut<'_, Outgoing>> {
        self.entity(player).and_then(|entity| self.world.get_mut::<Outgoing>(entity))
    }
}

/// Lists players whose client finished loading and calls `on_join` for them, runs chat, commands,
/// block requests and form answers, removes players whose connection ended, with `on_quit` for
/// those who had joined, and ends with `on_tick`.
pub(crate) fn dispatch(world: &mut World, plugins: &mut Plugins) {
    let Arrivals { joined, left } = std::mem::take(&mut *world.resource_mut::<Arrivals>());
    // Players who came from another shard at the start of the tick, before what they then sent.
    worlds::dispatch(world, plugins);
    for entity in joined {
        let Some(player) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        world.resource_mut::<Players>().in_world += 1;
        roster::arrive(world, entity);
        worlds::focus_on(world, entity);
        plugins.each(world, |handler, ctx| handler.on_join(ctx, &player));
    }
    chat::dispatch(world, plugins);
    commands::dispatch(world, plugins);
    blocks::release_buttons(world);
    blocks::press_plates(world);
    blocks::apply_redstone(world);
    blocks::flow_liquids(world);
    blocks::dispatch(world, plugins);
    blocks::dispatch_sign_edits(world);
    sleep::tick(world);
    campfires::tick(world);
    // After the requests: a block placed over a gap falls in the tick it was placed, as on BDS.
    blocks::fall_blocks(world);
    inventory::dispatch(world, plugins);
    containers::dispatch_closes(world);
    containers::show_lids_again(world);
    late_sounds::play_due(world);
    containers::close_out_of_reach(world);
    furnaces::tick(world);
    hoppers::tick(world);
    entities::remove_left(world);
    drops::fly(world);
    drops::pick_up(world);
    orbs::tick(world);
    creatures::tick(world);
    entities::follow(world);
    forms::dispatch(world);
    for entity in left {
        let had_joined = world.get::<Stage>(entity) == Some(&Stage::InWorld);
        containers::close(world, entity);
        inventory::save(world, entity);
        let Some(player) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        if had_joined {
            roster::depart(world, entity);
        }
        worlds::focus_on(world, entity);
        world.despawn(entity);
        if had_joined {
            world.resource_mut::<Players>().in_world -= 1;
            plugins.each(world, |handler, ctx| handler.on_quit(ctx, &player));
        }
    }
    worlds::focus(world, None);
    plugins.each(world, |handler, ctx| handler.on_tick(ctx));
    worlds::dispatch(world, plugins);
    // Last: BDS evaluates circuits at the end of its tick, after all that changed blocks.
    blocks::evaluate_redstone(world);
}
