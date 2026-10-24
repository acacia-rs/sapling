//! Where plugin handlers meet the ECS: they run after a tick's systems with a [`ShardCtx`] over
//! the world, never seeing a bevy type. What a ctx call does: backend.rs.
//!
//! [`ShardCtx`]: sapling_api::ShardCtx

mod backend;

use bevy_ecs::prelude::*;
use sapling_api::{JoinEvent, PlayerId, QuitEvent, Scope, TickEvent};

use crate::components::{Arrivals, Identity, Layer, Outgoing, PacketQueues, Players, Stage};
use crate::plugins;
use crate::{actions, blocks, campfires, chat, commands, containers, creatures, dimensions, drops, entities, forms, furnaces, hoppers, inventory, late_sounds, orbs, portals, roster, sleep, tnt, vitals, worlds};

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

    pub(crate) fn out(&mut self, player: PlayerId) -> Option<Mut<'_, Outgoing>> {
        self.entity(player).and_then(|entity| self.world.get_mut::<Outgoing>(entity))
    }
}

/// Lists players whose client finished loading and calls `on_join` for them, runs chat, commands,
/// block requests and form answers, removes players whose connection ended, with `on_quit` for
/// those who had joined, and ends with `on_tick`.
pub(crate) fn dispatch(world: &mut World) {
    // First: a block changed in a tick reaches its circuit in the next, as on BDS
    // (research/captures/bds-redstone-phase.txt), and what an evaluation finds shows a tick later.
    blocks::read_daylight(world);
    blocks::apply_redstone(world);
    blocks::evaluate_redstone(world);
    blocks::move_pistons(world);
    let Arrivals { joined, left } = std::mem::take(&mut *world.resource_mut::<Arrivals>());
    // Players who came from another shard at the start of the tick, before what they then sent.
    worlds::dispatch(world);
    for entity in joined {
        let Some(player) = world.get::<Identity>(entity).map(|i| i.0.clone()) else { continue };
        world.resource_mut::<Players>().in_world += 1;
        roster::arrive(world, entity);
        worlds::focus_on(world, entity);
        plugins::offer(world, JoinEvent::new(player));
    }
    // Before anything acts on where players are: a cancelled move puts one back.
    actions::dispatch_moves(world);
    chat::dispatch(world);
    commands::dispatch(world);
    blocks::release_buttons(world);
    blocks::press_plates(world);
    blocks::flow_liquids(world);
    blocks::dispatch(world);
    blocks::dispatch_sign_edits(world);
    blocks::portals::settle(world);
    let actions = world.resource_mut::<PacketQueues>().take::<acacia_proto::packets::PlayerAction>();
    dimensions::acknowledge(world, &actions);
    sleep::tick(world, &actions);
    campfires::tick(world);
    // After the requests: a block placed over a gap falls in the tick it was placed, as on BDS.
    blocks::fall_blocks(world);
    tnt::burn(world);
    blocks::burn_fires(world);
    blocks::melt_frost(world);
    blocks::dim_ores(world);
    blocks::settle_scaffolds(world);
    blocks::random_ticks(world);
    inventory::dispatch(world);
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
    crate::dragon::tick(world);
    crate::eyes::tick(world);
    vitals::tick(world);
    portals::tick(world);
    portals::arrive(world);
    portals::end::arrive(world);
    crate::weather::tick(world);
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
        crate::simulation::report(world, entity);
        world.despawn(entity);
        let id = player.id;
        if had_joined {
            world.resource_mut::<Players>().in_world -= 1;
            plugins::offer(world,QuitEvent::new(player));
        }
        // After the quit: a listener for this player alone hears it.
        plugins::end_scope(world, Scope::Player(id));
    }
    worlds::focus(world, None);
    plugins::run_tasks(world);
    plugins::offer(world,TickEvent);
    worlds::dispatch(world);
}
