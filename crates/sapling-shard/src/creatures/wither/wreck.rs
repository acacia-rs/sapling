//! The wither's block breaking, as BDS's `_destroyBlocks` (its hurt kind): a second after it is
//! hurt, where mobs may grief, every block its box reaches into, and a block more about it, but
//! the ones no wither breaks, goes and leaves what it would.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::Wither;
use crate::blocks;
use crate::body::Body;
use crate::components::Layer;

/// Ticks from a hurt to the breaking.
pub(super) const AFTER: u32 = 20;
const RANGE: i32 = 1;
const GRIEFING: &str = "mobGriefing";
const LASTING: &[&str] = &[
    "bedrock", "end_portal", "end_portal_frame", "end_gateway", "command_block", "repeating_command_block", "chain_command_block", "barrier", "border_block", "allow",
    "invisible_bedrock", "camera", "chalkboard", "structure_block", "structure_void", "reinforced_deepslate",
];

fn breaks(name: &str) -> bool {
    let bare = name.strip_prefix("minecraft:").unwrap_or(name);
    !["air", "water", "flowing_water", "lava", "flowing_lava"].contains(&bare) && !LASTING.contains(&bare)
}

/// The cells of `level` about `body` the wither breaks.
fn cells(level: &sapling_world::World, body: &Body) -> Vec<BlockPos> {
    let low = [body.at[0] - body.size[0] / 2.0, body.at[1], body.at[2] - body.size[0] / 2.0].map(|c| c.floor() as i32 - RANGE);
    let high = [body.at[0] + body.size[0] / 2.0, body.at[1] + body.size[1], body.at[2] + body.size[0] / 2.0].map(|c| c.floor() as i32 + RANGE);
    let all = (low[0]..=high[0]).flat_map(|x| (low[1]..=high[1]).flat_map(move |y| (low[2]..=high[2]).map(move |z| [x, y, z])));
    all.filter(|&[x, y, z]| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| breaks(state.name))).collect()
}

/// Counts down to the breaking, and breaks when it comes.
pub(super) fn tick(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    let Some(left) = world.get::<Wither>(wither).and_then(|kept| kept.wrecks) else { return };
    let left = left.saturating_sub(1);
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.wrecks = (left > 0).then_some(left);
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| left == 0 && level.level().rules.on(GRIEFING)) else { return };
    let broken = cells(level, body);
    if broken.is_empty() {
        return;
    }
    blocks::blow_out(world, layer, &broken, 1.0);
    let middle = [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]];
    crate::entities::after_announcing(world, layer, middle.map(|c| c.floor() as i32), vec![super::heads::sound("break_block", middle)]);
}

#[cfg(test)]
mod tests {
    use super::breaks;

    #[test]
    fn a_wither_breaks_all_but_the_lasting_blocks() {
        assert!(breaks("minecraft:obsidian") && breaks("minecraft:stone"));
        assert!(!breaks("minecraft:bedrock") && !breaks("minecraft:end_portal_frame") && !breaks("minecraft:air"));
    }
}
