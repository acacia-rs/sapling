//! Health boost, on a player or a mob alike (research/captures/bds-hits2.txt): each level adds 4
//! to the most health it has, told with the boost's modifier over its own most health.

use acacia_proto::packets::UpdateAttributes;
use acacia_proto::types::PlayerAttributesItemModifiersItem;

/// Health a level of health boost adds to the most there is.
const BOOSTS: f32 = 4.0;
const BOOSTS_AS: &str = "5d6f0ba2-1186-46ac-b896-c61c5cee99cc";

fn more(level: Option<u8>) -> Option<f32> {
    level.map(|level| BOOSTS * (f32::from(level) + 1.0))
}

/// The most health of one whose own is `base`, under the health boost of `level`.
pub(crate) fn boosted_max(base: f32, level: Option<u8>) -> f32 {
    base + more(level).unwrap_or(0.0)
}

/// What tells the health `left` of `id`, whose own most is `base`, under the health boost of `level`.
pub(crate) fn boosted_health(id: u64, left: f32, base: f32, level: Option<u8>) -> UpdateAttributes {
    let mut told = sapling_data::mob_health_update(id, left, boosted_max(base, level));
    (told.attributes[0].default, told.attributes[0].default_max) = (base, base);
    told.attributes[0].modifiers = more(level).map(|amount| PlayerAttributesItemModifiersItem { id: BOOSTS_AS.into(), name: "HealthBoost".into(), amount, operation: 0, operand: 1, serializable: false }).into_iter().collect();
    told
}
