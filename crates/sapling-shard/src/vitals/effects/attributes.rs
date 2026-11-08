//! The attributes effects change on a player, as BDS writes them: its walking speed and the
//! health on top of its own. Powder snow's freeze (vitals/frost.rs) and Soul Speed on soul sand
//! (vitals/soles.rs) add to the walking speed too.

use acacia_proto::packets::UpdateAttributes;
use acacia_proto::types::{PlayerAttributesItem, PlayerAttributesItemModifiersItem};
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{QUICKENS, SLOWS, level};
use crate::components::Tick;

/// A player's walking speed.
const WALKS: f32 = 0.1;
const QUICKENS_AS: &str = "91aeaa56-376b-4498-935b-2f7f68070635";
const SLOWS_AS: &str = "7107de5e-7ce8-4030-940e-514c1f160890";
/// BDS's freeze and Soul Speed modifiers, the second under the sprint's name (research/captures/
/// bds-freeze.txt, bds-soles.txt).
const FREEZES_AS: &str = "f1800c58-22ab-11eb-adc1-0242ac120002";
const SOLES_AS: &str = "87f46a96-686f-4796-b035-22e16ee9e038";
/// The most absorption the client shows.
const MOST_CUSHION: f32 = 16.0;

/// What a player's walking speed adds besides its effects: the freeze's and Soul Speed's.
#[derive(Component, Default, Clone, Copy, PartialEq)]
pub(crate) struct Paced {
    pub freeze: f32,
    pub soles: f32,
}

/// The player's walking speed under levels of speed and of slowness and what `paced` adds; and
/// that speed less the freeze's share, which the server's own step adds itself. The adds are listed
/// first and go before the effects multiply (research/captures/bds-r1fx.txt).
fn pace(id: u64, quick: Option<u8>, slow: Option<u8>, paced: Paced, tick: u64) -> (UpdateAttributes, f32) {
    let adds = |amount: f32, id: &str, name: &str, serializable: bool| (amount != 0.0).then(|| PlayerAttributesItemModifiersItem { id: id.into(), name: name.into(), amount, operation: 0, operand: 2, serializable });
    let times = effect_modifiers(quick, slow);
    let freeze = adds(paced.freeze * acacia_physics::constants::FREEZE_SPEED_MODIFIER, FREEZES_AS, "Freeze effect", true);
    let modifiers: Vec<_> = [freeze, adds(paced.soles, SOLES_AS, "Sprinting speed boost", false)].into_iter().chain(times).flatten().collect();
    let speed = |base: f32| modifiers.iter().filter(|modifier| modifier.operation == 2).fold(base, |pace, modifier| pace * (1.0 + modifier.amount)).max(0.0);
    let added: f32 = modifiers.iter().filter(|modifier| modifier.operation == 0).map(|modifier| modifier.amount).sum();
    let unfrozen = speed(WALKS + paced.soles);
    let movement = PlayerAttributesItem { min: 0.0, max: f32::MAX, current: speed(WALKS + added), default_min: 0.0, default_max: f32::MAX, default: WALKS, name: "minecraft:movement".into(), modifiers };
    (UpdateAttributes { runtime_entity_id: id, attributes: vec![movement], tick }, unfrozen)
}

/// The modifiers levels of speed and of slowness put on a walking speed, a player's or a mob's.
fn effect_modifiers(quick: Option<u8>, slow: Option<u8>) -> [Option<PlayerAttributesItemModifiersItem>; 2] {
    let modifier = |levels: Option<u8>, id: &str, name: &str, each: f32| {
        levels.map(|levels| PlayerAttributesItemModifiersItem { id: id.into(), name: name.into(), amount: each * (f32::from(levels) + 1.0), operation: 2, operand: 2, serializable: false })
    };
    [modifier(quick, QUICKENS_AS, "MovementSpeed", QUICKENS), modifier(slow, SLOWS_AS, "MovementSlowdown", -SLOWS)]
}

