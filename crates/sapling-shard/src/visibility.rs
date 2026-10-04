//! Who sees whom: a player in the world is an entity for every other player of its layer whose
//! chunk view covers it. Once a tick the sets are diffed (AddPlayer / RemoveEntity), then what
//! happened to each player is handed to those who see it, encoded once.
// TODO: an index of players by chunk; the diff and the fan-out are O(players^2) per layer.

use std::collections::{HashMap, HashSet};

use acacia_proto::packets::RemoveEntity;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddPlayerOptions;
use sapling_world::ChunkPos;

use crate::components::{Identity, InLayer, NetId, Outgoing, Position, Stage};
use crate::movement::Motion;
use crate::roster::Profile;
use crate::view::View;

/// The other players this player's client has as entities.
#[derive(Component, Default)]
pub(crate) struct Sees(pub HashSet<Entity>);

/// This tick's packets about a player, for everyone who sees it.
#[derive(Resource, Default)]
pub(crate) struct ToViewers(Vec<(Entity, Bytes)>);

impl ToViewers {
    pub fn push<T: acacia_proto::Packet>(&mut self, about: Entity, packet: &T) {
        self.0.push((about, sapling_net::encode(packet)));
    }
}

type Viewer<'a> = (Entity, &'a Stage, &'a InLayer, &'a View, &'a mut Sees, &'a mut Outgoing);
type Seen<'a> = (Entity, &'a Stage, &'a InLayer, &'a NetId, &'a Identity, &'a Position, &'a Motion, &'a Profile);

fn spawn_packet(id: &NetId, identity: &Identity, position: &Position, motion: &Motion, profile: &Profile) -> Bytes {
    sapling_net::encode(&sapling_data::add_player(&AddPlayerOptions {
        uuid: profile.uuid,
        name: &identity.0.name,
        id: id.0,
        feet: position.feet,
        yaw: position.yaw,
        pitch: position.pitch,
        head_yaw: position.head_yaw,
        flags: motion.pose.flags(),
        device_id: &profile.device_id,
        device_os: profile.device_os,
    }))
}

/// Adds the players that came into each player's view and removes those that left it.
pub(crate) fn update(mut viewers: Query<Viewer>, seen: Query<Seen>) {
    let mut spawns: HashMap<Entity, Bytes> = HashMap::new();
    for (viewer, stage, in_layer, view, mut sees, mut out) in &mut viewers {
        if *stage != Stage::InWorld {
            continue;
        }
        let mut now = HashSet::new();
        for (other, stage, layer, id, identity, position, motion, profile) in &seen {
            let [x, _, z] = position.feet.map(|c| c.floor() as i32);
            if other == viewer || *stage != Stage::InWorld || layer.0 != in_layer.0 || !view.covers(ChunkPos::of_block(x, z)) {
                continue;
            }
            now.insert(other);
            if !sees.0.contains(&other) {
                out.0.push(spawns.entry(other).or_insert_with(|| spawn_packet(id, identity, position, motion, profile)).clone());
            }
        }
        for gone in sees.0.difference(&now) {
            if let Ok((_, _, _, id, ..)) = seen.get(*gone) {
                out.push(&RemoveEntity { entity_id_self: id.0 as i64 });
            }
        }
        sees.0 = now;
    }
}

/// Hands this tick's packets about each player to the players who see it.
pub(crate) fn fan_out(mut to_viewers: ResMut<ToViewers>, mut viewers: Query<(&Sees, &mut Outgoing)>) {
    if to_viewers.0.is_empty() {
        return;
    }
    for (sees, mut out) in &mut viewers {
        if !sees.0.is_empty() {
            out.0.extend(to_viewers.0.iter().filter(|(about, _)| sees.0.contains(about)).map(|(_, packet)| packet.clone()));
        }
    }
    to_viewers.0.clear();
}
