//! Entity properties: the `description.properties` of the vanilla packs that clients are told of
//! (`data/entity-properties.tsv`, README "Entity properties"). BDS sends each kind's as a
//! SyncEntityProperty after StartGame and numbers them in that order, ints and floats alike, in
//! AddEntity's and SetEntityData's `properties`.

use std::sync::LazyLock;

use acacia_proto::nbt::{List, Nbt, Str, Value};
use acacia_proto::packets::SyncEntityProperty;
use acacia_proto::types::{EntityProperties, EntityPropertiesIntsItem};
use bytes::Bytes;

/// What one property holds.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyShape {
    Int { min: i32, max: i32 },
    Float { min: f32, max: f32 },
    Bool,
    /// Its values' names; one is sent as its place among them.
    Enum(Vec<&'static str>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityProperty {
    pub kind: &'static str,
    pub name: &'static str,
    /// Its number in the kind's `properties`.
    pub index: u32,
    pub shape: PropertyShape,
    /// The pack's default, as sent; a Molang default (a bee's nectar) is 0.
    pub default: i32,
}

impl EntityProperty {
    /// What `value` (an enum value's name, `true`/`false`, or a number) is sent as.
    pub fn value_of(&self, value: &str) -> Option<i32> {
        match &self.shape {
            PropertyShape::Enum(values) => values.iter().position(|known| *known == value).map(|at| at as i32),
            PropertyShape::Bool => ["false", "true"].iter().position(|known| *known == value).map(|at| at as i32),
            PropertyShape::Int { min, max } => value.parse().ok().filter(|number| (*min..=*max).contains(number)),
            PropertyShape::Float { .. } => None,
        }
    }

    fn nbt(&self) -> Value {
        let named = |key: &str, value: Value| (Str::from(key), value);
        let type_code = |code: i32| named("type", Value::Int(code));
        let mut fields = match &self.shape {
            PropertyShape::Int { min, max } => vec![named("max", Value::Int(*max)), named("min", Value::Int(*min)), type_code(0)],
            PropertyShape::Float { min, max } => vec![named("max", Value::Float(*max)), named("min", Value::Float(*min)), type_code(1)],
            PropertyShape::Bool => vec![type_code(2)],
            PropertyShape::Enum(values) => {
                let items = values.iter().map(|value| Value::String(Str::from(*value))).collect();
                vec![named("enum", Value::List(List { tag: 8, items })), type_code(3)]
            }
        };
        let at = fields.len() - 1;
        fields.insert(at, named("name", Value::String(Str::from(self.name))));
        Value::Compound(fields)
    }
}

static PROPERTIES: LazyLock<Vec<EntityProperty>> = LazyLock::new(|| {
    let mut rows: Vec<EntityProperty> = Vec::new();
    for line in include_str!("../data/entity-properties.tsv").lines().filter(|line| !line.is_empty()) {
        let [kind, name, shape, values, default] = line.splitn(5, '\t').collect::<Vec<_>>()[..] else { panic!("entity-properties.tsv: {line}") };
        let bounds = |values: &'static str| values.split_once("..").unwrap_or_else(|| panic!("entity-properties.tsv range: {line}"));
        let shape = match shape {
            "int" => {
                let (min, max) = bounds(values);
                PropertyShape::Int { min: min.parse().expect("int bound"), max: max.parse().expect("int bound") }
            }
            "float" => {
                let (min, max) = bounds(values);
                PropertyShape::Float { min: min.parse().expect("float bound"), max: max.parse().expect("float bound") }
            }
            "bool" => PropertyShape::Bool,
            "enum" => PropertyShape::Enum(values.split(',').collect()),
            other => panic!("entity-properties.tsv: no shape {other}"),
        };
        let index = rows.iter().filter(|row| row.kind == kind).count() as u32;
        let mut property = EntityProperty { kind, name, index, shape, default: 0 };
        property.default = property.value_of(default).unwrap_or(0);
        rows.push(property);
    }
    rows
});

/// The properties of `kind` clients are told of, in their order.
pub fn entity_properties(kind: &str) -> impl Iterator<Item = &'static EntityProperty> {
    PROPERTIES.iter().filter(move |property| property.kind == kind)
}

pub fn entity_property(kind: &str, name: &str) -> Option<&'static EntityProperty> {
    entity_properties(kind).find(|property| property.name == name)
}

