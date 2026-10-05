use std::net::SocketAddr;

use sapling_api::{Command, Difficulty, GameMode, Handler, Priority};
use sapling_net::{Advert, Listener, Motd, NetConfig, NetherNet, SigningKeys};
use sapling_shard::{Shard, ShardConfig, ShardHandle};
use sapling_world::{World, WorldFolder};

/// Half the side of the default spawn platform, in blocks.
const PLATFORM_HALF: i32 = 8;
const PLATFORM_Y: i32 = 64;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("binding the server socket: {0}")]
    Bind(#[from] std::io::Error),
    #[cfg(feature = "online")]
    #[error(transparent)]
    Keys(#[from] sapling_net::KeyError),
    #[error("online mode needs the `online` feature, which fetches the login signing keys")]
    OnlineUnavailable,
}

pub struct ServerBuilder {
    addr: SocketAddr,
    net: NetConfig,
    shard: ShardConfig,
    world: Option<World>,
    handlers: Vec<(Priority, Box<dyn Handler>)>,
    /// Each registers one command type on the shard once it exists.
    commands: Vec<fn(&mut Shard)>,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self { addr: ([0, 0, 0, 0], 19132).into(), net: NetConfig::default(), shard: ShardConfig::default(), world: None, handlers: Vec::new(), commands: Vec::new() }
    }
}

impl ServerBuilder {
    /// The UDP address to listen on (default `0.0.0.0:19132`; port 0 picks a free one).
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.addr = addr;
        self
    }

    /// Admit only players Xbox Live vouches for (default). Off, anyone joins under any name.
    pub fn online_mode(mut self, online: bool) -> Self {
        self.net.online_mode = online;
        self
    }

    /// Also host over NetherNet: direct connect on the TCP side of the port, and the LAN list if
    /// `nethernet.lan` is set. Vanilla clients that find it no longer join over RakNet.
    pub fn nethernet(mut self, nethernet: NetherNet) -> Self {
        self.net.nethernet = Some(nethernet);
        self
    }

    pub fn max_players(mut self, max: usize) -> Self {
        self.net.max_players = max;
        self
    }

    /// The name in the server list and of the world.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.net.motd = Motd { name: name.clone(), ..self.net.motd };
        self.shard.world_name = name;
        self
    }

    pub fn game_mode(mut self, mode: GameMode) -> Self {
        self.shard.game_mode = mode;
        self
    }

    /// The world players join, and where in it they appear (feet position). Without one the
    /// server has a stone platform in the void.
    pub fn world(mut self, world: World, spawn: [f32; 3]) -> Self {
        self.world = Some(world);
        self.shard.spawn = spawn;
        self
    }

    /// A world opened from a folder, with the spawn, default game mode and difficulty its
    /// `level.dat` names. Call [`ServerBuilder::game_mode`] after it to override the folder's.
    pub fn folder(mut self, opened: WorldFolder) -> Self {
        let game_mode = opened.game_type.and_then(|saved| [(0, GameMode::Survival), (1, GameMode::Creative), (2, GameMode::Adventure), (6, GameMode::Spectator)].into_iter().find(|(id, _)| *id == saved));
        let difficulty = opened.difficulty.and_then(|saved| [Difficulty::Peaceful, Difficulty::Easy, Difficulty::Normal, Difficulty::Hard].get(usize::try_from(saved).ok()?).copied());
        self.shard.game_mode = game_mode.map_or(self.shard.game_mode, |(_, mode)| mode);
        self.shard.difficulty = difficulty.unwrap_or(self.shard.difficulty);
        self.world(opened.world, opened.spawn)
    }

    pub fn difficulty(mut self, difficulty: Difficulty) -> Self {
        self.shard.difficulty = difficulty;
        self
    }

    /// Whether clients that keep a blob cache get chunk data through it (default: yes).
    pub fn blob_cache(mut self, enabled: bool) -> Self {
        self.shard.blob_cache = enabled;
        self
    }

    /// Adds a handler at [`Priority::Normal`]; equal priorities are called in the order they were added.
    pub fn handler(self, handler: impl Handler) -> Self {
        self.handler_at(Priority::Normal, handler)
    }

    /// Adds a handler that runs before those of lower priority (see [`Priority`]).
    pub fn handler_at(mut self, priority: Priority, handler: impl Handler) -> Self {
        self.handlers.push((priority, Box::new(handler)));
        self
    }

    /// Adds a command: clients that may use it see its syntax and get completions.
    pub fn command<C: Command>(mut self) -> Self {
        self.commands.push(|shard| shard.add_command::<C>());
        self
    }

    /// Fetches the signing keys (online mode), binds the socket on the current tokio runtime and
    /// starts the game thread.
    pub async fn start(self) -> Result<Server, StartError> {
        let keys = if self.net.online_mode { signing_keys().await? } else { SigningKeys::empty() };
        let (listener, outbox, inbox) = sapling_net::listen(self.addr, self.net, keys).await?;
        let mut shard = Shard::new(self.shard, self.world.unwrap_or_else(platform_world), inbox, outbox);
        self.handlers.into_iter().for_each(|(priority, handler)| shard.add_handler_at(priority, handler));
        self.commands.into_iter().for_each(|register| register(&mut shard));
        tracing::info!(addr = %listener.local_addr(), "sapling listening");
        Ok(Server { listener, shard: shard.spawn() })
    }
}

