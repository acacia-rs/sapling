//! Goat horns and spyglasses: items used held, not used up (research/captures/bds-small.txt). A
//! horn's use is heard at once as `horn_call<variant>` from 1.52 above the feet and cools horns for
//! 140 ticks, told as it starts and again when it completes 20 ticks on with a CompletedUsingItem;
//! a use while it cools does nothing. A spyglass is in use until let go.

use acacia_proto::packets::{CompletedUsingItem, CompletedUsingItemUseMethod, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::components::{InLayer, NetId, Outgoing, Position, Tick};
use crate::inventory::{self, Inventory};
use crate::vitals::eating;

const HORN: &str = "minecraft:goat_horn";
const SPYGLASS: &str = "minecraft:spyglass";
const HORN_COOLDOWN: u64 = 140;
/// Ticks from a horn's use to its completion, as BDS told it.
const HORN_USE: u64 = 20;
/// Above the feet, where BDS played a horn.
const HORN_HEIGHT: f32 = 1.52;

/// An item held in use: its network id, and for a horn the tick its use completes.
#[derive(Component)]
struct InUse {
    id: i16,
    completes: Option<u64>,
}

fn held(world: &World, player: Entity) -> Option<(String, i16, i16)> {
    let hand = world.get::<Inventory>(player)?.in_hand();
    let stack = &hand.as_ref()?.stack;
    Some((stack.name.clone(), stack.damage, inventory::wire(hand, false).network_id as i16))
}

/// A click in the air with a horn or a spyglass starts its use; false for anything else.
pub(crate) fn start(world: &mut World, player: Entity) -> bool {
    let Some((name, variant, id)) = held(world, player).filter(|(name, ..)| name == HORN || name == SPYGLASS) else { return false };
    if world.get::<InUse>(player).is_some() {
        return true;
    }
    let completes = if name == HORN {
        if crate::cooldowns::is_cooling(world, player, HORN) {
            return true;
        }
        crate::cooldowns::tell(world, player, HORN, HORN_COOLDOWN, true);
        blow(world, player, variant);
        Some(world.resource::<Tick>().0 + HORN_USE)
    } else {
        None
    };
    eating::show(world, player, true);
    world.entity_mut(player).insert(InUse { id, completes });
    true
}

fn blow(world: &mut World, player: Entity, variant: i16) {
    let (Some(layer), Some([x, y, z]), Some(id)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|at| at.feet), world.get::<NetId>(player).map(|id| id.0)) else { return };
    let sound = LevelSoundEvent {
        sound_id: format!("horn_call{variant}"),
        position: Vec3f { x, y: y + HORN_HEIGHT, z },
        extra_data: -1,
        entity_type: "minecraft:player".into(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: id as i64,
        fire_at_position: None,
    };
    crate::view::to_viewers(world, layer, [x, y, z].map(|c| c.floor() as i32), &[sapling_net::encode(&sound)]);
}

/// The use let go, or the item gone from the hand: it ends.
pub(crate) fn release(world: &mut World, player: Entity) {
    if world.entity_mut(player).take::<InUse>().is_some() {
        eating::show(world, player, false);
    }
}

/// Ends the uses whose item left the hand, and completes the horns whose time has come.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let using: Vec<(Entity, i16, Option<u64>)> = world.query::<(Entity, &InUse)>().iter(world).map(|(player, used)| (player, used.id, used.completes)).collect();
    for (player, id, completes) in using {
        if held(world, player).is_none_or(|(.., held)| held != id) {
            release(world, player);
        } else if completes.is_some_and(|tick| now >= tick) {
            crate::cooldowns::tell(world, player, HORN, HORN_COOLDOWN, false);
            if let Some(mut out) = world.get_mut::<Outgoing>(player) {
                out.push(&CompletedUsingItem { used_item_id: id, use_method: CompletedUsingItemUseMethod::Unknown(-1) });
            }
            inventory::show_windows(world, player, false);
            release(world, player);
        }
    }
}
