//! What the world does on its own, and portal trips, as plugin events: each scene runs without a
//! plugin and with one that cancels everything (veto_events.rs has explosions, spawns, spreads).

use crate::common;

use std::sync::{Arc, Mutex, mpsc};

use acacia_proto::packets::ChangeDimension;
use acacia_proto::types::InputData;
use common::{Harness, all, input_at};
use sapling_api::{
    Block, CropTrampleEvent, GameMode, LeavesDecayEvent, PlayerId, Plugin, PluginBuilder, PortalCreateEvent, PortalUseEvent, Priority, RedstoneUpdateEvent,
    RuleValue, ShardCtx,
};
use sapling_world::World;

const ALEX: PlayerId = PlayerId(1);

#[derive(Clone, Default)]
struct Veto(Arc<Mutex<Vec<String>>>);

impl Veto {
    fn heard(&self, what: &str) -> bool {
        self.0.lock().unwrap().iter().any(|line| line.starts_with(what))
    }
}

macro_rules! veto {
    ($plugin:expr, $log:expr, $($event:ty => $say:expr),*) => {$(
        let log = $log.clone();
        $plugin.on::<$event>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push($say(&*event));
            event.cancel();
        });
    )*};
}

impl Plugin for Veto {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        veto!(plugin, self.0,
            CropTrampleEvent => |e: &CropTrampleEvent| format!("trample {:?}", e.pos()),
            LeavesDecayEvent => |e: &LeavesDecayEvent| format!("decay {}", e.leaves().name),
            PortalCreateEvent => |e: &PortalCreateEvent| format!("portal {}", e.cells().len()),
            PortalUseEvent => |e: &PortalUseEvent| format!("use {:?}", e.portal())
        );
        let log = self.0.clone();
        plugin.on::<RedstoneUpdateEvent>(Priority::Normal, move |_, event| log.lock().unwrap().push(format!("redstone {:?}", event.pos())));
    }
}

fn harness(veto: Option<&Veto>) -> Harness {
    let mut h = Harness::new();
    let mut nether = World::void_in(1);
    let netherrack = nether.block_id("minecraft:netherrack").unwrap();
    nether.fill([-24, 40, -24], [24, 40, 24], netherrack);
    assert!(h.shard.add_dimension("default", nether));
    if let Some(veto) = veto {
        h.shard.add_plugin(veto);
    }
    h.enter(1, "Alex");
    h
}

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ask(ctx)).expect("asked"));
    h.tick_all();
    rx.recv().expect("answered")
}

fn name(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name).unwrap_or_default())
}

fn run(veto: Option<&Veto>, scene: impl Fn(&mut Harness) -> String) -> String {
    scene(&mut harness(veto))
}

#[test]
fn cancelled_trampling_keeps_the_farmland() {
    let scene = |h: &mut Harness| {
        ask(h, |ctx| assert!(ctx.set_block([0, 64, 0], &Block::new("farmland"))));
        for y in [70.0, 68.0, 66.0, 65.0, 64.9375] {
            let mut input = input_at([0.5, y, 0.5]);
            input.input_data = if y < 65.0 { vec![InputData::VerticalCollision] } else { Vec::new() };
            h.receive(1, &input);
            h.tick_all();
        }
        (0..3).for_each(|_| drop(h.tick_all()));
        name(h, [0, 64, 0])
    };
    let veto = Veto::default();
    assert_eq!(run(None, scene), "minecraft:dirt");
    assert_eq!(run(Some(&veto), scene), "minecraft:farmland");
    assert!(veto.heard("trample [0, 64, 0]"));
}

#[test]
fn cancelled_decay_keeps_the_leaves() {
    let scene = |h: &mut Harness| {
        ask(h, |ctx| {
            assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(4096)));
            assert!(ctx.set_block([3, 65, 0], &Block::new("oak_leaves").with("update_bit=1,persistent_bit=0")));
        });
        (0..100).for_each(|_| drop(h.tick_all()));
        name(h, [3, 65, 0])
    };
    let veto = Veto::default();
    assert_eq!(run(None, scene), "minecraft:air");
    assert_eq!(run(Some(&veto), scene), "minecraft:oak_leaves");
    assert!(veto.heard("decay minecraft:oak_leaves"));
}

/// Lights an obsidian frame on the platform (portals.rs) and puts Alex, in creative, in it.
fn through_a_portal(h: &mut Harness) -> String {
    ask(h, |ctx| {
        let frame = (1..=4).flat_map(|x| (64..=68).map(move |y| [x, y, 3])).filter(|[x, y, _]| *x == 1 || *x == 4 || *y == 64 || *y == 68);
        frame.for_each(|at| assert!(ctx.set_block(at, &Block::new("obsidian"))));
    });
    ask(h, |ctx| assert!(ctx.set_block([2, 65, 3], &Block::new("fire"))));
    (0..2).for_each(|_| drop(h.tick_all()));
    ask(h, |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.teleport(ALEX, [2.5, 65.0, 3.5]);
    });
    let travelled = (0..20).any(|_| !all::<ChangeDimension>(h.tick_all().to(1)).is_empty());
    format!("{} {travelled}", name(h, [2, 66, 3]))
}

#[test]
fn a_cancelled_portal_is_not_lit_and_a_cancelled_trip_is_not_made() {
    let veto = Veto::default();
    assert_eq!(run(None, through_a_portal), "minecraft:portal true");
    assert_eq!(run(Some(&veto), through_a_portal), "minecraft:air false");
    assert!(veto.heard("portal 6"));

    // A portal lit, then the trip refused.
    let trips = Veto::default();
    let mut h = harness(None);
    h.shard.add_plugin(&Trips(trips.0.clone()));
    assert_eq!(through_a_portal(&mut h), "minecraft:portal false");
    assert!(trips.heard("use Nether"));
}

struct Trips(Arc<Mutex<Vec<String>>>);

impl Plugin for Trips {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        veto!(plugin, self.0, PortalUseEvent => |e: &PortalUseEvent| format!("use {:?}", e.portal()));
    }
}

#[test]
fn a_redstone_change_is_heard() {
    let veto = Veto::default();
    let mut h = harness(Some(&veto));
    ask(&mut h, |ctx| assert!(ctx.update_block([3, 65, 0], &Block::new("redstone_lamp"))));
    ask(&mut h, |ctx| assert!(ctx.update_block([4, 65, 0], &Block::new("redstone_block"))));
    (0..6).for_each(|_| drop(h.tick_all()));
    assert!(veto.heard("redstone [3, 65, 0]"), "{:?}", veto.0.lock().unwrap());
}
