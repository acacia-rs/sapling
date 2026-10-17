//! Armour wears by the hits it stops, as BDS's did (docs/research/player-damage.md, "Armour
//! wear"): each piece by a quarter of the hit, the helmet by far more of what falls on it.
// TODO: a piece that breaks goes without a sound, and nobody else sees it go.

use acacia_proto::packets::{InventorySlot, PlayerArmorDamage};
use acacia_proto::types::{ArmorDamageEntry, ArmorDamageEntryArmorSlot, WindowIDVarint};
use bevy_ecs::prelude::*;

use super::wear::{damage, set_damage, wear_chance};
use super::{Inventory, wire};
use crate::blocks::Dice;
use crate::components::Outgoing;

/// A hit wears each piece by its amount over this, and by one at least.
const SHARE: f32 = 4.0;
/// What falls on a helmet wears it by this many times the hit more, and by up to this many
/// times the hit above that.
const ON_HEAD: f32 = 4.0;
const ON_HEAD_MORE: f32 = 2.0;

/// A hit of `amount` that armour stops wears what a player has on; `on_head`, it fell on the
/// helmet. The client is told what each piece has lost, and sent the helmet anew if it was hit.
pub(crate) fn wear_armour(world: &mut World, player: Entity, amount: f32, on_head: bool) {
    let (rolls, more): ([u32; 4], Option<i32>) = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (std::array::from_fn(|_| dice.roll(100)), on_head.then(|| (amount * ON_HEAD + dice.fraction() * amount * ON_HEAD_MORE) as i32))
    };
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    let Ok((mut inventory, mut out)) = query.get_mut(world, player) else { return };
    if inventory.armor.iter().all(Option::is_none) {
        return;
    }
    let share = ((amount / SHARE) as i32).max(1);
    let mut broke = false;
    for piece in 0..inventory.armor.len() {
        let Some(held) = inventory.armor[piece].as_mut() else { continue };
        let Some(lasts) = sapling_data::armour_lasts(&held.stack.name) else { continue };
        let uses = if rolls[piece] < wear_chance(held) { share } else { 0 } + more.filter(|_| piece == 0).unwrap_or(0);
        let worn = damage(&held.stack.tag) + uses;
        if worn > i32::from(lasts) {
            (inventory.armor[piece], broke) = (None, true);
        } else if uses > 0 {
            set_damage(&mut held.stack.tag, worn);
        }
    }
    inventory.unsaved = true;
    if !inventory.sent {
        return;
    }
    // The slots go as 0, 2, 4 and 6, as BDS sent them, which the packet's names do not fit;
    // what fell on the helmet is not in it.
    let unsaid = |piece: usize| more.filter(|_| piece == 0).unwrap_or(0);
    let lost = |piece: usize| inventory.armor[piece].as_ref().map(|held| ArmorDamageEntry { armor_slot: ArmorDamageEntryArmorSlot::from_raw(piece as i64 * 2), damage: (damage(&held.stack.tag) - unsaid(piece)) as i16 });
    out.push(&PlayerArmorDamage { entries: (0..inventory.armor.len()).filter_map(lost).collect() });
    if broke {
        inventory.send_all(&mut out);
    } else if more.is_some() && inventory.armor[0].is_some() {
        let id = inventory.new_id();
        inventory.armor[0].as_mut().expect("worn").id = id;
        out.push(&InventorySlot { window_id: WindowIDVarint::Armor, slot: 0, container: None, storage_item: None, item: wire(&inventory.armor[0], true) });
    }
}
