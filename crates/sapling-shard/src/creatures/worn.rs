//! The armour a mob wears (helmet, chestplate, leggings, boots, as a world folder's `Armor`): shown
//! in `MobArmorEquipment` and worn against hits over its own skin, as BDS's zombie in a diamond
//! chestplate took 5.824 of a diamond sword's 8 (research/captures/bds-hits2.txt).
// TODO: it is not written back to a world folder, worn down, enchanted or dropped.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ItemStack;

use super::Creature;
use super::health::Health;
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities::{self, Kind};
use crate::inventory::worn_in;

const PIECES: usize = 4;
/// Kinds with 2 points of armour of their own: BDS's zombie and baby zombie took 0.94 of fire's 1
/// and 7.872 of a sword's 8 (research/captures/bds-burn2.txt, bds-hits2.txt); the rest of the family
/// as the Java game's, not drilled.
const SKINNED: [&str; 4] = ["minecraft:zombie", "minecraft:husk", "minecraft:drowned", "minecraft:zombie_villager_v2"];
const SKIN: f32 = 2.0;

#[derive(Component, Clone, Default)]
pub(super) struct Worn([Option<ItemStack>; PIECES]);

/// The points and toughness of the armour a mob of `kind` has of its own.
pub(super) fn skin(kind: &str) -> [f32; 2] {
    [if SKINNED.contains(&kind) { SKIN } else { 0.0 }, 0.0]
}

fn stack(item: &ItemStack) -> sapling_data::Stack<'_> {
    sapling_data::Stack { name: &item.name, count: item.count, damage: item.damage, block: item.block, tag: item.tag.as_ref() }
}

/// A piece in a slot that does not wear it (stone on the feet) is shown to no one, as BDS's
/// `/replaceitem` zombie (research/captures/bds-fixreplace.txt).
fn shown_as(id: u64, worn: &Worn) -> Bytes {
    let slots: Vec<_> = worn.0.iter().enumerate().map(|(slot, piece)| piece.as_ref().filter(|piece| worn_in(&piece.name) == Some(slot)).map(stack)).collect();
    sapling_net::encode(&sapling_data::mob_armor(id, &slots))
}

/// What tells a viewer what the mob `id` wears, where its stored look does not already.
pub(super) fn shown(world: &World, entity: Entity, id: u64) -> Option<Bytes> {
    let looks = world.get::<Creature>(entity).is_some_and(|creature| creature.look.is_some());
    world.get::<Worn>(entity).filter(|worn| !looks && worn.0.iter().any(Option::is_some)).map(|worn| shown_as(id, worn))
}

/// Has the mob's health wear its skin and its armour.
fn dress(world: &mut World, mob: Entity) {
    let kind = world.get::<Kind>(mob).map(|kind| kind.0.clone()).unwrap_or_default();
    let pieces = world.get::<Worn>(mob).map(|worn| worn.0.iter().flatten().filter_map(|piece| sapling_data::armour(&piece.name)).collect::<Vec<_>>()).unwrap_or_default();
    let armour = pieces.into_iter().fold(skin(&kind), |[points, toughness], [more, tougher]| [points + more, toughness + tougher]);
    if let Some(mut health) = world.get_mut::<Health>(mob) {
        health.wear(armour, Default::default());
    }
}

/// A mob a world folder kept wears what it was stored in.
pub(super) fn adopt(world: &mut World, mob: Entity, armor: &[Option<ItemStack>]) {
    if armor.iter().any(Option::is_some) {
        world.entity_mut(mob).insert(Worn(std::array::from_fn(|piece| armor.get(piece).cloned().flatten())));
        dress(world, mob);
    }
}

/// Puts `item` on the living mob as its `piece` (0 the helmet to 3 the boots), or takes that piece
/// off; its viewers see it and its health wears it. False for a piece past the boots.
pub(crate) fn put_on(world: &mut World, mob: Entity, piece: usize, item: Option<ItemStack>) -> bool {
    let Some((shown, _)) = wear(world, mob, piece, item) else { return false };
    let (Some(layer), Some(at)) = (world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<Body>(mob).map(|body| body.at)) else { return true };
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), shown.into_iter().collect());
    true
}

/// [`put_on`] telling no one: what would show the mob's armour now, and the piece taken off.
/// `None` where [`put_on`] is false.
pub(super) fn wear(world: &mut World, mob: Entity, piece: usize, item: Option<ItemStack>) -> Option<(Option<Bytes>, Option<ItemStack>)> {
    if piece >= PIECES || world.get::<Health>(mob).is_none() || world.get::<super::Dying>(mob).is_some() {
        return None;
    }
    let mut worn = world.get::<Worn>(mob).cloned().unwrap_or_default();
    let off = std::mem::replace(&mut worn.0[piece], item);
    if let Some(look) = world.get_mut::<Creature>(mob).and_then(|creature| creature.into_inner().look.as_mut()) {
        look.armor = worn.0.to_vec();
    }
    world.entity_mut(mob).insert(worn.clone());
    dress(world, mob);
    super::shown::reshow(world, mob);
    Some((world.get::<NetId>(mob).map(|id| shown_as(id.0, &worn)), off))
}
