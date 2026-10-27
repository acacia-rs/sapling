//! The wait in water, by BDS's `FishingHook::normalTick` (1429c8960): after 5 to 29 seconds (5
//! fewer a level of Lure, a quarter near water animals) a fish comes; teased all the while, more
//! often as it nears, it swims 20 to 79 ticks to the hook, shown by its place, then bites for 10 to
//! 29 ticks, tugging the hook down. Each tick runs one, or none (under a roof, half the ticks), or
//! two (rain in the open, or a quarter of the ticks in a `fast_fishing` biome) ticks of waiting.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;
use sapling_data::enchant::{self, Enchant};

use super::Hook;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId};
use crate::dragon::cloud::item;
use crate::inventory::Inventory;
use crate::{entities, weather};

/// BDS turns the fish's angle, in degrees, by its sine as if it were radians.
const DEGREES: f32 = 57.29578;
const NIBBLE_PULL: f32 = 0.5;
/// Families a hook draws fish quicker near, within this far of its box.
const WATER_ANIMALS: &[&str] = &["minecraft:cod", "minecraft:salmon", "minecraft:pufferfish", "minecraft:tropicalfish", "minecraft:squid", "minecraft:glow_squid", "minecraft:dolphin"];
const NEAR: f32 = 8.0;

/// How many ticks of waiting this tick runs: three draws, each deciding one rule.
fn speed(world: &mut World, layer: Entity, at: [f32; 3], dice: &mut Dice) -> i32 {
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return 1 };
    let mut speed = 1;
    if dice.fraction() < 0.25 && level.biome(x, y, z).is_some_and(|biome| sapling_data::biome_has_tag(biome, "fast_fishing")) {
        speed += 1;
    }
    let rains = weather::of(world, layer) != sapling_api::Weather::Clear && level.light_at(x, y + 1, z).is_some_and(|(sky, _)| sky == 15) && !level.is_cold(x, y + 1, z);
    if dice.fraction() < 0.25 && rains {
        speed = 2;
    }
    let roofed = level.surface(x, z).is_some_and(|top| y + 1 <= top);
    if dice.fraction() < 0.5 && roofed {
        speed -= 1;
    }
    speed
}

fn event(id: u64, event_id: EntityEventEventId) -> bytes::Bytes {
    sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id, data: 0, fire_at_position: None })
}

fn fish_at(id: u64, x: f32, z: f32, angle: Option<f32>) -> bytes::Bytes {
    let mut metadata = vec![item(Key::FishX, Plain::Float(x)), item(Key::FishZ, Plain::Float(z))];
    metadata.extend(angle.map(|angle| item(Key::FishAngle, Plain::Float(angle))));
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// Lure's level on what the fisher holds.
fn lure(world: &World, fisher: Entity) -> i32 {
    let held = world.get::<Inventory>(fisher).and_then(|inventory| inventory.in_hand().as_ref());
    i32::from(held.map_or(0, |held| enchant::level(held.stack.tag.as_ref(), Enchant::Lure)))
}

fn water_animals_near(world: &mut World, layer: Entity, at: [f32; 3]) -> bool {
    let mut bodies = world.query::<(&InLayer, &Body, &entities::Kind)>();
    bodies.iter(world).any(|(in_layer, body, kind)| in_layer.0 == layer && WATER_ANIMALS.contains(&kind.0.as_str()) && (0..3).all(|axis| (body.at[axis] - at[axis]).abs() <= NEAR))
}

/// One tick of waiting for the hook in water at `body`: the motion it adds up.
pub(super) fn tick(world: &mut World, hook: Entity, layer: Entity, body: &Body) -> f32 {
    let Some(id) = world.get::<NetId>(hook).map(|id| id.0) else { return 0.0 };
    let mut dice = std::mem::take(&mut *world.get_resource_or_insert_with(Dice::default));
    let speed = speed(world, layer, body.at, &mut dice);
    let Some(state) = world.get::<Hook>(hook) else { return 0.0 };
    let (fisher, mut lured, mut hooked, mut nibble, mut angle) = (state.fisher, state.lured, state.hooked, state.nibble, state.angle);
    let near = lured <= 0 && hooked <= 0 && nibble <= 0 && water_animals_near(world, layer, body.at);
    let lure = lure(world, fisher);
    let mut told = Vec::new();
    let mut pull = 0.0;
    if nibble > 0 {
        nibble -= 1;
        pull = -NIBBLE_PULL * dice.fraction() * dice.fraction() * dice.fraction();
        if nibble <= 0 {
            (hooked, lured) = (0, 0);
        }
    } else if hooked > 0 {
        hooked -= speed;
        if hooked <= 0 {
            nibble = dice.roll(20) as i32 + 10;
            told.push(event(id, EntityEventEventId::FishHookHook));
        } else {
            angle += dice.gaussian() / 512.0;
            let turned = angle * DEGREES;
            let (x, z) = (turned.sin() * hooked as f32 * 0.1, turned.cos() * hooked as f32 * 0.1);
            told.push(fish_at(id, x, z, Some(turned)));
            let spot = [(body.at[0] + x).floor() as i32, body.at[1].floor() as i32, (body.at[2] + z).floor() as i32];
            if world.get::<Layer>(layer).is_some_and(|level| crate::creatures::holds(&level.world, spot, &["minecraft:water", "minecraft:flowing_water"])) {
                told.push(event(id, EntityEventEventId::FishHookPosition));
            }
        }
    } else if lured > 0 {
        lured -= speed;
        let chance = match lured {
            ..=19 => 0.15 + (20 - lured) as f32 * 0.05,
            ..=39 => 0.15 + (40 - lured) as f32 * 0.02,
            ..=59 => 0.15 + (60 - lured) as f32 * 0.01,
            _ => 0.15,
        };
        if dice.fraction() < chance {
            let (turned, far) = ((dice.fraction() * 360.0) * DEGREES, dice.fraction() * 35.0 + 25.0);
            told.push(fish_at(id, body.at[0] + turned.sin() * far * 0.1, body.at[2] + turned.cos() * far * 0.1, None));
            told.push(event(id, EntityEventEventId::FishHookTease));
        }
        if lured < 1 {
            angle = dice.fraction() * 360.0;
            hooked = dice.roll(60) as i32 + 20;
        }
    } else {
        let waited = dice.roll(25) as i32 + 5 - 5 * lure;
        let waited = if near { (waited as f32 * 0.25).floor() as i32 } else { waited };
        lured = waited.max(1) * 20;
    }
    *world.resource_mut::<Dice>() = dice;
    if let Some(mut state) = world.get_mut::<Hook>(hook) {
        (state.lured, state.hooked, state.nibble, state.angle) = (lured, hooked, nibble, angle);
    }
    entities::tell_viewers(world, hook, layer, body.at.map(|c| c.floor() as i32), told);
    pull
}
