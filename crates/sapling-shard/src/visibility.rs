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
use crate::hidden::Hidden;
use crate::inventory::{self, Inventory};
use crate::labels::Labels;
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

type Viewer<'a> = (Entity, &'a Stage, &'a InLayer, &'a View, &'a mut Sees, &'a mut Outgoing, Option<&'a Hidden>);
type Seen<'a> = (Entity, &'a Stage, &'a InLayer, &'a NetId, &'a Identity, &'a Position, &'a Motion, &'a Profile, &'a Inventory, Option<&'a Labels>);

/// The player as a new viewer needs it: with what it holds, then what it wears, then the labels
/// plugins gave it.
fn spawn_packets((id, identity, position, motion, profile, inventory, labels): (&NetId, &Identity, &Position, &Motion, &Profile, &Inventory, Option<&Labels>)) -> Vec<Bytes> {
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
    std::iter::once(player).chain(inventory::spawn_packets(id.0, inventory)).chain(crate::labels::packet(labels, id.0)).collect()
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
    for (viewer, stage, in_layer, view, mut sees, mut out, hidden) in &mut viewers {
        if *stage != Stage::InWorld {
            continue;
        }
        let mut covered: Vec<(u64, Entity)> = grid.covered(in_layer.0, view).copied().filter(|(_, other)| *other != viewer && !Hidden::hides(hidden, *other)).collect();
        covered.sort_unstable();
        for (_, other) in covered.iter().filter(|(_, other)| !sees.0.contains(other)) {
            if let Ok((_, _, _, id, identity, position, motion, profile, inventory, labels)) = seen.get(*other) {
                out.0.extend_from_slice(spawns.entry(*other).or_insert_with(|| spawn_packets((id, identity, position, motion, profile, inventory, labels))));
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

/// Removes from a player's client every player and mob it has as an entity: they are of a world
/// it leaves. The next diff shows it those of where it goes.
pub(crate) fn forget_seen(world: &mut World, player: Entity) {
    let seen: Vec<Entity> = world.get_mut::<Sees>(player).map(|mut sees| std::mem::take(&mut sees.0).into_iter().collect()).unwrap_or_default();
    let mut ids: Vec<u64> = seen.into_iter().filter_map(|other| world.get::<NetId>(other).map(|id| id.0)).collect();
    ids.extend(world.get_mut::<crate::mobs::SeesMobs>(player).map(|mut sees| sees.take_ids()).unwrap_or_default());
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        ids.into_iter().for_each(|id| out.push(&RemoveEntity { entity_id_self: id as i64 }));
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
