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

/// One of the stack the player holds becomes `into`: in its slot when it was the last, else added
/// to the inventory or, without room, thrown.
pub(crate) fn swap_held(world: &mut World, player: Entity, into: &str) {
    let new = ItemStack { name: into.to_owned(), count: 1, damage: 0, block: None, tag: None };
    let spilled = super::change(world, player, |inventory| {
        let slot = super::Place::Main(inventory.selected.into());
        inventory.take(slot, 1).ok()?;
        if inventory.slot(slot).is_some_and(Option::is_none) {
            inventory.put(slot, Some(new.clone()));
            return None;
        }
        (inventory.add(&new) > 0).then(|| new.clone())
    });
    if let Some(Some(stack)) = spilled {
        crate::drops::throw(world, player, stack);
    }
}

/// Takes one of the stack the player holds; false when it holds nothing.
pub(crate) fn use_up_held(world: &mut World, player: Entity) -> bool {
    super::change(world, player, |inventory| {
        let slot = super::Place::Main(inventory.selected.into());
        inventory.take(slot, 1).is_ok()
    })
    .unwrap_or(false)
}

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
