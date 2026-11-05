//! A player's items as plugins name them: [`Slot`]s and [`Item`]s.

use acacia_proto::nbt::{List, Str, Value};
use bevy_ecs::prelude::*;
use sapling_api::{Item, ItemLock, Slot};
use sapling_world::{CAN_DESTROY, CAN_PLACE_ON, ItemStack};

use super::{Inventory, Place, change};

const ENCH: &str = "ench";
pub(super) const KEEP_ON_DEATH: &str = "minecraft:keep_on_death";
/// 1 locks a stack in its slot, 2 in the inventory (research/captures/bds-itemcmds.txt).
const ITEM_LOCK: &str = "minecraft:item_lock";
const DAMAGE: &str = "Damage";
/// `TAG_Compound`, the element tag of `ench`, and `TAG_String`, of the block lists.
const COMPOUND: u8 = 10;
const STRING: u8 = 8;

fn place(slot: Slot) -> Place {
    match slot {
        Slot::Main(slot) => Place::Main(slot.into()),
        Slot::Armor(slot) => Place::Armor(slot.into()),
        Slot::Offhand => Place::Offhand,
    }
}

/// A plugin's item as a stack: a block item places its block's first state.
pub(crate) fn stored(item: &Item, registry: &sapling_world::BlockRegistry) -> Option<ItemStack> {
    stored_over(item, registry, None)
}

/// [`stored`], keeping what `previous`, the tag of the stack it replaces, holds beyond the item's
/// own fields.
fn stored_over(item: &Item, registry: &sapling_world::BlockRegistry, previous: Option<&Value>) -> Option<ItemStack> {
    let first = registry.states_of(&item.name).next().map(|(id, _)| id).or_else(|| crate::blocks::custom::placed_by(&item.name));
    let block = first.map(|id| sapling_world::block_hash(registry, id));
    let mut tag = tag(item, previous);
    let worn = wears(&item.name);
    // Even unworn: `/replaceitem ... iron_helmet` came with `{Damage: 0}` (research/captures/bds-t2edon.txt).
    if worn {
        super::wear::set_damage(&mut tag, item.damage.into());
    }
    let damage = if worn { 0 } else { item.damage };
    sapling_data::is_item(&item.name).then(|| ItemStack { name: item.name.clone(), count: item.count, damage, block, tag })
}

/// Whether the item wears with use, its wear kept in its `Damage` tag, not its data value: BDS's
/// `/replaceitem ... iron_helmet 1 164` left a helmet one hit from breaking (bds-hurt2.txt).
fn wears(name: &str) -> bool {
    sapling_data::armour_lasts(name).is_some() || sapling_data::gear(name).is_some_and(|gear| gear.lasts > 0)
}

fn tag(item: &Item, previous: Option<&Value>) -> Option<Value> {
    let mut fields: Vec<(Str, Value)> = match previous {
        Some(Value::Compound(fields)) => fields.iter().filter(|(key, _)| ![ENCH, KEEP_ON_DEATH, ITEM_LOCK, DAMAGE, CAN_PLACE_ON, CAN_DESTROY].contains(&key.as_str())).cloned().collect(),
        _ => Vec::new(),
    };
    let enchantment = |&(id, level): &(i16, i16)| Value::Compound(vec![(Str::from("id"), Value::Short(id)), (Str::from("lvl"), Value::Short(level))]);
    if !item.enchantments.is_empty() {
        fields.push((Str::from(ENCH), Value::List(List { tag: COMPOUND, items: item.enchantments.iter().map(enchantment).collect() })));
    }
    if item.keep_on_death {
        fields.push((Str::from(KEEP_ON_DEATH), Value::Byte(1)));
    }
    if let Some(lock) = item.lock {
        fields.push((Str::from(ITEM_LOCK), Value::Byte(if lock == ItemLock::InSlot { 1 } else { 2 })));
    }
    for (key, blocks) in [(CAN_PLACE_ON, &item.can_place_on), (CAN_DESTROY, &item.can_destroy)].into_iter().filter(|(_, blocks)| !blocks.is_empty()) {
        fields.push((Str::from(key), Value::List(List { tag: STRING, items: blocks.iter().map(|block| Value::String(block.as_str().into())).collect() })));
    }
    // BDS's `/give` and `/replaceitem` banners carry `{Type: 0}` (research/captures/bds-banners.txt, bds-item-names.txt).
    if item.name == "minecraft:banner" && !fields.iter().any(|(key, _)| key.as_str() == "Type") {
        fields.push((Str::from("Type"), Value::Int(0)));
    }
    (!fields.is_empty()).then_some(Value::Compound(fields))
}

