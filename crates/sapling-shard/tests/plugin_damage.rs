//! What plugins make of harm that is not a hit: what a body's surroundings do to it, and `/kill`.

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{CommandRequest, EntityEvent, EntityEventEventId};
use acacia_proto::types::{CommandOrigin, InputData};
use common::{Harness, all, input_at};
use sapling_api::{Block, Command, DamageCause, DamageEvent, EntityId, Output, PlayerId, Plugin, PluginBuilder, Priority, Sender, ShardCtx, Target};

/// Spares everyone everything, noting what it spared.
#[derive(Clone, Default)]
struct Spare(Arc<Mutex<Vec<(Target, DamageCause)>>>);

impl Spare {
    fn take(&self) -> Vec<(Target, DamageCause)> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

impl Plugin for Spare {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let spared = self.0.clone();
        plugin.on::<DamageEvent>(Priority::Normal, move |_, event| {
            spared.lock().unwrap().push((event.victim().clone(), event.cause()));
            event.cancel();
        });
    }
}

/// Hurt flashes in `sent`, of the entity `id` or of anyone.
fn flashes(sent: &[acacia_proto::RawPacket], id: Option<u64>) -> usize {
    all::<EntityEvent>(sent).iter().filter(|event| id.is_none_or(|id| event.runtime_entity_id == id) && event.event_id == EntityEventEventId::HurtAnimation).count()
}

#[test]
fn a_spared_fall_is_offered_once_and_not_again_each_tick() {
    let spare = Spare::default();
    let mut h = Harness::new();
    h.shard.add_plugin(&spare);
    h.enter(1, "Alex");
    for height in [73.0, 77.0, 73.0, 69.0] {
        h.receive(1, &input_at([0.5, height, 0.5]));
        h.tick_all();
    }
    let mut landing = input_at([0.5, 65.0, 0.5]);
    (landing.input_data, landing.delta.y) = (vec![InputData::VerticalCollision], -1.0);
    h.receive(1, &landing);
    let flashed: usize = (0..20).map(|_| flashes(h.tick_all().to(1), None)).sum();
    let falls = spare.take().into_iter().filter(|(_, cause)| *cause == DamageCause::Fall).count();
    assert_eq!((falls, flashed), (1, 0));
    h.remote.run(|ctx| assert_eq!(ctx.health(PlayerId(1)), Some(20.0)));
    h.tick_all();
}

#[test]
fn a_mob_spared_its_surroundings_is_not_hurt_and_a_spared_kill_kills_nothing() {
    let spare = Spare::default();
    let mut h = Harness::new();
    h.shard.add_plugin(&spare);
    h.enter(1, "Alex");
    let spawned = Arc::new(Mutex::new(None));
    let slot = spawned.clone();
    h.remote.run(move |ctx| {
        assert!(ctx.set_block([6, 65, 0], &Block::new("minecraft:lava")));
        *slot.lock().unwrap() = ctx.spawn_entity("minecraft:pig", [6.5, 65.0, 0.5]);
    });
    h.tick_all();
    let pig: EntityId = spawned.lock().unwrap().expect("a pig");
    let flashed: usize = (0..30).map(|_| flashes(h.tick_all().to(1), Some(pig.0))).sum();
    let causes: Vec<DamageCause> = spare.take().into_iter().filter(|(victim, _)| *victim == Target::Entity(pig)).map(|(_, cause)| cause).collect();
    assert!(causes.contains(&DamageCause::Fire) && causes.contains(&DamageCause::Lava), "{causes:?}");
    assert_eq!(flashed, 0);

    // From a command: what a plugin's own task does is not offered to plugins again.
    h.shard.add_command::<Smite>();
    h.receive(1, &CommandRequest { command: "/smite".into(), origin: CommandOrigin { r#type: "player".into(), uuid: Uuid([1; 16]), request_id: "req".into(), player_entity_id: 0 }, internal: false, version: "latest".into() });
    h.tick_all();
    assert_eq!(*SMITTEN.lock().unwrap(), [false, false], "both kills were spared");
    assert_eq!(spare.take().iter().filter(|(_, cause)| *cause == DamageCause::Generic).count(), 2);
    h.remote.run(move |ctx| assert!(ctx.entity(pig).is_some() && ctx.health(PlayerId(1)) == Some(20.0)));
    h.tick_all();
}

static SMITTEN: Mutex<Vec<bool>> = Mutex::new(Vec::new());

/// Kills every pig and the sender: `/smite`.
#[derive(Command)]
#[command(crate = "sapling_api")]
struct Smite;

impl Command for Smite {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        let pigs: Vec<EntityId> = ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:pig").map(|entity| entity.id).collect();
        let mut smitten = SMITTEN.lock().unwrap();
        smitten.extend(pigs.into_iter().map(|pig| ctx.kill_entity(pig)));
        smitten.push(ctx.kill_player(sender.player().expect("a player").id));
    }
}