/// `kind`'s properties at their defaults, each set where `set` names it, as AddEntity carries them.
pub fn property_values(kind: &str, set: &[(u32, i32)]) -> EntityProperties {
    let ints = entity_properties(kind)
        .filter(|property| !matches!(property.shape, PropertyShape::Float { .. }))
        .map(|property| EntityPropertiesIntsItem { index: property.index, value: set.iter().find(|(index, _)| *index == property.index).map_or(property.default, |(_, value)| *value) })
        .collect();
    EntityProperties { ints, floats: Vec::new() }
}

/// The SyncEntityProperty BDS sends after StartGame, one per kind, in its order and bytes.
pub fn property_registry_packets() -> &'static [Bytes] {
    static PACKETS: LazyLock<Vec<Bytes>> = LazyLock::new(|| {
        let mut kinds: Vec<&str> = PROPERTIES.iter().map(|property| property.kind).collect();
        kinds.dedup();
        kinds.into_iter().map(|kind| crate::templates::encoded(&registry(kind))).collect()
    });
    &PACKETS
}

fn registry(kind: &str) -> SyncEntityProperty {
    let items = entity_properties(kind).map(EntityProperty::nbt).collect();
    let fields = vec![(Str::from("properties"), Value::List(List { tag: 10, items })), (Str::from("type"), Value::String(Str::from(kind)))];
    SyncEntityProperty { nbt: (&Nbt { name: String::new(), value: Value::Compound(fields) }).into() }
}

#[cfg(test)]
mod tests {
    use acacia_proto::Packet;
    use bytes::BytesMut;

    use super::*;

    fn hex(packet: &SyncEntityProperty) -> String {
        let mut out = BytesMut::new();
        packet.encode(&mut out);
        out.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Bodies BDS 1.26.52 sent at join (acacia-testserver/fixtures/bds-1.26.52.script).
    #[test]
    fn the_registry_is_bds_byte_for_byte() {
        let bee = "0a00090a70726f706572746965730a0208046e616d65146d696e6563726166743a6861735f6e656374617203047479706504000804747970650d6d696e6563726166743a62656500";
        let creaking = "0a00090a70726f706572746965730a040904656e756d080a076e65757472616c10686f7374696c655f6f6273657276656412686f7374696c655f756e6f6273657276656409747769746368696e67096372756d626c696e6708046e616d65186d696e6563726166743a637265616b696e675f7374617465030474797065060003036d61780c03036d696e0008046e616d65206d696e6563726166743a637265616b696e675f73776179696e675f7469636b730304747970650000080474797065126d696e6563726166743a637265616b696e6700";
        let golem = "0a00090a70726f706572746965730a060904656e756d080a046e6f6e650474616b650974616b655f6661696c03707574087075745f6661696c08046e616d651b6d696e6563726166743a63686573745f696e746572616374696f6e030474797065060008046e616d65146d696e6563726166743a6861735f666c6f77657203047479706504000904656e756d08080a756e6f786964697a6564076578706f73656409776561746865726564086f786964697a656408046e616d65196d696e6563726166743a6f7869646174696f6e5f6c6576656c0304747970650600080474797065166d696e6563726166743a636f707065725f676f6c656d00";
        assert_eq!([hex(&registry("minecraft:bee")), hex(&registry("minecraft:creaking")), hex(&registry("minecraft:copper_golem"))], [bee, creaking, golem]);
        assert_eq!(property_registry_packets().len(), 13);
    }

    #[test]
    fn values_are_numbered_as_bds_numbers_them() {
        let pig = entity_property("minecraft:pig", "minecraft:sound_variant").unwrap();
        assert_eq!((pig.index, pig.value_of("mini"), pig.value_of("nope")), (1, Some(2), None));
        let ghast = property_values("minecraft:happy_ghast", &[]);
        assert_eq!(ghast.ints, [EntityPropertiesIntsItem { index: 0, value: 1 }]);
        let chicken = property_values("minecraft:chicken", &[(1, 1)]);
        assert_eq!(chicken.ints.iter().map(|item| (item.index, item.value)).collect::<Vec<_>>(), [(0, 0), (1, 1)]);
        assert!(property_values("minecraft:zombie", &[]).ints.is_empty());
    }
}
