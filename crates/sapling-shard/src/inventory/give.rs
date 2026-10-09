//! Items handed to a player by a command or plugin.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_world::ItemStack;

use crate::components::{InLayer, Position};
use crate::late_sounds;

/// Ticks from a given item to its `pop`, as BDS sent it (research/captures/bds-containers.txt).
const POP_LATE: u64 = 1;

/// Adds `stack` to the player's items, and has those near hear the `pop` at their eyes when any
/// fit; returns how many did not, `None` for no player.
pub(crate) fn give_and_pop(world: &mut World, player: Entity, stack: &ItemStack) -> Option<u8> {
    let left = super::change(world, player, |inventory| inventory.add(stack))?;
    let at = world.get::<InLayer>(player).map(|layer| layer.0).zip(world.get::<Position>(player).map(|position| position.feet));
    if let Some((layer, [x, y, z])) = at.filter(|_| left < stack.count) {
        let position = Vec3f { x, y: y + EYE_HEIGHT, z };
        let pop = LevelSoundEvent { sound_id: "pop".into(), position, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
        late_sounds::play_later(world, POP_LATE, layer, [x, y, z].map(|c| c.floor() as i32), sapling_net::encode(&pop));
    }
    Some(left)
}
