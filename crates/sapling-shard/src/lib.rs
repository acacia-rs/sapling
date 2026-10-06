//! A shard: one thread, one `bevy_ecs` world, ticking at 20 Hz. It drains the network inbox at the
//! start of each tick, runs its systems in a fixed order, calls plugin handlers, tells players
//! about each other and hands each player's packets to the outbox as one batch. See
//! docs/DESIGN.md "Threads", "Worlds", "Step 3 as built" and "Step 5 as built".

mod arenas;
mod blobs;
mod blocks;
mod body;
mod campfires;
mod chat;
mod chunks;
mod clock;
mod commands;
mod components;
mod config;
mod containers;
mod drops;
mod experience;
mod forms;
mod furnaces;
mod grid;
mod handlers;
mod handoff;
mod hoppers;
mod hud;
mod inventory;
mod join;
mod mobs;
mod movement;
mod orbs;
mod player;
mod plugins;
mod roster;
mod shard;
mod skin;
mod sleep;
mod sync;
mod ticking;
mod view;
mod visibility;
mod worlds;

pub use config::{ShardConfig, TICK};
pub use handoff::Link;
pub use shard::{Shard, ShardHandle};
