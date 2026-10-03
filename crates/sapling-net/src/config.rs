use std::time::Duration;

use acacia_proto::{GAME_VERSION, PROTOCOL_VERSION};

#[derive(Debug, Clone)]
pub struct NetConfig {
    /// Admit only logins the authorization service or Mojang signed.
    pub online_mode: bool,
    /// From the RakNet connection to PlayStatus(LoginSuccess); slower clients are dropped.
    pub login_timeout: Duration,
    pub max_players: usize,
    pub motd: Motd,
    /// A player whose unsent backlog grows past this is kicked rather than buffered without end.
    pub max_queued_bytes: usize,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            online_mode: true,
            login_timeout: Duration::from_secs(10),
            max_players: 100,
            motd: Motd::default(),
            max_queued_bytes: 16 * 1024 * 1024,
        }
    }
}

/// What the server list shows, sent in answer to unconnected pings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motd {
    pub name: String,
    pub sub_name: String,
    pub game_mode: String,
}

impl Default for Motd {
    fn default() -> Self {
        Self { name: "sapling".into(), sub_name: "sapling".into(), game_mode: "Survival".into() }
    }
}

impl Motd {
    /// The pong payload; a `;` in a name would shift every field after it.
    pub(crate) fn pong(&self, guid: u64, players: usize, max_players: usize, port: u16) -> String {
        let clean = |s: &str| s.replace(';', "");
        let (name, sub_name, game_mode) = (clean(&self.name), clean(&self.sub_name), clean(&self.game_mode));
        format!("MCPE;{name};{PROTOCOL_VERSION};{GAME_VERSION};{players};{max_players};{guid};{sub_name};{game_mode};1;{port};{port};0;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pong_has_the_fields_the_server_list_reads() {
        let motd = Motd { name: "a;b".into(), ..Motd::default() };
        let pong = motd.pong(7, 3, 20, 19132);
        let fields: Vec<&str> = pong.split(';').collect();
        assert_eq!(fields[..2], ["MCPE", "ab"]);
        assert_eq!(fields[2], PROTOCOL_VERSION.to_string());
        assert_eq!(fields[4..12], ["3", "20", "7", "sapling", "Survival", "1", "19132", "19132"]);
    }
}
