//! Several shards ticked by hand behind the network's router, each with named worlds: a
//! handoff is stepped through message by message.

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{RequestChunkRadius, ResourcePackClientResponseResponseStatus as PackStatus, SetLocalPlayerAsInitialized, Text, TextCategory, TextContent, TextContentChat, TextType};
use acacia_proto::{Packet, RawPacket};
use sapling_api::{ChatEvent, Handler, Player, PlayerId, ShardCtx};
use sapling_net::{ConnId, NetEvent, Outbound, ShardsLoopback};
use sapling_shard::{Link, Shard, ShardConfig};
use sapling_world::World;

use super::{Remote, Sent, config, login, pack_response};

/// A void world with the 16x16 platform of `block` at y 64 around the origin.
pub fn platform_of(block: &str) -> World {
    let mut level = World::void();
    let id = level.block_id(block).unwrap();
    level.fill([-8, 64, -8], [7, 64, 7], id);
    level
}

/// Writes down what a shard's handlers are told, with the shard's index in front.
#[derive(Clone)]
struct Journal {
    shard: usize,
    lines: Arc<Mutex<Vec<String>>>,
}

impl Journal {
    fn note(&self, line: String) {
        self.lines.lock().unwrap().push(format!("{}: {line}", self.shard));
    }
}

impl Handler for Journal {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        self.note(format!("join {} in {}", player.name, ctx.world()));
    }

    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        self.note(format!("quit {} in {}", player.name, ctx.world()));
    }

    fn on_chat(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, event: &mut ChatEvent) {
        self.note(format!("chat {} {} in {}", player.name, event.message, ctx.world()));
    }

    fn on_world_change(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, from: &str, to: &str) {
        self.note(format!("change {} {from} -> {to} ({} here)", player.name, ctx.players_in(to).len()));
    }

    fn on_transfer_failed(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, to: &str) {
        self.note(format!("failed {} {to}, still in {:?}", player.name, ctx.world_of(player.id)));
    }
}

pub struct Cluster {
    /// `None` once stopped.
    pub shards: Vec<Option<Shard>>,
    pub net: ShardsLoopback,
    remotes: Vec<Remote>,
    journal: Arc<Mutex<Vec<String>>>,
    /// Connections a shard disconnected.
    pub kicked: Vec<u64>,
}

impl Cluster {
    /// One shard per list of world names; players join the first world of the first shard,
    /// which is of stone, the others of sandstone.
    pub fn new(layout: &[&[&str]]) -> Self {
        let (net, outbox, inboxes) = sapling_net::loopback_shards(layout.len(), 0);
        let journal = Arc::new(Mutex::new(Vec::new()));
        let (mut shards, mut remotes) = (Vec::new(), Vec::new());
        for ((index, (names, inbox)), link) in layout.iter().zip(inboxes).enumerate().zip(Link::mesh(layout.len(), 0)) {
            let block = |name: &&str| if index == 0 && *name == names[0] { "minecraft:stone" } else { "minecraft:sandstone" };
            // Creative: the tests here break blocks at once, which survival times.
            let cfg = ShardConfig { world_id: names[0].to_owned(), game_mode: sapling_api::GameMode::Creative, ..config() };
            let mut shard = Shard::linked(cfg, platform_of(block(&names[0])), inbox, outbox.clone(), link);
            names[1..].iter().for_each(|name| assert!(shard.add_world(name, platform_of(block(name)), [0.5, 65.0, 0.5])));
            shard.add_handler(Box::new(Journal { shard: index, lines: journal.clone() }));
            let remote = Remote::default();
            shard.add_handler(Box::new(remote.clone()));
            shards.push(Some(shard));
            remotes.push(remote);
        }
        let mut cluster = Self { shards, net, remotes, journal, kicked: Vec::new() };
        // The shards learn of each other's worlds.
        cluster.tick();
        cluster
    }

    pub fn join(&mut self, conn: u64, name: &str) {
        self.net.router.deliver(NetEvent::Joined { conn: ConnId::new(conn), addr: "127.0.0.1:5000".parse().unwrap(), login: login(name) });
    }

    /// Hands a client packet to the shard the router has for the connection; false if none took it.
    pub fn receive<T: Packet>(&mut self, conn: u64, packet: &T) -> bool {
        let packet = RawPacket::parse(sapling_net::encode(packet)).unwrap();
        self.net.router.deliver(NetEvent::Packet { conn: ConnId::new(conn), packet })
    }

