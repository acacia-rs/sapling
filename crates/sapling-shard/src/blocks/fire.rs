//! Fire lit by hand: flint and steel or a fire charge puts it over a solid block, soul fire over
//! soul sand (docs/research/fire.md).
// TODO: fire neither spreads nor burns a block away, as BDS's did planks and wool; when it goes
// out is a guess (BDS's stayed the six seconds it was watched); `doFireTick` is not read; fire
// is lit only over a block, not against the side of one that burns.

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::{Dice, level, send};
use crate::components::Tick;

const FIRE: &str = "minecraft:fire";
const SOUL_FIRE: &str = "minecraft:soul_fire";
/// Fire over these burns on.
const ETERNAL: [&str; 4] = ["minecraft:netherrack", "minecraft:magma", "minecraft:soul_sand", "minecraft:soul_soil"];
/// The ticks any other fire lasts: from this many, to this many more.
const LASTS: [u32; 2] = [200, 400];

/// Fires that will go out: when, and where.
#[derive(Resource, Default)]
pub(crate) struct Fires(Vec<(u64, Entity, BlockPos)>);

/// Lights the free cell `into` over a solid block; false where no fire can be.
pub(super) fn light(world: &mut World, layer: Entity, into: BlockPos) -> bool {
    let [x, y, z] = into;
    let Some(level) = level(world, layer).filter(|level| level.in_height(y) && level.in_height(y - 1)) else { return false };
    let registry = level.registry();
    let Some(under) = registry.get(level.block(x, y - 1, z)).filter(|under| under.is_solid()).map(|under| under.name) else { return false };
    if !registry.get(level.block(x, y, z)).is_some_and(|here| here.is_air()) {
        return false;
    }
    let burns_on = ETERNAL.contains(&under);
    let Some(fire) = send::resolve(registry, &Block::new(if under.contains("soul_") { SOUL_FIRE } else { FIRE })) else { return false };
    send::set(world, layer, into, fire);
    if !burns_on {
        let [least, more] = LASTS;
        let lasts = u64::from(least + world.get_resource_or_insert_with(Dice::default).roll(more));
        let out_at = world.resource::<Tick>().0 + lasts;
        world.get_resource_or_insert_with(Fires::default).0.push((out_at, layer, into));
    }
    true
}

/// Puts out the fires whose time is up.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut fires) = world.get_resource_mut::<Fires>().filter(|fires| !fires.0.is_empty()) else { return };
    let (due, burning): (Vec<_>, Vec<_>) = std::mem::take(&mut fires.0).into_iter().partition(|(when, ..)| *when <= now);
    fires.0 = burning;
    for (_, layer, [x, y, z]) in due {
        let Some(level) = level(world, layer) else { continue };
        let registry = level.registry();
        if registry.get(level.block(x, y, z)).is_some_and(|state| state.name == FIRE) {
            let air = registry.air_id();
            send::set(world, layer, [x, y, z], air);
        }
    }
}
