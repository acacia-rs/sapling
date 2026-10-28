use std::net::SocketAddr;
use std::sync::Arc;

use sapling_api::{Command, Difficulty, GameMode, Handler, Plugin, Priority};
use sapling_net::{Admission, Advert, Listener, Motd, NetConfig, NetherNet, SigningKeys};
use sapling_shard::{Link, Shard, ShardConfig, ShardHandle};
use sapling_world::{Template, World};

use crate::layout::{self, WorldSpec};

mod error;
mod listening;
mod ops;
mod packs;
mod properties;
mod worlds;

pub use error::StartError;
pub use ops::Metrics;
pub use properties::{ConfigError, ServerProperties};

type HandlerFactory = Box<dyn Fn() -> Box<dyn Handler> + Send>;

pub struct ServerBuilder {
    addr: SocketAddr,
    pub(crate) net: NetConfig,
    shard: ShardConfig,
    /// The default world, named `shard.world_id`; without one it is the stone platform.
    world: Option<World>,
    worlds: Vec<WorldSpec>,
    join: Option<String>,
    templates: Vec<(String, Arc<Template>, [f32; 3])>,
    handlers: Vec<(Priority, Box<dyn Handler>)>,
    handlers_each: Vec<(Priority, HandlerFactory)>,
    plugins: Vec<Arc<dyn Plugin>>,
    /// Each registers one command type on a shard once it exists.
    commands: Vec<fn(&mut Shard)>,
    /// Other dimensions, each with the name of the world it belongs to.
    dimensions: Vec<(Option<String>, World)>,
    ops: ops::Ops,
    console: bool,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self {
            addr: ([0, 0, 0, 0], 19132).into(),
            net: NetConfig::default(),
            shard: ShardConfig::default(),
            world: None,
            worlds: Vec::new(),
            join: None,
            templates: Vec::new(),
            handlers: Vec::new(),
            handlers_each: Vec::new(),
            plugins: Vec::new(),
            commands: Vec::new(),
            dimensions: Vec::new(),
            ops: ops::Ops::default(),
            console: false,
        }
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

    pub fn max_players(self, max: usize) -> Self {
        self.net.max_players.reset(max);
        self
    }

    /// Limits on each address's connections (default: 8 at once, 10 new ones per 10 s; loopback
    /// exempt). [`Admission::off`] for load tests from another machine.
    pub fn admission(mut self, admission: Admission) -> Self {
        self.net.admission = admission;
        self
    }

