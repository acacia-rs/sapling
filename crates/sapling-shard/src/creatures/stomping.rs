//! Zombies and their kin going for turtle eggs (ED `behavior.stomp_turtle_egg` on the zombie, husk, drowned,
//! zombie villager and zombie pigman), as BDS's `StompEggGoal` (Linux ELF: findTargetBlock 0x533bc10, tick
//! 0x533bf30, read under gdb in bds-ay2gdb.txt; docs/research/leftovers.md "Stomping"): one tick in 20 it
//! picks any turtle egg within 10 across and 2 up or down at random and walks there. Its target is the egg
//! cell's corner one up; it has reached it within 1.14 of its box's middle height, and from then a count
//! runs every tick. Standing in the egg cell (or the one above) it crumbles the eggs while the count is over
//! 0, hops 0.3 up on the ground and is heard every 6th count, is pushed 0.3 down 3 counts after, and stomps
//! the whole block out from count 61. Beside the cell it walks on, stepping up onto the eggs (body/step.rs). It gives up a walk of 1200 ticks (JAVA `RemoveBlockGoal`). Only with
//! `mobGriefing` on, and not while it hunts (creatures.rs).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::legs;
use crate::blocks::Dice;
use crate::body::Body;

const KINDS: [&str; 6] = ["minecraft:zombie", "minecraft:husk", "minecraft:drowned", "minecraft:zombie_villager", "minecraft:zombie_villager_v2", "minecraft:zombie_pigman"];
const ODDS: u32 = 20;
const ACROSS: i32 = 10;
const UP: i32 = 2;
/// Its goal radius (1.14), squared, as BDS holds it.
const REACH_SQ: f32 = 1.2996;
const HOP: f32 = 0.3;
const HOP_EVERY: u32 = 6;
const PUSHED_DOWN: u32 = 3;
const STOMPED_AFTER: u32 = 60;
const GIVES_UP: u32 = 1200;
const GRIEFING: &str = "mobGriefing";

/// The eggs a mob goes for, the ticks it has walked, whether it is there, and its count from first getting there.
#[derive(Component, Clone, Copy)]
pub(super) struct Stomping {
    eggs: BlockPos,
    walked: u32,
    reached: bool,
    count: u32,
}

fn unique(world: &World, entity: Entity) -> i64 {
    world.get::<crate::tags::UniqueId>(entity).map_or(-1, |id| id.0)
}

fn griefs(world: &World, layer: Entity) -> bool {
    world.get::<crate::components::Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING))
}

/// Whether a mob whose count runs is still at the eggs, or has come to them: BDS keeps the reach while its
/// block corner is near the target across, else measures again from its box's middle height.
fn reached(body: &Body, target: [f32; 3], counting: bool) -> bool {
    let corner = body.at.map(|c| c.floor());
    let across = (target[0] - corner[0]).powi(2) + (target[2] - corner[2]).powi(2);
    if counting && across <= REACH_SQ {
        return true;
    }
    let middle = [body.at[0], body.at[1] + body.size[1] * 0.5, body.at[2]];
    (0..3).map(|axis| (target[axis] - middle[axis]).powi(2)).sum::<f32>() <= REACH_SQ
}

fn in_egg_cell(body: &Body, [x, y, z]: BlockPos) -> bool {
    let [fx, fy, fz] = body.at.map(|c| c.floor() as i32);
    fx == x && fz == z && (fy == y || fy == y + 1)
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
            let near = crate::blocks::turtle_eggs_near(world, layer, body.at, ACROSS, UP);
            if near.is_empty() {
                return false;
            }
            let eggs = near[world.resource_mut::<Dice>().roll(near.len() as u32) as usize];
            Stomping { eggs, walked: 0, reached: false, count: 0 }
        }
        None => return false,
    };
    let gone = crate::blocks::turtle_egg_near(world, layer, stomping.eggs.map(|c| c as f32), 0, 0).is_none();
    if gone || stomping.walked > GIVES_UP || !griefs(world, layer) {
        world.entity_mut(entity).remove::<Stomping>();
        return false;
    }
    let [x, y, z] = stomping.eggs;
    let middle = [x as f32 + 0.5, z as f32 + 0.5];
    let speed = sapling_data::mob_traits(kind).strolls.unwrap_or(0.23);
    if !reached(&body, [x as f32, y as f32 + 1.0, z as f32], stomping.reached) {
        legs::walk(world, entity, layer, &body, middle, speed, 0.0);
        world.entity_mut(entity).insert(Stomping { walked: stomping.walked + 1, reached: false, ..stomping });
        return true;
    }
    let count = stomping.count;
    if !in_egg_cell(&body, stomping.eggs) {
        legs::walk(world, entity, layer, &body, middle, speed, 0.0);
        world.entity_mut(entity).insert(Stomping { reached: true, count: count + 1, ..stomping });
        return true;
    }
    legs::rest(world, entity);
    let rise = match count % HOP_EVERY {
        PUSHED_DOWN => Some(-HOP),
        0 if body.on_ground => Some(HOP),
        _ => None,
    };
    if let Some(rise) = rise
        && let Some(mut body) = world.get_mut::<Body>(entity)
    {
        (body.motion[0], body.motion[2]) = (0.0, 0.0);
        body.walk([0.0, 0.0], Some(rise));
    }
    let unique = unique(world, entity);
    crate::blocks::stomping_turtle_eggs(world, layer, stomping.eggs, kind, unique, count > 0, count % HOP_EVERY == 0);
    if count > STOMPED_AFTER {
        crate::blocks::stomp_turtle_eggs(world, layer, stomping.eggs, kind, unique);
        world.entity_mut(entity).remove::<Stomping>();
    } else {
        world.entity_mut(entity).insert(Stomping { reached: true, count: count + 1, ..stomping });
    }
    true
}
