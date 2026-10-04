use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::Instant;

use acacia_proto::Packet;
use acacia_proto::packets::PlayerAuthInput;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use sapling_api::{Command, Commands, Handler, Player, PlayerId, Priority};
use sapling_net::{ConnId, Inbox, NetEvent, Outbox, VerifiedLogin};

use crate::blobs::BlobCache;
use crate::commands::{Access, CommandLists};
use crate::components::{Arrivals, Conn, DefaultLayer, Identity, Layer, Mode, Net, NetId, Outgoing, PacketQueues, Players, Position, Settings, Stage, Tick};
use crate::config::{IDS_PER_SHARD, ShardConfig, TICK};
use crate::forms::Forms;
use crate::handoff::{Handoffs, Link, Peers, ShardMsg};
use crate::hud::Hud;
use crate::movement::Motion;
use crate::player::{Carried, Local};
use crate::plugins::{PluginState, Plugins};
use crate::roster::{Elsewhere, Profile};
use crate::view::View;
use crate::visibility::ToViewers;
use crate::worlds::{Atlas, Changes, Scope};
use crate::{arenas, blobs, blocks, chunks, clock, commands, handlers, handoff, join, mobs, movement, visibility, worlds};

/// The disconnect screen of the players of a shard that stops.
const CLOSED: &str = "disconnect.closed";

/// One game thread's state: an ECS world of layers and players, ticked 20 times a second by
/// [`Shard::spawn`] or by hand with [`Shard::tick`] (tests).
pub struct Shard {
    world: World,
    schedule: Schedule,
    /// Runs after the handlers, so what they and this tick's arrivals caused goes out in the tick.
    after_handlers: Schedule,
    inbox: Inbox,
    /// What the other shards tell this one.
    mailbox: Receiver<ShardMsg>,
    plugins: Plugins,
}

impl Shard {
    /// A server's only shard: its players arrive from `inbox` and are answered through `outbox`,
    /// and `level` is its first world.
    pub fn new(cfg: ShardConfig, level: sapling_world::World, inbox: Inbox, outbox: Outbox) -> Self {
        Self::linked(cfg, level, inbox, outbox, Link::alone())
    }

    /// One of a server's shards (see [`Link::mesh`]): `inbox` is the one the network's router
    /// has for this shard's index, `outbox` is shared by all.
    pub fn linked(cfg: ShardConfig, level: sapling_world::World, inbox: Inbox, outbox: Outbox, link: Link) -> Self {
        let mut world = World::new();
        world.insert_resource(Peers::new(link.index, link.peers));
        world.insert_resource(Net(outbox));
        world.insert_resource(Players { next_net_id: link.index as u64 * IDS_PER_SHARD + 1, ..Players::default() });
        world.init_resource::<Atlas>();
        world.init_resource::<Scope>();
        world.init_resource::<Changes>();
        world.init_resource::<Handoffs>();
        world.init_resource::<Elsewhere>();
        world.init_resource::<arenas::Templates>();
        world.init_resource::<Tick>();
        world.init_resource::<Arrivals>();
        world.init_resource::<PacketQueues>();
        world.init_resource::<ToViewers>();
        world.init_resource::<blocks::BlockInputs>();
        let layer = worlds::add(&mut world, &cfg.world_id, level, cfg.spawn).expect("a new shard has no world of that name");
        world.insert_resource(DefaultLayer(layer));
        world.insert_resource(Settings(cfg));

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
        Self { world, schedule, after_handlers, inbox, mailbox: link.mailbox, plugins: Plugins::default() }
    }

    /// Adds a world players can be moved to, with where they appear in it (feet). False for a
    /// name this shard already has or knows of another shard.
    pub fn add_world(&mut self, name: &str, level: sapling_world::World, spawn: [f32; 3]) -> bool {
        worlds::add(&mut self.world, name, level, spawn).is_some()
    }

    /// Adds a template handlers make worlds from (`ShardCtx::create_world`), with where players
    /// appear in such a world (feet).
    pub fn add_template(&mut self, name: &str, template: Arc<sapling_world::Template>, spawn: [f32; 3]) {
        self.world.resource_mut::<arenas::Templates>().add(name, template, spawn);
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

    /// One tick: take what the other shards and the network delivered, run the systems, then
    /// the handlers, then tell players about each other and send what the tick produced.
    pub fn tick(&mut self) {
        self.take_messages();
        while let Ok(event) = self.inbox.try_recv() {
            self.accept(event);
        }
        handoff::expire(&mut self.world).into_iter().for_each(|event| self.accept(event));
        self.schedule.run(&mut self.world);
        handlers::dispatch(&mut self.world, &mut self.plugins);
        self.after_handlers.run(&mut self.world);
        self.world.resource_mut::<PacketQueues>().clear();
        self.world.resource_mut::<Tick>().0 += 1;
    }

    fn take_messages(&mut self) {
        while let Ok(message) = self.mailbox.try_recv() {
            handoff::receive(&mut self.world, message).into_iter().for_each(|event| self.accept(event));
        }
    }

    /// Runs the shard on its own thread until the handle is stopped or dropped, then closes it.
    pub fn spawn(mut self) -> ShardHandle {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let index = self.world.resource::<Peers>().index;
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
            self.close();
        };
        let thread = std::thread::Builder::new().name(format!("sapling-shard-{index}")).spawn(run).expect("spawning the shard thread");
        ShardHandle { stop, thread: Some(thread) }
    }

    /// A stopping shard's last act: the players promised to it are taken in, the forwards it
    /// owes other shards go out, everyone it owns is disconnected and its worlds are saved.
    pub fn close(&mut self) {
        self.take_messages();
        handoff::close(&mut self.world);
        worlds::withdraw(&self.world);
        let net = self.world.resource::<Net>().0.clone();
        handoff::owned(&mut self.world).into_iter().for_each(|conn| net.kick(conn, CLOSED));
        self.save_worlds();
    }

    /// Saves what every world still has unsaved and waits for storage.
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
        let Some(event) = handoff::intercept(&mut self.world, event) else { return };
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
            NetEvent::Rerouted { .. } => {}
        }
    }

    fn add_player(&mut self, conn: ConnId, login: &VerifiedLogin) {
        let layer = self.world.resource::<DefaultLayer>().0;
        let spawn = self.world.get::<Layer>(layer).expect("the default layer lives as long as the shard").spawn;
        let mode = Mode(self.world.resource::<Settings>().0.game_mode);
        let mut players = self.world.resource_mut::<Players>();
        let net_id = players.next_net_id;
        players.next_net_id += 1;
        let identity = &login.identity;
        tracing::debug!(conn = conn.get(), name = %identity.display_name, authenticated = login.authenticated, "login verified");
        let player = Player { id: PlayerId(conn.get()), name: identity.display_name.clone(), uuid: identity.uuid, xuid: identity.xuid.clone() };
        let mut out = Outgoing::default();
        out.push(&join::resource_packs_info());
        let carried = Carried {
            conn: Conn(conn),
            identity: Identity(player),
            net_id: NetId(net_id),
            position: Position { feet: spawn, yaw: 0.0, pitch: 0.0, head_yaw: 0.0, on_ground: true },
            stage: Stage::PacksInfo,
            view: View::default(),
            blobs: BlobCache::default(),
            motion: Motion::default(),
            profile: Profile::new(login, net_id),
            out,
            access: Access::default(),
            mode,
            forms: Forms::default(),
            hud: Hud::default(),
        };
        let entity = self.world.spawn((carried, Local::in_layer(layer))).id();
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

/// A running shard thread. Stopping the last one drops the server's channels, which closes its
/// listener.
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
