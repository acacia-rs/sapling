//! The SetEntityData BDS sends right after a new entity's AddEntity (mobs, arrows, minecarts):
//! the announced metadata less the items still as an actor is made with them
//! (research/captures/bds-mountflags.txt, bds-farmobs.txt). An entity shown again, shown to one
//! who comes later, or loaded from a world folder gets none.

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;
use bytes::Bytes;

use crate::mobs;

/// A new entity whose first data is still to follow its announcement.
#[derive(Component)]
pub(super) struct Due;

/// Sends each new entity's first data, as it is announced now, to those just shown it; runs after
/// the tick's announcements.
pub(super) fn send(world: &mut World) {
    let due: Vec<Entity> = world.query_filtered::<Entity, With<Due>>().iter(world).collect();
    for entity in due {
        world.entity_mut(entity).remove::<Due>();
        if let Some(data) = mobs::announced(world, entity).and_then(|shown| after(shown.first()?)) {
            super::tell::to_seers(world, entity, &[data]);
        }
    }
}

/// Whether `item` is as every actor is made: BDS's first data leaves those out.
fn as_made(item: &MetadataDictionaryItem) -> bool {
    match (item.key, &item.value) {
        (Key::FlagsExtended, Value::FlagsExtended(flags)) => flags.0 == 0,
        (Key::Scale, Value::Default(Plain::Float(scale))) => *scale == 1.0,
        (Key::BoundingboxWidth, Value::Default(Plain::Float(width))) => *width == 0.6,
        (Key::BoundingboxHeight, Value::Default(Plain::Float(height))) => *height == 1.8,
        (Key::JumpDuration | Key::ControllingRiderSeatNumber, Value::Default(Plain::Byte(0))) => true,
        (Key::ArrowShooterId | Key::MinecartDisplayBlock, Value::Default(Plain::Long(0))) => true,
        (Key::RiderSeatPosition, Value::Default(Plain::Vec3f(at))) => [at.x, at.y, at.z] == [0.0; 3],
        _ => false,
    }
}

/// The first data that follows `announcement`, an `AddEntity`; `None` for anything else (an
/// item's announcement holds its own: mobs.rs `item_shown`).
fn after(announcement: &Bytes) -> Option<Bytes> {
    let added = RawPacket::parse(announcement.clone()).ok()?.decode::<AddEntity>().ok()?;
    let metadata = added.metadata.into_iter().filter(|item| !as_made(item)).collect();
    Some(sapling_net::encode(&SetEntityData { runtime_entity_id: added.runtime_id, metadata, properties: added.properties, tick: 0 }))
}

#[cfg(test)]
mod tests {
    use acacia_proto::types::Vec3f;

    use super::*;

    fn item(key: Key, value: Value) -> MetadataDictionaryItem {
        MetadataDictionaryItem { key, r#type: acacia_proto::types::MetadataDictionaryItemType::Float, legacy_type: 3, value }
    }

    #[test]
    fn only_the_items_still_as_made_are_left_out() {
        let made = [
            item(Key::Scale, Value::Default(Plain::Float(1.0))),
            item(Key::BoundingboxWidth, Value::Default(Plain::Float(0.6))),
            item(Key::RiderSeatPosition, Value::Default(Plain::Vec3f(Vec3f { x: 0.0, y: 0.0, z: 0.0 }))),
            item(Key::ArrowShooterId, Value::Default(Plain::Long(0))),
        ];
        let changed = [item(Key::Scale, Value::Default(Plain::Float(0.5))), item(Key::BoundingboxWidth, Value::Default(Plain::Float(0.9))), item(Key::Health, Value::Default(Plain::Int(1)))];
        assert!(made.iter().all(as_made));
        assert!(!changed.iter().any(as_made));
    }
}
