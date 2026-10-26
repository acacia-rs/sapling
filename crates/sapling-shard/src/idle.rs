//! BDS's `player-idle-timeout` (`ShardConfig::idle_timeout`): a player in the world who neither
//! moves, turns nor sends an action for that long is kicked. Runs first in the tick, before any
//! system takes the tick's packets.

use std::collections::{HashMap, HashSet};

use acacia_proto::Packet;
use acacia_proto::packets::{Animate, CommandRequest, Interact, InventoryTransaction, ItemStackRequest, MobEquipment, ModalFormResponse, PlayerAction, Text};
use acacia_proto::types::DisconnectFailReason;
use bevy_ecs::prelude::*;

use crate::components::{Conn, Net, PacketQueues, Position, Settings, Stage, Tick};
use crate::config::TICK;

/// The client's own string for it; whether BDS sends this key or another was not recorded.
const IDLE: &str = "disconnectionScreen.body.idleTooLong";

/// What counts as acting, beside moving and turning.
const ACTIONS: [u32; 9] =
    [Text::ID, CommandRequest::ID, InventoryTransaction::ID, ItemStackRequest::ID, Interact::ID, Animate::ID, PlayerAction::ID, ModalFormResponse::ID, MobEquipment::ID];

/// Where a player last stood and looked, and the tick it last did something.
pub(crate) struct Last {
    at: [f32; 5],
    active: u64,
    kicked: bool,
}

pub(crate) fn kick_idle(
    settings: Res<Settings>,
    tick: Res<Tick>,
    net: Res<Net>,
    queues: Res<PacketQueues>,
    players: Query<(Entity, &Conn, &Position, &Stage)>,
    mut last: Local<HashMap<Entity, Last>>,
) {
    let Some(timeout) = settings.0.idle_timeout else { return };
    let limit = (timeout.as_millis() / TICK.as_millis()).max(1) as u64;
    let acted: HashSet<Entity> = queues.senders(&ACTIONS).collect();
    last.retain(|entity, _| players.contains(*entity));
    for (entity, conn, position, stage) in &players {
        if *stage != Stage::InWorld {
            continue;
        }
        let at = [position.feet[0], position.feet[1], position.feet[2], position.yaw, position.pitch];
        let seen = last.entry(entity).or_insert(Last { at, active: tick.0, kicked: false });
        if seen.at != at || acted.contains(&entity) {
            (seen.at, seen.active) = (at, tick.0);
        } else if !seen.kicked && tick.0 - seen.active >= limit {
            seen.kicked = true;
            tracing::info!(conn = conn.0.get(), "kicking an idle player");
            net.0.kick_for(conn.0, DisconnectFailReason::KickedForIdle, IDLE);
        }
    }
}
