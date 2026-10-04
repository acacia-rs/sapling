use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use acacia_proto::Packet;
use acacia_proto::packets::PlayerAuthInput;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use sapling_api::{Command, Commands, Difficulty, GameMode, Handler, Player, PlayerId, Priority};
use sapling_net::{ConnId, Inbox, NetEvent, Outbox, VerifiedLogin};

use crate::blobs::BlobCache;
use crate::commands::{Access, CommandLists};
use crate::components::{Arrivals, Conn, DefaultLayer, Identity, InLayer, Layer, Mode, Net, NetId, Outgoing, PacketQueues, Players, Position, Settings, Stage, Tick};
use crate::forms::Forms;
use crate::hud::Hud;
use crate::movement::Motion;
use crate::plugins::{PluginState, Plugins};
use crate::roster::Profile;
use crate::view::View;
use crate::visibility::{Sees, ToViewers};
use crate::mobs::SeesMobs;
use crate::{blobs, blocks, chunks, clock, commands, handlers, join, mobs, movement, visibility};

pub const TICK: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub struct ShardConfig {
    pub world_name: String,
    pub game_mode: GameMode,
    pub difficulty: Difficulty,
    /// Where players appear (feet), in the shard's first world.
    pub spawn: [f32; 3],
    /// Cap on the view distance clients ask for, in chunks.
    pub max_view_radius: i32,
    /// Columns announced to one player per tick.
    pub columns_per_tick: usize,
    /// Serve chunk data through the client blob cache to clients that report one.
    pub blob_cache: bool,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            world_name: "sapling".into(),
            game_mode: GameMode::Survival,
            difficulty: Difficulty::Peaceful,
            spawn: [0.5, 65.0, 0.5],
            max_view_radius: 8,
            columns_per_tick: 8,
            blob_cache: true,
        }
    }
}

/// One game thread's state: an ECS world of layers and players, ticked 20 times a second by
/// [`Shard::spawn`] or by hand with [`Shard::tick`] (tests).
pub struct Shard {
    world: World,
    schedule: Schedule,
    /// Runs after the handlers, so what they and this tick's arrivals caused goes out in the tick.
    after_handlers: Schedule,
    inbox: Inbox,
    plugins: Plugins,
}

impl Shard {
    /// A shard whose players arrive from `inbox` and are answered through `outbox`, with `level`
    /// as its first world.
    pub fn new(cfg: ShardConfig, level: sapling_world::World, inbox: Inbox, outbox: Outbox) -> Self {
        let mut world = World::new();
        let layer = world.spawn(Layer { world: level, spawn: cfg.spawn }).id();
        world.insert_resource(DefaultLayer(layer));
        world.insert_resource(Settings(cfg));
        world.insert_resource(Net(outbox));
        world.insert_resource(Players { next_net_id: 1, ..Players::default() });
        world.init_resource::<Tick>();
        world.init_resource::<Arrivals>();
        world.init_resource::<PacketQueues>();
        world.init_resource::<ToViewers>();
        world.init_resource::<blocks::BlockInputs>();

        let mut schedule = Schedule::default();
        // Shards scale across threads; inside one, systems run in this order every tick.
        schedule.set_executor(SingleThreadedExecutor::new());
        let join = (join::resource_packs, join::chunk_radius, blobs::cache_status).chain();
        let chunks = (
            chunks::poll_worlds,
            mobs::spawn_arrived,
            movement::player_input,
            chunks::send_chunks,
            chunks::sub_chunk_requests,
            blobs::blob_status,
            chunks::save_and_unload_columns,
            mobs::despawn_unloaded,
        );
        schedule.add_systems((join, clock::advance, chunks.chain(), join::initialized).chain());
        let mut after_handlers = Schedule::default();
        after_handlers.set_executor(SingleThreadedExecutor::new());
        after_handlers.add_systems((visibility::update, mobs::update, movement::announce, movement::relay_swings, visibility::fan_out, commands::announce, flush).chain());
        world.insert_resource(CommandLists::new(&Commands::default()));
        world.init_resource::<PluginState>();
        Self { world, schedule, after_handlers, inbox, plugins: Plugins::default() }
    }

    /// Adds a handler at [`Priority::Normal`].
    pub fn add_handler(&mut self, handler: Box<dyn Handler>) {
        self.add_handler_at(Priority::Normal, handler);
    }

    /// Adds a handler; see [`Priority`] for when it runs.
    pub fn add_handler_at(&mut self, priority: Priority, handler: Box<dyn Handler>) {
        self.plugins.add(priority, handler);
    }

    /// Adds a command (see [`sapling_api::command`]). Players already in the world keep the list
    /// they have until their permission changes.
    pub fn add_command<C: Command>(&mut self) {
        self.plugins.commands.register::<C>();
        self.world.insert_resource(CommandLists::new(&self.plugins.commands));
    }

