//! A crossbow loaded, held in use for 25 ticks less 5 a level of Quick Charge, and shot at the next
//! use (BDS's `CrossbowItem::use` 1436ec1a0): every arrow critical, three with Multishot, of which
//! the side ones are never picked up, through as many more bodies as Piercing's level. The loaded
//! arrow lies in the crossbow's `chargedItem`, as the client shows it; it wears 3 with Multishot.
// TODO: `chargedItem` as remembered of Bedrock's item tag; no capture of a loaded crossbow was read.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_data::enchant::{self, Enchant};
use sapling_data::shots::{self, CROSSBOW_SPEED};
use sapling_world::ItemStack;

use super::ammo;
use super::kinds::PLAYER_ARROW;
use crate::creatures::{Loosed, Pickup};

pub(super) const CROSSBOW: &str = "minecraft:crossbow";
const CHARGED: &str = "chargedItem";
const MULTISHOT_WEAR: i32 = 3;

/// The arrow a crossbow is loaded with.
fn loaded(stack: &ItemStack) -> Option<ItemStack> {
    let charged = stack.tag.as_ref()?.get(CHARGED)?;
    let Some(Value::String(name)) = charged.get("Name") else { return None };
    let damage = match charged.get("Damage") {
        Some(Value::Short(damage)) => *damage,
        _ => 0,
    };
    Some(ItemStack { name: name.to_string(), count: 1, damage, block: None, tag: None })
}

/// `stack` loaded with `arrow`, or emptied.
fn charged(stack: &ItemStack, arrow: Option<&ItemStack>) -> ItemStack {
    let mut tag = stack.tag.clone().unwrap_or(Value::Compound(Vec::new()));
    if let Value::Compound(fields) = &mut tag {
        fields.retain(|(key, _)| key != CHARGED);
        if let Some(arrow) = arrow {
            let item = vec![("Name".into(), Value::String(arrow.name.as_str().into())), ("Count".into(), Value::Byte(1)), ("Damage".into(), Value::Short(arrow.damage)), ("WasPickedUp".into(), Value::Byte(0))];
            fields.push((CHARGED.into(), Value::Compound(item)));
        }
    }
    let tag = match tag {
        Value::Compound(fields) if fields.is_empty() => None,
        tag => Some(tag),
    };
    ItemStack { tag, ..stack.clone() }
}

/// Whether the player may load the `stack` it holds: an empty crossbow, with an arrow or in creative.
pub(super) fn may_load(world: &World, player: Entity, stack: &ItemStack) -> bool {
    stack.name == CROSSBOW && loaded(stack).is_none() && (super::is_creative(world, player) || ammo::find(world, player).is_some())
}

/// The crossbow `stack` let go after `ticks` of loading: loaded, if that was long enough.
pub(super) fn load(world: &mut World, player: Entity, stack: &ItemStack, ticks: u64) {
    if ticks < enchant::charge_ticks(enchant::level(stack.tag.as_ref(), Enchant::QuickCharge)) {
        return;
    }
    let creative = super::is_creative(world, player);
    let Some((place, arrow)) = ammo::find(world, player).map(|(place, arrow)| (Some(place), arrow)).or_else(|| creative.then(|| (None, ammo::plain()))) else { return };
    if let Some(place) = place.filter(|_| !creative) {
        ammo::spend(world, player, place);
    }
    super::set_hand(world, player, Some(charged(stack, Some(&arrow))));
    super::heard(world, player, "crossbow.loading.end");
}

/// The player shoots the loaded crossbow `stack` it holds; true when it was one.
pub(super) fn shoot(world: &mut World, player: Entity, stack: &ItemStack) -> bool {
    let Some(arrow) = loaded(stack).filter(|_| stack.name == CROSSBOW) else { return false };
    let creative = super::is_creative(world, player);
    let tag = stack.tag.as_ref();
    let turns = enchant::multishot(enchant::level(tag, Enchant::Multishot));
    let pierces = enchant::piercing(enchant::level(tag, Enchant::Piercing));
    for (index, turn) in turns.iter().enumerate() {
        let pickup = match (index, creative) {
            (0, false) => Pickup::Anyone,
            (0, true) => Pickup::Creative,
            _ => Pickup::Never,
        };
        let loosed = Loosed { critical: true, tipped: ammo::tip(&arrow), pierces, ..Loosed::new(player, pickup, Some(arrow.clone())) };
        super::loose(world, player, PLAYER_ARROW, CROSSBOW_SPEED, 0.0, 1.0, *turn, loosed);
    }
    super::set_hand(world, player, Some(charged(stack, None)));
    if !creative {
        let uses = if turns.len() > 1 { MULTISHOT_WEAR } else { 1 };
        crate::inventory::wear_hand_by(world, player, uses, shots::shooter_lasts(CROSSBOW).unwrap_or(u16::MAX));
    }
    super::heard(world, player, "crossbow.shoot");
    true
}
