//! Iron golems a village spawns, as BDS's `Village::tick` (0x142a23bc0) does: with at least
//! max(20, villagers) villagers holding a bed (the claimed POIs' bed slot), three in four of its
//! villagers having worked within a day and fewer golems than a tenth of its villagers, a 1 in 700
//! draw a tick spawns one with `minecraft:from_village` about the village's middle (spawn-around
//! 0x143905ed0: 10 tries, x and z drawn 8 either way, the first standable cell from 6 up to 6 down).
//! Villages are sapling's (raids/village.rs), its villagers those whose village it is
//! (docs/research/villagers.md "Golems").

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, SpawnReason};

use super::Trader;
use super::golems::VillageGolem;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Settings, Tick};
use crate::entities::Kind;

/// Ticks between looks; each look draws for every tick since the last.
const LOOKS: u64 = 20;
const ODDS: u32 = 700;
const LEAST_BEDDED: usize = 20;
/// A villager counts as working if it started work within this many ticks.
const WORKED_WITHIN: u64 = 24_000;
/// How far from the middle golems count.
const REACH: f32 = 48.0;
const TRIES: usize = 10;
const ACROSS: i32 = 8;
const UP: i32 = 6;
const GOLEM: &str = "minecraft:iron_golem";

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// A village: its villagers, how many hold a bed, and how many worked within a day.
#[derive(Default)]
struct Village {
    villagers: usize,
    bedded: usize,
    worked: usize,
}

/// Each village's looks: every [`LOOKS`] ticks, villages that may spawn a golem draw for one.
pub(in crate::creatures) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    if !now.is_multiple_of(LOOKS) || world.resource::<Settings>().0.difficulty == Difficulty::Peaceful {
        return;
    }
    let villagers: Vec<(Entity, [f32; 3], bool, Option<u64>)> = world
        .query::<(&InLayer, &Body, &Trader)>()
        .iter(world)
        .filter(|(_, _, trader)| trader.leaves_at.is_none())
        .map(|(layer, body, trader)| (layer.0, body.at, trader.bed.is_some(), trader.worked))
        .collect();
    let mut villages: HashMap<(Entity, [i32; 3]), Village> = HashMap::new();
    for (layer, at, bedded, worked) in villagers {
        let Some(middle) = super::super::raids::village_centre(world, layer, at) else { continue };
        let village = villages.entry((layer, middle.map(|c| c.floor() as i32))).or_default();
        village.villagers += 1;
        village.bedded += usize::from(bedded);
        village.worked += usize::from(worked.is_some_and(|worked| worked + WORKED_WITHIN > now));
    }
    for ((layer, middle), village) in villages {
        if crate::rules::on(world, layer, "doMobSpawning") && wants_golem(world, layer, middle, &village) && draws(world) {
            spawn(world, layer, middle);
        }
    }
}

/// Whether any of the [`LOOKS`] ticks' 1 in [`ODDS`] draws came up.
fn draws(world: &mut World) -> bool {
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    (0..LOOKS).any(|_| dice.roll(ODDS) == 0)
}

fn wants_golem(world: &mut World, layer: Entity, middle: [i32; 3], village: &Village) -> bool {
    let centre = middle.map(|c| c as f32 + 0.5);
    let golems = world
        .query_filtered::<(&InLayer, &Kind, &Body), With<VillageGolem>>()
        .iter(world)
        .filter(|(on, kind, body)| on.0 == layer && kind.0 == GOLEM && distance(body.at, centre) <= REACH)
        .count();
    village.bedded >= LEAST_BEDDED.max(village.villagers) && village.worked as f32 >= village.villagers as f32 * 0.75 && golems < village.villagers / 10
}

/// A village golem at the first standable cell of [`TRIES`] drawn about `middle`.
fn spawn(world: &mut World, layer: Entity, middle: [i32; 3]) {
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    let cell = world.get::<Layer>(layer).and_then(|kept| {
        (0..TRIES).find_map(|_| {
            let x = middle[0] + dice.roll(2 * ACROSS as u32 + 1) as i32 - ACROSS;
            let z = middle[2] + dice.roll(2 * ACROSS as u32 + 1) as i32 - ACROSS;
            (-UP..=UP).rev().map(|dy| [x, middle[1] + dy, z]).find(|cell| super::super::paths::is_standable(&kept.world, *cell))
        })
    });
    world.insert_resource(dice);
    if let Some([x, y, z]) = cell {
        super::super::spawn_as(world, layer, GOLEM, [x as f32 + 0.5, y as f32, z as f32 + 0.5], SpawnReason::Natural, 0.0, Some("minecraft:from_village"));
    }
}
