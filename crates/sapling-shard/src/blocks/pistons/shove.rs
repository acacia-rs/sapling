//! What stands in the way of a piston going out is pushed before it, as BDS's armour stands
//! were (docs/research/redstone.md, "Pistons and entities"). What stands on a block it moves
//! sideways goes with it, half a block in the tick it starts; going out, it is then put before the
//! block's front, and slime flings it on at 1 a tick (research/captures/bds-u3carry.txt). A player
//! in the way is pushed as a body is and told by a correction (bds-pushed.txt); one on a moved
//! block is left to its client, but for a fling up (bds-y3fling.txt).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use crate::body::Body;
use crate::components::{InLayer, Position, in_world};

/// A player's box, wide and high.
const PLAYER: [f32; 2] = [0.6, 1.8];
/// How near a body's feet are to a block's top to stand on it.
const ON_TOP: f32 = 1e-3;

/// What a moved block's top does to what stands on it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Top {
    Plain,
    /// Flings what it pushes along at 1 a tick.
    Slime,
    /// Keeps what stands on it where the half-block carry left it.
    Honey,
}

/// The [`Top`] of the block at `cell`.
pub(super) fn top_of(world: &World, layer: Entity, cell: BlockPos) -> Top {
    match super::block_name(world, layer, cell).map(|(_, name, _)| name) {
        Some("slime") => Top::Slime,
        Some("honey_block") => Top::Honey,
        _ => Top::Plain,
    }
}

/// Every body of `layer`, players with theirs made up, and whether it is a player's.
fn bodies(world: &mut World, layer: Entity) -> Vec<(Entity, Body, bool)> {
    let mut bodies = world.query::<(Entity, &InLayer, &Body)>();
    let mut all: Vec<(Entity, Body, bool)> = bodies.iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).map(|(entity, _, body)| (entity, *body, false)).collect();
    let here = |player: &Entity| world.get::<InLayer>(*player).map(|in_layer| in_layer.0) == Some(layer);
    all.extend(in_world(world).into_iter().filter(here).filter_map(|player| Some((player, Body::new(world.get::<Position>(player)?.feet, [0.0; 3], PLAYER), true))));
    all
}

/// Keeps each moved body, told as the piston left it at the point given (its feet).
fn keep(world: &mut World, moved: Vec<(Entity, Body, bool, [f32; 3])>) {
    for (entity, body, is_player, told_at) in moved {
        if is_player {
            crate::movement::push(world, entity, body.at);
        } else if let Some(mut kept) = world.get_mut::<Body>(entity) {
            *kept = body;
            if let Some(mut told) = world.get_mut::<crate::sync::Told>(entity) {
                told.pistoned(told_at, body.pushed_ground());
            }
        }
    }
}

/// Whether `body` stands on the top of the block at `cell`, or of where it has gone to `low`.
fn rides(body: &Body, cell: BlockPos, low: [f32; 3]) -> bool {
    let (min, max) = body.bounds();
    let span = |i: usize| (low[i].min(cell[i] as f32), low[i].max(cell[i] as f32) + 1.0);
    (min[1] - (cell[1] + 1) as f32).abs() < ON_TOP && [0, 2].iter().all(|&i| max[i] > span(i).0 && min[i] < span(i).1)
}

/// Moves what stands on the blocks at `cells` by `by` towards `dir`, through the blocks there; as a
/// push starts (`starts_out`), what is not on honey goes as far again, off its block, and falls in
/// that tick, told as its own move (bds-u3carry.txt: 3.5, 4.0 told as the piston's, then 4.5 falling).
pub(super) fn carry_riders(world: &mut World, layer: Entity, cells: &[BlockPos], dir: Face, by: f32, starts_out: bool) {
    let all = bodies(world, layer);
    let tops: Vec<Top> = cells.iter().map(|cell| top_of(world, layer, *cell)).collect();
    let Some(level) = super::level(world, layer) else { return };
    let carried: Vec<_> = all
        .into_iter()
        // BDS left a player on slime pushed sideways to its own client (bds-y3fling.txt).
        .filter(|(_, _, is_player)| !is_player)
        .filter_map(|(entity, mut body, is_player)| {
            let top = cells.iter().zip(&tops).find(|(cell, _)| rides(&body, **cell, cell.map(|c| c as f32)))?.1;
            let step = dir.offset().map(|c| c as f32 * by);
            body.push(level, step, cells);
            let told_at = body.at;
            if starts_out && *top != Top::Honey {
                body.push(level, step, cells);
            }
            Some((entity, body, is_player, told_at))
        })
        .collect();
    keep(world, carried);
}

/// Puts every body of `layer` that reaches into a block going `dir` from one of the cells `from`,
/// `gone` of a block on its way, before that block's front; with a top in `tops` (one for each of
/// `from`), what stands on that block too, unless honey holds it.
pub(super) fn shove(world: &mut World, layer: Entity, from: &[BlockPos], dir: Face, gone: f32, tops: &[Option<Top>]) {
    let offset = dir.offset();
    let Some(axis) = offset.iter().position(|step| *step != 0) else { return };
    let ahead = offset[axis] > 0;
    let all = bodies(world, layer);
    let Some(level) = super::level(world, layer) else { return };
    let moving: Vec<BlockPos> = from.iter().map(|cell| dir.neighbour(*cell)).collect();
    let (mut shoved, mut flung_players) = (Vec::new(), Vec::new());
    for (entity, mut body, is_player) in all {
        let start = body.at;
        let (mut flung, mut touched) = (false, false);
        for (i, cell) in from.iter().enumerate() {
            let top = tops.get(i).copied().flatten();
            // The block's box, `gone` on its way.
            let low: [f32; 3] = std::array::from_fn(|i| cell[i] as f32 + offset[i] as f32 * gone);
            let (min, max) = body.bounds();
            let rider = top.is_some_and(|top| top != Top::Honey) && axis != 1 && rides(&body, *cell, low);
            if (rider && is_player) || (!rider && (0..3).any(|i| max[i] <= low[i] || min[i] >= low[i] + 1.0)) {
                continue;
            }
            let mut by = [0.0; 3];
            by[axis] = if ahead { low[axis] + 1.0 - min[axis] } else { low[axis] - max[axis] };
            body.push(level, by, &moving);
            touched = true;
            flung |= top == Some(Top::Slime);
        }
        if flung && is_player && axis == 1 {
            // Its first move of the fling is in the correction: 1 up, then a tick's gravity and drag.
            let mut feet = body.at;
            feet[1] += offset[1] as f32;
            flung_players.push((entity, feet, (offset[1] as f32 - 0.08) * 0.98));
            continue;
        }
        if flung && !is_player {
            let mut speed = body.motion;
            speed[axis] = offset[axis] as f32;
            body.throw(speed);
        }
        // A body the block reaches but cannot move is told so too (bds-ag3pin.txt: pinned stands, no axes).
        if body.at != start || flung || (touched && !is_player) {
            shoved.push((entity, body, is_player, body.at));
        }
    }
    keep(world, shoved);
    for (player, feet, rise) in flung_players {
        crate::movement::push_off_ground(world, player, feet, rise);
    }
}