#[cfg(feature = "online")]
async fn signing_keys() -> Result<SigningKeys, StartError> {
    let keys = sapling_net::fetch_signing_keys().await?;
    tracing::info!(keys = keys.len(), "fetched the login signing keys");
    Ok(keys)
}

#[cfg(not(feature = "online"))]
async fn signing_keys() -> Result<SigningKeys, StartError> {
    Err(StartError::OnlineUnavailable)
}

/// A void overworld with a 16x16 stone platform under the default spawn.
fn platform_world() -> World {
    let mut world = World::void();
    let stone = world.block_id("minecraft:stone").expect("the vanilla palette has stone");
    world.fill([-PLATFORM_HALF, PLATFORM_Y, -PLATFORM_HALF], [PLATFORM_HALF - 1, PLATFORM_Y, PLATFORM_HALF - 1], stone);
    world
}

/// A running server. Dropping it stops the game thread; [`Server::shutdown`] also waits until
/// the players have been told.
pub struct Server {
    listener: Listener,
    shard: ShardHandle,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr()
    }

    /// Where the server answers LAN discovery, if it does.
    pub fn lan(&self) -> Option<Advert> {
        self.listener.lan()
    }

    /// Stops the game, disconnects every player and closes the socket.
    pub async fn shutdown(self) {
        let Server { listener, shard } = self;
        // Joining the game thread can take a tick; keep that off the async runtime.
        let _ = tokio::task::spawn_blocking(move || shard.stop()).await;
        listener.closed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_world_is_a_platform_under_the_default_spawn() {
        let world = platform_world();
        let stone = world.block_id("minecraft:stone").unwrap();
        let [x, y, z] = ShardConfig::default().spawn.map(|c| c.floor() as i32);
        assert_eq!(world.block(x, y - 1, z), stone);
        assert_eq!((world.block(-8, 64, -8), world.block(7, 64, 7)), (stone, stone));
        let air = world.registry().air_id();
        assert_eq!((world.block(x, y, z), world.block(8, 64, 0), world.block(0, 63, 0)), (air, air, air));
    }

    #[test]
    fn the_name_reaches_the_server_list_and_the_world() {
        let builder = Server::builder().name("lobby").online_mode(false).max_players(7);
        assert_eq!((builder.net.motd.name.as_str(), builder.shard.world_name.as_str()), ("lobby", "lobby"));
        assert_eq!((builder.net.online_mode, builder.net.max_players), (false, 7));
    }
}
