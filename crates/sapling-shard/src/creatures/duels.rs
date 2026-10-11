//! A player's hit on another player, as BDS's landed (docs/research/player-damage.md, "Another
//! player's hit"): what the hand holds costs as it does a mob, on any difficulty, where the
//! `pvp` rule is on.
// TODO: a weapon does not wear by it, and a sprinting hitter throws no further.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::components::{InLayer, Layer, Mode, NetId, Position, in_world, in_world_identity};
use crate::inventory::Inventory;
use crate::vitals;

const RULE: &str = "pvp";
/// The furthest from a hitter's eye to the middle of the one it hits.
const REACH: f32 = 4.0;
const HALF_PLAYER: f32 = 0.9;
/// A hit throws this fast away and up.
const THROW: f32 = 0.4;

/// The player `hitter` hits the player its client knows as `id`; `None` where that is no
/// player, so that the hit may be a mob's.
pub(super) fn hit(world: &mut World, hitter: Entity, id: u64) -> Option<()> {
    let struck = in_world(world).into_iter().find(|player| *player != hitter && world.get::<NetId>(*player).is_some_and(|net| net.0 == id))?;
    let (layer, from, to) = (world.get::<InLayer>(hitter)?.0, world.get::<Position>(hitter)?.feet, world.get::<Position>(struck)?.feet);
    let far = (0..3).map(|axis| (to[axis] + if axis == 1 { HALF_PLAYER - sapling_data::EYE_HEIGHT } else { 0.0 } - from[axis]).powi(2)).sum::<f32>().sqrt();
    let is_on = world.get::<Layer>(layer)?.world.level().rules.on(RULE);
    if !is_on || world.get::<InLayer>(struck)?.0 != layer || world.get::<Mode>(hitter)?.0 == sapling_api::GameMode::Spectator || far > REACH {
        return Some(());
    }
    let [x, y, z] = from;
    let swing = LevelSoundEvent { sound_id: "attack.strong".into(), position: Vec3f { x, y: y + HALF_PLAYER, z }, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    crate::blocks::to_viewers(world, layer, to.map(|c| c.floor() as i32), &[sapling_net::encode(&swing)]);
    let held = world.get::<Inventory>(hitter).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone()));
    let harm = vitals::hits_for(world, hitter, sapling_data::item_attack(held.as_deref()));
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let apart = dx.hypot(dz);
    let away = if apart > 1e-4 { [dx / apart * THROW, dz / apart * THROW] } else { [0.0; 2] };
    let name = in_world_identity(world, hitter)?.name;
    if vitals::punched(world, struck, harm, &name, [away[0], THROW, away[1]]) {
        vitals::tire(world, hitter, vitals::HIT_TIRES);
    }
    Some(())
}
