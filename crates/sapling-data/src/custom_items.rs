//! Custom items plugins register (`sapling_api::CustomItem`): network ids after the recorded
//! registry's, ItemRegistry entries whose components are laid out as BDS lays out its own
//! data-driven items' (`minecraft:apple`, `minecraft:iron_spear` in ItemRegistry.bin), and what the
//! item tables answer for them. Registration is for the whole process and only adds: a name
//! registered again keeps its first id (docs/research/custom-content.md).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, RwLock};

use acacia_proto::nbt::{List, Nbt, Str, Value};
use acacia_proto::types::{ItemstatesItem, ItemstatesItemVersion};
use sapling_api::{CreativeCategory, CustomItem};

use crate::food::Food;
use crate::items::NETWORK_IDS;

/// A custom item or a custom block's item, as registered.
#[derive(Debug)]
pub(crate) struct Registered {
    pub name: &'static str,
    pub display_name: &'static str,
    pub id: i16,
    pub entry: ItemstatesItem,
    pub max_stack: u8,
    pub food: Option<Food>,
    pub durability: Option<u16>,
    /// Armour slot (0 head to 3 feet) and points.
    pub armour: Option<(u8, f32)>,
    pub attack: Option<f32>,
    pub category: CreativeCategory,
    pub group: String,
    /// The network hash of the block it places, for a block's item.
    pub block: Option<u32>,
}

static ITEMS: RwLock<Vec<Arc<Registered>>> = RwLock::new(Vec::new());
/// Bumped on each registration, so that the registry packets are made again.
static GENERATION: AtomicUsize = AtomicUsize::new(0);
static VANILLA_TOP: LazyLock<i16> = LazyLock::new(|| NETWORK_IDS.values().copied().max().unwrap_or(0));

pub(crate) fn generation() -> usize {
    GENERATION.load(Ordering::Acquire)
}

pub(crate) fn all() -> Vec<Arc<Registered>> {
    ITEMS.read().expect("custom items lock").clone()
}

/// `None` at once for a `minecraft:` name, which no custom item has.
pub(crate) fn by_name(name: &str) -> Option<Arc<Registered>> {
    if name.starts_with("minecraft:") || generation() == 0 {
        return None;
    }
    ITEMS.read().expect("custom items lock").iter().find(|item| item.name == name).cloned()
}

pub(crate) fn by_id(id: i16) -> Option<Arc<Registered>> {
    ITEMS.read().expect("custom items lock").iter().find(|item| item.id == id).cloned()
}

/// Leaks `name` once: item names are `&'static str` everywhere.
fn leak(name: &str) -> &'static str {
    Box::leak(name.to_owned().into_boxed_str())
}

fn insert(make: impl FnOnce(&[Arc<Registered>]) -> Registered, name: &str) -> i16 {
    let mut items = ITEMS.write().expect("custom items lock");
    if let Some(known) = items.iter().find(|item| item.name == name) {
        return known.id;
    }
    let item = make(&items);
    let id = item.id;
    items.push(Arc::new(item));
    GENERATION.fetch_add(1, Ordering::Release);
    id
}

/// Registers a custom item; its network id. A name the recorded registry has is refused.
pub fn register_item(item: &CustomItem) -> Option<i16> {
    if NETWORK_IDS.contains_key(&item.identifier) || item.identifier.starts_with("minecraft:") {
        return None;
    }
    Some(insert(|items| {
        let next = items.iter().map(|known| known.id).chain([*VANILLA_TOP]).max().unwrap_or(0) + 1;
        let name = leak(&item.identifier);
        let food = item.food.map(|food| Food {
            hunger: f32::from(food.nutrition),
            saturation: f32::from(food.nutrition) * food.saturation_modifier * 2.0,
            always: food.can_always_eat,
            ticks: food.use_ticks,
            ..crate::food::plain(0.0, 0.0)
        });
        let entry = ItemstatesItem { name: name.to_owned(), runtime_id: next, component_based: true, version: ItemstatesItemVersion::DataDriven, nbt: Nbt { name: String::new(), value: components(item) }.into() };
        Registered {
            name,
            display_name: leak(&item.display_name),
            id: next,
            entry,
            max_stack: item.max_stack,
            food,
            durability: item.durability,
            armour: item.armour.map(|(slot, points)| (slot.index(), f32::from(points))),
            attack: item.attack_damage.map(f32::from),
            category: item.category,
            group: item.group.clone(),
            block: None,
        }
    }, &item.identifier))
}

/// Registers the item of a custom block numbered `block_id`, placing the state `hash`: id
/// `255 - block_id`, as BDS numbers the items of blocks past 255.
pub fn register_block_item(name: &str, display_name: &str, block_id: i32, hash: u32, category: CreativeCategory, group: &str) -> i16 {
    insert(|_| {
        let id = i16::try_from(255 - block_id).unwrap_or(i16::MIN);
        let name = leak(name);
        let entry = ItemstatesItem { name: name.to_owned(), runtime_id: id, component_based: false, version: ItemstatesItemVersion::None, nbt: Nbt { name: String::new(), value: Value::Compound(Vec::new()) }.into() };
        Registered { name, display_name: leak(display_name), id, entry, max_stack: 64, food: None, durability: None, armour: None, attack: None, category, group: group.to_owned(), block: Some(hash) }
    }, name)
}

fn compound(fields: Vec<(&str, Value)>) -> Value {
    Value::Compound(fields.into_iter().map(|(key, value)| (Str::from(key), value)).collect())
}

fn text(value: &str) -> Value {
    Value::String(Str::from(value))
}

