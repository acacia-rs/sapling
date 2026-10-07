//! The worlds side of the handlers' context.

use bevy_ecs::prelude::*;
use sapling_api::{Player, PlayerId, RuleValue, WorldBackend};
use sapling_net::ConnId;

use super::{Atlas, Place, Scope, current, enter, name_of};
use crate::components::{Identity, InLayer, in_world};
use crate::handlers::Ctx;
use crate::handoff::{self, Peers, ShardMsg};
use crate::{arenas, chat, rules};

impl Ctx<'_> {
    fn place(&self, name: &str) -> Option<Place> {
        self.world.resource::<Atlas>().find(name)
    }

    fn layer_of(&self, player: Entity) -> Option<Entity> {
        self.world.get::<InLayer>(player).map(|in_layer| in_layer.0)
    }
}

impl WorldBackend for Ctx<'_> {
    fn worlds(&self) -> Vec<String> {
        self.world.resource::<Atlas>().names()
    }

    fn world(&self) -> String {
        name_of(self.world, current(self.world))
    }

    fn focus(&mut self, name: Option<&str>) -> bool {
        let layer = match name.map(|name| self.place(name)) {
            None => None,
            Some(Some(Place::Here(layer))) => Some(layer),
            Some(_) => return false,
        };
        self.world.resource_mut::<Scope>().0 = layer;
        true
    }

    fn world_of(&self, player: PlayerId) -> Option<String> {
        self.layer_of(self.entity(player)?).map(|layer| name_of(self.world, layer))
    }

    fn players_in(&self, name: &str) -> Vec<Player> {
        let Some(Place::Here(layer)) = self.place(name) else { return Vec::new() };
        let here = in_world(self.world).into_iter().filter(|player| self.layer_of(*player) == Some(layer));
        here.filter_map(|player| self.world.get::<Identity>(player).map(|identity| identity.0.clone())).collect()
    }

    fn transfer(&mut self, player: PlayerId, name: &str) -> bool {
        let (Some(entity), conn) = (self.entity(player), ConnId::new(player.0)) else { return false };
        if handoff::is_moving(self.world, conn) {
            return false;
        }
        match self.place(name) {
            Some(Place::Here(layer)) if self.layer_of(entity) != Some(layer) => {
                enter(self.world, entity, layer);
                true
            }
            Some(Place::Shard(shard)) => handoff::ask(self.world, conn, shard, name),
            _ => false,
        }
    }

    fn broadcast_in(&mut self, name: &str, message: &str) {
        let packet = chat::raw_packet(message);
        match self.place(name) {
            Some(Place::Here(layer)) => chat::send_to_layer(self.world, layer, &packet),
            Some(Place::Shard(shard)) => drop(self.world.resource::<Peers>().tell(shard, ShardMsg::Say { world: Some(name.to_owned()), packet })),
            None => {}
        }
    }

    fn broadcast_all(&mut self, message: &str) {
        let packet = chat::raw_packet(message);
        chat::send_to_all(self.world, &packet);
        self.world.resource::<Peers>().tell_others(|| ShardMsg::Say { world: None, packet: packet.clone() });
    }

    fn create_world(&mut self, name: &str, template: &str) -> bool {
        arenas::create(self.world, name, template)
    }

    fn remove_world(&mut self, name: &str) -> bool {
        arenas::remove(self.world, name)
    }

    fn reset_world(&mut self, name: &str) -> bool {
        arenas::reset(self.world, name)
    }

    fn game_rules(&self) -> Vec<(String, RuleValue)> {
        rules::all(self.world, current(self.world))
    }

    fn set_game_rule(&mut self, name: &str, value: RuleValue) -> bool {
        let layer = current(self.world);
        rules::set(self.world, layer, name, value)
    }
}