    /// The name in the server list and the one clients show for the world.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.net.motd = Motd { name: name.clone(), ..self.net.motd };
        self.shard.world_name = name;
        self
    }

    /// Also the mode the server list shows.
    pub fn game_mode(mut self, mode: GameMode) -> Self {
        self.shard.game_mode = mode;
        self.net.motd.game_mode = listening::motd_mode(mode).into();
        self
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

    /// What the server makes of where clients say their players are (default: Observe, BDS's 0.5
    /// threshold and 5-tick correction delay); a plugin may give one player its own policy.
    pub fn movement_check(mut self, check: sapling_shard::MovementCheck) -> Self {
        self.shard.movement_check = check;
        self
    }

    /// Wall time between ticks (default 50 ms); shorter only for drills (examples/drill_server.rs).
    pub fn tick(mut self, tick: std::time::Duration) -> Self {
        self.shard.tick = tick;
        self
    }

    /// Adds a handler at [`Priority::Normal`]; equal priorities are called in the order they
    /// were added. For a server of one shard: see [`ServerBuilder::handler_each`].
    pub fn handler(self, handler: impl Handler) -> Self {
        self.handler_at(Priority::Normal, handler)
    }

    /// Adds a handler that runs before those of lower priority (see [`Priority`]).
    pub fn handler_at(mut self, priority: Priority, handler: impl Handler) -> Self {
        self.handlers.push((priority, Box::new(handler)));
        self
    }

    /// Adds a handler to every shard: `make` is called once per shard, and each handler sees
    /// the events of its own shard's worlds. They run after the handlers added one by one.
    pub fn handler_each<H: Handler>(self, make: impl Fn() -> H + Send + 'static) -> Self {
        self.handler_each_at(Priority::Normal, make)
    }

    pub fn handler_each_at<H: Handler>(mut self, priority: Priority, make: impl Fn() -> H + Send + 'static) -> Self {
        self.handlers_each.push((priority, Box::new(move || Box::new(make()))));
        self
    }

    /// Adds a plugin: [`Plugin::build`] runs once on every shard.
    pub fn plugin(mut self, plugin: impl Plugin) -> Self {
        self.plugins.push(Arc::new(plugin));
        self
    }

    /// Adds a command: clients that may use it see its syntax and get completions.
    pub fn command<C: Command>(mut self) -> Self {
        self.commands.push(|shard| shard.add_command::<C>());
        self
    }

    /// Fetches the signing keys (online mode), binds the socket on the current tokio runtime and
    /// starts the game threads, one per shard.
    pub async fn start(self) -> Result<Server, StartError> {
        let default = WorldSpec { name: self.shard.world_id.clone(), world: self.world.unwrap_or_else(layout::platform_world), spawn: self.shard.spawn, shard: 0 };
        let join = self.join.unwrap_or_else(|| default.name.clone());
        let default_name = default.name.clone();
        let (layout, entry) = layout::plan(std::iter::once(default).chain(self.worlds).collect(), &join)?;
        let mut dimensions: Vec<(String, World)> = self.dimensions.into_iter().map(|(of, world)| (of.unwrap_or_else(|| default_name.clone()), world)).collect();
        if let Some((unknown, _)) = dimensions.iter().find(|(of, _)| !layout.iter().flatten().any(|world| world.name == *of)) {
            return Err(StartError::UnknownWorld(unknown.clone()));
        }
        if layout.len() > 1 && !self.handlers.is_empty() {
            return Err(StartError::HandlerPerShard);
        }
        self.ops.apply_allow_list(&self.net)?;
        let keys = if self.net.online_mode { signing_keys().await? } else { SigningKeys::empty() };
        let (listener, outbox, inboxes) = sapling_net::listen_shards(self.addr, self.net, keys, layout.len(), entry).await?;
        let count = layout.len();
        let mut single = Some(self.handlers);
        let mut shards = Vec::with_capacity(count);
        for ((worlds, inbox), link) in layout.into_iter().zip(inboxes).zip(Link::mesh(count, entry)) {
            let mut worlds = worlds.into_iter();
            let first = worlds.next().expect("the plan leaves no shard empty");
            let mut names = vec![first.name.clone()];
            let cfg = ShardConfig { world_id: first.name, spawn: first.spawn, ..self.shard.clone() };
            let mut shard = Shard::linked(cfg, first.world, inbox, outbox.clone(), link);
            for world in worlds {
                names.push(world.name.clone());
                shard.add_world(&world.name, world.world, world.spawn);
            }
            let (here, elsewhere): (Vec<_>, Vec<_>) = dimensions.into_iter().partition(|(of, _)| names.contains(of));
            dimensions = elsewhere;
            for (of, world) in here {
                let dimension = world.dimension();
                if !shard.add_dimension(&of, world) {
                    tracing::warn!(world = %of, dimension, "not added: an overworld, or a dimension the world has");
                }
            }
            self.templates.iter().for_each(|(name, template, spawn)| shard.add_template(name, template.clone(), *spawn));
            single.take().into_iter().flatten().for_each(|(priority, handler)| shard.add_handler_at(priority, handler));
            self.handlers_each.iter().for_each(|(priority, make)| shard.add_handler_at(*priority, make()));
            self.plugins.iter().for_each(|plugin| shard.add_plugin(plugin.as_ref()));
            self.commands.iter().for_each(|register| register(&mut shard));
            shards.push(shard);
        }
        if self.console {
            shards[entry].add_handler(Box::new(crate::console::start()));
        }
        tracing::info!(addr = %listener.local_addr(), shards = count, "sapling listening");
        let watching = self.ops.watch(&shards, self.shard.crash_reports.clone());
        // Started together: each shard's first tick finds the others' worlds announced.
        Ok(Server { listener, shards: shards.into_iter().map(Shard::spawn).collect(), entry, watching })
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

/// A running server. Dropping it stops the game threads; [`Server::shutdown`] also waits until
/// the players have been told.
pub struct Server {
    listener: Listener,
    shards: Vec<ShardHandle>,
    /// The shard new players join on, which keeps what is kept of every player.
    entry: usize,
    watching: ops::Watching,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr()
    }

    /// How many shards (game threads) the server runs.
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Where the server answers LAN discovery, if it does.
    pub fn lan(&self) -> Option<Advert> {
        self.listener.lan()
    }

    /// Stops the game, disconnects every player and closes the socket.
    pub async fn shutdown(self) {
        let Server { listener, mut shards, entry, watching } = self;
        drop(watching);
        // The entry shard last: the others hand it their players' items as they stop.
        let last = shards.remove(entry);
        // Joining the game threads can take a tick; keep that off the async runtime.
        let _ = tokio::task::spawn_blocking(move || shards.into_iter().chain([last]).for_each(ShardHandle::stop)).await;
        listener.closed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_reaches_the_server_list_and_the_world() {
        let builder = Server::builder().name("lobby").online_mode(false).max_players(7);
        assert_eq!((builder.net.motd.name.as_str(), builder.shard.world_name.as_str()), ("lobby", "lobby"));
        assert_eq!((builder.net.online_mode, builder.net.max_players.get()), (false, 7));
    }
}
