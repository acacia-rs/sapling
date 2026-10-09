//! A stored actor's health and fire, written into its compound as BDS keeps them: `Current` of
//! the `minecraft:health` entry of `Attributes`, and `Fire`, the ticks it has left to burn.

use acacia_proto::nbt::{self, LittleEndian, List, Value};

use crate::Error;
use crate::actors::written;

const HEALTH: &str = "minecraft:health";
const COMPOUND_TAG: u8 = 10;

/// One stored actor's compound with `left` of its `max` health and `fire` ticks to burn; the
/// rest as it was. An actor that never stored its health gets the entry only once it is hurt.
pub fn harmed_actor(compound: &[u8], left: f32, max: f32, fire: i16) -> Result<Vec<u8>, Error> {
    let Value::Compound(mut fields) = nbt::read::<LittleEndian>(&mut &compound[..])?.value else { return Err(Error::Actor("an actor that is no compound")) };
    match fields.iter_mut().find(|(name, _)| name == "Fire") {
        Some((_, old)) => *old = Value::Short(fire),
        None if fire != 0 => fields.push(("Fire".into(), Value::Short(fire))),
        None => {}
    }
    if !fields.iter().any(|(name, _)| name == "Attributes") && left != max {
        fields.push(("Attributes".into(), Value::List(List { tag: COMPOUND_TAG, items: Vec::new() })));
    }
    if let Some((_, Value::List(attributes))) = fields.iter_mut().find(|(name, _)| name == "Attributes") {
        let is_health = |attribute: &Value| matches!(attribute.get("Name"), Some(Value::String(name)) if name == HEALTH);
        match attributes.items.iter_mut().find(|attribute| is_health(attribute)) {
            Some(Value::Compound(health)) => match health.iter_mut().find(|(name, _)| name == "Current") {
                Some((_, current)) => *current = Value::Float(left),
                None => health.push(("Current".into(), Value::Float(left))),
            },
            None if left != max => {
                let entry = [("Base", max), ("Current", left), ("DefaultMax", max), ("DefaultMin", 0.0), ("Max", max), ("Min", 0.0)];
                let mut health: Vec<_> = entry.into_iter().map(|(key, value)| (key.into(), Value::Float(value))).collect();
                health.push(("Name".into(), Value::String(HEALTH.into())));
                attributes.items.push(Value::Compound(health));
            }
            _ => {}
        }
    }
    Ok(written(Value::Compound(fields)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::{actors, new_actor};

    #[test]
    fn a_hurt_actor_reads_back_hurt_and_an_unhurt_one_is_left_as_it_was() {
        let pig = new_actor(-7, "minecraft:pig", [0.5, 65.0, 0.5], [0.0; 3]);
        assert_eq!(harmed_actor(&pig, 10.0, 10.0, 0).unwrap(), pig);
        let hurt = harmed_actor(&pig, 6.0, 10.0, 120).unwrap();
        let read = actors(&hurt).unwrap().remove(0);
        assert_eq!((read.health, read.fire, read.unique_id), (Some(6.0), 120, -7));
        let healed = actors(&harmed_actor(&hurt, 10.0, 10.0, 0).unwrap()).unwrap().remove(0);
        assert_eq!((healed.health, healed.fire), (Some(10.0), 0));
    }
}
