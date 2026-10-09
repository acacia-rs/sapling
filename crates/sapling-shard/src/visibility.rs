//! Who sees whom: a player in the world is an entity for every other player of its layer whose
//! chunk view covers it. Once a tick the sets are diffed (AddPlayer / RemoveEntity), then what
//! happened to each player is handed to those who see it, encoded once. Both look only at the
//! players near a view (grid.rs), not at every pair.

use std::collections::{HashMap, HashSet};

use acacia_proto::packets::RemoveEntity;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddPlayerOptions;
use sapling_world::ChunkPos;

use crate::components::{Identity, InLayer, NetId, Outgoing, Position, Stage};
use crate::grid::Grid;
use crate::inventory::{self, Inventory};
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
type Seen<'a> = (Entity, &'a Stage, &'a InLayer, &'a NetId, &'a Identity, &'a Position, &'a Motion, &'a Profile, &'a Inventory);

/// The player as a new viewer needs it: with what it holds, then what it wears.
fn spawn_packets(id: &NetId, identity: &Identity, position: &Position, motion: &Motion, profile: &Profile, inventory: &Inventory) -> Vec<Bytes> {
    let player = sapling_net::encode(&sapling_data::add_player(&AddPlayerOptions {
        held: inventory::in_hand(inventory),
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
    }));
    std::iter::once(player).chain(inventory::spawn_packets(id.0, inventory)).collect()
}

/// Adds the players that came into each player's view and removes those that left it.
pub(crate) fn update(mut viewers: Query<Viewer>, seen: Query<Seen>) {
    let mut grid = Grid::default();
    for (other, stage, layer, id, _, position, ..) in &seen {
        if *stage == Stage::InWorld {
            let [x, _, z] = position.feet.map(|c| c.floor() as i32);
            grid.insert(layer.0, ChunkPos::of_block(x, z), (id.0, other));
        }
    }
    let mut spawns: HashMap<Entity, Vec<Bytes>> = HashMap::new();
    for (viewer, stage, in_layer, view, mut sees, mut out) in &mut viewers {
        if *stage != Stage::InWorld {
            continue;
        }
        let mut covered: Vec<(u64, Entity)> = grid.covered(in_layer.0, view).copied().filter(|(_, other)| *other != viewer).collect();
        covered.sort_unstable();
        for (_, other) in covered.iter().filter(|(_, other)| !sees.0.contains(other)) {
            if let Ok((_, _, _, id, identity, position, motion, profile, inventory)) = seen.get(*other) {
                out.0.extend_from_slice(spawns.entry(*other).or_insert_with(|| spawn_packets(id, identity, position, motion, profile, inventory)));
            }
        }
        let now: HashSet<Entity> = covered.into_iter().map(|(_, other)| other).collect();
        for gone in sees.0.difference(&now) {
            if let Ok((_, _, _, id, ..)) = seen.get(*gone) {
                out.push(&RemoveEntity { entity_id_self: id.0 as i64 });
            }
        }
        sees.0 = now;
    }
}

/// Hands this tick's packets about each player to the players who see it.
pub(crate) fn fan_out(mut to_viewers: ResMut<ToViewers>, mut viewers: Query<(Entity, &Sees, &mut Outgoing)>) {
    if to_viewers.0.is_empty() {
        return;
    }
    let mut watchers: HashMap<Entity, Vec<Entity>> = to_viewers.0.iter().map(|(about, _)| (*about, Vec::new())).collect();
    for (viewer, sees, _) in &viewers {
        for about in &sees.0 {
            if let Some(watching) = watchers.get_mut(about) {
                watching.push(viewer);
            }
        }
    }
    for (about, packet) in to_viewers.0.drain(..) {
        for viewer in &watchers[&about] {
            if let Ok((_, _, mut out)) = viewers.get_mut(*viewer) {
                out.0.push(packet.clone());
            }
        }
    }
}
