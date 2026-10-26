//! A shard on its own thread: the tick loop, and what happens when a tick panics. The panicking
//! shard saves its worlds and players, disconnects its players, writes a crash report and stops;
//! the other shards and the process go on.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Instant;

use bevy_ecs::prelude::*;

use crate::components::{InLayer, Layer, Settings};
use crate::crash::{self, Report};
use crate::handoff::Peers;
use crate::shard::Shard;
use crate::worlds::Named;

impl Shard {
    /// Runs the shard on its own thread until the handle is stopped or dropped, then closes it.
    pub fn spawn(mut self) -> ShardHandle {
        crash::install_hook();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let index = self.world.resource::<Peers>().index;
        let tick = self.world.resource::<Settings>().0.tick;
        let run = move || {
            let (mut next, mut behind) = (Instant::now(), 0u64);
            while !stopped.load(Ordering::Relaxed) {
                if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(|| self.tick())) {
                    return self.crashed(index, payload.as_ref());
                }
                next += tick;
                match next.checked_duration_since(Instant::now()) {
                    Some(wait) => std::thread::sleep(wait),
                    // Behind by more than a tick: skip the missed ones instead of bursting.
                    None => {
                        next = Instant::now();
                        behind += 1;
                        // A drill that compresses time (drill_server.rs) is invalid after even one.
                        if behind.is_power_of_two() {
                            tracing::warn!(shard = index, times = behind, "shard fell behind its tick");
                        }
                    }
                }
            }
            self.close();
        };
        let thread = std::thread::Builder::new().name(format!("{}{index}", crash::THREAD_PREFIX)).spawn(run).expect("spawning the shard thread");
        ShardHandle { stop, thread: Some(thread) }
    }

    /// After a panic in a tick: saves and closes what can be, then reports.
    fn crashed(&mut self, index: usize, payload: &(dyn Any + Send)) {
        let message = crash::panic_message(payload);
        let (location, backtrace) = crash::take_panic();
        let timing = self.meter.shared();
        let (tick, phase) = (timing.stats().ticks, timing.phase());
        self.meter.abandon();
        tracing::error!(shard = index, tick, ?phase, %message, "the shard panicked: saving its worlds and players and stopping it");
        let worlds = panic::catch_unwind(AssertUnwindSafe(|| describe_worlds(&mut self.world))).unwrap_or_default();
        let saved = match panic::catch_unwind(AssertUnwindSafe(|| self.close())) {
            Ok(()) => "yes, and its players were disconnected".to_owned(),
            Err(again) => format!("no: saving panicked too ({})", crash::panic_message(again.as_ref())),
        };
        let dir = self.world.get_resource::<Settings>().and_then(|settings| settings.0.crash_reports.clone());
        let report = Report { kind: "panic", shard: index, tick, phase, message, location, backtrace, worlds, saved };
        if let Some(path) = report.file(dir.as_deref()) {
            tracing::error!(shard = index, path = %path.display(), "crash report written");
        }
    }
}

/// A line per world of the shard: its name, dimension, loaded columns and players.
fn describe_worlds(world: &mut World) -> Vec<String> {
    let mut players = world.query::<&InLayer>();
    let in_layers: Vec<Entity> = players.iter(world).map(|in_layer| in_layer.0).collect();
    let mut layers = world.query::<(Entity, &Layer, Option<&Named>)>();
    let line = |(entity, layer, name): (Entity, &Layer, Option<&Named>)| {
        let name = name.map_or("?", |name| name.0.as_str());
        let (dimension, columns) = (layer.world.dimension(), layer.world.loaded().count());
        let here = in_layers.iter().filter(|of| **of == entity).count();
        format!("{name} (dimension {dimension}, {columns} columns loaded, {here} players)")
    };
    layers.iter(world).map(line).collect()
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
