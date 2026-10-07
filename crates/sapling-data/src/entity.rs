//! A mob entering someone's view: what BDS announced one of its kind with (`data/Mobs.bin`,
//! README "Mobs"), placed and dressed as the stored one. That brings the kind's hitbox,
//! attributes and flags. A kind BDS was not recorded for gets [`plain`].

use std::collections::HashMap;
use std::sync::LazyLock;

use acacia_proto::packets::{AddEntity, AddPainting};
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f,
};

use crate::templates::{decode, records};

/// The hitbox of a kind without a recording, in blocks.
const WIDTH: f32 = 0.9;
const HEIGHT: f32 = 0.9;
const FALLING_BLOCK_SIZE: f32 = 0.98;
const HEALTH: &str = "minecraft:health";

struct Recorded {
    grown: AddEntity,
    baby: Option<AddEntity>,
}

static RECORDED: LazyLock<HashMap<String, Recorded>> = LazyLock::new(|| {
    let (mut grown, mut babies) = (HashMap::new(), HashMap::new());
    for body in records(include_bytes!("../data/Mobs.bin")) {
        let mob: AddEntity = decode(body);
        if is_baby(&mob) { &mut babies } else { &mut grown }.insert(mob.entity_type.clone(), mob);
    }
    let mut kinds: HashMap<String, Recorded> = grown.into_iter().map(|(kind, grown)| (kind.clone(), Recorded { grown, baby: babies.remove(&kind) })).collect();
    // A kind that is only ever a baby (a tadpole) is its own grown one.
    kinds.extend(babies.into_iter().map(|(kind, baby)| (kind, Recorded { grown: baby, baby: None })));
    kinds
});

fn is_baby(mob: &AddEntity) -> bool {
    mob.metadata.iter().any(|item| matches!(&item.value, Value::Flags(flags) if flags.contains(MetadataFlags1::BABY)))
}

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
    /// Set on top of the kind's own, e.g. `SADDLED`.
    pub flags: MetadataFlags1,
    /// The health it has left; `None` for the kind's full health.
    pub health: Option<f32>,
    /// The name tag; empty for none.
    pub name: &'a str,
}

/// Kinds a world may still store that BDS announces as their successors.
const SUCCEEDED: [(&str, &str); 2] = [("minecraft:villager", "minecraft:villager_v2"), ("minecraft:zombie_villager", "minecraft:zombie_villager_v2")];

/// A painting on its wall: `direction` 0 south, 1 west, 2 north, 3 east; `motif` names the picture.
pub fn add_painting(id: u64, position: [f32; 3], direction: i32, motif: &str) -> AddPainting {
    let [x, y, z] = position;
    AddPainting { entity_id_self: id as i64, runtime_entity_id: id, coordinates: Vec3f { x, y, z }, direction, title: motif.to_owned() }
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

/// A mob of no particular kind: it collides, falls and has the few items [`add_entity`] sets.
fn plain(identifier: &str) -> AddEntity {
    let flags = MetadataFlags1(MetadataFlags1::HAS_COLLISION.0 | MetadataFlags1::AFFECTED_BY_GRAVITY.0);
    let metadata = vec![
        Item { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) },
        item(Key::Variant, Plain::Int(0)),
        item(Key::Color, Plain::Byte(0)),
        item(Key::Nametag, Plain::String(String::new())),
        item(Key::Scale, Plain::Float(1.0)),
        item(Key::MarkVariant, Plain::Int(0)),
        item(Key::BoundingboxWidth, Plain::Float(WIDTH)),
        item(Key::BoundingboxHeight, Plain::Float(HEIGHT)),
    ];
    let still = Vec3f { x: 0.0, y: 0.0, z: 0.0 };
    AddEntity {
        unique_id: 0,
        runtime_id: 0,
        entity_type: identifier.to_owned(),
        position: still.clone(),
        velocity: still,
        pitch: 0.0,
        yaw: 0.0,
        head_yaw: 0.0,
        body_yaw: 0.0,
        attributes: Vec::new(),
        metadata,
        properties: EntityProperties { ints: Vec::new(), floats: Vec::new() },
        links: Vec::new(),
    }
}

