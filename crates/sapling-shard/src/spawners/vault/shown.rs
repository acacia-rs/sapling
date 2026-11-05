//! When a vault's changes are shown, as BDS showed them (research/captures/bds-x2ran.txt): a state its own
//! update enters (active, inactive) is written at once and told by its animation, but its block (listed twice),
//! its entity (twice) and its sound go out the next tick; a key's unlocking is shown again a tick after it.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::super::eject::sound;
use super::super::now;
use crate::components::Layer;

struct Shown {
    layer: Entity,
    pos: BlockPos,
    at: i64,
    settled: bool,
    sound: Option<&'static str>,
}

#[derive(Resource, Default)]
struct Due(Vec<Shown>);

/// Shows the vault at `pos` next tick: `settled`, as a state its update entered (then `heard` after it);
/// else its block and entity once more.
pub(super) fn next_tick(world: &mut World, layer: Entity, pos: BlockPos, settled: bool, heard: Option<&'static str>) {
    let at = now(world) + 1;
    world.get_resource_or_insert_with(Due::default).0.push(Shown { layer, pos, at, settled, sound: heard });
}

/// Shows what is due by this tick.
pub(in crate::spawners) fn show_due(world: &mut World) {
    let now = now(world);
    let Some(mut due) = world.get_resource_mut::<Due>() else { return };
    let (ready, waiting): (Vec<Shown>, Vec<Shown>) = std::mem::take(&mut due.0).into_iter().partition(|shown| shown.at <= now);
    due.0 = waiting;
    for shown in ready {
        show(world, &shown);
    }
}

fn show(world: &mut World, shown: &Shown) {
    let (layer, pos) = (shown.layer, shown.pos);
    if !shown.settled {
        return crate::blocks::show_whole(world, layer, pos);
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    let entity: Vec<_> = level.block_entity(pos).map(|entity| crate::blocks::block_entity_data(pos, entity)).into_iter().collect();
    crate::blocks::show_states(world, layer, &[(pos, id), (pos, id)]);
    for _ in 0..2 {
        crate::blocks::to_viewers(world, layer, pos, &entity);
    }
    if let Some(heard) = shown.sound {
        sound(world, layer, pos, heard, pos.map(|c| c as f32), -1);
    }
}
