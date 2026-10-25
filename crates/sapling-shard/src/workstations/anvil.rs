//! An anvil wears with use: now and then a craft chips it, and a damaged anvil that wears again
//! is gone (docs/research/workstations.md, "Anvil").

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::enchant::anvil::{WEAR_CHANCE, WEARS};

use crate::blocks::{self, Dice};
use crate::components::Layer;

/// The anvils, each as the one before wears into it.
const ANVILS: [&str; 3] = ["minecraft:anvil", "minecraft:chipped_anvil", "minecraft:damaged_anvil"];

pub(super) fn is_anvil(block: &str) -> bool {
    ANVILS.contains(&block)
}

/// Throws whether the anvil at `pos` wears from a craft (never a creative player's); one that does
/// is the next anvil, facing as it did, or gone, and those near hear which.
pub(super) fn wear(world: &mut World, layer: Entity, pos: BlockPos, creative: bool) {
    if creative || world.get_resource_or_insert_with(Dice::default).roll(WEARS) >= WEAR_CHANCE {
        return sound(world, layer, pos, LevelEventEvent::SoundAnvilUse);
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let registry = level.registry();
    let Some(state) = registry.get(level.block(pos[0], pos[1], pos[2])) else { return };
    let Some(at) = ANVILS.iter().position(|anvil| *anvil == state.name) else { return };
    let next = ANVILS.get(at + 1).and_then(|next| registry.find(next, state.properties));
    let air = registry.air_id();
    blocks::set(world, layer, pos, next.unwrap_or(air));
    sound(world, layer, pos, if next.is_some() { LevelEventEvent::SoundAnvilUse } else { LevelEventEvent::SoundAnvilBreak });
}

fn sound(world: &mut World, layer: Entity, pos: BlockPos, event: LevelEventEvent) {
    let [x, y, z] = pos.map(|c| c as f32);
    blocks::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event, position: Vec3f { x, y, z }, data: 0 })]);
}
