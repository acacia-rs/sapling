//! A player's hit on another player, as BDS's landed (docs/research/player-damage.md, "Another
//! player's hit"): what the hand holds costs as it does a mob, on any difficulty, where the
//! `pvp` rule is on; Sharpness, Knockback and Fire Aspect add as they do on a mob, and the
//! struck player's Thorns hurt back. A falling hitter's is critical, a sprinting one's throws
//! further, and the weapon wears (blows.rs); a raised shield turns it away (vitals/shield.rs).
//! A mace smashes (smash.rs) and its Breach pierces armour.

use bevy_ecs::prelude::*;
use sapling_data::enchant::{self, Enchant};

use super::{blows, smash};
use crate::components::{InLayer, Layer, Mode, NetId, Position, in_world, in_world_identity};
use crate::inventory::Inventory;
use crate::vitals;

const RULE: &str = "pvp";
/// The furthest from a hitter's eye to the middle of the one it hits.
const REACH: f32 = 4.0;
const HALF_PLAYER: f32 = 0.9;

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
    let held = world.get::<Inventory>(hitter).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone()));
    let weapon = crate::enchants::weapon(world, hitter);
    let smash = smash::smash(world, hitter, held.as_deref());
    let bonus = enchant::melee_bonus(weapon.as_ref(), None) + smash::bonus(smash, weapon.as_ref());
    let blow = blows::blow(world, hitter, vitals::hits_for(world, hitter, sapling_data::item_attack(held.as_deref()) + bonus));
    let cell = to.map(|c| c.floor() as i32);
    crate::blocks::to_viewers(world, layer, cell, &[sapling_net::encode(&blows::sound(blow, from))]);
    let pierced = enchant::mace::breach(enchant::level(weapon.as_ref(), Enchant::Breach));
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let apart = dx.hypot(dz);
    let away = if apart > 1e-4 { [dx / apart, dz / apart] } else { [0.0; 2] };
    // The server keeps no motion of a player's: it is thrown as from standing.
    let thrown = enchant::knockback(enchant::level(weapon.as_ref(), Enchant::Knockback), blow.sprinting, [0.0; 3], away);
    let name = in_world_identity(world, hitter)?.name;
    let axe = held.as_deref().is_some_and(|held| held.ends_with("_axe"));
    let blocked = vitals::blocks(world, struck, from, blow.harm, super::Cause::Attack, axe);
    blows::wear(world, hitter);
    if !blocked && vitals::punched(world, struck, blow.harm, &name, thrown, pierced) {
        if let Some(sparks) = blows::sparks(blow, id) {
            crate::blocks::to_viewers(world, layer, cell, &[sapling_net::encode(&sparks)]);
        }
        if let Some(smash) = smash {
            crate::blocks::to_viewers(world, layer, cell, &smash::shown(smash, from, to));
            smash::follow(world, hitter, struck, to, weapon.as_ref());
        }
        vitals::tire(world, hitter, vitals::HIT_TIRES);
        vitals::set_alight(world, struck, enchant::aspect_burn(enchant::level(weapon.as_ref(), Enchant::FireAspect)));
        crate::enchants::thorns(world, struck, hitter);
        super::owner_hurt(world, struck, hitter);
    }
    Some(())
}
