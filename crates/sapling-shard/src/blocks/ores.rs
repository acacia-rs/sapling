//! Lit redstone ore goes out by itself (docs/research/block-uses.md, "Redstone ore").
// TODO: ore a world folder holds lit stays lit until it is written again; only a survival
// player lights ore by standing on it, no mob and no creative player.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{Dice, level, send};
use crate::components::Tick;

const LIT: &str = "minecraft:lit_";
const ORE: &str = "redstone_ore";
/// The ticks a lit ore lasts on average: of BDS's 25, 16 were out after 90 seconds, at any time.
const MEAN: f32 = 1760.0;
/// No ore lasts more than this many times the average.
const LONGEST: f32 = 4.0;
/// The feet are less than this over the cell they stand on.
const UNDERFOOT: f32 = 0.2;

/// Lit ores: when each goes out, and where.
#[derive(Resource, Default)]
pub(crate) struct LitOres(Vec<(u64, Entity, BlockPos)>);

fn lit_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(u32, &str)> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    let id = level.block(x, y, z);
    level.registry().get(id).map(|state| state.name).filter(|name| name.starts_with(LIT) && name.ends_with(ORE)).map(|name| (id, name))
}

/// Lights the redstone ore at `pos`, if that is what is there.
pub(crate) fn light(world: &mut World, layer: Entity, [x, y, z]: BlockPos) {
    let lit = level(world, layer).filter(|level| level.in_height(y)).and_then(|level| {
        let (registry, id) = (level.registry(), level.block(x, y, z));
        let state = registry.get(id).filter(|state| state.name.ends_with(ORE) && !state.name.starts_with(LIT))?;
        registry.find(&state.name.replacen("minecraft:", LIT, 1), state.properties)
    });
    if let Some(lit) = lit {
        send::set(world, layer, [x, y, z], lit);
    }
}

/// A body stands at `feet`: redstone ore under it lights, as BDS's did a tick after it was set
/// under a standing player.
pub(crate) fn tread(world: &mut World, layer: Entity, feet: [f32; 3]) {
    light(world, layer, [feet[0].floor() as i32, (feet[1] - UNDERFOOT).floor() as i32, feet[2].floor() as i32]);
}

/// A block was written at `pos`: a lit ore, however it came there, will go out.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    if lit_at(world, layer, pos).is_none() {
        return;
    }
    let chance = world.get_resource_or_insert_with(Dice::default).fraction();
    let lasts = (-(1.0 - chance).ln()).min(LONGEST) * MEAN;
    let when = world.resource::<Tick>().0 + lasts as u64 + 1;
    let mut ores = world.get_resource_or_insert_with(LitOres::default);
    if !ores.0.iter().any(|(_, in_layer, at)| (*in_layer, *at) == (layer, pos)) {
        ores.0.push((when, layer, pos));
    }
}

/// Puts out the ores whose time is up.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut ores) = world.get_resource_mut::<LitOres>().filter(|ores| !ores.0.is_empty()) else { return };
    let (due, lit): (Vec<_>, Vec<_>) = std::mem::take(&mut ores.0).into_iter().partition(|(when, ..)| *when <= now);
    ores.0 = lit;
    for (_, layer, pos) in due {
        let unlit = lit_at(world, layer, pos).and_then(|(id, name)| {
            let registry = level(world, layer)?.registry();
            registry.find(&name.replacen("lit_", "", 1), registry.get(id)?.properties)
        });
        if let Some(unlit) = unlit {
            send::set(world, layer, pos, unlit);
        }
    }
}