fn flag(on: bool) -> Value {
    Value::Byte(i8::from(on))
}

/// `{components: {...}}` as BDS sends a data-driven item's.
fn components(item: &CustomItem) -> Value {
    let (use_animation, use_duration) = if item.food.is_some() { (1, item.food.map_or(0, |food| food.use_ticks as i32)) } else { (0, 0) };
    let properties = compound(vec![
        ("allow_off_hand", flag(item.allow_off_hand)),
        ("can_destroy_in_creative", flag(true)),
        ("creative_category", Value::Int(item.category.id())),
        ("creative_group", text(&item.group)),
        ("damage", Value::Int(item.attack_damage.map_or(0, i32::from))),
        ("enchantable_slot", text("none")),
        ("enchantable_value", Value::Int(0)),
        ("foil", flag(item.glint)),
        ("frame_count", Value::Int(1)),
        ("hand_equipped", flag(item.hand_equipped)),
        ("hidden_in_commands", Value::Byte(2)),
        ("liquid_clipped", flag(false)),
        ("max_stack_size", Value::Int(i32::from(item.max_stack))),
        ("minecraft:icon", compound(vec![("textures", compound(vec![("default", text(&item.identifier))]))])),
        ("mining_speed", Value::Float(1.0)),
        ("should_despawn", flag(true)),
        ("stacked_by_data", flag(false)),
        ("use_animation", Value::Int(use_animation)),
        ("use_duration", Value::Int(use_duration)),
    ]);
    let mut fields = vec![
        ("item_properties", properties),
        ("item_tags", Value::List(List { tag: 8, items: Vec::new() })),
        ("minecraft:display_name", compound(vec![("value", text(&item.display_name))])),
        ("minecraft:max_stack_size", compound(vec![("value", Value::Byte(item.max_stack as i8))])),
    ];
    if let Some(food) = item.food {
        fields.push(("minecraft:food", compound(vec![
            ("can_always_eat", flag(food.can_always_eat)),
            ("nutrition", Value::Int(i32::from(food.nutrition))),
            ("saturation_modifier", Value::Float(food.saturation_modifier)),
            ("using_converts_to", compound(Vec::new())),
        ])));
        fields.push(("minecraft:use_animation", compound(vec![("value", text("eat"))])));
        fields.push(("minecraft:use_modifiers", compound(vec![
            ("emit_vibrations", flag(true)),
            ("movement_modifier", Value::Float(0.35)),
            ("start_using", text("always")),
            ("use_duration", Value::Float(food.use_ticks as f32 / 20.0)),
        ])));
    }
    if let Some(uses) = item.durability {
        let chance = compound(vec![("max", Value::Int(100)), ("min", Value::Int(0))]);
        fields.push(("minecraft:durability", compound(vec![("damage_chance", chance), ("max_durability", Value::Int(i32::from(uses)))])));
    }
    if let Some((slot, points)) = item.armour {
        fields.push(("minecraft:wearable", compound(vec![("protection", Value::Int(i32::from(points))), ("slot", text(slot.slot_name()))])));
        fields.push(("minecraft:armor", compound(vec![("protection", Value::Int(i32::from(points)))])));
    }
    if item.hand_equipped {
        fields.push(("minecraft:hand_equipped", compound(vec![("value", flag(true))])));
    }
    if let Some(damage) = item.attack_damage {
        fields.push(("minecraft:damage", compound(vec![("value", Value::Short(i16::from(damage)))])));
    }
    if let Some((category, ticks)) = &item.cooldown {
        fields.push(("minecraft:cooldown", compound(vec![("category", text(category)), ("duration", Value::Float(*ticks as f32 / 20.0)), ("type", text("use"))])));
    }
    if item.glint {
        fields.push(("minecraft:glint", compound(vec![("value", flag(true))])));
    }
    fields.sort_by(|a, b| a.0.cmp(b.0));
    compound(vec![("components", compound(fields))])
}

#[cfg(test)]
mod tests {
    use sapling_api::{ArmourSlot, CustomFood};

    use super::*;

    #[test]
    fn custom_items_take_ids_after_the_recorded_registry_and_keep_them() {
        let ruby = CustomItem::new("test_items:ruby", "Ruby", Vec::new()).max_stack(16);
        let id = register_item(&ruby).unwrap();
        assert!(id > *VANILLA_TOP);
        assert_eq!(register_item(&ruby), Some(id), "registered again, the same id");
        assert_eq!(register_item(&CustomItem::new("minecraft:stone", "Stone", Vec::new())), None);
        let helmet = CustomItem::new("test_items:ruby_helmet", "Ruby Helmet", Vec::new()).armour(ArmourSlot::Head, 3).durability(200);
        let pie = CustomItem::new("test_items:pie", "Pie", Vec::new()).food(CustomFood::new(8, 0.6));
        let (helmet_id, _) = (register_item(&helmet).unwrap(), register_item(&pie).unwrap());
        assert!(helmet_id > id);
        let helmet = by_name("test_items:ruby_helmet").unwrap();
        assert_eq!((helmet.max_stack, helmet.armour, helmet.durability), (1, Some((0, 3.0)), Some(200)));
        let components = helmet.entry.nbt.decode().unwrap().value;
        let components = components.get("components").unwrap();
        assert_eq!(components.get("minecraft:wearable").and_then(|wearable| wearable.get("slot")), Some(&text("slot.armor.head")));
        let pie = by_name("test_items:pie").unwrap().food.unwrap();
        assert_eq!((pie.hunger, (pie.saturation * 10.0).round()), (8.0, 96.0));
    }
}
