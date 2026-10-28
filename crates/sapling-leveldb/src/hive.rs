//! The bees a beehive or a bee nest holds, as a BDS 1.26 world kept them
//! (research/captures/bds-keephive-world.txt): its `Beehive` entity's `Occupants`, each the bee's
//! whole actor compound (`SaveData`, whose `properties` say if it carries pollen) under
//! `ActorIdentifier` `minecraft:bee<>`, with the ticks it has left inside (`TicksLeftToStay`),
//! beside `BlockEntityVersion` 0 and `ShouldSpawnBees` 1.

use acacia_proto::nbt::{self, List, LittleEndian, Str, Value};

use crate::actors::written;
use crate::container::new_block_entity;

const OCCUPANTS: &str = "Occupants";
const PROPERTIES: &str = "properties";
const NECTAR: &str = "minecraft:has_nectar";
const COMPOUND: u8 = 10;
const BEE: &str = "minecraft:bee<>";

/// A bee inside a hive: its actor compound, the ticks before it leaves, and whether it brought
/// pollen in.
#[derive(Debug, Clone, PartialEq)]
pub struct Occupant {
    pub actor: Vec<u8>,
    pub ticks_left: i32,
    pub nectar: bool,
}

/// A hive's entity at `pos` holding no bees, as BDS wrote one placed by a command.
pub fn new_hive(pos: [i32; 3]) -> Value {
    let mut entity = new_block_entity("Beehive", pos);
    if let Value::Compound(fields) = &mut entity {
        fields.extend([("BlockEntityVersion".into(), Value::Int(0)), ("ShouldSpawnBees".into(), Value::Byte(1))]);
    }
    entity
}

/// The bees a hive's entity holds.
pub fn hive_occupants(entity: &Value) -> Vec<Occupant> {
    let Some(Value::List(list)) = entity.get(OCCUPANTS) else { return Vec::new() };
    let occupant = |stored: &Value| {
        let actor = stored.get("SaveData")?;
        let nectar = matches!(actor.get(PROPERTIES).and_then(|properties| properties.get(NECTAR)), Some(Value::Byte(1)));
        let Value::Int(ticks_left) = stored.get("TicksLeftToStay")? else { return None };
        Some(Occupant { actor: written(actor.clone()), ticks_left: *ticks_left, nectar })
    };
    list.items.iter().filter_map(occupant).collect()
}

/// `fields` with `key` set to `value`, in its place or after the rest.
fn set(fields: &mut Vec<(Str, Value)>, key: &str, value: Value) {
    match fields.iter_mut().find(|(name, _)| name == key) {
        Some((_, old)) => *old = value,
        None => fields.push((key.into(), value)),
    }
}

/// A bee's actor compound as a hive keeps it, its pollen among its `properties`.
fn kept_bee(occupant: &Occupant) -> Option<Value> {
    let Value::Compound(mut actor) = nbt::read::<LittleEndian>(&mut &occupant.actor[..]).ok()?.value else { return None };
    let mut properties = match actor.iter().find(|(name, _)| name == PROPERTIES) {
        Some((_, Value::Compound(properties))) => properties.clone(),
        _ => Vec::new(),
    };
    set(&mut properties, NECTAR, Value::Byte(i8::from(occupant.nectar)));
    set(&mut actor, PROPERTIES, Value::Compound(properties));
    Some(Value::Compound(vec![("ActorIdentifier".into(), Value::String(BEE.into())), ("SaveData".into(), Value::Compound(actor)), ("TicksLeftToStay".into(), Value::Int(occupant.ticks_left))]))
}

/// A hive's entity holding `occupants`, the rest as it was; a bee whose compound cannot be read is
/// left out.
pub fn occupied_hive(entity: &Value, occupants: &[Occupant]) -> Value {
    let mut entity = entity.clone();
    if let Value::Compound(fields) = &mut entity {
        set(fields, OCCUPANTS, Value::List(List { tag: COMPOUND, items: occupants.iter().filter_map(kept_bee).collect() }));
    }
    entity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::new_actor;

    #[test]
    fn bees_go_into_a_hive_and_come_back() {
        let hive = new_hive([6, -26, 0]);
        assert!(hive_occupants(&hive).is_empty());
        let bee = |id| new_actor(id, "minecraft:bee", [6.5, -25.0, 0.5], [0.0; 3]);
        let bees = [Occupant { actor: bee(7), ticks_left: 600, nectar: false }, Occupant { actor: bee(8), ticks_left: 2400, nectar: true }];
        let full = occupied_hive(&hive, &bees);
        let back = hive_occupants(&full);
        assert_eq!(back.iter().map(|bee| (bee.ticks_left, bee.nectar)).collect::<Vec<_>>(), [(600, false), (2400, true)]);
        assert_eq!(hive_occupants(&occupied_hive(&full, &back)), back, "a second pass changes nothing");
        assert!(hive_occupants(&occupied_hive(&full, &[])).is_empty());
    }
}
