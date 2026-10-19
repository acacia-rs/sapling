//! Plugins on a shard: listeners and tasks for a player or a world, dropped when it is gone.

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{MovePlayer, Respawn, SetEntityMotion, Text, UpdateAttributes};
use acacia_proto::types::{TransactionTransactionData, TransactionTransactionDataItemUseOnEntityActionType as OnEntity, Vec3f};
use common::cluster::platform_of;
use common::{Harness, all, attack, input_at};
use sapling_api::{
    Attacker, DamageEvent, DeathEvent, DeathMessage, EntityAttackEvent, EntityId, EntityInteractEvent, JoinEvent, MoveEvent, Plugin, PluginBuilder, Priority, QuitEvent,
    RespawnEvent, Target,
};
use sapling_world::Template;

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn push(&self, line: impl Into<String>) {
        self.0.lock().unwrap().push(line.into());
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

/// Gives every joining player a task of their own and hears them quit.
struct Watch(Log);

impl Plugin for Watch {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let log = self.0.clone();
        plugin.on::<JoinEvent>(Priority::Normal, move |ctx, event| {
            let (id, name) = (event.player().id, event.player().name.clone());
            let mut scoped = ctx.for_player(id).expect("a joining player is on the shard");
            let ticks = log.clone();
            scoped.every(5, move |_| ticks.push(format!("tick {name}")));
            let quits = log.clone();
            scoped.on::<QuitEvent>(Priority::Normal, move |_, event| quits.push(format!("quit {}", event.player().name)));
        });
    }
}

#[test]
fn a_players_task_runs_until_they_leave_and_their_listener_hears_the_quit() {
    let log = Log::default();
    let mut h = Harness::new();
    h.shard.add_plugin(&Watch(log.clone()));
    h.enter(1, "Steve");
    (0..11).for_each(|_| drop(h.tick(1)));
    assert_eq!(log.take(), ["tick Steve", "tick Steve"]);

    h.leave(1);
    (0..11).for_each(|_| h.shard.tick());
    assert_eq!(log.take(), ["quit Steve"]);
}

/// Keeps players west of x 5 and hits off mobs, and says what it saw.
struct Fence(Log);

impl Plugin for Fence {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let log = self.0.clone();
        plugin.on::<MoveEvent>(Priority::Normal, move |_, event| {
            log.push(format!("move to {} {}", event.to().feet[0], event.changed_block()));
            if event.to().feet[0] > 5.0 {
                event.cancel();
            }
        });
        let log = self.0.clone();
        plugin.on::<EntityAttackEvent>(Priority::Normal, move |_, event| {
            log.push(format!("attack {:?}", event.target()));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<EntityInteractEvent>(Priority::Normal, move |_, event| log.push(format!("use {:?}", event.target())));
    }
}

#[test]
fn a_cancelled_move_puts_the_player_back() {
    let log = Log::default();
    let mut h = Harness::new();
    h.shard.add_plugin(&Fence(log.clone()));
    h.enter(1, "Steve");
    h.receive(1, &input_at([3.5, 65.0, 0.5]));
    assert!(all::<MovePlayer>(&h.tick(1)).is_empty());
    h.receive(1, &input_at([3.7, 65.0, 0.5]));
    h.tick(1);
    assert_eq!(log.take(), ["move to 3.5 true", "move to 3.7 false"]);

    h.receive(1, &input_at([8.5, 65.0, 0.5]));
    let back = all::<MovePlayer>(&h.tick(1));
    assert_eq!(log.take(), ["move to 8.5 true"]);
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].position.x, 3.7, "teleported to where it stood");
}

#[test]
fn plugins_hear_entity_clicks_and_may_stop_a_hit() {
    let log = Log::default();
    let mut h = Harness::new();
    h.shard.add_plugin(&Fence(log.clone()));
    h.enter(1, "Steve");
    let spawned = Arc::new(Mutex::new(None));
    let slot = spawned.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.spawn_entity("minecraft:pig", [2.5, 65.0, 0.5]));
    h.tick(1);
    let pig: EntityId = spawned.lock().unwrap().expect("a pig");

    h.receive(1, &attack(pig.0));
    let mut interact = attack(pig.0);
    if let TransactionTransactionData::ItemUseOnEntity(on) = &mut interact.transaction.transaction_data {
        on.action_type = OnEntity::Interact;
    }
    h.receive(1, &interact);
    h.receive(1, &attack(999_999));
    h.tick(1);
    let target = format!("{:?}", Target::Entity(pig));
    assert_eq!(log.take(), [format!("attack {target}"), format!("use {target}")], "nothing for an id that names nothing");
    h.remote.run(move |ctx| assert!(ctx.entity(pig).is_some(), "the cancelled hit left the pig standing"));
    h.tick(1);
}