/// A mob's walking speed of `base` under levels of speed and of slowness, as BDS told its viewers
/// (research/captures/bds-mob-effects.txt: a cow's 0.25 at 0.35 under speed II, 0 under slowness 255).
pub(crate) fn mob_walking(id: u64, base: f32, quick: Option<u8>, slow: Option<u8>) -> UpdateAttributes {
    let modifiers: Vec<_> = effect_modifiers(quick, slow).into_iter().flatten().collect();
    let current = modifiers.iter().fold(base, |pace, modifier| pace * (1.0 + modifier.amount)).max(0.0);
    let movement = PlayerAttributesItem { min: 0.0, max: f32::MAX, current, default_min: 0.0, default_max: f32::MAX, default: base, name: "minecraft:movement".into(), modifiers };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![movement], tick: 0 }
}

/// A player's walking speed now, as it is told.
pub(crate) fn walking(world: &World, player: Entity, id: u64) -> PlayerAttributesItem {
    let (quick, slow) = (level(world, player, Effect::Speed), level(world, player, Effect::Slowness));
    let paced = world.get::<Paced>(player).copied().unwrap_or_default();
    pace(id, quick, slow, paced, 0).0.attributes.remove(0)
}

/// Tells a player its walking speed now, and has the server's step take it.
pub(crate) fn tell_pace(world: &mut World, player: Entity, id: u64) {
    let tick = world.resource::<Tick>().0;
    let told = UpdateAttributes { runtime_entity_id: id, attributes: vec![take_pace(world, player, id)], tick };
    crate::vitals::tell(world, player, &told);
}

/// Has the server's step take a player's walking speed now, and gives it as it is told.
pub(super) fn take_pace(world: &mut World, player: Entity, id: u64) -> PlayerAttributesItem {
    let (quick, slow) = (level(world, player, Effect::Speed), level(world, player, Effect::Slowness));
    let paced = world.get::<Paced>(player).copied().unwrap_or_default();
    let (mut told, unfrozen) = pace(id, quick, slow, paced, 0);
    if let Some(mut simulated) = world.get_mut::<crate::simulation::Simulated>(player) {
        simulated.set_speed(unfrozen);
    }
    told.attributes.remove(0)
}

/// The health on top of a player's own.
pub(crate) fn cushion(id: u64, amount: f32, tick: u64) -> UpdateAttributes {
    let absorption = PlayerAttributesItem { min: 0.0, max: MOST_CUSHION, current: amount, default_min: 0.0, default_max: MOST_CUSHION, default: 0.0, name: "minecraft:absorption".into(), modifiers: Vec::new() };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![absorption], tick }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_freeze_and_soul_speed_add_to_the_walk_as_bds_told_them() {
        // research/captures/bds-freeze.txt at full freeze, bds-soles.txt on soul sand in Soul Speed III.
        let [(frozen, unfrozen), (soled, _)] = [Paced { freeze: 1.0, soles: 0.0 }, Paced { freeze: 0.0, soles: 0.0615 }].map(|paced| pace(1, None, None, paced, 0)).map(|(told, unfrozen)| (told.attributes[0].clone(), unfrozen));
        assert_eq!((frozen.current, frozen.modifiers[0].name.as_str(), frozen.modifiers[0].amount, unfrozen), (0.05, "Freeze effect", -0.05, WALKS));
        assert_eq!((soled.current, soled.modifiers[0].name.as_str(), soled.modifiers[0].operation), (0.1615, "Sprinting speed boost", 0));
    }

    #[test]
    fn the_adds_are_listed_first_and_go_before_speed_and_slowness_multiply() {
        // research/captures/bds-r1fx.txt: 0.22610001 and 0.19218501 on soul sand; 0.1395 a tick into powder snow.
        let told = |quick, slow, paced| pace(1, quick, slow, paced, 0).0.attributes[0].clone();
        let soled = told(Some(1), Some(0), Paced { freeze: 0.0, soles: 0.0615 });
        let names: Vec<&str> = soled.modifiers.iter().map(|modifier| modifier.name.as_str()).collect();
        assert!((soled.current - 0.19218501).abs() < 1e-6, "{}", soled.current);
        assert_eq!(names, ["Sprinting speed boost", "MovementSpeed", "MovementSlowdown"]);
        let frozen = told(Some(1), None, Paced { freeze: 1.0 / 140.0, soles: 0.0 });
        assert!((frozen.current - 0.1395).abs() < 1e-6, "{}", frozen.current);
        assert_eq!(frozen.modifiers[0].name, "Freeze effect");
    }
}
