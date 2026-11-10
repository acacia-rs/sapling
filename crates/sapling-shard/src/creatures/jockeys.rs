//! Mobs riding mobs, as BDS's were (research/captures/bds-j3ride.txt): a seated hunter takes prey and
//! hits it from its seat as one on foot; its mount walks at that prey at its own `movement` times its
//! pack's `mount_pathing` pace (a chicken under a baby zombie 1.5), or, under a rider holding a spear,
//! the rider's `use_kinetic_weapon` pace (`hijack_mount_navigation`: a zombie horse under its zombie 1.4),
//! through its prey and on to the rider's reposition spot (hunting/strike/lances.rs).

use bevy_ecs::prelude::*;

use super::{Creature, hunting, shown, strides::Strode};
use crate::body::Body;
use crate::entities::Kind;

/// Each mount's `behavior.mount_pathing` speed multiplier (the 1.26.52 packs).
const PATHING: &[(&str, f32)] = &[
    ("minecraft:camel_husk", 4.0),
    ("minecraft:cat", 1.25),
    ("minecraft:cave_spider", 1.25),
    ("minecraft:chicken", 1.5),
    ("minecraft:cow", 1.5),
    ("minecraft:donkey", 1.5),
    ("minecraft:horse", 1.5),
    ("minecraft:husk", 1.25),
    ("minecraft:llama", 1.25),
    ("minecraft:mooshroom", 1.5),
    ("minecraft:mule", 1.5),
    ("minecraft:ocelot", 1.25),
    ("minecraft:panda", 1.5),
    ("minecraft:pig", 1.25),
    ("minecraft:ravager", 1.25),
    ("minecraft:sheep", 1.5),
    ("minecraft:skeleton_horse", 1.5),
    ("minecraft:spider", 1.25),
    ("minecraft:trader_llama", 1.25),
    ("minecraft:wolf", 1.25),
    ("minecraft:zombie", 1.25),
    ("minecraft:zombie_horse", 1.5),
    ("minecraft:zombie_nautilus", 1.0),
    ("minecraft:zombie_pigman", 1.25),
    ("minecraft:zombie_villager", 1.25),
    ("minecraft:zombie_villager_v2", 1.25),
];
/// A spear rider's kind, the mount its `use_kinetic_weapon` group rides, and that group's pace.
const HIJACKS: &[(&str, &str, f32)] = &[("minecraft:zombie", "minecraft:zombie_horse", 1.4), ("minecraft:husk", "minecraft:camel_husk", 4.0)];

/// A seated mob's thinking: a hunter keeps or takes its prey and hits it within reach.
pub(super) fn think(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if world.get::<hunting::Hunter>(entity).is_none() {
        return;
    }
    let prey = hunting::pick(world, entity, layer, id, kind, body);
    hunting::lances::plan(world, entity, kind, body.at, prey);
    hunting::strike(world, entity, layer, id, kind, body);
}

/// Where and how fast a mount of `kind` that the mob `rider` rides walks: at the rider's prey, if
/// it has any.
pub(crate) fn rein(world: &World, mount: Entity, kind: &str, rider: Entity) -> Option<([f32; 2], f32)> {
    world.get::<Creature>(rider)?;
    let prey = hunting::lances::aim(world, rider).or_else(|| hunting::prey_at(world, rider))?;
    let rider_kind = world.get::<Kind>(rider)?.0.as_str();
    let spear = super::command_gear::main_hand(world, rider).is_some_and(|item| item.name.ends_with("_spear"));
    let hijack = HIJACKS.iter().find(|(of, under, _)| spear && *of == rider_kind && *under == kind).map(|(.., pace)| *pace);
    let pace = hijack.or_else(|| PATHING.iter().find(|(of, _)| *of == kind).map(|(_, pace)| *pace))?;
    let width = world.get::<Body>(mount).map_or(0.0, |body| body.size[0]);
    let to = if hijack.is_some() && width > 1.0 { [node_middle(prey[0], width), node_middle(prey[2], width)] } else { [prey[0], prey[2]] };
    Some((to, movement(world, mount, kind)? * pace))
}

/// Where a mob `width` wide stands on the path node it would take for `at`: the node is the cell of
/// its box's low corner, its stand the node plus half its whole width plus one. BDS's zombie horses
/// (1.4 wide) ran at x 0.0 for an actor at x 0.5 (research/captures/bds-am2ride.txt).
fn node_middle(at: f32, width: f32) -> f32 {
    (at - width / 2.0).floor() + (width + 1.0).trunc() * 0.5
}

/// The mob's `movement`: its strides' if it rolled them, else its kind's.
pub(super) fn movement(world: &World, mob: Entity, kind: &str) -> Option<f32> {
    world.get::<Strode>(mob).map(|strode| strode.0.movement).or_else(|| sapling_data::mob_pace(kind, shown::is_baby(world, mob)))
}
