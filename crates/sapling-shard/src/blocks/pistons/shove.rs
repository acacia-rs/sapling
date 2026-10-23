//! What stands in the way of a piston going out is pushed before it, as BDS's armour stands
//! were (docs/research/redstone.md, "Pistons and entities").
// TODO: a body pinned against a block stays in the arm, unhurt; nothing is carried by slime or
// honey, or thrown by a piston pushing up. A player is pushed as a body is and told by a
// correction (research/captures/bds-pushed.txt: beside the arm, and lifted by one under it).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use crate::body::Body;
use crate::components::{InLayer, Position, in_world};

/// A player's box, wide and high.
const PLAYER: [f32; 2] = [0.6, 1.8];

/// Puts every body of `layer` that reaches into a block going `dir` from one of the cells `from`,
/// `gone` of a block on its way, before that block's front.
pub(super) fn shove(world: &mut World, layer: Entity, from: &[BlockPos], dir: Face, gone: f32) {
    let offset = dir.offset();
    let Some(axis) = offset.iter().position(|step| *step != 0) else { return };
    let ahead = offset[axis] > 0;
    let mut bodies = world.query::<(Entity, &InLayer, &Body)>();
    let mut moved: Vec<(Entity, Body, bool)> = bodies.iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).map(|(entity, _, body)| (entity, *body, false)).collect();
    let here = |player: &Entity| world.get::<InLayer>(*player).map(|in_layer| in_layer.0) == Some(layer);
    moved.extend(in_world(world).into_iter().filter(here).filter_map(|player| Some((player, Body::new(world.get::<Position>(player)?.feet, [0.0; 3], PLAYER), true))));
    let Some(level) = super::level(world, layer) else { return };
    let mut shoved = Vec::new();
    for (entity, mut body, is_player) in moved {
        let start = body.at;
        for cell in from {
            // The block's box, `gone` on its way.
            let low: [f32; 3] = std::array::from_fn(|i| cell[i] as f32 + offset[i] as f32 * gone);
            let (min, max) = body.bounds();
            if (0..3).any(|i| max[i] <= low[i] || min[i] >= low[i] + 1.0) {
                continue;
            }
            let mut by = [0.0; 3];
            by[axis] = if ahead { low[axis] + 1.0 - min[axis] } else { low[axis] - max[axis] };
            body.push(level, by);
        }
        if body.at != start {
            shoved.push((entity, body, is_player));
        }
    }
    for (entity, body, is_player) in shoved {
        if is_player {
            crate::movement::push(world, entity, body.at);
        } else if let Some(mut kept) = world.get_mut::<Body>(entity) {
            *kept = body;
        }
    }
}
