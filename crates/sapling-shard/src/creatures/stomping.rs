//! Zombies and their kin going for turtle eggs (ED `behavior.stomp_turtle_egg` on the zombie,
//! husk, drowned, zombie villager and zombie pigman: one tick in 20, eggs within 10 across and 2
//! up or down, within 1.14 is there). There, as BDS showed: it hops on the eggs, 0.3 up every 6
//! ticks, heard each hop, the eggs crumbling every tick, and 48 ticks in stomps the whole
//! block out at once. It gives up a walk of 1200 ticks (JAVA `RemoveBlockGoal`). Only with `mobGriefing` on, and not while it hunts (creatures.rs).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::legs;
use crate::blocks::Dice;
use crate::body::Body;

const KINDS: [&str; 6] = ["minecraft:zombie", "minecraft:husk", "minecraft:drowned", "minecraft:zombie_villager", "minecraft:zombie_villager_v2", "minecraft:zombie_pigman"];
const ODDS: u32 = 20;
const ACROSS: i32 = 10;
const UP: i32 = 2;
const NEAR: f32 = 1.14;
const HOP: f32 = 0.3;
/// It hops, heard, every 6 ticks at the eggs from its third (research/captures/bds-stomping.txt).
const HOP_EVERY: u32 = 6;
const HOP_FIRST: u32 = 2;
/// Ticks at the eggs till they go: 48 and 51 on BDS (research/captures/bds-stomping.txt).
const STOMPS_FOR: u32 = 48;
const GIVES_UP: u32 = 1200;
const GRIEFING: &str = "mobGriefing";

/// The eggs a mob goes for, the ticks it has walked and those it has stomped.
#[derive(Component, Clone, Copy)]
pub(super) struct Stomping {
    eggs: BlockPos,
    walked: u32,
    stomped: u32,
}

fn unique(world: &World, entity: Entity) -> i64 {
    world.get::<crate::tags::UniqueId>(entity).map_or(-1, |id| id.0)
}

fn griefs(world: &World, layer: Entity) -> bool {
    world.get::<crate::components::Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING))
}

/// Before the move of a zombie that does not hunt: it may set off for turtle eggs, walks to them,
/// and hops on them till they are stomped out. False when it is not about eggs.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    if !KINDS.contains(&kind) {
        return false;
    }
    let Some(body) = world.get::<Body>(entity).copied() else { return false };
    let stomping = match world.get::<Stomping>(entity).copied() {
        Some(stomping) => stomping,
        None if griefs(world, layer) && world.get_resource_or_insert_with(Dice::default).roll(ODDS) == 0 => {
            let Some(eggs) = crate::blocks::turtle_egg_near(world, layer, body.at, ACROSS, UP) else { return false };
            Stomping { eggs, walked: 0, stomped: 0 }
        }
        None => return false,
    };
    let there = [stomping.eggs[0] as f32 + 0.5, stomping.eggs[2] as f32 + 0.5];
    let gone = crate::blocks::turtle_egg_near(world, layer, stomping.eggs.map(|c| c as f32), 0, 0).is_none();
    if gone || stomping.walked > GIVES_UP || !griefs(world, layer) {
        world.entity_mut(entity).remove::<Stomping>();
        return false;
    }
    let next = if (body.at[0] - there[0]).hypot(body.at[2] - there[1]) > NEAR {
        let speed = sapling_data::mob_traits(kind).strolls.unwrap_or(0.23);
        legs::walk(world, entity, layer, &body, there, speed, 0.0);
        Stomping { walked: stomping.walked + 1, ..stomping }
    } else {
        legs::rest(world, entity);
        let hops = stomping.stomped % HOP_EVERY == HOP_FIRST;
        if hops
            && body.on_ground
            && let Some(mut body) = world.get_mut::<Body>(entity)
        {
            body.walk([0.0, 0.0], Some(HOP));
        }
        crate::blocks::stomping_turtle_eggs(world, layer, stomping.eggs, kind, unique(world, entity), hops);
        Stomping { stomped: stomping.stomped + 1, ..stomping }
    };
    if next.stomped >= STOMPS_FOR {
        crate::blocks::stomp_turtle_eggs(world, layer, stomping.eggs, kind, unique(world, entity));
        world.entity_mut(entity).remove::<Stomping>();
    } else {
        world.entity_mut(entity).insert(next);
    }
    true
}
