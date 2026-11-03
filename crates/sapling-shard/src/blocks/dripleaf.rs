//! Big dripleaf tilting, the Java game's `BigDripleafBlock` (Bedrock's `big_dripleaf_tilt`): a
//! body standing on an unpowered leaf makes it unstable, then it tilts partly 10 ticks later and
//! fully 10 after that, and comes back up 100 after that; a shot tilts it fully at once. Power
//! stops it and sets it back (docs/research/blocks.md).

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::queued::{self, Job};
use super::{level, send, with_property};

const LEAF: &str = "minecraft:big_dripleaf";
const TILT: &str = "big_dripleaf_tilt";
const HEAD: &str = "big_dripleaf_head";
/// A body this far up the leaf's cell, or higher, stands on it (Java's `canEntityTilt`).
const ON_TOP: f32 = 0.6875;
const NONE: &str = "none";
const UNSTABLE: &str = "unstable";
const PARTIAL: &str = "partial_tilt";
const FULL: &str = "full_tilt";

/// The tilt of the leaf at `pos`; `None` for no big dripleaf's head.
fn tilt(world: &World, layer: Entity, pos: BlockPos) -> Option<String> {
    let level = level(world, layer).filter(|level| level.in_height(pos[1]))?;
    let state = level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| state.name == LEAF)?;
    (state.property(HEAD)? != "0").then_some(())?;
    Some(state.property(TILT)?.to_owned())
}

/// The ticks until the next step from `tilt`.
fn wait(tilt: &str) -> u64 {
    if tilt == FULL { 100 } else { 10 }
}

/// Sets the leaf at `pos` to `tilt`, heard as BDS names it, and queues its next step. A leaf on
/// ground it may not stand on breaks instead, as BDS's did on stone (research/captures/bds-bkblocks.txt).
fn set(world: &mut World, layer: Entity, pos: BlockPos, tilt: &str) {
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    if !super::ground::may_stand(level, pos, id) {
        super::updates::pop(world, layer, pos);
        return;
    }
    let Some(tilted) = with_property(level.registry(), id, TILT, tilt) else { return };
    send::set(world, layer, pos, tilted);
    let heard = match tilt {
        PARTIAL | FULL => Some("tilt_down.big_dripleaf"),
        NONE => Some("tilt_up.big_dripleaf"),
        _ => None,
    };
    if let Some(heard) = heard {
        // At the cell's corner, as BDS (research/captures/bds-bkblocks.txt).
        let [x, y, z] = pos.map(|c| c as f32);
        send::to_viewers(world, layer, pos, &[send::noise(heard, Vec3f { x, y, z }, -1)]);
    }
    queued::cancel(world, layer, pos, Job::Dripleaf);
    if tilt != NONE {
        queued::after(world, layer, pos, wait(tilt), Job::Dripleaf);
    }
}

/// The cells a body stands on: a mob's under its middle, high enough up; each a player's feet
/// overlap, however low, so a leaf coming back up under a player who fell into it tilts again at
/// once, as BDS's did with the player's middle a cell over; a cow there did not
/// (research/captures/bds-leafstand.txt).
fn stood_on(body: &super::plates::Body) -> Vec<BlockPos> {
    let y = body.min[1].floor() as i32;
    if !body.is_player {
        let [x, z] = [0, 2].map(|axis| ((body.min[axis] + body.max[axis]) / 2.0).floor() as i32);
        return if body.min[1] - y as f32 >= ON_TOP { vec![[x, y, z]] } else { Vec::new() };
    }
    let span = |axis: usize| body.min[axis].floor() as i32..=(body.max[axis] - 1e-4).floor() as i32;
    span(0).flat_map(|x| span(2).map(move |z| [x, y, z])).collect()
}

/// Makes unstable each still, unpowered leaf something stands on ([`stood_on`]).
pub(crate) fn tilt_under(world: &mut World) {
    let mut stood: Vec<(Entity, BlockPos)> =
        super::plates::bodies(world).into_iter().filter(|body| !body.spectator).flat_map(|body| stood_on(&body).into_iter().map(move |cell| (body.layer, cell))).collect();
    stood.sort_unstable();
    stood.dedup();
    for (layer, pos) in stood {
        if tilt(world, layer, pos).as_deref() == Some(NONE) && super::redstone::power_at(world, layer, pos) == 0 {
            set(world, layer, pos, UNSTABLE);
        }
    }
}

/// A shot met the block at `pos`: a leaf there tilts fully.
pub(super) fn shot(world: &mut World, layer: Entity, pos: BlockPos) {
    if tilt(world, layer, pos).is_some_and(|tilt| tilt != FULL) {
        set(world, layer, pos, FULL);
    }
}

/// The leaf at `pos` takes its next step, or goes back up while powered.
pub(super) fn step(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(now) = tilt(world, layer, pos) else { return };
    let next = match now.as_str() {
        _ if super::redstone::power_at(world, layer, pos) > 0 => NONE,
        UNSTABLE => PARTIAL,
        PARTIAL => FULL,
        FULL => NONE,
        _ => return,
    };
    if next != now {
        set(world, layer, pos, next);
    }
    // BDS tilted it again in the same update when back up under a body.
    if next == NONE {
        tilt_under(world);
    }
}
