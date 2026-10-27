//! What a stored mob keeps of its passing state, as a BDS 1.26 world held it
//! (research/captures/bds-keepmobs.txt): its effects (`ActiveEffects`), a primed TNT's `Fuse`
//! byte beside `IsFuseLit`, and a wither's charge (`Invul` and `SpawningFrames`, the same ticks),
//! `Phase` (1 charging, 0 after), `AirAttack` (0 once armoured) and `maxHealth`.

use acacia_proto::nbt::{self, List, LittleEndian, Str, Value};

use crate::Error;
use crate::actors::{with_fields, written};
use crate::player::parts::effect;

const EFFECTS: &str = "ActiveEffects";
const COMPOUND: u8 = 10;
const DURATIONS: [&str; 4] = ["Duration", "DurationEasy", "DurationHard", "DurationNormal"];

fn fields_of(compound: &[u8]) -> Result<Vec<(Str, Value)>, Error> {
    match nbt::read::<LittleEndian>(&mut &compound[..])?.value {
        Value::Compound(fields) => Ok(fields),
        _ => Err(Error::Actor("an actor that is no compound")),
    }
}

fn int(fields: &[(Str, Value)], key: &str) -> Option<i32> {
    fields.iter().find(|(name, _)| name == key).and_then(|(_, value)| match value {
        Value::Byte(byte) => Some(i32::from(*byte)),
        Value::Int(int) => Some(*int),
        _ => None,
    })
}

/// One effect as BDS wrote a mob's: `[id, amplifier, ticks left, particles shown]`, every
/// difficulty's duration the same, and the fade data of an effect that does not fade.
fn effect_fields([id, amplifier, left, shown]: &[i32; 4]) -> Value {
    let mut fields: Vec<(Str, Value)> = vec![("Ambient".into(), Value::Byte(0)), ("Amplifier".into(), Value::Byte(*amplifier as i8)), ("DisplayOnScreenTextureAnimation".into(), Value::Byte(0))];
    fields.extend(DURATIONS.map(|key| (key.into(), Value::Int(*left))));
    let floats = [("factor_current", 0.0), ("factor_previous", 0.0), ("factor_start", 0.0), ("factor_target", 1.0)].map(|(key, value)| (key.into(), Value::Float(value)));
    let mut factor: Vec<(Str, Value)> = floats.into();
    factor.extend([("had_applied".into(), Value::Byte(1)), ("had_last_tick".into(), Value::Byte(0)), ("padding_duration".into(), Value::Int(0)), ("ticks_active".into(), Value::Int(0))]);
    fields.extend([("FactorCalculationData".into(), Value::Compound(factor)), ("Id".into(), Value::Byte(*id as i8)), ("ShowParticles".into(), Value::Byte(*shown as i8))]);
    Value::Compound(fields)
}

/// The effects a stored mob is under, each `[id, amplifier, ticks left, particles shown]`.
pub fn actor_effects(compound: &[u8]) -> Vec<[i32; 4]> {
    let Ok(fields) = fields_of(compound) else { return Vec::new() };
    match fields.iter().find(|(name, _)| name == EFFECTS) {
        Some((_, Value::List(list))) => list.items.iter().filter_map(effect).collect(),
        _ => Vec::new(),
    }
}

/// One stored mob's compound under `effects`, or without the field for none; the rest as it was.
pub fn effected_actor(compound: &[u8], effects: &[[i32; 4]]) -> Result<Vec<u8>, Error> {
    let mut fields = fields_of(compound)?;
    fields.retain(|(name, _)| name != EFFECTS);
    if !effects.is_empty() {
        fields.push((EFFECTS.into(), Value::List(List { tag: COMPOUND, items: effects.iter().map(effect_fields).collect() })));
    }
    Ok(written(Value::Compound(fields)))
}

/// The ticks left of a stored primed TNT's fuse.
pub fn actor_fuse(compound: &[u8]) -> Option<i32> {
    int(&fields_of(compound).ok()?, "Fuse")
}

/// One stored TNT's compound lit with `fuse` ticks left.
pub fn fused_actor(compound: &[u8], fuse: i32) -> Result<Vec<u8>, Error> {
    with_fields(compound, [("Fuse", Value::Byte(fuse.clamp(0, i32::from(i8::MAX)) as i8)), ("IsFuseLit", Value::Byte(1))])
}

/// What a stored wither keeps of its fight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WitherRecord {
    /// Ticks of its charge left.
    pub charging: i32,
    pub armoured: bool,
    /// The max health its difficulty gave it.
    pub max_health: i32,
}

/// A stored wither's fight, where it keeps one.
pub fn wither_record(compound: &[u8]) -> Option<WitherRecord> {
    let fields = fields_of(compound).ok()?;
    Some(WitherRecord { charging: int(&fields, "Invul")?, armoured: int(&fields, "AirAttack") == Some(0), max_health: int(&fields, "maxHealth")? })
}

/// One stored wither's compound keeping `record`; the rest as it was.
pub fn withered_actor(compound: &[u8], record: WitherRecord) -> Result<Vec<u8>, Error> {
    let WitherRecord { charging, armoured, max_health } = record;
    with_fields(
        compound,
        [
            ("AirAttack", Value::Byte(i8::from(!armoured))),
            ("Invul", Value::Int(charging)),
            ("SpawningFrames", Value::Int(charging)),
            ("Phase", Value::Int(i32::from(charging > 0))),
            ("maxHealth", Value::Int(max_health)),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::new_actor;

    #[test]
    fn effects_fuses_and_withers_go_into_an_actor_and_come_back() {
        let zombie = new_actor(3, "minecraft:zombie", [0.5, 64.0, 0.5], [0.0; 3]);
        let effects = [[1, 1, 1900, 1], [11, 2, 3920, 0]];
        let under = effected_actor(&zombie, &effects).unwrap();
        assert_eq!(actor_effects(&under), effects);
        assert_eq!(effected_actor(&under, &[]).unwrap(), zombie, "no effects, no field");
        let tnt = new_actor(4, "minecraft:tnt", [0.5, 64.0, 0.5], [0.0; 3]);
        assert_eq!(actor_fuse(&tnt), None);
        assert_eq!(actor_fuse(&fused_actor(&tnt, 60).unwrap()), Some(60));
        let wither = new_actor(5, "minecraft:wither", [0.5, 64.0, 0.5], [0.0; 3]);
        assert_eq!(wither_record(&wither), None);
        let record = WitherRecord { charging: 40, armoured: false, max_health: 300 };
        assert_eq!(wither_record(&withered_actor(&wither, record).unwrap()), Some(record));
        let armoured = WitherRecord { charging: 0, armoured: true, max_health: 450 };
        assert_eq!(wither_record(&withered_actor(&wither, armoured).unwrap()), Some(armoured));
    }
}
