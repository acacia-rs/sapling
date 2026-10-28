//! Things thrown at a click: snowballs, eggs, ender pearls, splash and lingering potions, bottles o'
//! enchanting and wind charges, each from the eyes along the look at its pack's `power` (bottles
//! 20 degrees higher, their `angle_offset`, which BDS turns by the look's cos), the arm swung and
//! the throw heard; outside creative one is spent. A pearl (20 ticks) and a wind charge (10) then
//! cool down, the client told (research/captures/bds-small.txt).

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;
use sapling_data::shots::{BOTTLE_SPEED, BOTTLE_TILT, PEARL_COOLDOWN, THROWN_SPEED, WIND_COOLDOWN};
use sapling_world::ItemStack;

use super::kinds::{BOTTLE, EGG, LINGERING, PEARL, SNOWBALL, SPLASH, WIND};
use crate::components::{InLayer, NetId, Position};
use crate::creatures::{Lands, Loosed, Pickup, Shot};
use crate::inventory::{self, Inventory};

/// The items thrown, what each becomes, how fast and how far above the look, and its cooldown.
const THROWN: &[(&str, Shot, f32, bool, u64)] = &[
    ("minecraft:snowball", SNOWBALL, THROWN_SPEED, false, 0),
    ("minecraft:egg", EGG, THROWN_SPEED, false, 0),
    ("minecraft:blue_egg", EGG, THROWN_SPEED, false, 0),
    ("minecraft:brown_egg", EGG, THROWN_SPEED, false, 0),
    ("minecraft:ender_pearl", PEARL, THROWN_SPEED, false, PEARL_COOLDOWN),
    ("minecraft:splash_potion", SPLASH, BOTTLE_SPEED, true, 0),
    ("minecraft:lingering_potion", LINGERING, BOTTLE_SPEED, true, 0),
    ("minecraft:experience_bottle", BOTTLE, BOTTLE_SPEED, true, 0),
    ("minecraft:wind_charge", WIND, THROWN_SPEED, false, WIND_COOLDOWN),
];

/// The player throws the `stack` it holds, if it is a thing thrown; true when it is one.
pub(super) fn throw(world: &mut World, player: Entity, stack: &ItemStack) -> bool {
    let Some(&(name, shot, speed, bottle, cooldown)) = THROWN.iter().find(|(name, ..)| *name == stack.name) else { return false };
    if crate::cooldowns::is_cooling(world, player, name) {
        return true;
    }
    let shot = match shot.lands {
        Lands::Splash(_) => Shot { lands: Lands::Splash(stack.damage), ..shot },
        Lands::Lingers(_) => Shot { lands: Lands::Lingers(stack.damage), ..shot },
        _ => shot,
    };
    let tilt = if bottle { tilted(world, player) } else { 0.0 };
    if super::loose(world, player, shot, speed, tilt, 1.0, 0.0, Loosed::new(player, Pickup::Never, None)).is_none() {
        return true;
    }
    if cooldown > 0 {
        crate::cooldowns::tell(world, player, name, cooldown, true);
    }
    if !super::is_creative(world, player) {
        inventory::change(world, player, Inventory::take_one_in_hand);
    }
    swing(world, player);
    super::heard(world, player, shot.sound);
    true
}

/// A bottle's `angle_offset`, as BDS turns it by the cos of the look's pitch (142401a60).
fn tilted(world: &World, player: Entity) -> f32 {
    let pitch = world.get::<Position>(player).map_or(0.0, |position| position.pitch);
    BOTTLE_TILT * pitch.to_radians().cos()
}

/// The thrower's arm is seen to swing.
pub(super) fn swing(world: &mut World, player: Entity) {
    let (Some(id), Some(layer), Some(feet)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|position| position.feet)) else { return };
    let swing = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArmSwing, data: 0, fire_at_position: None };
    crate::view::to_viewers(world, layer, feet.map(|c| c.floor() as i32), &[sapling_net::encode(&swing)]);
}
