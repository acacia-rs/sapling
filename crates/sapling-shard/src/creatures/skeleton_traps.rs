//! Skeleton horse traps (the pack's `minecraft:skeleton_trap`: `behavior.skeleton_horse_trap`
//! within 10, for 900 s). A trap horse, made by its `minecraft:set_trap` event or a thunderstorm
//! (lightning/storms.rs), springs the tick a player of any mode but spectator is within 10 of it:
//! BDS put four skeleton horses where it stood, each ridden by a skeleton in an iron helmet and
//! pushed off at random, all with their `spring_trap` event, then a bolt there, then took the trap
//! away (research/captures/bds-u1trap.txt, creative and survival alike). Unsprung, it goes after
//! its 900 s. A stored trap keeps its `minecraft:skeleton_trap` group and is a trap again when loaded.
//! Each rider's bow and helmet are enchanted at a cost of 5 plus a draw below 13
//! (`SkeletonHorseTrapGoal` 0x149fee7b0: `5 + special` raised by `nextInt(18 - cost)`, the special
//! multiplier 0 here); BDS's eight helmets all came out with an empty `ench` list, its bows with
//! unbreaking or power 1 or 2.

use acacia_proto::nbt::{List, Value};
use bevy_ecs::prelude::*;
use sapling_api::{GameMode, SpawnReason};
use sapling_world::ItemStack;

use super::birth;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Mode, Position, Tick, in_world};
use crate::entities::{self, Stored};
use crate::mobs;

const HORSE: &str = "minecraft:skeleton_horse";
const RIDER: &str = "minecraft:skeleton";
const SET: &str = "minecraft:set_trap";
const SPRUNG: &str = "minecraft:spring_trap";
const GROUP: &str = "minecraft:skeleton_trap";
const HELMET: &str = "minecraft:iron_helmet";
const BOW: &str = "minecraft:bow";
const REACH: f32 = 10.0;
const LASTS: u64 = 900 * 20;
const RIDERS: usize = 4;
/// The enchanting cost's floor and the bound its draw is raised below.
const COST: (u8, u32) = (5, 18);
/// The most each new horse is pushed along x and z (the Java game's `triangle(0, 1.1485)`; BDS's
/// eight pushes fell within it).
const PUSH: f32 = 1.1485;

/// A trap horse, and the tick it goes unsprung.
#[derive(Component)]
pub(crate) struct Trap {
    until: u64,
}

/// Makes the mob a trap from now where it is a skeleton horse come by `set_trap` or stored as one;
/// a new trap's kept compound gains the group.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str, event: Option<&str>) {
    if kind != HORSE {
        return;
    }
    let kept = world.get::<Stored>(entity).is_some_and(|stored| sapling_world::actor_groups(&stored.compound).iter().any(|group| group == GROUP));
    let set = event.is_some_and(|event| event.trim_start_matches("minecraft:") == SET.trim_start_matches("minecraft:"));
    if set && !kept {
        entities::restore(world, entity, |compound, _| sapling_world::regrouped_actor(compound, &[], &[GROUP]).ok());
    }
    if set || kept {
        let until = world.resource::<Tick>().0 + LASTS;
        world.entity_mut(entity).insert(Trap { until });
    }
}

/// A trap horse with its feet at `feet`, as a thunderstorm's bolt leaves one.
pub(crate) fn set_trap(world: &mut World, layer: Entity, feet: [f32; 3]) {
    birth::spawn_as(world, layer, HORSE, feet, SpawnReason::Lightning, 0.0, Some(SET));
}

fn player_near(world: &World, layer: Entity, at: [f32; 3]) -> bool {
    in_world(world).into_iter().any(|player| {
        let here = world.get::<InLayer>(player).is_some_and(|on| on.0 == layer) && world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Spectator);
        here && world.get::<Position>(player).is_some_and(|position| super::hunting::distance(position.feet, at) <= REACH)
    })
}

/// Springs or ends each trap that is due.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let traps: Vec<(Entity, u64, Entity, [f32; 3])> = world.query::<(Entity, &Trap, &InLayer, &Body)>().iter(world).map(|(horse, trap, on, body)| (horse, trap.until, on.0, body.at)).collect();
    for (horse, until, layer, at) in traps {
        if now >= until {
            mobs::remove(world, horse);
        } else if player_near(world, layer, at) {
            spring(world, horse, layer, at);
        }
    }
}

fn item(name: &str) -> ItemStack {
    ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag: None }
}

/// The rider's helmet and bow, the bow enchanted.
fn gear(world: &mut World) -> [ItemStack; 2] {
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let empty = Value::Compound(vec![("ench".into(), Value::List(List { tag: 0, items: Vec::new() }))]);
    let helmet = ItemStack { tag: Some(empty), ..item(HELMET) };
    let bow_cost = COST.0 + dice.roll(COST.1 - u32::from(COST.0)) as u8;
    let bow = item(BOW);
    let stack = sapling_data::Stack { name: &bow.name, count: 1, damage: 0, block: None, tag: None };
    let made = sapling_data::enchant::table::with_levels(&stack, bow_cost, false, &mut *dice);
    [helmet, ItemStack { tag: made.and_then(|made| made.tag), ..bow }]
}

fn spring(world: &mut World, trap: Entity, layer: Entity, at: [f32; 3]) {
    for _ in 0..RIDERS {
        let Some(horse) = birth::spawn_as(world, layer, HORSE, at, SpawnReason::Summoned, 0.0, Some(SPRUNG)).and_then(|id| entities::find(world, id)) else { continue };
        let push = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            [PUSH * (dice.fraction() - dice.fraction()), 0.0, PUSH * (dice.fraction() - dice.fraction())]
        };
        if let Some(mut body) = world.get_mut::<Body>(horse) {
            body.motion = push;
        }
        let Some(rider) = birth::spawn_as(world, layer, RIDER, at, SpawnReason::Summoned, 0.0, Some(SPRUNG)).and_then(|id| entities::find(world, id)) else { continue };
        let [helmet, bow] = gear(world);
        // Told in its announcement only, once (bds-u1trap.txt), as hold_from_birth reshows it.
        let _ = super::worn::wear(world, rider, 0, Some(helmet));
        super::command_gear::hold_from_birth(world, rider, bow);
        let _ = crate::riding::seat(world, rider, horse);
    }
    crate::lightning::strike(world, layer, at, false);
    mobs::remove(world, trap);
}
