//! What a player's melee hit adds to its weapon's harm, on a mob (attacks.rs) or a player
//! (duels.rs): a critical hit as it falls (BDS `Player::attack` 1402b84b0), a sprint's throw,
//! and the weapon's wear.

use acacia_proto::packets::{Animate, AnimateActionId, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_world::{Tool, ToolKind};

use crate::components::Mode;
use crate::inventory::Inventory;
use crate::movement::Motion;

/// A critical hit costs half again (research/captures/bds-crit.txt: a fist's 1 cost 1.5).
const CRITICAL: f32 = 1.5;
/// What BDS sent as the critical hit's `data` (bds-crit.txt).
const CRITICAL_DATA: f32 = 55.0;
/// A player's sounds come from the middle of its box.
const HALF_PLAYER: f32 = 0.9;

/// How a player's hit lands: its harm, made critical where the player falls, and whether the
/// player sprints, which throws further (`sapling_data::enchant::knockback`).
#[derive(Clone, Copy)]
pub(super) struct Blow {
    pub harm: f32,
    pub critical: bool,
    pub sprinting: bool,
}

/// The blow of a player whose weapon costs `harm`.
pub(super) fn blow(world: &World, player: Entity, harm: f32) -> Blow {
    let critical = crate::vitals::falls_freely(world, player);
    let sprinting = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sprinting);
    Blow { harm: if critical { harm * CRITICAL } else { harm }, critical, sprinting }
}

/// The sound of the blow, heard from the middle of the hitter at `feet`.
pub(super) fn sound(blow: Blow, [x, y, z]: [f32; 3]) -> LevelSoundEvent {
    let name = if blow.critical { "attack.critical" } else { "attack.strong" };
    LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y: y + HALF_PLAYER, z }, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None }
}

/// What a critical blow that landed shows on what it struck, the one known as `id`.
pub(super) fn sparks(blow: Blow, id: u64) -> Option<Animate> {
    blow.critical.then_some(Animate { action_id: AnimateActionId::CriticalHit, runtime_entity_id: id, data: CRITICAL_DATA, has_swing_source: false, swing_source: None })
}

/// A blow wears what the hitter holds, landed or not, unless the hitter is in creative: a sword by
/// one, another tool by two (research/captures/bds-swing-wear.txt: a wooden sword's 57 went to 58,
/// an axe's and a pickaxe's to 59, a sword's twice in two hits in one tick). BDS showed the hitter
/// all its windows after any hit.
pub(super) fn wear(world: &mut World, player: Entity) {
    let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == sapling_api::GameMode::Creative);
    let tool = world.get::<Inventory>(player).and_then(|inventory| Tool::from_identifier(&inventory.in_hand().as_ref()?.stack.name));
    match tool.filter(|_| !creative) {
        Some(tool) => crate::inventory::wear_hand_by(world, player, if tool.kind == ToolKind::Sword { 1 } else { 2 }, tool.max_durability()),
        None => crate::inventory::show_windows(world, player, false),
    }
}