/// The width and height of a grown mob of `identifier`'s box, as BDS announced one; `None` for
/// a kind it was not recorded announcing.
pub fn mob_size(identifier: &str) -> Option<[f32; 2]> {
    let grown = &RECORDED.get(identifier)?.grown;
    let size = |key: Key| grown.metadata.iter().find(|item| item.key == key).and_then(|item| if let Value::Default(Plain::Float(size)) = item.value { Some(size) } else { None });
    Some([size(Key::BoundingboxWidth)?, size(Key::BoundingboxHeight)?])
}

/// A falling block: BDS names the block by its network id in `Variant`, after `Health` 1; the
/// hitbox is Java's.
pub fn add_falling_block(id: u64, position: [f32; 3], velocity: [f32; 3], block_hash: u32) -> AddEntity {
    let options = AddEntityOptions {
        id,
        identifier: "minecraft:falling_block",
        feet: position,
        yaw: 0.0,
        pitch: 0.0,
        variant: block_hash as i32,
        mark_variant: 0,
        color: 0,
        baby: false,
        flags: MetadataFlags1(0),
        health: None,
        name: "",
    };
    let mut block = add_entity(&options);
    for item in &mut block.metadata {
        if matches!(item.key, Key::BoundingboxWidth | Key::BoundingboxHeight) {
            item.value = Value::Default(Plain::Float(FALLING_BLOCK_SIZE));
        }
    }
    let after_flags = block.metadata.iter().position(|item| item.key == Key::Flags).map_or(0, |at| at + 1);
    block.metadata.insert(after_flags, item(Key::Health, Plain::Int(1)));
    let [x, y, z] = velocity;
    AddEntity { velocity: Vec3f { x, y, z }, ..block }
}

/// An experience orb worth `value` at `at`, moving at `velocity`, in BDS's recorded shape.
pub fn add_experience_orb(id: u64, at: [f32; 3], velocity: [f32; 3], value: i32) -> AddEntity {
    const ORB: &str = "minecraft:xp_orb";
    let mut orb = RECORDED.get(ORB).map_or_else(|| plain(ORB), |kind| kind.grown.clone());
    orb.metadata.retain(|item| item.key != Key::ExperienceValue);
    orb.metadata.push(item(Key::ExperienceValue, Plain::Int(value)));
    let ([x, y, z], [dx, dy, dz]) = (at, velocity);
    AddEntity { unique_id: id as i64, runtime_id: id, position: Vec3f { x, y, z }, velocity: Vec3f { x: dx, y: dy, z: dz }, ..orb }
}

pub fn add_entity(o: &AddEntityOptions<'_>) -> AddEntity {
    let identifier = SUCCEEDED.iter().find(|(old, _)| *old == o.identifier).map_or(o.identifier, |(_, new)| new);
    let recorded = RECORDED.get(identifier);
    let baby = recorded.and_then(|kind| kind.baby.as_ref()).filter(|_| o.baby);
    let mut mob = baby.or(recorded.map(|kind| &kind.grown)).cloned().unwrap_or_else(|| plain(identifier));
    let extra = MetadataFlags1(o.flags.0 | if o.baby { MetadataFlags1::BABY.0 } else { 0 });
    for item in &mut mob.metadata {
        match (&item.key, &mut item.value) {
            (Key::Flags, Value::Flags(flags)) => flags.insert(extra),
            (Key::Variant, value) => *value = Value::Default(Plain::Int(o.variant)),
            (Key::MarkVariant, value) => *value = Value::Default(Plain::Int(o.mark_variant)),
            (Key::Color, value) => *value = Value::Default(Plain::Byte(o.color)),
            (Key::Nametag, value) => *value = Value::Default(Plain::String(o.name.to_owned())),
            _ => {}
        }
    }
    for attribute in mob.attributes.iter_mut().filter(|attribute| attribute.name == HEALTH) {
        attribute.value = o.health.map_or(attribute.value, |health| health.clamp(attribute.min, attribute.max));
    }
    let [x, y, z] = o.feet;
    AddEntity {
        unique_id: o.id as i64,
        runtime_id: o.id,
        position: Vec3f { x, y, z },
        velocity: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
        pitch: o.pitch,
        yaw: o.yaw,
        head_yaw: o.yaw,
        body_yaw: o.yaw,
        ..mob
    }
}

