//! A mob becoming another, as the packs' `minecraft:transformation` has it: struck pigs and
//! villagers (struck.rs), piglins out of the Nether (zombifying.rs), villagers a zombie killed and
//! zombie villagers cured (villagers/conversion.rs). The new mob comes where the old stood, facing
//! the same way, then the old goes and the turning is heard from the new one's middle
//! (research/captures/bds-n3struck.txt, bds-t2barter.txt).

use acacia_proto::packets::LevelSoundEvent;
use bevy_ecs::prelude::*;
use sapling_api::{MobSlot, SpawnReason};

use super::legs;
use crate::body::Body;
use crate::components::{InLayer, Tick};
use crate::{entities, mobs};

/// The event a turned mob comes with; BDS played no `spawn` for one (birth.rs).
pub(super) const TRANSFORMED: &str = "minecraft:entity_transformed";

/// A mob turning into another at the tick `at`.
#[derive(Component)]
pub(super) struct Turning {
    pub into: &'static str,
    pub sound: Option<&'static str>,
    pub at: u64,
    /// What it holds and wears goes with it (`preserve_equipment`).
    pub keeps_gear: bool,
    pub reason: SpawnReason,
}

/// The mob takes the place of one of `turning.into`; the new one's entity.
fn turn(world: &mut World, mob: Entity, layer: Entity, turning: &Turning) -> Option<Entity> {
    let body = *world.get::<Body>(mob)?;
    let yaw = world.get::<legs::Legs>(mob).map_or(0.0, |legs| legs.yaw);
    let event = if super::shown::is_baby(world, mob) { "minecraft:as_baby" } else { TRANSFORMED };
    let gear = turning.keeps_gear.then(|| super::command_gear::gear_of(world, mob));
    let turned = super::spawn_as(world, layer, turning.into, body.at, turning.reason, yaw, Some(event)).and_then(|id| entities::find(world, id));
    if let (Some(turned), Some(gear)) = (turned, gear) {
        for (slot, item) in gear {
            let _ = super::command_gear::set_quietly(world, turned, slot, item);
        }
    }
    mobs::remove(world, mob);
    let (turned, sound) = (turned?, turning.sound?);
    entities::tell_viewers(world, turned, layer, body.at.map(|c| c.floor() as i32), vec![heard(sound, &body)]);
    Some(turned)
}

/// A turning's sound from the middle of `body`, of no entity.
pub(super) fn heard(sound: &str, body: &Body) -> bytes::Bytes {
    let position = acacia_proto::types::Vec3f { x: body.at[0], y: body.at[1] + body.size[1] / 2.0, z: body.at[2] };
    sapling_net::encode(&LevelSoundEvent { sound_id: sound.into(), position, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None })
}

/// A tick: cures near beds and iron bars speed up, then mobs whose time came turn.
pub(super) fn tick(world: &mut World) {
    super::villagers::hasten_cures(world);
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, Entity)> = world.query::<(Entity, &Turning, &InLayer)>().iter(world).filter(|(_, turning, _)| turning.at <= now).map(|(mob, _, layer)| (mob, layer.0)).collect();
    for (mob, layer) in due {
        if super::is_dying(world, mob) {
            continue;
        }
        if let Some(turning) = world.entity_mut(mob).take::<Turning>() {
            let kept = super::villagers::kept_through_turning(world, mob);
            if let Some(turned) = turn(world, mob, layer, &turning) {
                super::villagers::turned(world, turned, kept);
            }
        }
    }
}

/// The slots `preserve_equipment` carries.
pub(super) const GEAR: [MobSlot; 6] = [MobSlot::MainHand, MobSlot::OffHand, MobSlot::Armor(0), MobSlot::Armor(1), MobSlot::Armor(2), MobSlot::Armor(3)];