pub(crate) fn described(stack: &ItemStack) -> Item {
    let field = |key: &str| stack.tag.as_ref().and_then(|tag| tag.get(key));
    let short = |entry: &Value, key: &str| if let Some(Value::Short(value)) = entry.get(key) { Some(*value) } else { None };
    let enchantments = match field(ENCH) {
        Some(Value::List(list)) => list.items.iter().filter_map(|entry| short(entry, "id").zip(short(entry, "lvl"))).collect(),
        _ => Vec::new(),
    };
    let lock = match field(ITEM_LOCK) {
        Some(Value::Byte(1)) => Some(ItemLock::InSlot),
        Some(Value::Byte(2)) => Some(ItemLock::InInventory),
        _ => None,
    };
    let keep_on_death = matches!(field(KEEP_ON_DEATH), Some(Value::Byte(1)));
    let damage = if wears(&stack.name) { super::wear::damage(&stack.tag) as i16 } else { stack.damage };
    let (can_place_on, can_destroy) = (blocks_in(stack, CAN_PLACE_ON), blocks_in(stack, CAN_DESTROY));
    Item { damage, enchantments, keep_on_death, lock, can_place_on, can_destroy, ..Item::new(stack.name.clone(), stack.count) }
}

/// Whether the stack in the player's hand lets it, in adventure mode, place on (`destroy` false)
/// or break (`destroy` true) the block `name`.
pub(crate) fn hand_allows(world: &World, player: Entity, destroy: bool, name: &str) -> bool {
    let key = if destroy { CAN_DESTROY } else { CAN_PLACE_ON };
    world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).is_some_and(|held| blocks_in(&held.stack, key).iter().any(|block| block == name))
}

/// The block names a stack's list `key` holds ([`CAN_PLACE_ON`] or [`CAN_DESTROY`]).
pub(crate) fn blocks_in(stack: &ItemStack, key: &str) -> Vec<String> {
    match stack.tag.as_ref().and_then(|tag| tag.get(key)) {
        Some(Value::List(list)) => list.items.iter().filter_map(|item| if let Value::String(name) = item { Some(name.to_string()) } else { None }).collect(),
        _ => Vec::new(),
    }
}

/// What a slot holds, for a plugin.
pub(crate) fn item_at(world: &World, player: Entity, slot: Slot) -> Option<Item> {
    world.get::<Inventory>(player)?.slot(place(slot))?.as_ref().map(|held| described(&held.stack))
}

/// Sets a slot for a plugin, the item of the same name it replaces keeping its other NBT (a
/// custom name); false for a slot that is not there and an item the game does not have.
pub(crate) fn set_item(world: &mut World, player: Entity, slot: Slot, item: Option<&Item>) -> bool {
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.slot(place(slot))?.as_ref().map(|held| held.stack.clone()));
    let previous = held.filter(|held| item.is_some_and(|item| item.name == held.name)).and_then(|held| held.tag);
    let layer = crate::worlds::current(world);
    let stack = match item {
        Some(item) => match world.get::<crate::components::Layer>(layer).and_then(|layer| stored_over(item, layer.world.registry(), previous.as_ref())) {
            Some(stack) => Some(stack),
            None => return false,
        },
        None => None,
    };
    change(world, player, |inventory| inventory.put(place(slot), stack)) == Some(true)
}
