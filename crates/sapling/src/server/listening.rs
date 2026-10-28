//! The builder's settings for how the server is reached and run beside the game: the IPv6 socket,
//! the query, the console.

use std::net::SocketAddr;

use sapling_api::GameMode;

use super::ServerBuilder;

/// The game mode as the server list names it.
pub(super) fn motd_mode(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Survival => "Survival",
        GameMode::Creative => "Creative",
        GameMode::Adventure => "Adventure",
        GameMode::Spectator => "Spectator",
    }
}

impl ServerBuilder {
    /// Also listen on this IPv6 address (BDS's `server-portv6`, which it binds on `[::]`; default:
    /// none). The pong names its port beside the main one.
    pub fn bind_v6(mut self, addr: SocketAddr) -> Self {
        self.net.bind_v6 = Some(addr);
        self
    }

    /// Answer the GameSpy4 query on the game ports (`enable-query`; default: no): name, level
    /// name, players and their names, max players, version, port.
    pub fn query(mut self, enabled: bool) -> Self {
        self.net.query = enabled;
        self
    }

    /// Run the lines typed on stdin as commands of the server and log their answers, as BDS's
    /// console does (default: no). Tab completes command names on a terminal.
    pub fn console(mut self, enabled: bool) -> Self {
        self.console = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Server, ServerProperties};

    #[test]
    fn server_properties_reach_the_ipv6_socket_the_query_and_the_server_list() {
        let props = ServerProperties::parse("gamemode=creative\nserver-portv6=19133\nenable-query=true\n").unwrap();
        let builder = Server::builder().properties(&props);
        assert_eq!((builder.net.bind_v6, builder.net.query), (Some("[::]:19133".parse().unwrap()), true));
        assert_eq!(builder.net.motd.game_mode, "Creative");
    }
}
