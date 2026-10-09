//! What a shard's ECS world holds. Players and layers are entities; a player points at its layer.

use std::collections::HashMap;

use acacia_proto::{Packet, RawPacket};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_net::{ConnId, Outbox};

use crate::ShardConfig;

#[derive(Component)]
pub(crate) struct Conn(pub ConnId);

#[derive(Component)]
pub(crate) struct Identity(pub sapling_api::Player);

/// The entity's id on the wire, both its runtime and its unique id. Unique per shard.
#[derive(Component, Clone, Copy)]
pub(crate) struct NetId(pub u64);

#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct Position {
    pub feet: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub on_ground: bool,
}

#[derive(Component)]
pub(crate) struct InLayer(pub Entity);

/// How far a player's join has come; each stage waits for one client packet (join.rs).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Stage {
    /// ResourcePacksInfo sent; waiting for the client to say it has all packs.
    PacksInfo,
    /// ResourcePackStack sent; waiting for the client to accept it.
    PackStack,
    /// StartGame and the registries sent; waiting for the client's chunk radius.
    StartGame,
    /// Radius known; chunks and PlayStatus(PlayerSpawn) go out on this tick.
    Chunks,
    /// Spawned on the client; waiting for it to leave its loading screen.
    Loading,
    InWorld,
}

/// Packets for one player, sent as one batch at the end of the tick.
#[derive(Component, Default)]
pub(crate) struct Outgoing(pub Vec<Bytes>);

impl Outgoing {
    pub fn push<T: Packet>(&mut self, packet: &T) {
        self.0.push(sapling_net::encode(packet));
    }
}

/// A world players can be in: its blocks and where newcomers stand.
#[derive(Component)]
pub(crate) struct Layer {
    pub world: sapling_world::World,
    pub spawn: [f32; 3],
}

#[derive(Resource)]
pub(crate) struct DefaultLayer(pub Entity);

#[derive(Resource)]
pub(crate) struct Settings(pub ShardConfig);

#[derive(Resource)]
pub(crate) struct Net(pub Outbox);

#[derive(Resource, Default)]
pub(crate) struct Tick(pub u64);

#[derive(Resource, Default)]
pub(crate) struct Players {
    pub by_conn: HashMap<ConnId, Entity>,
    /// Those `on_join` ran for and `on_quit` has not.
    pub in_world: usize,
    pub next_net_id: u64,
}

impl Players {
    pub fn entity(&self, id: sapling_api::PlayerId) -> Option<Entity> {
        self.by_conn.get(&ConnId::new(id.0)).copied()
    }
}

/// The players in the world (past their loading screen), in join order.
pub(crate) fn in_world(world: &World) -> Vec<Entity> {
    let is_in = |entity: &Entity| world.get::<Stage>(*entity) == Some(&Stage::InWorld);
    let mut players: Vec<Entity> = world
        .resource::<Players>()
        .by_conn
        .values()
        .copied()
        .filter(is_in)
        .collect();
    players.sort_by_key(|entity| world.get::<NetId>(*entity).map(|id| id.0));
    players
}

/// Who `entity` is, if it is a player in the world.
pub(crate) fn in_world_identity(world: &World, entity: Entity) -> Option<sapling_api::Player> {
    let in_world = world.get::<Stage>(entity) == Some(&Stage::InWorld);
    world
        .get::<Identity>(entity)
        .filter(|_| in_world)
        .map(|identity| identity.0.clone())
}

#[derive(Component)]
pub(crate) struct Mode(pub sapling_api::GameMode);

/// Counts what each player sent in a tick, to drop what exceeds `max`.
pub(crate) struct PerTick {
    max: usize,
    seen: HashMap<Entity, usize>,
}

impl PerTick {
    pub fn new(max: usize) -> Self {
        Self {
            max,
            seen: HashMap::new(),
        }
    }

    pub fn allow(&mut self, from: Entity) -> bool {
        let count = self.seen.entry(from).or_default();
        *count += 1;
        *count <= self.max
    }
}

/// Players whose client finished loading this tick, and players whose connection ended.
#[derive(Resource, Default)]
pub(crate) struct Arrivals {
    pub joined: Vec<Entity>,
    pub left: Vec<Entity>,
}

/// This tick's client packets by packet id, each queue in arrival order. A system takes the types
/// it handles; what nobody takes is dropped when the tick ends.
#[derive(Resource, Default)]
pub(crate) struct PacketQueues(HashMap<u32, Vec<(Entity, RawPacket)>>);

impl PacketQueues {
    pub fn push(&mut self, from: Entity, packet: RawPacket) {
        self.0.entry(packet.id).or_default().push((from, packet));
    }

    /// Removes and decodes every `T` received this tick; undecodable ones are dropped.
    pub fn take<T: Packet>(&mut self) -> Vec<(Entity, T)> {
        let raw = self
            .0
            .get_mut(&T::ID)
            .map(std::mem::take)
            .unwrap_or_default();
        let decode = |(from, packet): (Entity, RawPacket)| match packet.decode::<T>() {
            Ok(decoded) => Some((from, decoded)),
            Err(error) => {
                tracing::debug!(packet = T::NAME, %error, "dropping an undecodable packet");
                None
            }
        };
        raw.into_iter().filter_map(decode).collect()
    }

    pub fn clear(&mut self) {
        self.0.values_mut().for_each(Vec::clear);
    }
}
