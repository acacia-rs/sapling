//! A player's way to a world on another shard, by messages only (docs/DESIGN.md "Step 5 as
//! built"): ask, be answered, commit the player's components, tell the network's directory, and
//! forward what the network still delivered here. Between commit and forward the old shard only
//! buffers and the new one holds what reaches it directly, so every packet is handled once, in
//! order, by the shard that owns the player. Nothing here waits on another thread.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};

use acacia_proto::packets::RemoveEntity;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_net::{ConnId, NetEvent};

use crate::components::{Conn, DefaultLayer, InLayer, Net, NetId, Outgoing, Players, Stage, Tick};
use crate::mobs::SeesMobs;
use crate::player::{Carried, Local};
use crate::roster::{self, Elsewhere, ListEntry};
use crate::visibility::Sees;
use crate::worlds::{self, Atlas, Change, Changes, Place};
use crate::{containers, inventory};

/// Ticks a shard waits for the answer to a transfer before it gives up.
const ANSWER_TICKS: u64 = 100;
/// Ticks an arrived player's packets wait for the old shard's forward before they are let in.
const HOLD_TICKS: u64 = 100;

/// A player on its way: its components, and the worlds it leaves and goes to.
pub(crate) struct Travel {
    carried: Carried,
    from: String,
    to: String,
}

pub(crate) enum ShardMsg {
    /// Would you take this player into `world`?
    Ask { from: usize, conn: ConnId, world: String },
    Answer { conn: ConnId, world: String, accepted: bool },
    /// The player is the receiver's from here on.
    Commit(Box<Travel>),
    /// What reached the old shard after the commit; nothing more follows.
    Forward { conn: ConnId, events: Vec<NetEvent> },
    /// A chat line for one world, or for everyone.
    Say { world: Option<String>, packet: Bytes },
    Listed { conn: ConnId, entry: ListEntry },
    Unlisted { conn: ConnId },
    WorldAdded { name: String, shard: usize },
    WorldRemoved { name: String },
}

/// A shard's place among the shards of a server: its index and the mailboxes.
pub struct Link {
    pub(crate) index: usize,
    pub(crate) mailbox: Receiver<ShardMsg>,
    pub(crate) peers: Vec<Sender<ShardMsg>>,
}

impl Link {
    /// The link of a server's only shard.
    pub fn alone() -> Self {
        Self::mesh(1).remove(0)
    }

