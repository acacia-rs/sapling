//! Plugins on a shard: listeners and tasks for a player or a world, dropped when it is gone.

mod common;

use std::sync::{Arc, Mutex};

use common::Harness;
use common::cluster::platform_of;
use sapling_api::{JoinEvent, Plugin, PluginBuilder, Priority, QuitEvent};
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
