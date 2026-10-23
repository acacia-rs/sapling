//! Orbs joining up as BDS's do (docs/research/experience.md 1.5 and 3.2): every twentieth tick of
//! its age an orb takes in the orbs in the block-sized box over its position whose ids agree with
//! its own mod 13, and new experience let out where 13 or more orbs lie is shared among them.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::Orb;
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::{blocks, mobs};

const EVERY: u32 = 20;
const KIN: u64 = 13;
/// Orbs in a cell past which new experience goes to them instead.
const CROWD: usize = 13;

fn inside(at: [f32; 3], other: [f32; 3]) -> bool {
    (0..3).all(|axis| (at[axis]..=at[axis] + 1.0).contains(&other[axis]))
}

/// The other orbs of `layer` in the box over `at`, with their ids and worth.
fn around(world: &mut World, layer: Entity, at: [f32; 3]) -> Vec<(Entity, u64, i32, u32)> {
    let mut orbs = world.query::<(Entity, &InLayer, &NetId, &Body, &Orb)>();
    orbs.iter(world).filter(|(_, in_layer, _, body, _)| in_layer.0 == layer && inside(at, body.at)).map(|(orb, _, id, _, kept)| (orb, id.0, kept.value, kept.age)).collect()
}

/// Tells viewers an orb is worth `value` now.
fn tell_worth(world: &mut World, layer: Entity, id: u64, at: [f32; 3], value: i32) {
    let item = MetadataDictionaryItem { key: Key::ExperienceValue, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(value)) };
    let told = SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[sapling_net::encode(&told)]);
}

/// On the orb's twentieth ticks, the kin it takes in.
pub(super) fn take_in_kin(world: &mut World, orb: &mut Orb, layer: Entity, id: u64, at: [f32; 3]) {
    if orb.age % EVERY != 1 {
        return;
    }
    let mut grew = false;
    for (other, _, value, age) in around(world, layer, at).into_iter().filter(|(_, other_id, ..)| other_id % KIN == id % KIN) {
        let Some(sum) = orb.value.checked_add(value) else { continue };
        (orb.value, orb.age, grew) = (sum, orb.age.min(age), true);
        mobs::remove(world, other);
    }
    if grew {
        tell_worth(world, layer, id, at, orb.value);
    }
}

/// Shares `value` among the orbs at `at` when 13 or more lie there; false where it should be an orb.
pub(super) fn shared(world: &mut World, layer: Entity, at: [f32; 3], value: i32) -> bool {
    let crowd = around(world, layer, at);
    if crowd.len() < CROWD {
        return false;
    }
    let (each, rest) = (value / crowd.len() as i32, value % crowd.len() as i32);
    for (i, (orb, id, worth, _)) in crowd.into_iter().enumerate() {
        let worth = worth.saturating_add(each + if i == 0 { rest } else { 0 });
        let Some(kept_at) = world.get::<Body>(orb).map(|body| body.at) else { continue };
        if let Some(mut kept) = world.get_mut::<Orb>(orb) {
            kept.value = worth;
        }
        tell_worth(world, layer, id, kept_at, worth);
    }
    true
}