    /// One tick: take everything the network delivered, run the systems, then the handlers, then
    /// tell players about each other and send what the tick produced.
    pub fn tick(&mut self) {
        while let Ok(event) = self.inbox.try_recv() {
            self.accept(event);
        }
        self.schedule.run(&mut self.world);
        handlers::dispatch(&mut self.world, &mut self.plugins);
        self.after_handlers.run(&mut self.world);
        self.world.resource_mut::<PacketQueues>().clear();
        self.world.resource_mut::<Tick>().0 += 1;
    }

    /// Runs the shard on its own thread until the handle is stopped or dropped, then saves its worlds.
    pub fn spawn(mut self) -> ShardHandle {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let run = move || {
            let mut next = Instant::now();
            while !stopped.load(Ordering::Relaxed) {
                self.tick();
                next += TICK;
                match next.checked_duration_since(Instant::now()) {
                    Some(wait) => std::thread::sleep(wait),
                    // Behind by more than a tick: skip the missed ones instead of bursting.
                    None => next = Instant::now(),
                }
            }
            self.save_worlds();
        };
        let thread = std::thread::Builder::new().name("sapling-shard".into()).spawn(run).expect("spawning the shard thread");
        ShardHandle { stop, thread: Some(thread) }
    }

    /// Saves what every world still has unsaved and waits for storage: a stopping shard's last act.
    pub fn save_worlds(&mut self) {
        let mut layers = self.world.query::<&mut Layer>();
        for mut layer in layers.iter_mut(&mut self.world) {
            let failed = layer.world.save_and_wait();
            if failed > 0 {
                tracing::error!(failed, "columns could not be saved and are lost");
            }
        }
    }

    fn accept(&mut self, event: NetEvent) {
        match event {
            NetEvent::Joined { conn, login, .. } => self.add_player(conn, &login),
            NetEvent::Packet { conn, packet } => {
                if let Some(&entity) = self.world.resource::<Players>().by_conn.get(&conn) {
                    if packet.id != PlayerAuthInput::ID {
                        tracing::debug!(conn = conn.get(), id = packet.id, len = packet.body.len(), stage = ?self.world.get::<Stage>(entity), "packet received");
                    }
                    self.world.resource_mut::<PacketQueues>().push(entity, packet);
                }
            }
            NetEvent::Left { conn, reason } => {
                if let Some(entity) = self.world.resource_mut::<Players>().by_conn.remove(&conn) {
                    tracing::debug!(conn = conn.get(), %reason, stage = ?self.world.get::<Stage>(entity), "connection ended");
                    self.world.resource_mut::<Arrivals>().left.push(entity);
                }
            }
        }
    }

    fn add_player(&mut self, conn: ConnId, login: &VerifiedLogin) {
        let layer = self.world.resource::<DefaultLayer>().0;
        let spawn = self.world.get::<Layer>(layer).expect("the default layer lives as long as the shard").spawn;
        let api = (Access::default(), Mode(self.world.resource::<Settings>().0.game_mode), Forms::default(), Hud::default());
        let mut players = self.world.resource_mut::<Players>();
        let net_id = players.next_net_id;
        players.next_net_id += 1;
        let identity = &login.identity;
        tracing::debug!(conn = conn.get(), name = %identity.display_name, authenticated = login.authenticated, "login verified");
        let player = Player { id: PlayerId(conn.get()), name: identity.display_name.clone(), uuid: identity.uuid, xuid: identity.xuid.clone() };
        let mut out = Outgoing::default();
        out.push(&join::resource_packs_info());
        let components = (
            Conn(conn),
            Identity(player),
            NetId(net_id),
            Position { feet: spawn, yaw: 0.0, pitch: 0.0, head_yaw: 0.0, on_ground: true },
            InLayer(layer),
            Stage::PacksInfo,
            View::default(),
            BlobCache::default(),
            chunks::SectionBacklog::default(),
            Motion::default(),
            Sees::default(),
            SeesMobs::default(),
            Profile::new(login, net_id),
            out,
        );
        let entity = self.world.spawn((components, api)).id();
        self.world.resource_mut::<Players>().by_conn.insert(conn, entity);
    }
}

/// Hands each player's packets of this tick to the network as one batch.
fn flush(net: Res<Net>, mut players: Query<(&Conn, &mut Outgoing)>) {
    for (conn, mut out) in &mut players {
        if !out.0.is_empty() {
            net.0.send(conn.0, std::mem::take(&mut out.0));
        }
    }
}

/// A running shard thread. Stopping it drops the shard's channels, which closes its listener.
pub struct ShardHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ShardHandle {
    /// Stops after the current tick and waits for the thread.
    pub fn stop(mut self) {
        self.halt();
    }

    fn halt(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for ShardHandle {
    fn drop(&mut self) {
        self.halt();
    }
}