/// Alex (1) and Steve (2) on easy, where Alex's fist costs Steve 1 (attacks.rs).
fn duel(plugin: &dyn Plugin) -> Harness {
    let mut h = Harness::new();
    h.shard.add_plugin(plugin);
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| ctx.set_difficulty(sapling_api::Difficulty::Easy));
    h.tick_all();
    h
}

fn health(sent: &[acacia_proto::RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

/// Spares everyone everything.
struct Truce;

impl Plugin for Truce {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        plugin.on::<DamageEvent>(Priority::Normal, |_, event| event.cancel());
    }
}

#[test]
fn a_cancelled_hit_neither_hurts_nor_throws() {
    let mut h = duel(&Truce);
    h.receive(1, &attack(2));
    let sent = h.tick_all().to(2).to_vec();
    assert!(health(&sent).is_empty() && all::<SetEntityMotion>(&sent).is_empty());
}

/// Makes every hit kill, says deaths its own way, keeps what the dead held and raises them at x 5.
struct Sudden(Log);

impl Plugin for Sudden {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let log = self.0.clone();
        plugin.on::<DamageEvent>(Priority::Normal, move |_, event| {
            let by = match event.attacker() {
                Some(Attacker::Player(player)) => player.name.clone(),
                other => format!("{other:?}"),
            };
            log.push(format!("{by} hurts {:?} for {}", event.victim(), event.amount));
            event.amount = 100.0;
        });
        let log = self.0.clone();
        plugin.on::<DeathEvent>(Priority::Normal, move |_, event| {
            log.push(format!("{} died of {:?}", event.player().name, event.cause()));
            event.keep_inventory = true;
            event.message = DeathMessage::Text(format!("{} is out", event.player().name));
        });
        plugin.on::<RespawnEvent>(Priority::Normal, |_, event| event.feet = [5.5, 65.0, 0.5]);
    }
}

#[test]
fn plugins_set_a_hits_harm_the_death_message_and_where_the_dead_rise() {
    let log = Log::default();
    let mut h = duel(&Sudden(log.clone()));
    h.receive(1, &attack(2));
    let sent = h.tick_all();
    let lines = log.take();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("Alex hurts Player(Player {") && lines[0].ends_with("for 1"), "{lines:?}");
    assert_eq!(lines[1], "Steve died of Attack");
    assert_eq!(health(sent.to(2)), [0.0]);
    let told = |conn: u64| all::<Text>(sent.to(conn)).iter().any(|text| format!("{:?}", text.content).contains("Steve is out"));
    assert!(told(1) && told(2), "everyone in the world reads the plugin's words");

    h.receive(2, &Respawn { position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
    let risen = all::<Respawn>(h.tick_all().to(2));
    assert_eq!(risen.iter().filter(|respawn| respawn.state == 1).map(|respawn| respawn.position.x).collect::<Vec<_>>(), [5.5]);
}

#[test]
fn a_worlds_task_runs_in_it_until_the_world_is_removed() {
    let log = Log::default();
    let mut h = Harness::new();
    h.shard.add_template("map", Template::of(platform_of("minecraft:sandstone")), [0.5, 65.0, 0.5]);
    h.enter(1, "Steve");
    let in_arena = log.clone();
    h.remote.run(move |ctx| {
        assert!(ctx.create_world("arena", "map"));
        let mut arena = ctx.in_world("arena").expect("made on this shard");
        arena.every(1, move |ctx| in_arena.push(format!("{} {:?}", ctx.world(), ctx.block([0, 64, 0]).map(|b| b.name))));
    });
    (0..3).for_each(|_| drop(h.tick(1)));
    let lines = log.take();
    assert!(!lines.is_empty() && lines.iter().all(|line| line == r#"arena Some("minecraft:sandstone")"#), "{lines:?}");

    h.remote.run(|ctx| assert!(ctx.remove_world("arena")));
    (0..3).for_each(|_| drop(h.tick(1)));
    log.take();
    (0..3).for_each(|_| drop(h.tick(1)));
    assert!(log.take().is_empty(), "the task went with its world");
}
