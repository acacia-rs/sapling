//! Powder snow's freeze on a mob, as BDS's (research/captures/bds-k2freeze.txt): while its box is
//! in powder snow the freeze climbs 1/140 a tick, and full it shows the frozen flag (`VIBRATING`,
//! bit 40) and is hurt 1 (freezing) each tick its age is a multiple of 40, its health told a tick
//! later; a zombie, cow and pig held there 140 ticks froze and were hurt 20 ticks after. The packs'
//! `freezing_immune` kinds never freeze; a skeleton instead shakes from its first tick in it and
//! after 20 s becomes a stray (skeleton.json's `got_in_powder_snow` timer), heard `convert_to_stray`.
// The thaw out of snow is taken from the player's (vitals/frost.rs); the `freezing_vulnerable`
// kinds' 5 is the Java game's: neither was drilled on a mob.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use acacia_proto::types::MetadataFlags1;
use sapling_api::SpawnReason;

use super::turning::Turning;
use crate::components::Tick;
use crate::entities::{self, Born};

pub(super) const POWDER_SNOW: &[&str] = &["minecraft:powder_snow"];
const GAIN: f32 = 1.0 / 140.0;
const LOSS: f32 = 1.0 / 70.0;
const HURTS_EVERY: u64 = 40;
const HARM: f32 = 1.0;
const VULNERABLE_HARM: f32 = 5.0;
const IMMUNE: [&str; 8] = [
    "minecraft:creaking", "minecraft:ender_dragon", "minecraft:polar_bear", "minecraft:skeleton", "minecraft:snow_golem", "minecraft:stray", "minecraft:sulfur_cube", "minecraft:wither",
];
const VULNERABLE: [&str; 3] = ["minecraft:blaze", "minecraft:magma_cube", "minecraft:strider"];
const SKELETON: &str = "minecraft:skeleton";
const STRAY: &str = "minecraft:stray";
/// From the first tick a creature tick sees it in the snow, a tick after it came (as zombifying.rs's).
const TURNS: u64 = 402;

/// A mob's freeze, 0 to 1.
#[derive(Component, Default)]
pub(super) struct Frost(f32);

/// A skeleton in powder snow, shaking.
#[derive(Component)]
struct Straying;

/// A tick of the mob's freeze, its box `in_snow` or not: the frozen flag's change for its viewers,
/// and the harm it takes now, if any.
pub(super) fn chill(world: &mut World, entity: Entity, id: u64, kind: &str, in_snow: bool) -> (Option<Bytes>, Option<f32>) {
    if kind == SKELETON {
        return (stray(world, entity, id, kind, in_snow), None);
    }
    if IMMUNE.contains(&kind) || (!in_snow && world.get::<Frost>(entity).is_none()) {
        return (None, None);
    }
    let now = world.resource::<Tick>().0;
    let age = world.get::<Born>(entity).map_or(0, |born| now.saturating_sub(born.0));
    world.entity_mut(entity).insert_if_new(Frost::default());
    let mut frost = world.get_mut::<Frost>(entity).expect("put above");
    let was = frost.0;
    frost.0 = if in_snow { (was + GAIN).min(1.0) } else { (was - LOSS).max(0.0) };
    let level = frost.0;
    if level <= 0.0 {
        world.entity_mut(entity).remove::<Frost>();
    }
    let turned = ((level >= 1.0) != (was >= 1.0)).then(|| told_flag(world, entity, id, kind, level >= 1.0));
    let harm = (level >= 1.0 && age.is_multiple_of(HURTS_EVERY)).then_some(if VULNERABLE.contains(&kind) { VULNERABLE_HARM } else { HARM });
    (turned, harm)
}

fn told_flag(world: &mut World, entity: Entity, id: u64, kind: &str, on: bool) -> Bytes {
    let flags = entities::mob_flags(world, entity, kind, MetadataFlags1::VIBRATING, on);
    sapling_net::encode(&sapling_data::mob_flags_set(id, flags))
}

/// A skeleton comes into the snow shaking and turning into a stray, or leaves it and stops.
fn stray(world: &mut World, entity: Entity, id: u64, kind: &str, in_snow: bool) -> Option<Bytes> {
    if in_snow == world.get::<Straying>(entity).is_some() {
        return None;
    }
    if in_snow {
        let at = world.resource::<Tick>().0 + TURNS;
        world.entity_mut(entity).insert((Straying, Turning { into: STRAY, sound: Some("convert_to_stray"), at, keeps_gear: true, reason: SpawnReason::Transformed }));
    } else {
        world.entity_mut(entity).remove::<(Straying, Turning)>();
    }
    Some(told_flag(world, entity, id, kind, in_snow))
}