    /// Links for `count` shards that reach each other; the shard given link `i` is shard `i`.
    pub fn mesh(count: usize) -> Vec<Self> {
        let (peers, mailboxes): (Vec<_>, Vec<_>) = (0..count).map(|_| channel()).unzip();
        mailboxes.into_iter().enumerate().map(|(index, mailbox)| Self { index, mailbox, peers: peers.clone() }).collect()
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

#[derive(Resource)]
pub(crate) struct Peers {
    pub index: usize,
    senders: Vec<Sender<ShardMsg>>,
}

impl Peers {
    pub fn new(index: usize, senders: Vec<Sender<ShardMsg>>) -> Self {
        Self { index, senders }
    }

    /// Hands a message to a shard; gives it back when that shard is gone.
    pub fn tell(&self, shard: usize, message: ShardMsg) -> Result<(), ShardMsg> {
        match self.senders.get(shard) {
            Some(sender) => sender.send(message).map_err(|refused| refused.0),
            None => Err(message),
        }
    }

    pub fn tell_others(&self, message: impl Fn() -> ShardMsg) {
        for shard in (0..self.senders.len()).filter(|shard| *shard != self.index) {
            let _ = self.tell(shard, message());
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct Handoffs {
    /// Asked of another shard and not answered: the world, its shard, the tick the wait ends.
    asked: HashMap<ConnId, (String, usize, u64)>,
    /// Committed to another shard: what the network still sends here, until it says `Rerouted`.
    departing: HashMap<ConnId, (usize, Vec<NetEvent>)>,
    /// Arrived from another shard: what the network sends directly, until the forward is in.
    held: HashMap<ConnId, (Vec<NetEvent>, u64)>,
}

/// Whether a player is between two shards, either way.
pub(crate) fn is_moving(world: &World, conn: ConnId) -> bool {
    let handoffs = world.resource::<Handoffs>();
    handoffs.asked.contains_key(&conn) || handoffs.held.contains_key(&conn)
}

/// Asks the shard of `to` for a place; false when that shard is gone.
pub(crate) fn ask(world: &mut World, conn: ConnId, shard: usize, to: &str) -> bool {
    let (peers, deadline) = (world.resource::<Peers>(), world.resource::<Tick>().0 + ANSWER_TICKS);
    let asked = peers.tell(shard, ShardMsg::Ask { from: peers.index, conn, world: to.to_owned() }).is_ok();
    if asked {
        world.resource_mut::<Handoffs>().asked.insert(conn, (to.to_owned(), shard, deadline));
    }
    asked
}

fn in_world_entity(world: &World, conn: ConnId) -> Option<Entity> {
    let entity = world.resource::<Players>().by_conn.get(&conn).copied()?;
    (world.get::<Stage>(entity) == Some(&Stage::InWorld)).then_some(entity)
}

fn fail(world: &mut World, conn: ConnId, to: String) {
    if let Some(entity) = in_world_entity(world, conn) {
        world.resource_mut::<Changes>().0.push(Change::Failed { entity, to });
    }
}

/// Gives a player to shard `shard`. Its last packets from here, the removal of everything its
/// client has from this world among them, travel with it. If the shard is gone the player stays.
fn commit(world: &mut World, entity: Entity, shard: usize, to: String) {
    let (Some(layer), Some(id)) = (world.get::<InLayer>(entity).map(|l| l.0), world.get::<NetId>(entity).map(|id| id.0)) else { return };
    let from = worlds::name_of(world, layer);
    let seen: Vec<u64> = world.get_mut::<Sees>(entity).map(|mut sees| std::mem::take(&mut sees.0)).unwrap_or_default().into_iter().filter_map(|other| world.get::<NetId>(other).map(|id| id.0)).collect();
    let mobs = world.get_mut::<SeesMobs>(entity).map(|mut sees| sees.take_ids()).unwrap_or_default();
    if let Some(mut out) = world.get_mut::<Outgoing>(entity) {
        seen.into_iter().chain(mobs).for_each(|id| out.push(&RemoveEntity { entity_id_self: id as i64 }));
    }
    let listing = roster::list_entry(world, entity);
    // The items go along; this shard's storage keeps what they were when the player left it.
    containers::close(world, entity);
    inventory::clear_grid(world, entity);
    inventory::save(world, entity);
    let Some(carried) = world.entity_mut(entity).take::<Carried>() else { return };
    let conn = carried.conn.0;
    let index = world.resource::<Peers>().index;
    match world.resource::<Peers>().tell(shard, ShardMsg::Commit(Box::new(Travel { carried, from, to }))) {
        Ok(()) => {
            world.resource::<Net>().0.route(conn, index, shard);
            world.resource_mut::<Handoffs>().departing.insert(conn, (shard, Vec::new()));
            // Still listed for everyone here: the entry is one of another shard's from now on.
            world.resource_mut::<Elsewhere>().0.extend(listing);
            roster::vanish(world, entity, id);
            world.despawn(entity);
            let mut players = world.resource_mut::<Players>();
            players.by_conn.remove(&conn);
            players.in_world -= 1;
        }
        // The sets it saw are empty, so the client is shown this world's entities again.
        Err(ShardMsg::Commit(travel)) => {
            let Travel { carried, to, .. } = *travel;
            world.entity_mut(entity).insert(carried);
            world.resource_mut::<Changes>().0.push(Change::Failed { entity, to });
        }
        Err(_) => unreachable!("a refused message comes back as it was sent"),
    }
}

/// Takes a committed player in: into the world it was promised, or the shard's first if that
/// one is gone by now.
fn arrive(world: &mut World, travel: Travel) {
    let Travel { carried, from, to } = travel;
    let layer = match world.resource::<Atlas>().find(&to) {
        Some(Place::Here(layer)) => layer,
        _ => world.resource::<DefaultLayer>().0,
    };
    let (conn, deadline) = (carried.conn.0, world.resource::<Tick>().0 + HOLD_TICKS);
    let entity = world.spawn((carried, Local::in_layer(layer))).id();
    let mut players = world.resource_mut::<Players>();
    players.by_conn.insert(conn, entity);
    players.in_world += 1;
    world.resource_mut::<Handoffs>().held.insert(conn, (Vec::new(), deadline));
    world.resource_mut::<Elsewhere>().0.remove(&conn);
    worlds::settle(world, entity, layer);
    let to = worlds::name_of(world, layer);
    world.resource_mut::<Changes>().0.push(Change::Moved { entity, from, to });
}

/// Handles one message from another shard; returns the network events to take in now.
pub(crate) fn receive(world: &mut World, message: ShardMsg) -> Vec<NetEvent> {
    match message {
        ShardMsg::Ask { from, conn, world: name } => {
            let accepted = matches!(world.resource::<Atlas>().find(&name), Some(Place::Here(_)));
            let _ = world.resource::<Peers>().tell(from, ShardMsg::Answer { conn, world: name, accepted });
        }
        ShardMsg::Answer { conn, world: name, accepted } => {
            let asked = world.resource_mut::<Handoffs>().asked.remove(&conn);
            let Some((_, shard, _)) = asked.filter(|(to, ..)| *to == name) else { return Vec::new() };
            match (accepted, in_world_entity(world, conn)) {
                (true, Some(entity)) => commit(world, entity, shard, name),
                _ => fail(world, conn, name),
            }
        }
        ShardMsg::Commit(travel) => arrive(world, *travel),
        ShardMsg::Forward { conn, mut events } => {
            events.extend(world.resource_mut::<Handoffs>().held.remove(&conn).map(|(held, _)| held).unwrap_or_default());
            return events;
        }
        ShardMsg::Say { world: Some(name), packet } => {
            if let Some(Place::Here(layer)) = world.resource::<Atlas>().find(&name) {
                crate::chat::send_to_layer(world, layer, &packet);
            }
        }
        ShardMsg::Say { world: None, packet } => crate::chat::send_to_all(world, &packet),
        ShardMsg::Listed { conn, entry } => roster::listed(world, conn, entry),
        ShardMsg::Unlisted { conn } => roster::unlisted(world, conn),
        ShardMsg::WorldAdded { name, shard } => world.resource_mut::<Atlas>().learn(name, shard),
        ShardMsg::WorldRemoved { name } => world.resource_mut::<Atlas>().forget(&name),
    }
    Vec::new()
}

/// Takes a network event that belongs to a handoff: one for a player who is leaving (kept for
/// the forward), one for a player whose forward is not in yet (held), and the directory's word
/// that a leaving player's events have ended (the forward goes out). Others come back.
pub(crate) fn intercept(world: &mut World, event: NetEvent) -> Option<NetEvent> {
    let conn = match &event {
        NetEvent::Joined { .. } => return Some(event),
        NetEvent::Packet { conn, .. } | NetEvent::Left { conn, .. } | NetEvent::Rerouted { conn } => *conn,
    };
    if matches!(event, NetEvent::Rerouted { .. }) {
        if let Some((shard, events)) = world.resource_mut::<Handoffs>().departing.remove(&conn) {
            let _ = world.resource::<Peers>().tell(shard, ShardMsg::Forward { conn, events });
        }
        return None;
    }
    let handoffs = world.resource_mut::<Handoffs>().into_inner();
    let waiting = match (handoffs.departing.get_mut(&conn), handoffs.held.get_mut(&conn)) {
        (Some((_, events)), _) | (None, Some((events, _))) => events,
        (None, None) => return Some(event),
    };
    waiting.push(event);
    None
}

/// Gives up on answers that did not come and lets in what was held for a forward that did not.
pub(crate) fn expire(world: &mut World) -> Vec<NetEvent> {
    let tick = world.resource::<Tick>().0;
    let mut handoffs = world.resource_mut::<Handoffs>();
    let unanswered: Vec<ConnId> = handoffs.asked.iter().filter(|(_, (.., deadline))| *deadline <= tick).map(|(conn, _)| *conn).collect();
    let failed: Vec<(ConnId, String)> = unanswered.into_iter().filter_map(|conn| handoffs.asked.remove(&conn).map(|(to, ..)| (conn, to))).collect();
    let overdue: Vec<ConnId> = handoffs.held.iter().filter(|(_, (_, deadline))| *deadline <= tick).map(|(conn, _)| *conn).collect();
    let released = overdue.into_iter().filter_map(|conn| handoffs.held.remove(&conn)).flat_map(|(events, _)| events).collect();
    failed.into_iter().for_each(|(conn, to)| fail(world, conn, to));
    released
}

/// A stopping shard's part in the handoffs under way: the forwards it owes go out as they are.
pub(crate) fn close(world: &mut World) {
    let departing = std::mem::take(&mut world.resource_mut::<Handoffs>().departing);
    for (conn, (shard, events)) in departing {
        let _ = world.resource::<Peers>().tell(shard, ShardMsg::Forward { conn, events });
    }
}

/// The connections this shard owns.
pub(crate) fn owned(world: &mut World) -> Vec<ConnId> {
    world.query::<&Conn>().iter(world).map(|conn| conn.0).collect()
}
