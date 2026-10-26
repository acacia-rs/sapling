//! Natural spawning and despawning: in the chunks about players, mobs appear by the vanilla packs'
//! spawn rules (`sapling_data::spawn_rules`) where `doMobSpawning` is on, within the caps (herds.rs),
//! and go again by their kind's despawn rules (despawn.rs); spawn eggs (eggs.rs). BDS's numbers and
//! what is left out: docs/research/mob-spawning.md.

mod despawn;
mod eggs;
mod herds;
mod spots;

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_world::ChunkPos;

use crate::blocks::Dice;
use crate::components::{InLayer, Mode, Position, Stage};
use crate::view::View;

pub(crate) use despawn::tick as despawn;
pub(crate) use eggs::{hatch, kind_of as egg_kind};

const MOB_SPAWNING: &str = "doMobSpawning";
/// The chunks about a player that tick, BDS's default `tick-distance`.
const TICK_RANGE: i32 = 4;
/// Each ticked chunk makes an attempt with this chance in [`ODDS`] a tick.
const CHANCE: u32 = 11;
const ODDS: u32 = 2000;

/// A mob that appeared by itself, at the tick it holds: it despawns (despawn.rs). Spawned by a
/// command, an egg or a plugin, or loaded with its column, a mob stays.
#[derive(Component)]
pub(crate) struct Natural {
    pub born: u64,
}

/// Natural spawning's own dice, so that it leaves the rolls of the rest of the shard as they were.
#[derive(Resource, Default)]
pub(crate) struct SpawnDice(pub Dice);

/// A player that mobs appear around and stay near: one in a world, not a spectator.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Watcher {
    pub layer: Entity,
    pub feet: [f32; 3],
    view: Option<(ChunkPos, i32)>,
}

impl Watcher {
    pub fn distance(&self, at: [f32; 3]) -> f32 {
        (0..3).map(|axis| (self.feet[axis] - at[axis]).powi(2)).sum::<f32>().sqrt()
    }

    /// Whether its view covers the column.
    pub fn sees(&self, column: ChunkPos) -> bool {
        self.view.is_some_and(|(center, radius)| (column.x - center.x).pow(2) + (column.z - center.z).pow(2) <= radius * radius)
    }

    fn chunk(&self) -> ChunkPos {
        ChunkPos::of_block(self.feet[0].floor() as i32, self.feet[2].floor() as i32)
    }
}

pub(crate) fn watchers(world: &mut World) -> Vec<Watcher> {
    let mut players = world.query::<(&InLayer, &Position, &Stage, &Mode, &View)>();
    players
        .iter(world)
        .filter(|(_, _, stage, mode, _)| **stage == Stage::InWorld && mode.0 != GameMode::Spectator)
        .map(|(layer, position, _, _, view)| Watcher { layer: layer.0, feet: position.feet, view: view.center().map(|center| (center, view.radius())) })
        .collect()
}

/// Rolls an attempt in every chunk within [`TICK_RANGE`] of a player of a world where
/// `doMobSpawning` is on.
pub(crate) fn tick(world: &mut World) {
    let watchers = watchers(world);
    let mut ticked = BTreeSet::new();
    for watcher in watchers.iter().filter(|watcher| crate::rules::on(world, watcher.layer, MOB_SPAWNING)) {
        let center = watcher.chunk();
        for (dx, dz) in (-TICK_RANGE..=TICK_RANGE).flat_map(|dx| (-TICK_RANGE..=TICK_RANGE).map(move |dz| (dx, dz))).filter(|(dx, dz)| dx * dx + dz * dz <= TICK_RANGE * TICK_RANGE) {
            ticked.insert((watcher.layer, center.x + dx, center.z + dz));
        }
    }
    for (layer, x, z) in ticked {
        if world.get_resource_or_insert_with(SpawnDice::default).0.roll(ODDS) < CHANCE {
            herds::attempt(world, layer, ChunkPos::new(x, z), &watchers);
        }
    }
}
