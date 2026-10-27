//! Emotes, passed on as BDS passed them (research/captures/bds-packets.txt): a player's `Emote`
//! reaches those who see it, not the player, named by its own id and marked server-side; its
//! `EmoteList` goes to them the same way.

use acacia_proto::packets::{Emote, EmoteFlags, EmoteList};
use bevy_ecs::prelude::*;

use crate::components::{Identity, NetId, PacketQueues, Stage};
use crate::visibility::ToViewers;

/// An emote's id is a uuid; anything longer is not one.
const MAX_ID: usize = 64;
/// The emote wheel's slots, with room to spare.
const MAX_LISTED: usize = 32;

pub(crate) fn relay(mut packets: ResMut<PacketQueues>, mut to_viewers: ResMut<ToViewers>, players: Query<(&NetId, &Stage, &Identity)>) {
    for (entity, emote) in packets.take::<Emote>() {
        let Ok((id, Stage::InWorld, identity)) = players.get(entity) else { continue };
        if emote.emote_id.len() <= MAX_ID {
            let xuid = identity.0.xuid.clone();
            to_viewers.push(entity, &Emote { entity_id: id.0, emote_id: emote.emote_id, emote_length_ticks: emote.emote_length_ticks, xuid, platform_id: String::new(), flags: EmoteFlags::ServerSide });
        }
    }
    for (entity, list) in packets.take::<EmoteList>() {
        let Ok((id, Stage::InWorld, _)) = players.get(entity) else { continue };
        if list.emote_pieces.len() <= MAX_LISTED {
            to_viewers.push(entity, &EmoteList { player_id: id.0, emote_pieces: list.emote_pieces });
        }
    }
}
