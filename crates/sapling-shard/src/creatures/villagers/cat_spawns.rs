//! Cats a village brings, as BDS's `Village::tick` (0x142a23bc0) does every 1200 ticks from the
//! village's first tick: with fewer than 5 cats in the box 48 about its middle and fewer than a
//! quarter of its villagers, a cat `minecraft:spawn_from_village` (or, one time in two on a day
//! that is a multiple of 8, `minecraft:spawn_midnight_cat`) about the middle as a golem comes
//! (golem_spawns.rs); the village event's baby one time in four.

use std::collections::HashMap;

use bevy_ecs::prelude::*;

use super::golem_spawns::{spawn_around, villages};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Tick};
use crate::entities::{self, Kind};

const EVERY: u64 = 1200;
const LOOKS: u64 = 20;
const REACH: f32 = 48.0;
const MOST: usize = 5;
const VILLAGERS_A_CAT: usize = 4;
const CAT: &str = "minecraft:cat";
const DAY: i64 = 24_000;
const MIDNIGHT_EVERY: i64 = 8;
/// `spawn_from_village`'s `spawn_wild_baby` weight 1 of 4.
const BABY_ODDS: u32 = 4;

/// Each village's next look, by world and middle.
#[derive(Resource, Default)]
struct Looks(HashMap<(Entity, [i32; 3]), u64>);

/// Every 20 ticks, as the golems' looks: each village whose time came may bring a cat.
pub(in crate::creatures) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    if !now.is_multiple_of(LOOKS) {
        return;
    }
    let mut looks = world.remove_resource::<Looks>().unwrap_or_default();
    let found = villages(world);
    looks.0.retain(|village, _| found.contains_key(village));
    for ((layer, middle), village) in found {
        let next = looks.0.entry((layer, middle)).or_insert(now);
        if now < *next {
            continue;
        }
        *next = now + EVERY;
        if crate::rules::on(world, layer, "doMobSpawning") && cats_near(world, layer, middle) < (village.villagers / VILLAGERS_A_CAT).min(MOST) {
            bring(world, layer, middle);
        }
    }
    world.insert_resource(looks);
}

fn cats_near(world: &mut World, layer: Entity, middle: [i32; 3]) -> usize {
    let within = |at: [f32; 3]| (0..3).all(|axis| (at[axis] - middle[axis] as f32).abs() <= REACH);
    world.query::<(&InLayer, &Kind, &Body)>().iter(world).filter(|(on, kind, body)| on.0 == layer && kind.0 == CAT && within(body.at)).count()
}

fn bring(world: &mut World, layer: Entity, middle: [i32; 3]) {
    let day = world.get::<Layer>(layer).map_or(1, |kept| kept.world.level().time.div_euclid(DAY));
    let (midnight, baby) = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (day.rem_euclid(MIDNIGHT_EVERY) == 0 && dice.roll(2) == 0, dice.roll(BABY_ODDS) == 0)
    };
    let event = if midnight { "minecraft:spawn_midnight_cat" } else { "minecraft:spawn_from_village" };
    let Some(cat) = spawn_around(world, layer, middle, CAT, event).and_then(|id| entities::find(world, id)) else { return };
    if baby && !midnight {
        super::super::husbandry::be_born(world, cat, CAT);
    }
}
