//! What a witch drinks, by its pack's `drink_potion` (research/captures/bds-witch-drinks.txt): a
//! check a second (BDS's drank 3 and 12 seconds on, where a check a tick would have been at once)
//! for the first potion whose case holds and whose chance comes up: water breathing under water,
//! fire resistance on fire, healing when hurt, swiftness with prey 11 or more off. It holds the
//! potion 33 ticks, then has its effects (`afflictions`).
// TODO: it is not slowed as it drinks, and a viewer who comes while it drinks does not see the potion.

use acacia_proto::packets::MobEquipment;
use acacia_proto::types::WindowID;
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use crate::body::Body;
use crate::components::{Layer, Tick};
use crate::entities;
use super::health::Health;
use super::{afflictions, harm};

const KIND: &str = "minecraft:witch";
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// Ticks between checks, and ticks a draught takes.
const CHECKS: u64 = 20;
const DRINKS: u64 = 33;
/// The potions, by data value, and the chance of each at a check its case holds.
const WATER_BREATHING: (i16, f32) = (19, 0.15);
const FIRE_RESISTANCE: (i16, f32) = (12, 0.15);
const HEALING: (i16, f32) = (21, 0.05);
const SWIFTNESS: (i16, f32) = (14, 0.25);
/// Swiftness is drunk for prey this far off.
const FAR: f32 = 11.0;

#[derive(Component)]
pub(super) struct Drinker {
    /// Its own draws, so that they shift no other roll of the shard's.
    draws: u64,
    drinking: Option<(i16, u64)>,
}

impl Drinker {
    fn chance(&mut self, odds: f32) -> bool {
        self.draws ^= self.draws << 13;
        self.draws ^= self.draws >> 7;
        self.draws ^= self.draws << 17;
        ((self.draws >> 40) as f32 / (1u64 << 24) as f32) < odds
    }
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        world.entity_mut(entity).insert(Drinker { draws: entity.to_bits() ^ 0x9E37_79B9_7F4A_7C15, drinking: None });
    }
}

/// The witch `id` holds `potion`, or nothing.
fn hold(world: &mut World, entity: Entity, layer: Entity, id: u64, body: &Body, potion: Option<i16>) {
    let held = match potion {
        Some(data) => sapling_data::mob_hand(id, &sapling_data::Stack { name: "minecraft:potion", count: 1, damage: data, block: None, tag: None }, false),
        None => Some(MobEquipment { runtime_entity_id: id, item: sapling_data::no_item(), slot: 0, selected_slot: 0, window_id: WindowID::Inventory }),
    };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), held.into_iter().map(|held| sapling_net::encode(&held)).collect());
}

/// Before its move: a witch with prey at `prey` finishes a draught, or begins one.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, body: &Body, prey: Option<[f32; 3]>) {
    let now = world.resource::<Tick>().0;
    let Some(drinking) = world.get::<Drinker>(entity).map(|drinker| drinker.drinking) else { return };
    match drinking {
        Some((potion, since)) if now >= since + DRINKS => {
            hold(world, entity, layer, id, body, None);
            afflictions::splashed(world, entity, potion, 1.0);
            world.get_mut::<Drinker>(entity).expect("a drinker, above").drinking = None;
        }
        Some(_) => {}
        None if (now + id).is_multiple_of(CHECKS) => {
            let Some(health) = world.get::<Health>(entity).copied() else { return };
            let head = [body.at[0], body.at[1] + body.size[1], body.at[2]].map(|c| c.floor() as i32);
            let under_water = world.get::<Layer>(layer).is_some_and(|layer| harm::holds(&layer.world, head, WATER));
            let far = prey.is_some_and(|feet| (0..3).map(|axis| (feet[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() >= FAR);
            let without = |effect: Effect| afflictions::level(world, entity, effect).is_none();
            let cases = [
                (WATER_BREATHING, under_water && without(Effect::WaterBreathing)),
                (FIRE_RESISTANCE, health.is_burning() && without(Effect::FireResistance)),
                (HEALING, health.left() < health.max()),
                (SWIFTNESS, far && without(Effect::Speed)),
            ];
            let mut drinker = world.get_mut::<Drinker>(entity).expect("a drinker, above");
            let Some(((potion, _), _)) = cases.into_iter().find(|((_, odds), holds)| *holds && drinker.chance(*odds)) else { return };
            drinker.drinking = Some((potion, now));
            hold(world, entity, layer, id, body, Some(potion));
        }
        None => {}
    }
}
