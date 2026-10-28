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

/// Sets the leaf at `pos` to `tilt`, heard as BDS names it, and queues its next step.
fn set(world: &mut World, layer: Entity, pos: BlockPos, tilt: &str) {
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    let Some(tilted) = with_property(level.registry(), id, TILT, tilt) else { return };
    send::set(world, layer, pos, tilted);
    let heard = match tilt {
        PARTIAL | FULL => Some("tilt_down.big_dripleaf"),
        NONE => Some("tilt_up.big_dripleaf"),
        _ => None,
    };
    if let Some(heard) = heard {
        let [x, y, z] = pos.map(|c| c as f32 + 0.5);
        send::to_viewers(world, layer, pos, &[send::noise(heard, Vec3f { x, y, z }, -1)]);
    }
    queued::cancel(world, layer, pos, Job::Dripleaf);
    if tilt != NONE {
        queued::after(world, layer, pos, wait(tilt), Job::Dripleaf);
    }
}

/// Makes unstable each still, unpowered leaf something stands on.
pub(crate) fn tilt_under(world: &mut World) {
    let mut stood: Vec<(Entity, BlockPos)> = super::plates::bodies(world)
        .into_iter()
        .filter(|body| !body.spectator)
        .map(|body| (body.layer, [((body.min[0] + body.max[0]) / 2.0).floor() as i32, body.min[1].floor() as i32, ((body.min[2] + body.max[2]) / 2.0).floor() as i32], body.min[1]))
        .filter(|(_, [_, y, _], feet)| feet - *y as f32 >= ON_TOP)
        .map(|(layer, cell, _)| (layer, cell))
        .collect();
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
}