    pub fn chat(&mut self, conn: u64, message: &str) {
        let content = TextContent::Chat(TextContentChat { source_name: String::new(), message: message.into() });
        let text = Text { needs_translation: false, category: TextCategory::Authored, r#type: TextType::Chat, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None };
        self.receive(conn, &text);
    }

    pub fn leave(&mut self, conn: u64) {
        self.net.router.deliver(NetEvent::Left { conn: ConnId::new(conn), reason: "test".into() });
    }

    /// The shard the network sends the connection's packets to.
    pub fn owner(&self, conn: u64) -> Option<usize> {
        self.net.router.owner(ConnId::new(conn))
    }

    pub fn tick_shard(&mut self, shard: usize) {
        if let Some(shard) = &mut self.shards[shard] {
            shard.tick();
        }
    }

    /// Carries out the reroutes the shards asked for and returns what they sent, by connection.
    pub fn pump(&mut self) -> Sent {
        let mut sent = Sent::default();
        for command in self.net.pump() {
            match command {
                Outbound::Send { conn, packets } => sent.0.entry(conn.get()).or_default().extend(packets.into_iter().map(|p| RawPacket::parse(p).unwrap())),
                Outbound::Kick { conn, .. } => self.kicked.push(conn.get()),
                Outbound::Route { .. } => unreachable!("the loopback carries reroutes out"),
            }
        }
        sent
    }

    /// Ticks every shard once, in order, with the network catching up after each.
    pub fn tick(&mut self) -> Sent {
        let mut sent = Sent::default();
        for shard in 0..self.shards.len() {
            self.tick_shard(shard);
            for (conn, packets) in self.pump().0 {
                sent.0.entry(conn).or_default().extend(packets);
            }
        }
        sent
    }

    /// Ticks `rounds` times and returns everything sent.
    pub fn ticks(&mut self, rounds: usize) -> Sent {
        let mut sent = Sent::default();
        for _ in 0..rounds {
            for (conn, packets) in self.tick().0 {
                sent.0.entry(conn).or_default().extend(packets);
            }
        }
        sent
    }

    /// Walks `name` through the join into the first shard's first world.
    pub fn enter(&mut self, conn: u64, name: &str) {
        self.join(conn, name);
        self.tick();
        self.receive(conn, &pack_response(PackStatus::HaveAllPacks));
        self.tick();
        self.receive(conn, &pack_response(PackStatus::Completed));
        self.tick();
        self.receive(conn, &RequestChunkRadius { chunk_radius: 4, max_radius: 4 });
        self.ticks(5);
        self.receive(conn, &SetLocalPlayerAsInitialized { runtime_entity_id: 0 });
        self.tick();
    }

    /// Runs `action` in the shard's next tick.
    pub fn run(&self, shard: usize, action: impl FnOnce(&mut ShardCtx<'_>) + Send + 'static) {
        self.remotes[shard].run(action);
    }

    /// Writes what `line` returns in the shard's next tick into the journal.
    pub fn note(&self, shard: usize, line: impl FnOnce(&mut ShardCtx<'_>) -> String + Send + 'static) {
        let journal = Journal { shard, lines: self.journal.clone() };
        self.run(shard, move |ctx| journal.note(line(ctx)));
    }

    /// Asks `shard` in its next tick to move a player; the journal gets what it answered.
    pub fn transfer(&self, shard: usize, conn: u64, world: &'static str) {
        self.note(shard, move |ctx| format!("transfer {conn} to {world}: {}", ctx.transfer(PlayerId(conn), world)));
    }

    /// Notes, for each running shard, who is in which of its worlds.
    pub fn census(&mut self) -> Vec<String> {
        self.journal();
        for shard in (0..self.shards.len()).filter(|shard| self.shards[*shard].is_some()) {
            self.note(shard, |ctx| {
                let mut players: Vec<String> = ctx.players().map(|p| format!("{} in {}", p.name, ctx.world_of(p.id).unwrap())).collect();
                players.sort();
                format!("{} [{}]", ctx.player_count(), players.join(", "))
            });
        }
        self.tick();
        self.journal()
    }

    /// The journal's lines since the last call.
    pub fn journal(&self) -> Vec<String> {
        std::mem::take(&mut *self.journal.lock().unwrap())
    }

    /// Stops a shard as its thread would: closed, then dropped.
    pub fn stop(&mut self, shard: usize) -> Sent {
        let mut stopped = self.shards[shard].take().expect("a running shard");
        stopped.close();
        drop(stopped);
        self.pump()
    }
}
