//! What stands in the way of a piston going out is pushed before it, as BDS's armour stands
//! were (docs/research/redstone.md, "Pistons and entities"). What stands on a block it moves
//! sideways goes with it, half a block in the tick it starts; going out, it is then put before the
//! block's front, and slime flings it on at 1 a tick (research/captures/bds-u3carry.txt).
// TODO: a body pinned against a block is squeezed out to +z where BDS's went -z; pushed bodies are
// not told with `force_move_local_entity`; a player on slime is not flung. A player is pushed as a
// body is and told by a correction (research/captures/bds-pushed.txt).

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

fn keep(world: &mut World, moved: Vec<(Entity, Body, bool)>) {
    for (entity, body, is_player) in moved {
        if is_player {
            crate::movement::push(world, entity, body.at);
        } else if let Some(mut kept) = world.get_mut::<Body>(entity) {
            *kept = body;
        }
    }
}

/// Whether `body` stands on the top of the block at `cell`, or of where it has gone to `low`.
fn rides(body: &Body, cell: BlockPos, low: [f32; 3]) -> bool {
    let (min, max) = body.bounds();
    let span = |i: usize| (low[i].min(cell[i] as f32), low[i].max(cell[i] as f32) + 1.0);
    (min[1] - (cell[1] + 1) as f32).abs() < ON_TOP && [0, 2].iter().all(|&i| max[i] > span(i).0 && min[i] < span(i).1)
}

/// Moves what stands on the blocks at `cells` by `by` towards `dir`, through the blocks there.
pub(super) fn carry_riders(world: &mut World, layer: Entity, cells: &[BlockPos], dir: Face, by: f32) {
    let step = dir.offset().map(|c| c as f32 * by);
    let all = bodies(world, layer);
    let Some(level) = super::level(world, layer) else { return };
    let carried: Vec<_> = all
        .into_iter()
        .filter(|(_, body, _)| cells.iter().any(|cell| rides(body, *cell, cell.map(|c| c as f32))))
        .map(|(entity, mut body, is_player)| {
            body.push(level, step, cells);
            (entity, body, is_player)
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
    let mut shoved = Vec::new();
    for (entity, mut body, is_player) in all {
        let start = body.at;
        let mut flung = false;
        for (i, cell) in from.iter().enumerate() {
            let top = tops.get(i).copied().flatten();
            // The block's box, `gone` on its way.
            let low: [f32; 3] = std::array::from_fn(|i| cell[i] as f32 + offset[i] as f32 * gone);
            let (min, max) = body.bounds();
            let rider = top.is_some_and(|top| top != Top::Honey) && axis != 1 && rides(&body, *cell, low);
            if !rider && (0..3).any(|i| max[i] <= low[i] || min[i] >= low[i] + 1.0) {
                continue;
            }
            let mut by = [0.0; 3];
            by[axis] = if ahead { low[axis] + 1.0 - min[axis] } else { low[axis] - max[axis] };
            body.push(level, by, &moving);
            flung |= top == Some(Top::Slime);
        }
        if flung && !is_player {
            body.motion[axis] = offset[axis] as f32;
        }
        if body.at != start || flung {
            shoved.push((entity, body, is_player));
        }
    }
    keep(world, shoved);
}
