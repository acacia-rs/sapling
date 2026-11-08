//! Things thrown at a click: snowballs, eggs, ender pearls, splash and lingering potions, bottles o'
//! enchanting and wind charges, each from the eyes along the look at its pack's `power` (bottles
//! 20 degrees higher, their `angle_offset`, which BDS turns by the look's cos); outside creative one
//! is spent. A pearl (20 ticks) and a wind charge (10) then cool down, the client told
//! (research/captures/bds-small.txt). BDS first tells the thrower and its viewers all of its data,
//! then, after the thing's announcement, the throw heard (`thrown_heard_at`) and the thing heard
//! `disappeared` (flight/vanish.rs); it swings no arm for them (bds-small.txt, bds-bg2player.txt).

use bevy_ecs::prelude::*;
use sapling_data::shots::{BOTTLE_SPEED, BOTTLE_TILT, PEARL_COOLDOWN, THROWN_SPEED, WIND_COOLDOWN};
use sapling_world::ItemStack;

use super::aim::BELOW_EYES;
use super::kinds::{BOTTLE, EGG, LINGERING, PEARL, SNOWBALL, SPLASH, WIND};
use crate::components::{InLayer, NetId, Position};
use crate::creatures::{Lands, Loosed, Pickup, Shot};
use crate::inventory::{self, Inventory};

/// The items thrown, what each becomes, how far under the eyes it leaves (aim.rs), how fast and how
/// far above the look, and its cooldown.
const THROWN: &[(&str, Shot, f32, f32, bool, u64)] = &[
    ("minecraft:snowball", SNOWBALL, BELOW_EYES, THROWN_SPEED, false, 0),
    ("minecraft:egg", EGG, 0.0, THROWN_SPEED, false, 0),
    ("minecraft:blue_egg", EGG, 0.0, THROWN_SPEED, false, 0),
    ("minecraft:brown_egg", EGG, 0.0, THROWN_SPEED, false, 0),
    ("minecraft:ender_pearl", PEARL, 0.0, THROWN_SPEED, false, PEARL_COOLDOWN),
    ("minecraft:splash_potion", SPLASH, 0.0, BOTTLE_SPEED, true, 0),
    ("minecraft:lingering_potion", LINGERING, 0.0, BOTTLE_SPEED, true, 0),
    ("minecraft:experience_bottle", BOTTLE, 0.0, BOTTLE_SPEED, true, 0),
    ("minecraft:wind_charge", WIND, BELOW_EYES, WIND_SPEED, false, WIND_COOLDOWN),
];
/// The item's `launch_power_scale` over its projectile's `power` (bds-q3wind.txt: 2.26 a tick).
const WIND_SPEED: f32 = THROWN_SPEED * 1.5;
/// A throw is heard this far under the eyes and this far ahead along the look across.
const HEARD_BELOW_EYES: f32 = 0.3;
const HEARD_AHEAD: f32 = 0.15;

/// The player throws the `stack` it holds, if it is a thing thrown; true when it is one.
pub(super) fn throw(world: &mut World, player: Entity, stack: &ItemStack) -> bool {
    let Some(&(name, shot, below, speed, bottle, cooldown)) = THROWN.iter().find(|(name, ..)| *name == stack.name) else { return false };
    if crate::cooldowns::is_cooling(world, player, name) {
        return true;
    }
    let shot = match shot.lands {
        Lands::Splash(_) => Shot { lands: Lands::Splash(stack.damage), ..shot },
        Lands::Lingers(_) => Shot { lands: Lands::Lingers(stack.damage), ..shot },
        _ => shot,
    };
    if let Some(data) = crate::vitals::told_all::whole_data(world, player) {
        crate::vitals::tell(world, player, &data);
    }
    let tilt = if bottle { tilted(world, player) } else { 0.0 };
    let Some(thrown) = super::loose(world, player, shot, below, speed, tilt, 1.0, 0.0, Loosed::new(player, Pickup::Never, None)) else { return true };
    if cooldown > 0 {
        crate::cooldowns::tell(world, player, name, cooldown, true);
    }
    if !super::is_creative(world, player) {
        inventory::change(world, player, Inventory::take_one_in_hand);
    }
    heard_throwing(world, player, shot.sound, thrown);
    true
}

/// A bottle's `angle_offset`, as BDS turns it by the cos of the look's pitch (142401a60).
fn tilted(world: &World, player: Entity) -> f32 {
    let pitch = world.get::<Position>(player).map_or(0.0, |position| position.pitch);
    BOTTLE_TILT * pitch.to_radians().cos()
}

/// Where a throw from `position` is heard (bds-small.txt, bds-q3wind.txt, bds-y3arrow.txt).
pub(crate) fn thrown_heard_at(position: &Position) -> [f32; 3] {
    let (sin, cos) = position.yaw.to_radians().sin_cos();
    [position.feet[0] - sin * HEARD_AHEAD, position.feet[1] + sapling_data::EYE_HEIGHT - HEARD_BELOW_EYES, position.feet[2] + cos * HEARD_AHEAD]
}

/// The throw heard, as the player's, then `thrown` heard vanishing, after its announcement.
fn heard_throwing(world: &mut World, player: Entity, sound: &str, thrown: Entity) {
    let (Some(id), Some(layer), Some(position)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).copied()) else { return };
    let [x, y, z] = thrown_heard_at(&position);
    let heard = acacia_proto::packets::LevelSoundEvent {
        sound_id: sound.into(),
        position: acacia_proto::types::Vec3f { x, y, z },
        extra_data: -1,
        entity_type: "minecraft:player".into(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: id as i64,
        fire_at_position: None,
    };
    let mut packets = vec![sapling_net::encode(&heard)];
    packets.extend(crate::creatures::vanish_shot(world, thrown));
    crate::entities::on_announcing(world, layer, position.feet.map(|c| c.floor() as i32), packets);
}
