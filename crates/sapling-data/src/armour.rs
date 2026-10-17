//! What a worn piece of armour is worth: its points and its toughness. The Java game's numbers,
//! which BDS's damage was measured to follow (docs/research/player-damage.md, "Armour").

/// Points of a helmet, chestplate, leggings and boots of each material, and each piece's toughness.
const MATERIALS: &[(&str, [f32; 4], f32)] = &[
    ("leather", [1.0, 3.0, 2.0, 1.0], 0.0),
    ("chainmail", [2.0, 5.0, 4.0, 1.0], 0.0),
    ("iron", [2.0, 6.0, 5.0, 2.0], 0.0),
    ("golden", [2.0, 5.0, 3.0, 1.0], 0.0),
    ("diamond", [3.0, 8.0, 6.0, 3.0], 2.0),
    ("netherite", [3.0, 8.0, 6.0, 3.0], 3.0),
];
/// The uses a helmet of each material lasts, and what the pieces last for each 11 of a helmet's.
// TODO: the Java game's numbers, of which BDS showed none: no piece was worn out in a drill.
const LASTS: &[(&str, u16)] = &[("leather", 55), ("chainmail", 165), ("iron", 165), ("golden", 77), ("diamond", 363), ("netherite", 407)];
const LASTS_BY_PIECE: [u16; 4] = [11, 16, 15, 13];
const TURTLE_HELMET_LASTS: u16 = 275;
const PIECES: [&str; 4] = ["helmet", "chestplate", "leggings", "boots"];
const TURTLE_HELMET: (&str, [f32; 2]) = ("minecraft:turtle_helmet", [2.0, 0.0]);

/// The points and toughness of the item `name` (namespaced), if it is armour.
pub fn armour(name: &str) -> Option<[f32; 2]> {
    if name == TURTLE_HELMET.0 {
        return Some(TURTLE_HELMET.1);
    }
    let (material, piece) = name.strip_prefix("minecraft:")?.rsplit_once('_')?;
    let (_, points, toughness) = MATERIALS.iter().find(|(known, ..)| *known == material)?;
    Some([points[PIECES.iter().position(|known| *known == piece)?], *toughness])
}

/// The uses the item `name` (namespaced) lasts as armour, if it is armour that wears.
pub fn armour_lasts(name: &str) -> Option<u16> {
    if name == TURTLE_HELMET.0 {
        return Some(TURTLE_HELMET_LASTS);
    }
    let (material, piece) = name.strip_prefix("minecraft:")?.rsplit_once('_')?;
    let (_, helmet) = LASTS.iter().find(|(known, _)| *known == material)?;
    Some(helmet / LASTS_BY_PIECE[0] * LASTS_BY_PIECE[PIECES.iter().position(|known| *known == piece)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armour_is_known_by_its_material_and_piece() {
        assert_eq!([armour("minecraft:leather_chestplate"), armour("minecraft:netherite_boots"), armour("minecraft:turtle_helmet")], [Some([3.0, 0.0]), Some([3.0, 3.0]), Some([2.0, 0.0])]);
        assert_eq!([armour_lasts("minecraft:iron_helmet"), armour_lasts("minecraft:diamond_chestplate"), armour_lasts("minecraft:golden_boots"), armour_lasts("minecraft:stone")], [Some(165), Some(528), Some(91), None]);
        assert_eq!([armour("minecraft:iron_ingot"), armour("minecraft:diamond_sword"), armour("minecraft:stone")], [None; 3]);
    }
}