#[cfg(test)]
mod tests {
    use acacia_proto::Packet;
    use bytes::BytesMut;

    use super::*;

    fn options<'a>(identifier: &'a str, baby: bool) -> AddEntityOptions<'a> {
        AddEntityOptions { id: 9, identifier, feet: [1.5, 64.0, -2.5], yaw: 90.0, pitch: 0.0, variant: 3, mark_variant: 2, color: 0, baby, flags: MetadataFlags1::SADDLED, health: Some(5.5), name: "Bill" }
    }

    fn float(mob: &AddEntity, key: Key) -> Option<f32> {
        mob.metadata.iter().find(|item| item.key == key).and_then(|item| if let Value::Default(Plain::Float(float)) = item.value { Some(float) } else { None })
    }

    #[test]
    fn a_mob_survives_the_wire_with_its_look() {
        for kind in ["minecraft:horse", "sapling:nothing_recorded"] {
            let packet = add_entity(&options(kind, true));
            let mut out = BytesMut::new();
            packet.encode(&mut out);
            let read = AddEntity::decode(&mut &out[..]).unwrap();
            assert_eq!(read, packet);
            assert_eq!((read.unique_id, read.runtime_id, read.entity_type.as_str(), read.body_yaw), (9, 9, kind, 90.0));
            assert_eq!((read.position.x, read.position.y, read.position.z, read.velocity.y), (1.5, 64.0, -2.5, 0.0));
            let value = |key: Key| read.metadata.iter().find(|item| item.key == key).map(|item| item.value.clone());
            assert_eq!(value(Key::Variant), Some(Value::Default(Plain::Int(3))));
            assert_eq!(value(Key::MarkVariant), Some(Value::Default(Plain::Int(2))));
            assert_eq!(value(Key::Nametag), Some(Value::Default(Plain::String("Bill".into()))));
            assert!(is_baby(&read));
        }
    }

    #[test]
    fn a_recorded_kind_has_its_own_hitbox_and_attributes() {
        let horse = add_entity(&options("minecraft:horse", false));
        assert_eq!((float(&horse, Key::BoundingboxWidth), float(&horse, Key::BoundingboxHeight)), (Some(1.4), Some(1.6)));
        assert!(horse.attributes.iter().any(|attribute| attribute.name == HEALTH && attribute.value == 5.5 && attribute.max > 5.5) && !is_baby(&horse));
        assert!(horse.metadata.iter().any(|item| matches!(&item.value, Value::Flags(flags) if flags.contains(MetadataFlags1::SADDLED))));
        assert_eq!(add_entity(&options("minecraft:villager", false)).entity_type, "minecraft:villager_v2");
        assert_eq!([mob_size("minecraft:horse"), mob_size("sapling:nothing_recorded")], [Some([1.4, 1.6]), None]);
        let unknown = add_entity(&options("sapling:nothing_recorded", false));
        assert_eq!((float(&unknown, Key::BoundingboxWidth), unknown.attributes.len()), (Some(WIDTH), 0));
    }

    #[test]
    fn every_recorded_kind_has_a_grown_one() {
        // Some kinds are summoned as a baby by chance; the recorder summons those again.
        let mut only_babies: Vec<&str> = RECORDED.iter().filter(|(_, kind)| is_baby(&kind.grown)).map(|(kind, _)| kind.as_str()).collect();
        only_babies.sort();
        assert_eq!(only_babies, ["minecraft:tadpole"]);
        assert!(RECORDED.len() > 100 && RECORDED.values().all(|kind| kind.baby.as_ref().is_none_or(is_baby)), "{} kinds", RECORDED.len());
        let foal = add_entity(&options("minecraft:horse", true));
        assert!(float(&foal, Key::Scale).is_some_and(|scale| scale < 1.0), "a foal is recorded as one");
    }
}
