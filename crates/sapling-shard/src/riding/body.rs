//! `slot.armor.body`, the body slot any mob has (research/captures/bds-bodyslot.txt): horse or wolf
//! armour goes on any mob, tame or wild (every mount, a pig, a wolf), shown in
//! MobArmorEquipment and unheard, and stays on at death. Anything else (stone, a helmet, a carpet,
//! even on a llama) answers success and is shown to no one; air takes the armour off, told
//! even where there was none. Armour put in a tamed mount's slot 1 fills the body slot too
//! (riding/packs.rs `wear`); this leaves slot 1 alone.
// TODO: a world folder keeps only slot 1's armour, not what this alone put on.

use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::mounts::{Tack, body_shown};
use super::packs::HORSE_ARMOR;
use crate::components::NetId;
use crate::creatures::Creature;

const WOLF_ARMOR: &str = "minecraft:wolf_armor";

/// Whether `name` is armour a body slot shows.
fn is_body_armor(name: &str) -> bool {
    let metal = name.strip_prefix("minecraft:").and_then(|bare| bare.strip_suffix("_horse_armor"));
    name == WOLF_ARMOR || metal.is_some_and(|metal| HORSE_ARMOR.contains(&metal))
}

/// Puts `item` in a mob's body slot as `/replaceitem entity ... slot.armor.body` does; false for
/// an entity that is no mob.
pub(crate) fn put(world: &mut World, mob: Entity, item: Option<ItemStack>) -> bool {
    let Some(id) = world.get::<NetId>(mob).map(|id| id.0).filter(|_| world.get::<Creature>(mob).is_some()) else { return false };
    if item.as_ref().is_some_and(|item| !is_body_armor(&item.name)) {
        return true;
    }
    let worn = item.map(|item| ItemStack { count: 1, ..item });
    let mut tack = world.get::<Tack>(mob).cloned().unwrap_or_default();
    tack.body = worn.clone();
    world.entity_mut(mob).insert(tack);
    super::tell(world, mob, mob, vec![body_shown(id, worn.as_ref())]);
    super::retell(world, mob);
    true
}
