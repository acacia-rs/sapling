//! Redstone ore lights when trodden or struck (docs/research/block-uses.md, "Redstone ore"); lit, it
//! goes out on a random tick (growth/rules.rs), so it lasts less the higher `randomTickSpeed` is. Any
//! player but a spectator, mob or armour stand on it lights it (bds-orestand.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{level, send};

const LIT: &str = "minecraft:lit_";
const ORE: &str = "redstone_ore";
/// The feet are less than this over the cell they stand on.
const UNDERFOOT: f32 = 0.2;

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
