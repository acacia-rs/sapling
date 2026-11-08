//! Explorer maps: a cartographer's map of the square its structure lies in, pinned on the
//! structure (research/captures/bds-at1maps.txt; docs/research/villagers.md "Explorer maps").

use acacia_proto::types::MapDecorationType;
use bevy_ecs::prelude::*;

use super::render::SIZE;
use super::{kept, make, marks, tell_layer};

/// An explorer map's scale, its square 512 blocks.
pub(crate) const EXPLORER_SCALE: u8 = 2;

/// The marker BDS drew for a destination; trial chambers take index 24, which the proto's enum
/// names `MarkerWhite2`.
fn marker(destination: &str) -> Option<MapDecorationType> {
    Some(match destination {
        "monument" => MapDecorationType::Monument,
        "mansion" => MapDecorationType::Mansion,
        "village_desert" => MapDecorationType::VillageDesert,
        "village_plains" => MapDecorationType::VillagePlains,
        "village_savanna" => MapDecorationType::VillageSavanna,
        "village_snowy" => MapDecorationType::VillageSnowy,
        "village_taiga" => MapDecorationType::VillageTaiga,
        "jungle_temple" => MapDecorationType::JungleTemple,
        "swamp_hut" => MapDecorationType::WitchHut,
        "trial_chambers" => MapDecorationType::MarkerWhite2,
        _ => return None,
    })
}

/// Makes the map of the square around the column `at` of `destination`, pins it there and tells
/// the layer all of it; returns the map's id.
pub(crate) fn make_pinned(world: &mut World, layer: Entity, dimension: i32, destination: &str, at: [i32; 2]) -> i64 {
    let centre = at.map(|axis| super::centre(axis as f32, EXPLORER_SCALE));
    let id = make(world, layer, dimension, centre, EXPLORER_SCALE, false);
    let size = SIZE as usize;
    let whole = {
        let mut maps = kept::maps_mut(world, layer);
        let map = maps.maps.get_mut(&id).expect("made above");
        map.pin = marker(destination).map(|r#type| marks::pin(map, r#type, at));
        (map.tracked, map.marks) = map.pin.iter().cloned().unzip();
        marks::update(map, Some([0, 0, size, size]))
    };
    tell_layer(world, layer, &whole);
    id
}
