//! What the listener keeps of a logged-in player, and the disconnect screen it sends.

use std::net::IpAddr;

use acacia_proto::packets::{Disconnect, DisconnectContent};
use acacia_proto::types::DisconnectFailReason;
use acacia_session::server::ServerConnection;
use bytes::Bytes;
use uuid::Uuid;

use crate::config::NetConfig;
use crate::congestion::Congestion;
use crate::event::ConnId;

pub(crate) struct Player {
    pub id: ConnId,
    pub conn: ServerConnection,
    pub uuid: Uuid,
    pub name: String,
    pub xuid: String,
    pub ip: IpAddr,
    pub congested: bool,
}

/// Congested above the high watermark, clear again below the low one, so a backlog hovering at
/// either does not flap.
fn is_congested(was: bool, queued: usize, cfg: &NetConfig) -> bool {
    queued > if was { cfg.drained_bytes } else { cfg.congested_bytes }
}

impl Player {
    pub fn track_backlog(&mut self, queued: usize, cfg: &NetConfig, congestion: &Congestion) {
        let congested = is_congested(self.congested, queued, cfg);
        if congested != self.congested {
            self.congested = congested;
            congestion.set(self.id, congested);
        }
    }
}

pub(crate) fn disconnect(reason: DisconnectFailReason, message: &str) -> Bytes {
    let content = DisconnectContent { message: message.into(), filtered_message: String::new() };
    crate::encode(&Disconnect { reason, hide_disconnect_reason: false, content: Some(content) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn congestion_holds_between_the_watermarks() {
        let cfg = NetConfig { congested_bytes: 1000, drained_bytes: 100, ..NetConfig::default() };
        let mut congested = false;
        let trace: Vec<bool> = [500, 1001, 500, 101, 100, 500].into_iter().map(|queued| {
            congested = is_congested(congested, queued, &cfg);
            congested
        }).collect();
        assert_eq!(trace, [false, true, true, true, false, false]);
    }
}
