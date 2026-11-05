//! Armour put on from the hand by a click in the air, as BDS's (research/captures/bds-fx3.txt):
//! the piece goes to its slot and the hand takes what was there, a worn piece too, Binding or not
//! (bds-n13items.txt, in creative); heard as any piece put on.

use bevy_ecs::prelude::*;

use super::{Inventory, Place};

/// The armour slot (head 0 to feet 3) an item is worn in. Heads, skulls and carved pumpkins go on
/// the head by name, which every vanilla one fits.
pub(crate) fn worn_in(name: &str) -> Option<usize> {
    let plain = name.strip_prefix("minecraft:")?;
    match plain {
        "turtle_helmet" | "carved_pumpkin" => Some(0),
        "elytra" => Some(1),
        _ if plain.ends_with("_helmet") || plain.ends_with("_head") || plain.ends_with("_skull") => Some(0),
        _ if plain.ends_with("_chestplate") => Some(1),
        _ if plain.ends_with("_leggings") => Some(2),
        _ if plain.ends_with("_boots") => Some(3),
        _ => None,
    }
}

/// Whether an item worn on the head is a block: a click in the air does not put one on
/// (research/captures/bds-t2edon.txt: a pumpkin and five heads stayed in the hand).
fn is_head_block(name: &str) -> bool {
    let plain = name.strip_prefix("minecraft:").unwrap_or(name);
    plain == "carved_pumpkin" || plain.ends_with("_head") || plain.ends_with("_skull")
}

/// Puts on the armour in the player's hand; false if it holds none.
pub(crate) fn wear_from_hand(world: &mut World, player: Entity) -> bool {
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.name.clone());
    let Some(piece) = held.as_deref().filter(|name| !is_head_block(name)).and_then(worn_in) else { return false };
    super::change(world, player, |inventory| {
        let hand = Place::Main(inventory.selected.into());
        let [held, worn] = [hand, Place::Armor(piece)].map(|place| inventory.slot(place).cloned().flatten().map(|held| held.stack));
        inventory.put(Place::Armor(piece), held);
        inventory.put(hand, worn);
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_go_to_their_slots() {
        assert_eq!(["minecraft:diamond_helmet", "minecraft:iron_boots", "minecraft:elytra", "minecraft:iron_sword"].map(worn_in), [Some(0), Some(3), Some(1), None]);
        assert!(["minecraft:carved_pumpkin", "minecraft:dragon_head", "minecraft:skeleton_skull"].iter().all(|head| worn_in(head) == Some(0) && is_head_block(head)) && !is_head_block("minecraft:turtle_helmet"));
        // bds-glide2.txt: an elytra put on is heard.
        assert_eq!(super::super::equip_sound_name("minecraft:elytra").as_deref(), Some("armor.equip_elytra"));
    }
}
