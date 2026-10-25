//! A bow drawn and let go (BDS's `BowItem::releaseUsing` 142eee8d0): the draw, `(s² + 2s) / 3` of
//! its seconds up to 1, looses nothing under a tenth and is critical at a full one. Power, Punch and
//! Flame go with the arrow; outside creative an arrow is spent, but a plain one with Infinity, and
//! the bow wears. An arrow from creative or Infinity only a creative player picks up.

use bevy_ecs::prelude::*;
use sapling_data::enchant::{self, Enchant, FLAME_BURNS};
use sapling_data::shots::{self, BOW_SPEED, LEAST_DRAW};
use sapling_world::ItemStack;

use super::ammo;
use super::kinds::PLAYER_ARROW;
use crate::creatures::{Loosed, Pickup, Shot};

pub(super) const BOW: &str = "minecraft:bow";

/// Whether the player may draw the `stack` it holds: a bow, with an arrow or in creative.
pub(super) fn may_draw(world: &World, player: Entity, stack: &ItemStack) -> bool {
    stack.name == BOW && (super::is_creative(world, player) || ammo::find(world, player).is_some())
}

/// The bow `stack` let go after `ticks` of drawing.
pub(super) fn loose(world: &mut World, player: Entity, stack: &ItemStack, ticks: u64) {
    let draw = shots::bow_draw(ticks);
    let creative = super::is_creative(world, player);
    let found = ammo::find(world, player);
    if draw < LEAST_DRAW || (found.is_none() && !creative) {
        return;
    }
    let arrow = found.as_ref().map_or_else(ammo::plain, |(_, arrow)| arrow.clone());
    let tag = stack.tag.as_ref();
    let tipped = ammo::tip(&arrow);
    let infinite = enchant::level(tag, Enchant::Infinity) > 0 && tipped.is_none();
    let pickup = if creative || infinite { Pickup::Creative } else { Pickup::Anyone };
    let burns = if enchant::level(tag, Enchant::Flame) > 0 { FLAME_BURNS } else { 0 };
    let shot = Shot { burns, ..PLAYER_ARROW };
    let loosed = Loosed {
        power: enchant::level(tag, Enchant::Power),
        critical: draw >= 1.0,
        punch: enchant::level(tag, Enchant::Punch),
        tipped,
        ..Loosed::new(player, pickup, Some(arrow))
    };
    if super::loose(world, player, shot, BOW_SPEED * draw, 0.0, 1.0, 0.0, loosed).is_none() {
        return;
    }
    if !creative {
        if let Some((place, _)) = found.filter(|_| !infinite) {
            ammo::spend(world, player, place);
        }
        crate::inventory::wear_hand_by(world, player, 1, shots::shooter_lasts(BOW).unwrap_or(u16::MAX));
    }
    super::heard(world, player, "bow");
}
