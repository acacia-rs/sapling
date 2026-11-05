//! The mobs a spawn event puts on or under a new mob, as BDS did (bds-h3herds.txt): a zombie
//! horse's `spawn_adult_with_rider` and a camel husk's `spawn_with_rider` make their pack's
//! `minecraft:addrider` riders by their own spawn events (a zombie; a husk, then a parched in the
//! back seat); a drowned's `as_rider` makes the zombie nautilus it rides (`minecraft:spawn_entity`),
//! half a block up. Riders of a natural mount despawn as it does. The zombie and the husk roll
//! their rider group's `minecraft:equipment` table, an iron spear; the parched its own bow
//! (spawn_gear.rs), the drowned its trident (drowned.rs).
// TODO: the tables' `enchant_random_gear` (BDS's zombie's spear came enchanted) and their armour
// pools (difficulty conditions sapling's loot does not read) are not rolled.

use bevy_ecs::prelude::*;
use sapling_api::SpawnReason;

use super::birth;
use crate::body::Body;
use crate::components::InLayer;
use crate::entities;
use crate::spawning::Natural;

/// A mount's kind, its event, and the riders it makes with theirs.
const RIDERS: [(&str, &str, &[(&str, &str)]); 2] = [
    ("minecraft:zombie_horse", "spawn_adult_with_rider", &[("minecraft:zombie", "minecraft:spawn_as_rider")]),
    ("minecraft:camel_husk", "spawn_with_rider", &[("minecraft:husk", "minecraft:spawn_as_rider"), ("minecraft:parched", "minecraft:ranged_mode")]),
];
/// A rider's kind and the equipment table its rider event gives it.
const GEAR: [(&str, &str); 2] = [("minecraft:zombie", "loot_tables/entities/zombie_rider_equipment.json"), ("minecraft:husk", "loot_tables/entities/zombie_rider_equipment.json")];
/// A rider's kind, its event, and the mount it makes to ride.
const MOUNTS: [(&str, &str, &str); 1] = [("minecraft:drowned", "as_rider", "minecraft:zombie_nautilus")];
const MOUNT_UP: f32 = 0.5;

/// What `event` on the new mob `entity` of `kind` makes ride with it.
pub(super) fn arrive(world: &mut World, entity: Entity, kind: &str, event: &str) {
    let event = event.trim_start_matches("minecraft:");
    let (Some(layer), Some(feet)) = (world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).map(|body| body.at)) else { return };
    let natural = world.get::<Natural>(entity).is_some();
    let make =|world: &mut World, kind: &str, at: [f32; 3], event: Option<&str>| {
        let made = birth::spawn_as(world, layer, kind, at, SpawnReason::Summoned, 0.0, event).and_then(|id| entities::find(world, id))?;
        if natural {
            world.entity_mut(made).insert(Natural);
        }
        Some(made)
    };
    if let Some((.., riders)) = RIDERS.iter().find(|(mount, on, _)| *mount == kind && *on == event) {
        let yaw = world.get::<super::legs::Legs>(entity).map_or(0.0, |legs| legs.yaw);
        for (index, (rider, its_event)) in riders.iter().enumerate() {
            // BDS announced each rider already in its seat (bds-h3herds.txt).
            let seat = crate::riding::layout(kind).and_then(|layout| layout.seat(riders.len(), index));
            let up = seat.map_or([0.0; 3], |seat| crate::riding::seat_offset(&seat, rider, yaw));
            let at = [0, 1, 2].map(|axis| feet[axis] + up[axis]);
            if let Some(made) = make(world, rider, at, Some(its_event)) {
                if let Some((_, table)) = GEAR.iter().find(|(of, _)| of == rider) {
                    super::spawn_gear::equip(world, made, table);
                }
                let _ = crate::riding::seat(world, made, entity);
            }
        }
    }
    if let Some((.., mount)) = MOUNTS.iter().find(|(rider, on, _)| *rider == kind && *on == event)
        && let Some(mount) = make(world, mount, [feet[0], feet[1] + MOUNT_UP, feet[2]], None)
    {
        let _ = crate::riding::seat(world, entity, mount);
    }
}

