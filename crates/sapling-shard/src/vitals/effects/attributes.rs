//! The attributes effects change on a player, as BDS writes them: its walking speed and the
//! health on top of its own.

use acacia_proto::packets::UpdateAttributes;
use acacia_proto::types::{PlayerAttributesItem, PlayerAttributesItemModifiersItem};

use super::{QUICKENS, SLOWS};

/// A player's walking speed.
const WALKS: f32 = 0.1;
const QUICKENS_AS: &str = "91aeaa56-376b-4498-935b-2f7f68070635";
const SLOWS_AS: &str = "7107de5e-7ce8-4030-940e-514c1f160890";
/// The most absorption the client shows.
const MOST_CUSHION: f32 = 16.0;

/// The player's walking speed under levels of speed and of slowness.
// TODO: both at once were not drilled: each is taken to multiply the other's.
pub(super) fn pace(id: u64, quick: Option<u8>, slow: Option<u8>, tick: u64) -> UpdateAttributes {
    let modifier = |levels: Option<u8>, id: &str, name: &str, each: f32| {
        levels.map(|levels| PlayerAttributesItemModifiersItem { id: id.into(), name: name.into(), amount: each * (f32::from(levels) + 1.0), operation: 2, operand: 2, serializable: false })
    };
    let modifiers: Vec<_> = [modifier(quick, QUICKENS_AS, "MovementSpeed", QUICKENS), modifier(slow, SLOWS_AS, "MovementSlowdown", -SLOWS)].into_iter().flatten().collect();
    let movement = PlayerAttributesItem {
        min: 0.0,
        max: f32::MAX,
        current: modifiers.iter().fold(WALKS, |pace, modifier| pace * (1.0 + modifier.amount)).max(0.0),
        default_min: 0.0,
        default_max: f32::MAX,
        default: WALKS,
        name: "minecraft:movement".into(),
        modifiers,
    };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![movement], tick }
}

/// The health on top of a player's own.
pub(crate) fn cushion(id: u64, amount: f32, tick: u64) -> UpdateAttributes {
    let absorption = PlayerAttributesItem { min: 0.0, max: MOST_CUSHION, current: amount, default_min: 0.0, default_max: MOST_CUSHION, default: 0.0, name: "minecraft:absorption".into(), modifiers: Vec::new() };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![absorption], tick }
}
