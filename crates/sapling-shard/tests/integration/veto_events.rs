//! Plugins stopping what the world does on its own: explosions, spawns, spreads and growth. Each
//! scene runs once with a plugin that cancels everything, and once without it.

use crate::common;

use std::sync::{Arc, Mutex, mpsc};

use acacia_proto::manual::Uuid;
use acacia_proto::packets::CommandRequest;
use acacia_proto::types::CommandOrigin;
use common::Harness;
use sapling_api::{
    Block, BlockGrowEvent, BlockSpreadEvent, Command, EntitySpawnEvent, ExplosionEvent, Output, Plugin, PluginBuilder, Priority, RuleValue, Sender, ShardCtx,
};

/// Cancels every explosion, spawn, spread and growth, noting each.
#[derive(Clone, Default)]
struct Veto(Arc<Mutex<Vec<String>>>);

impl Veto {
    fn heard(&self, what: &str) -> bool {
        self.0.lock().unwrap().iter().any(|line| line.starts_with(what))
    }
}

impl Plugin for Veto {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let log = self.0.clone();
        plugin.on::<ExplosionEvent>(Priority::Normal, move |ctx, event| {
            log.lock().unwrap().push(format!("explosion {:?} in {}", event.source(), ctx.world()));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<EntitySpawnEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("spawn {} {:?}", event.kind(), event.reason()));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<BlockSpreadEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("spread {}", event.block().name));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<BlockGrowEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("grow {}", event.plant().name));
            event.cancel();
        });
    }
}

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ask(ctx)).expect("asked"));
    h.tick_all();
    rx.recv().expect("answered")
}

fn harness(veto: Option<&Veto>) -> Harness {
    let mut h = Harness::new();
    if let Some(veto) = veto {
        h.shard.add_plugin(veto);
    }
    h.shard.add_command::<Conjure>();
    h.enter(1, "Alex");
    h
}

fn block_name(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name).unwrap_or_default())
}

/// What is left of the cobblestone beside a primed TNT once its fuse is out.
fn beside_a_blast(veto: Option<&Veto>) -> String {
    let mut h = harness(veto);
    ask(&mut h, |ctx| {
        assert!(ctx.set_block([4, 65, 0], &Block::new("cobblestone")));
        assert!(ctx.spawn_entity("minecraft:tnt", [3.5, 65.0, 0.5]).is_some());
    });
    (0..90).for_each(|_| drop(h.tick_all()));
    block_name(&mut h, [4, 65, 0])
}

#[test]
fn a_cancelled_explosion_breaks_nothing() {
    let veto = Veto::default();
    assert_eq!(beside_a_blast(None), "minecraft:air");
    assert_eq!(beside_a_blast(Some(&veto)), "minecraft:cobblestone");
    assert!(veto.heard("explosion Some(\"minecraft:tnt\") in default"), "offered with its world in focus: {:?}", veto.0.lock().unwrap());
}

/// What a walled-in water source runs into once a wall is broken.
fn opened_onto(veto: Option<&Veto>) -> String {
    let mut h = harness(veto);
    ask(&mut h, |ctx| {
        for (at, name) in [([3, 65, 0], "stone"), ([5, 65, 0], "stone"), ([4, 65, 1], "stone"), ([4, 65, -1], "stone"), ([4, 65, 0], "water")] {
            assert!(ctx.set_block(at, &Block::new(name)));
        }
    });
    h.remote.run(|ctx| assert!(ctx.break_block([3, 65, 0])));
    (0..20).for_each(|_| drop(h.tick_all()));
    block_name(&mut h, [3, 65, 0])
}

#[test]
fn a_cancelled_spread_keeps_water_in() {
    let veto = Veto::default();
    assert!(opened_onto(None).contains("water"));
    assert_eq!(opened_onto(Some(&veto)), "minecraft:air");
    assert!(veto.heard("spread minecraft:flowing_water") || veto.heard("spread minecraft:water"), "{:?}", veto.0.lock().unwrap());
}

/// Wheat's growth after random ticks as fast as they go.
fn wheat_after(veto: Option<&Veto>) -> String {
    let mut h = harness(veto);
    ask(&mut h, |ctx| {
        assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(4096)));
        assert!(ctx.update_block([3, 64, 0], &Block::new("farmland")) && ctx.update_block([3, 65, 0], &Block::new("wheat")));
    });
    (0..100).for_each(|_| drop(h.tick_all()));
    ask(&mut h, |ctx| ctx.block([3, 65, 0]).map(|block| block.properties).unwrap_or_default())
}

#[test]
fn cancelled_growth_leaves_a_crop_as_it_was() {
    let veto = Veto::default();
    assert!(!wheat_after(None).contains("growth=0"));
    assert!(wheat_after(Some(&veto)).contains("growth=0"));
    assert!(veto.heard("grow minecraft:wheat"));
}

static CONJURED: Mutex<Vec<bool>> = Mutex::new(Vec::new());

/// Spawns a pig: `/conjure`. From a command, so plugins hear it (a plugin's own doing is not
/// offered to them again).
#[derive(Command)]
#[command(crate = "sapling_api")]
struct Conjure;

impl Command for Conjure {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, _: &mut Output) {
        CONJURED.lock().unwrap().push(ctx.spawn_entity("minecraft:pig", [2.5, 65.0, 0.5]).is_some());
    }
}

#[test]
fn a_cancelled_spawn_brings_no_mob() {
    let veto = Veto::default();
    for veto in [None, Some(&veto)] {
        let mut h = harness(veto);
        let origin = CommandOrigin { r#type: "player".into(), uuid: Uuid([1; 16]), request_id: "req".into(), player_entity_id: 0 };
        h.receive(1, &CommandRequest { command: "/conjure".into(), origin, internal: false, version: "latest".into() });
        h.tick_all();
    }
    assert_eq!(*CONJURED.lock().unwrap(), [true, false]);
    assert!(veto.heard("spawn minecraft:pig Command"));
}
