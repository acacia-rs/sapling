//! A vibration reaching listeners: each in range that hears it takes it as its candidate, and of the
//! candidates a listener gets in one tick it keeps the nearest, the higher frequency between two as near
//! (S5, `VibrationSelector::addCandidate`; research/captures/bds-b1sculk.txt: items a blast hurt 3 off were
//! heard over the blast 7 off). Its particle goes once the tick's posts are settled.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

use super::{CALIBRATED, CALIBRATED_RANGE, Coming, GameEvent, Listeners, RANGE, events, hears, name_at, occlusion, send};
use crate::components::Tick;

pub(super) fn reach_listeners(world: &mut World, layer: Entity, at: [f32; 3], event: GameEvent, by_player: bool) {
    let Some(listeners) = world.get_resource::<Listeners>() else { return };
    let reach = CALIBRATED_RANGE as i32;
    let corner = |dx: i32, dz: i32| send::column([at[0] as i32 + dx, 0, at[2] as i32 + dz]);
    let (low, high) = (corner(-reach, -reach), corner(reach, reach));
    let near: Vec<BlockPos> = (low.x..=high.x)
        .flat_map(|x| (low.z..=high.z).map(move |z| ChunkPos { x, z }))
        .filter_map(|column| listeners.listeners.get(&(layer, column)))
        .flatten()
        .copied()
        .collect();
    if near.is_empty() {
        return;
    }
    let now = world.resource::<Tick>().0;
    let cell = at.map(|c| c.floor() as i32);
    if event == GameEvent::BlockPlace && occlusion::dampens(world, layer, cell) {
        return;
    }
    for listener in near {
        let Some((name, properties, _)) = name_at(world, layer, listener) else { continue };
        let range = if name == CALIBRATED { CALIBRATED_RANGE } else { RANGE };
        let distance = (0..3).map(|axis| ((listener[axis] - cell[axis]) as f32).powi(2)).sum::<f32>().sqrt();
        if distance > range || !hears(world, layer, (name, properties), listener, cell, event, by_player) || occlusion::occluded(world, layer, cell, listener) {
            continue;
        }
        let strength = (15 - (15.0 * distance / range).floor() as i32).clamp(1, 15) as u8;
        let mut listeners = world.resource_mut::<Listeners>();
        let held = listeners.holding;
        let whole = distance as u64;
        let new = Coming { when: now + whole.max(1), posted: now, from: cell, whole, layer, listener, event, frequency: event.frequency(), strength, by_player, shown: !held };
        let taken = listeners.coming.iter().position(|coming| (coming.layer, coming.listener) == (layer, listener));
        match taken {
            None => listeners.coming.push(new),
            Some(at) if beats(&new, &listeners.coming[at]) => listeners.coming[at] = new,
            Some(_) => continue,
        }
        if !held {
            send::after_blocks(world, layer, cell, events::signal(cell, listener, whole));
        }
    }
}

/// Whether `new` takes the place of `old`, both on their way to one listener: only a candidate of the
/// same tick competes, and by whole blocks (S5).
fn beats(new: &Coming, old: &Coming) -> bool {
    old.posted == new.posted && (new.whole < old.whole || new.whole == old.whole && new.frequency > old.frequency)
}

/// Runs `post` with every particle held back, then sends those of the vibrations that won.
pub(super) fn settled(world: &mut World, post: impl FnOnce(&mut World)) {
    world.get_resource_or_insert_with(Listeners::default).holding = true;
    post(world);
    let mut listeners = world.resource_mut::<Listeners>();
    listeners.holding = false;
    let unshown: Vec<(Entity, BlockPos, BlockPos, u64)> = listeners.coming.iter_mut().filter(|coming| !coming.shown).map(|coming| {
        coming.shown = true;
        (coming.layer, coming.from, coming.listener, coming.whole)
    }).collect();
    for (layer, from, listener, whole) in unshown {
        send::after_blocks(world, layer, from, events::signal(from, listener, whole));
    }
}
