//! Where a stored mob calls home (`minecraft:home`): `HomePos`, three floats, and
//! `HomeDimensionId`. A turtle hatched from an egg keeps the egg's cell.

use acacia_proto::nbt::{self, List, LittleEndian, Value};

use crate::actors::written;
use crate::Error;

const POS: &str = "HomePos";
const DIMENSION: &str = "HomeDimensionId";
const FLOAT: u8 = 5;

/// A stored actor's home and its dimension, if it has one.
pub fn actor_home(compound: &[u8]) -> Option<([f32; 3], i32)> {
    let actor = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value;
    let Some(Value::List(pos)) = actor.get(POS) else { return None };
    let floats: Vec<f32> = pos.items.iter().filter_map(|item| if let Value::Float(float) = item { Some(*float) } else { None }).collect();
    let dimension = if let Some(Value::Int(dimension)) = actor.get(DIMENSION) { *dimension } else { 0 };
    Some(([*floats.first()?, *floats.get(1)?, *floats.get(2)?], dimension))
}

/// A stored actor whose home is `pos` in `dimension`.
pub fn homed_actor(compound: &[u8], pos: [f32; 3], dimension: i32) -> Result<Vec<u8>, Error> {
    let Value::Compound(mut fields) = nbt::read::<LittleEndian>(&mut &compound[..])?.value else { return Err(Error::Actor("an actor that is no compound")) };
    fields.retain(|(name, _)| name != POS && name != DIMENSION);
    fields.push((POS.into(), Value::List(List { tag: FLOAT, items: pos.map(Value::Float).into() })));
    fields.push((DIMENSION.into(), Value::Int(dimension)));
    Ok(written(Value::Compound(fields)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_home_goes_into_an_actor_and_comes_back() {
        let turtle = crate::new_actor(3, "minecraft:turtle", [0.5, 64.0, 0.5], [0.0; 3]);
        assert_eq!(actor_home(&turtle), None);
        let homed = homed_actor(&turtle, [4.0, 63.0, -2.0], 0).unwrap();
        assert_eq!(actor_home(&homed), Some(([4.0, 63.0, -2.0], 0)));
    }
}
