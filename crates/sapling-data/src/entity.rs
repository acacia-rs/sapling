//! A mob entering someone's view. BDS sends each kind's hitbox, attributes and some fifty
//! metadata items (`sapling` example `bds_entity_probe`); this sends the few that say what the
//! mob looks like.
// TODO: per-kind hitboxes and attributes. Neither is in a world folder or in BDS's files; they
// would have to be recorded from BDS kind by kind.

use acacia_proto::packets::AddEntity;
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f,
};

/// The hitbox every kind gets, in blocks.
const WIDTH: f32 = 0.9;
const HEIGHT: f32 = 0.9;

/// What appears where, for [`add_entity`].
pub struct AddEntityOptions<'a> {
    /// Both the runtime and the unique entity id.
    pub id: u64,
    /// Namespaced, e.g. `minecraft:cow`.
    pub identifier: &'a str,
    pub feet: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    /// The kind's look, e.g. a horse's coat (`Variant`) and its markings (`MarkVariant`).
    pub variant: i32,
    pub mark_variant: i32,
    /// E.g. a sheep's wool.
    pub color: i8,
    pub baby: bool,
    /// The name tag; empty for none.
    pub name: &'a str,
}

fn item(key: Key, value: Plain) -> Item {
    // The legacy type is the type's wire number, as BDS repeats it.
    let (r#type, legacy_type) = match value {
        Plain::Byte(_) => (Type::Byte, 0),
        Plain::Int(_) => (Type::Int, 2),
        Plain::Float(_) => (Type::Float, 3),
        Plain::String(_) => (Type::String, 4),
        _ => unreachable!("mob metadata is bytes, ints, floats and strings"),
    };
    Item { key, r#type, legacy_type, value: Value::Default(value) }
}

pub fn add_entity(o: &AddEntityOptions<'_>) -> AddEntity {
    let [x, y, z] = o.feet;
    let mut flags = MetadataFlags1(MetadataFlags1::HAS_COLLISION.0 | MetadataFlags1::AFFECTED_BY_GRAVITY.0);
    if o.baby {
        flags.insert(MetadataFlags1::BABY);
    }
    let metadata = vec![
        Item { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) },
        item(Key::Variant, Plain::Int(o.variant)),
        item(Key::Color, Plain::Byte(o.color)),
        item(Key::Nametag, Plain::String(o.name.to_owned())),
        item(Key::Scale, Plain::Float(1.0)),
        item(Key::MarkVariant, Plain::Int(o.mark_variant)),
        item(Key::BoundingboxWidth, Plain::Float(WIDTH)),
        item(Key::BoundingboxHeight, Plain::Float(HEIGHT)),
    ];
    AddEntity {
        unique_id: o.id as i64,
        runtime_id: o.id,
        entity_type: o.identifier.to_owned(),
        position: Vec3f { x, y, z },
        velocity: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
        pitch: o.pitch,
        yaw: o.yaw,
        head_yaw: o.yaw,
        body_yaw: o.yaw,
        attributes: Vec::new(),
        metadata,
        properties: EntityProperties { ints: Vec::new(), floats: Vec::new() },
        links: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use acacia_proto::Packet;
    use bytes::BytesMut;

    use super::*;

    #[test]
    fn a_mob_survives_the_wire_with_its_look() {
        let options = AddEntityOptions { id: 9, identifier: "minecraft:horse", feet: [1.5, 64.0, -2.5], yaw: 90.0, pitch: 0.0, variant: 3, mark_variant: 2, color: 0, baby: true, name: "Bill" };
        let packet = add_entity(&options);
        let mut out = BytesMut::new();
        packet.encode(&mut out);
        let read = AddEntity::decode(&mut &out[..]).unwrap();
        assert_eq!(read, packet);
        assert_eq!((read.unique_id, read.runtime_id, read.entity_type.as_str(), read.body_yaw), (9, 9, "minecraft:horse", 90.0));
        let value = |key: Key| read.metadata.iter().find(|item| item.key == key).map(|item| item.value.clone());
        assert_eq!(value(Key::Variant), Some(Value::Default(Plain::Int(3))));
        assert!(matches!(value(Key::Flags), Some(Value::Flags(flags)) if flags.contains(MetadataFlags1::BABY)));
    }
}
