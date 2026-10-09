//! The player list (pause menu): everyone in a world of the server, whoever sees whom and
//! whichever shard they are on. A client takes a player's skin from its list entry, so the entry
//! precedes any AddPlayer.

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{PlayerList, RemoveEntity};
use acacia_proto::types::{
    PlayerRecord, PlayerRecordContent, PlayerRecordContentAdd, PlayerRecordContentRemove,
    PlayerRecordType,
};
use bevy_ecs::prelude::*;
use std::collections::HashMap;

use bytes::Bytes;
use sapling_net::{ConnId, VerifiedLogin};

use crate::components::{Conn, NetId, Outgoing, in_world};
use crate::handoff::{Peers, ShardMsg};
use crate::visibility::Sees;
use crate::{chat, skin};

/// The colour BDS gave the bot of acacia-testserver's recording (locator bar dot).
const PLAYER_COLOUR: i32 = -303_174_145;
/// `legacy_type` of an add and of a remove record, as BDS sends them (`bds_probe`).
const LEGACY_ADD: u8 = 0;
const LEGACY_REMOVE: u8 = 1;

/// What a login tells other clients about its player.
#[derive(Component)]
pub(crate) struct Profile {
    pub uuid: Uuid,
    pub device_id: String,
    pub device_os: i32,
    /// PlayerList packets adding and removing this player, encoded once.
    list_add: Bytes,
    list_remove: Bytes,
}

fn list(r#type: PlayerRecordType, legacy_type: u8, content: PlayerRecordContent) -> Bytes {
    // One record per packet: the only shape seen from BDS.
    sapling_net::encode(&PlayerList {
        records: vec![PlayerRecord {
            r#type,
            legacy_type,
            content,
        }],
    })
}

impl Profile {
    pub fn new(login: &VerifiedLogin, id: u64) -> Self {
        let identity = &login.identity;
        let uuid: Uuid = identity
            .uuid
            .to_string()
            .parse()
            .expect("a uuid prints as a uuid");
        let add = PlayerRecordContentAdd {
            uuid,
            entity_unique_id: id as i64,
            username: identity.display_name.clone(),
            xbox_user_id: identity.xuid.clone(),
            platform_chat_id: String::new(),
            build_platform: login.client_data.device_os,
            skin_data: skin::wire_skin(&login.client_data.skin),
            is_teacher: false,
            is_host: false,
            is_subclient: false,
            player_color: PLAYER_COLOUR,
        };
        Self {
            uuid,
            device_id: login
                .client_data
                .device_id
                .chars()
                .filter(|c| c.is_ascii_graphic())
                .take(64)
                .collect(),
            device_os: login.client_data.device_os,
            list_add: list(
                PlayerRecordType::Add,
                LEGACY_ADD,
                PlayerRecordContent::Add(add),
            ),
            list_remove: list(
                PlayerRecordType::Remove,
                LEGACY_REMOVE,
                PlayerRecordContent::Remove(PlayerRecordContentRemove { uuid }),
            ),
        }
    }
}

/// A player's list packets, as the other shards keep them.
#[derive(Debug, Clone)]
pub(crate) struct ListEntry {
    add: Bytes,
    remove: Bytes,
}

/// The listed players of the other shards: the list is the server's, not the shard's.
#[derive(Resource, Default)]
pub(crate) struct Elsewhere(pub HashMap<ConnId, ListEntry>);

fn send(world: &mut World, to: Entity, packets: impl IntoIterator<Item = Bytes>) {
    if let Some(mut out) = world.get_mut::<Outgoing>(to) {
        out.0.extend(packets);
    }
}

pub(crate) fn list_entry(world: &World, player: Entity) -> Option<(ConnId, ListEntry)> {
    let (conn, profile) = (world.get::<Conn>(player)?, world.get::<Profile>(player)?);
    Some((
        conn.0,
        ListEntry {
            add: profile.list_add.clone(),
            remove: profile.list_remove.clone(),
        },
    ))
}

/// Lists a player who just entered the world for everyone, and everyone for that player.
pub(crate) fn arrive(world: &mut World, newcomer: Entity) {
    let present = in_world(world);
    let entry = |entity: &Entity| world.get::<Profile>(*entity).map(|p| p.list_add.clone());
    let (Some(own), here) = (
        list_entry(world, newcomer),
        present.iter().filter_map(entry),
    ) else {
        return;
    };
    let everyone: Vec<Bytes> = here
        .chain(
            world
                .resource::<Elsewhere>()
                .0
                .values()
                .map(|entry| entry.add.clone()),
        )
        .collect();
    for player in present {
        if player == newcomer {
            send(world, player, everyone.iter().cloned());
        } else {
            send(world, player, [own.1.add.clone()]);
        }
    }
    world.resource::<Peers>().tell_others(|| ShardMsg::Listed {
        conn: own.0,
        entry: own.1.clone(),
    });
}

/// Removes a player's entity from those who saw it: it left their world or the shard.
pub(crate) fn vanish(world: &mut World, leaver: Entity, id: u64) {
    let remove = sapling_net::encode(&RemoveEntity {
        entity_id_self: id as i64,
    });
    for player in in_world(world) {
        if world
            .get_mut::<Sees>(player)
            .is_some_and(|mut sees| sees.0.remove(&leaver))
        {
            send(world, player, [remove.clone()]);
        }
    }
}

/// Unlists a player whose connection ended and removes its entity from those who saw it.
pub(crate) fn depart(world: &mut World, leaver: Entity) {
    let (Some((conn, entry)), Some(id)) = (
        list_entry(world, leaver),
        world.get::<NetId>(leaver).map(|id| id.0),
    ) else {
        return;
    };
    vanish(world, leaver, id);
    chat::send_to_all(world, &entry.remove);
    world
        .resource::<Peers>()
        .tell_others(|| ShardMsg::Unlisted { conn });
}

/// A player of another shard entered the game: everyone here lists it.
pub(crate) fn listed(world: &mut World, conn: ConnId, entry: ListEntry) {
    chat::send_to_all(world, &entry.add);
    world.resource_mut::<Elsewhere>().0.insert(conn, entry);
}

/// A player of another shard left the game.
pub(crate) fn unlisted(world: &mut World, conn: ConnId) {
    if let Some(entry) = world.resource_mut::<Elsewhere>().0.remove(&conn) {
        chat::send_to_all(world, &entry.remove);
    }
}
