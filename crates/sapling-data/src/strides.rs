//! What each horse, donkey, mule, skeleton horse and zombie horse is born with: its health,
//! walking speed and jump strength, each rolled in its pack's range (`minecraft:health`,
//! `minecraft:movement`, `minecraft:horse.jump_strength`) and announced in its `AddEntity`, as
//! BDS's eight summoned horses each were (research/captures/bds-horse-jump.txt).
// TODO: a foal's are its parents' mixed, as BDS breeds them; it is rolled afresh here.

use acacia_proto::packets::AddEntity;

const HEALTH: &str = "minecraft:health";
const MOVEMENT: &str = "minecraft:movement";
const JUMP: &str = "minecraft:horse.jump_strength";

/// Each kind's ranges of health (whole), movement and jump strength, from BDS's packs.
const RANGES: [(&str, [f32; 2], [f32; 2], [f32; 2]); 5] = [
    ("minecraft:horse", [15.0, 30.0], [0.1125, 0.3375], [0.4, 1.0]),
    ("minecraft:donkey", [15.0, 30.0], [0.175, 0.175], [0.5, 0.5]),
    ("minecraft:mule", [15.0, 30.0], [0.175, 0.175], [0.5, 0.5]),
    ("minecraft:skeleton_horse", [15.0, 15.0], [0.2, 0.2], [0.4, 1.0]),
    ("minecraft:zombie_horse", [25.0, 25.0], [0.205, 0.275], [0.5, 0.7]),
];

/// A mount's health, walking speed and jump strength.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strides {
    pub health: f32,
    pub movement: f32,
    pub jump: f32,
}

/// Whether a mob of `kind` is born with [`Strides`].
pub fn has_strides(kind: &str) -> bool {
    RANGES.iter().any(|(known, ..)| *known == kind)
}

/// The strides of a `kind` born with three draws in 0..1, one for each.
pub fn roll_strides(kind: &str, [health, movement, jump]: [f32; 3]) -> Option<Strides> {
    let (_, healths, movements, jumps) = RANGES.iter().find(|(known, ..)| *known == kind)?;
    let within = |[low, high]: [f32; 2], draw: f32| low + (high - low) * draw;
    let whole = (healths[0] + ((healths[1] - healths[0] + 1.0) * health).floor()).min(healths[1]);
    Some(Strides { health: whole, movement: within(*movements, movement), jump: within(*jumps, jump) })
}

/// Announces `strides` in `added`, with `left` of its health (all of it for none).
pub fn dress_strides(added: &mut AddEntity, strides: &Strides, left: Option<f32>) {
    for attribute in &mut added.attributes {
        match attribute.name.as_str() {
            HEALTH => (attribute.value, attribute.max) = (left.unwrap_or(strides.health).min(strides.health), strides.health),
            MOVEMENT => attribute.value = strides.movement,
            JUMP => attribute.value = strides.jump,
            _ => {}
        }
    }
}

/// The attributes `strides` stand for, as a world folder keeps them: name, value and max.
pub fn stride_attributes(strides: &Strides) -> [(&'static str, f32, f32); 3] {
    [(HEALTH, strides.health, strides.health), (MOVEMENT, strides.movement, f32::MAX), (JUMP, strides.jump, f32::MAX)]
}

/// The strides a world folder kept as `attributes` (name, value, max), where it kept them all.
pub fn kept_strides(attributes: &[(String, f32, f32)]) -> Option<Strides> {
    let find = |name: &str| attributes.iter().find(|(known, ..)| known == name);
    Some(Strides { health: find(HEALTH)?.2, movement: find(MOVEMENT)?.1, jump: find(JUMP)?.1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_horse_rolls_within_its_packs_ranges_and_a_donkey_jumps_a_half() {
        assert_eq!(roll_strides("minecraft:horse", [0.0; 3]), Some(Strides { health: 15.0, movement: 0.1125, jump: 0.4 }));
        assert_eq!(roll_strides("minecraft:horse", [0.9999; 3]).map(|strides| strides.health), Some(30.0));
        assert_eq!(roll_strides("minecraft:donkey", [0.5; 3]).map(|strides| strides.jump), Some(0.5));
        assert_eq!(roll_strides("minecraft:cow", [0.5; 3]), None);
    }

    #[test]
    fn a_horse_is_announced_with_its_own_strides() {
        let options = crate::AddEntityOptions { id: 9, identifier: "minecraft:horse", feet: [0.0; 3], yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: acacia_proto::types::MetadataFlags1(0), health: None, name: "" };
        let mut added = crate::add_entity(&options);
        let strides = Strides { health: 23.0, movement: 0.25, jump: 0.8 };
        dress_strides(&mut added, &strides, None);
        let value = |name: &str| added.attributes.iter().find(|attribute| attribute.name == name).map(|attribute| (attribute.value, attribute.max));
        assert_eq!([value(HEALTH), value(MOVEMENT).map(|(value, _)| (value, 0.0)), value(JUMP).map(|(value, _)| (value, 0.0))], [Some((23.0, 23.0)), Some((0.25, 0.0)), Some((0.8, 0.0))]);
    }
}
