//! Drowned that throw tridents. As the pack has it, 25 in 400 come `as_ranged_adult` (or the
//! baby's or jockey's like): those get `ranged_equipment`, a trident, and a `mode_switcher` that
//! throws beyond 5 off and bites within 3, a throw every 1 to 3 seconds within 10 (`ranged_attack`).
//! research/captures/bds-drowned.txt: shown holding the trident as it came; it swung, and the
//! trident flew a tick later at 3.94 a tick, heard `item.trident.throw` from the drowned; it kept
//! its own, and the thrown one stuck where it fell.
// A viewer who comes later is shown the trident with the drowned.
// TODO: its drop on death (the pack sets only the offhand's chance; BDS's mainhand chance was not
// drilled), its enchantments and the 8% nautilus shell (`drowned_ranged_equipment.json`) are not done.

use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::arrows::{self, ARROW, Shot};
use super::hunting;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{NetId, Position, Tick};
use crate::entities;

const DROWNED: &str = "minecraft:drowned";
const RANGED: &str = "minecraft:as_ranged_adult";
/// Of this many drowned, `ARMED` come armed so.
const ARMED: u32 = 25;
const OF: u32 = 400;
/// It throws beyond `THROWS_PAST`, bites within `BITES_IN`, keeps its way between, and throws
/// within `RANGE` every `EVERY` ticks and up to `MORE` more.
const THROWS_PAST: f32 = 5.0;
const BITES_IN: f32 = 3.0;
const RANGE: f32 = 10.0;
const EVERY: u64 = 20;
const MORE: u32 = 40;
/// The pack's `thrown_trident`: power 4, gravity 0.1, 8 to whom it hits.
const TRIDENT: Shot = Shot {
    kind: "minecraft:thrown_trident",
    ahead: 0.0,
    held: 1.5,
    speed: 4.0,
    weight: 0.1,
    wet_drag: 0.99,
    harm: Some(sapling_data::shots::TRIDENT_HARM),
    sound: "item.trident.throw",
    heard: ["item.trident.hit", "item.trident.hit_ground"],
    size: [0.25, 0.35],
    ..ARROW
};

#[derive(Component, Default)]
pub(super) struct Thrower {
    throwing: bool,
    ready: u64,
    /// The tick its swing's trident flies.
    loosing: Option<u64>,
}

/// A drowned that comes: armed with a trident, shown in its hand, by the pack's odds or its spawn `event`.
pub(super) fn arm(world: &mut World, drowned: Entity, kind: &str, event: Option<&str>) {
    if kind != DROWNED {
        return;
    }
    let armed = event == Some(RANGED) || event.is_none() && world.get_resource_or_insert_with(Dice::default).roll(OF) < ARMED;
    let Some(id) = world.get::<NetId>(drowned).map(|id| id.0).filter(|_| armed) else { return };
    world.entity_mut(drowned).insert(Thrower::default());
    super::reshow(world, drowned);
    let (Some(held), Some(body)) = (held(world, drowned, id), world.get::<Body>(drowned).copied()) else { return };
    let layer = world.get::<crate::components::InLayer>(drowned).map(|layer| layer.0);
    if let Some(layer) = layer {
        entities::tell_viewers(world, drowned, layer, body.at.map(|c| c.floor() as i32), vec![held]);
    }
}

/// What shows an armed drowned's trident in its hand.
pub(super) fn held(world: &World, drowned: Entity, id: u64) -> Option<Bytes> {
    world.get::<Thrower>(drowned)?;
    let stack = sapling_data::Stack { name: "minecraft:trident", count: 1, damage: 0, block: None, tag: None };
    sapling_data::mob_hand(id, &stack, false).map(|held| sapling_net::encode(&held))
}

/// After its move: an armed drowned's throw, a tick after its swing.
pub(super) fn throw(world: &mut World, drowned: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let feet = hunting::quarry(world, drowned).and_then(|(prey, _)| world.get::<Position>(prey)).map(|at| at.feet);
    let thrower = world.get::<Thrower>(drowned)?;
    let (throwing, ready, loosing) = (thrower.throwing, thrower.ready, thrower.loosing);
    if let (Some(due), Some(feet)) = (loosing, feet)
        && now >= due
    {
        world.get_mut::<Thrower>(drowned)?.loosing = None;
        arrows::loose(world, layer, drowned, kind, body, feet, None, TRIDENT);
    }
    let feet = feet?;
    let far = (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]);
    let throwing = if far > THROWS_PAST { true } else if far < BITES_IN { false } else { throwing };
    world.get_mut::<Thrower>(drowned)?.throwing = throwing;
    if throwing && ready == 0 {
        // Java's first wait, by how far its prey is; BDS's one throw seen came 77 ticks on.
        world.get_mut::<Thrower>(drowned)?.ready = now + EVERY + (far.min(RANGE) / RANGE * MORE as f32) as u64;
        return Some(());
    }
    if throwing && far <= RANGE && now >= ready {
        let wait = EVERY + u64::from(world.get_resource_or_insert_with(Dice::default).roll(MORE + 1));
        let mut thrower = world.get_mut::<Thrower>(drowned)?;
        (thrower.ready, thrower.loosing) = (now + wait, Some(now + 1));
        hunting::swing(world, drowned, layer, id, body);
    }
    Some(())
}
