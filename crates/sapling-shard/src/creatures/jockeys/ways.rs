//! A hijacked mount's way to where its spear rider charges or repositions. `UseKineticWeaponGoal`'s tick
//! (Linux 0x53497c0; its approach and charge share the melee path upkeep at 0x526f3a0) asks the mount's
//! navigation for a path to the prey every 4 + nextInt(7) ticks, and then only if the prey has left the
//! path's last node (else 1 time in 20); the mount walks it node by node to each node's stand, so it keeps
//! to where its prey was and weaves behind a thrown prey (bds-am2ride.txt, bds-ap2ride.txt).

use bevy_ecs::prelude::*;

use super::super::{paths, strolling};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::riding::Riding;

type Cell = [i32; 3];

/// The ticks between two plans: the least, and the spread drawn over.
const REPLANS: (i32, u32) = (4, 7);
/// Of the due plans with the prey still in the way's last node, the share made anyway.
const STILL_REPLANS: f32 = 0.05;
/// A node is done this near its stand, or half the mount's width if more (`PathNavigation::updatePath`
/// 0xbd70f60, as strolls: strolling.rs `ARRIVED`).
const REACHED: f32 = 0.6;

/// The nodes left of the mount's way and the node it was planned to, the ticks to its next plan, and
/// the stand it last walked to.
#[derive(Component, Default)]
pub(super) struct MountWay {
    way: Vec<Cell>,
    end: Option<Cell>,
    due: i32,
    last: Option<[f32; 2]>,
}

/// The node a mob `width` wide at `at` stands on: the cell of its box's low corner.
fn node(at: [f32; 3], width: f32) -> Cell {
    [(at[0] - width / 2.0).floor() as i32, at[1].floor() as i32, (at[2] - width / 2.0).floor() as i32]
}

/// Where on `cell` a mob `width` wide stands: the node plus half its whole width plus one. BDS's zombie
/// horses (1.4 wide) ran at x 0.0 for an actor at x 0.5 (research/captures/bds-am2ride.txt).
fn stand([x, _, z]: Cell, width: f32) -> [f32; 2] {
    let half = (width + 1.0).trunc() * 0.5;
    [x as f32 + half, z as f32 + half]
}

/// On the spear rider's tick: its mount's way to `to` is planned anew when due, and the nodes the
/// mount has reached are dropped.
pub(super) fn plan(world: &mut World, rider: Entity, to: [f32; 3]) {
    let Some(mount) = world.get::<Riding>(rider).map(|riding| riding.vehicle) else { return };
    let (Some(body), Some(layer)) = (world.get::<Body>(mount).copied(), world.get::<InLayer>(mount).map(|layer| layer.0)) else { return };
    let width = body.size[0];
    let mut kept = world.entity_mut(rider).take::<MountWay>().unwrap_or_default();
    kept.due -= 1;
    let end = node(to, width);
    if kept.due <= 0 && (kept.end != Some(end) || world.get_resource_or_insert_with(Dice::default).fraction() < STILL_REPLANS) {
        let from = node(body.at, width);
        let found = world.get::<Layer>(layer).and_then(|layer| paths::route(&layer.world, from, end)).unwrap_or_default();
        kept.way = std::iter::once(from).chain(found.into_iter().filter(|cell| *cell != from)).collect();
        kept.end = Some(end);
        kept.due = REPLANS.0 + world.get_resource_or_insert_with(Dice::default).roll(REPLANS.1) as i32;
    }
    let reached = (width * 0.5).max(REACHED);
    while kept.way.first().is_some_and(|cell| {
        let [x, z] = stand(*cell, width);
        (x - body.at[0]).hypot(z - body.at[2]) < reached
    }) {
        kept.way.remove(0);
    }
    kept.last = kept.way.first().map(|cell| stand(*cell, width)).or(kept.last);
    world.entity_mut(rider).insert(kept);
}

/// Where the mount at `body` walks for its rider this tick, and the share of its pace it keeps:
/// to its way's next stand, slowing on the last; with its way walked it stands, facing the last.
pub(super) fn next(world: &World, rider: Entity, body: &Body) -> Option<([f32; 2], f32)> {
    let kept = world.get::<MountWay>(rider)?;
    let width = body.size[0];
    match kept.way.first() {
        Some(cell) => {
            let to = stand(*cell, width);
            Some((to, if kept.way.len() == 1 { strolling::last_block_share(body, to) } else { 1.0 }))
        }
        None => kept.last.map(|to| (to, 0.0)),
    }
}
