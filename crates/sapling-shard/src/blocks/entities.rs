//! What a block placed by a player holds from the start, as BDS writes it on placement, and what
//! its item gets back from it when it breaks (docs/research/block-entities.md).

use acacia_proto::nbt::{List, Str, Value};
use sapling_api::{BlockPos, Item};
use sapling_world::new_block_entity;

const SKULLS: &[&str] = &[
    "minecraft:skeleton_skull", "minecraft:wither_skeleton_skull", "minecraft:zombie_head", "minecraft:player_head", "minecraft:creeper_head", "minecraft:dragon_head",
    "minecraft:piglin_head",
];
const BANNERS: &[&str] = &["minecraft:standing_banner", "minecraft:wall_banner"];
const CAMPFIRES: &[&str] = &["minecraft:campfire", "minecraft:soul_campfire"];
/// Opaque black, ARGB.
const BLACK: i32 = -16_777_216;
/// Items that keep a field of their block's entity as their damage.
const KEPT_AS_DAMAGE: &[(&str, &str)] = &[("minecraft:bed", "color"), ("minecraft:banner", "Base")];
/// `TAG_Compound`, the element tag of an empty list of stacks.
const COMPOUND: u8 = 10;

/// How a block was placed, as far as its entity tells: the item's damage (a bed's or a
/// banner's colour), the placer's yaw and the block's properties.
pub(super) struct Placing<'a> {
    pub damage: i16,
    pub yaw: f32,
    pub properties: &'a str,
}

fn sign_side() -> Value {
    let fields = [
        ("IgnoreLighting", Value::Byte(0)),
        ("HideGlowOutline", Value::Byte(0)),
        ("SignTextColor", Value::Int(BLACK)),
        ("PersistFormatting", Value::Byte(1)),
        ("TextOwner", Value::String(Str::default())),
        ("Text", Value::String(Str::default())),
    ];
    Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect())
}

/// A floor skull's turn: the placer's yaw in sixteenths, unwrapped (BDS `142efa9c0`).
fn skull_rotation(placing: &Placing) -> f32 {
    let on_floor = placing.properties.split(',').any(|pair| pair == "facing_direction=1");
    if on_floor && placing.yaw.is_finite() { (placing.yaw * 16.0 / 360.0 + 0.5).floor() * 22.5 } else { 0.0 }
}

/// The entity the block `block` gets when placed at `pos`; `None` for a block without one.
/// Containers get theirs from containers.rs.
pub(super) fn placed(block: &str, pos: BlockPos, placing: &Placing) -> Option<Value> {
    let (id, fields) = match block {
        "minecraft:bed" => ("Bed", vec![("color", Value::Byte(placing.damage as i8))]),
        _ if block.ends_with("_hanging_sign") => ("HangingSign", vec![("FrontText", sign_side()), ("BackText", sign_side()), ("IsWaxed", Value::Byte(0))]),
        _ if block.ends_with("_sign") => ("Sign", vec![("FrontText", sign_side()), ("BackText", sign_side()), ("IsWaxed", Value::Byte(0))]),
        _ if SKULLS.contains(&block) => {
            // BDS leaves the kind unset: the client takes it from the block's name.
            let fields = [("SkullType", Value::Byte(-1)), ("Rotation", Value::Float(skull_rotation(placing))), ("DoingAnimation", Value::Byte(0)), ("MouthTickCount", Value::Int(0))];
            ("Skull", fields.into())
        }
        _ if BANNERS.contains(&block) => ("Banner", vec![("Base", Value::Int(placing.damage.into())), ("Type", Value::Int(0))]),
        "minecraft:flower_pot" => ("FlowerPot", Vec::new()),
        "minecraft:cauldron" => {
            let items = Value::List(List { tag: COMPOUND, items: Vec::new() });
            ("Cauldron", vec![("Items", items), ("PotionId", Value::Short(-1)), ("PotionType", Value::Short(-1))])
        }
        _ if CAMPFIRES.contains(&block) => ("Campfire", ["ItemTime1", "ItemTime2", "ItemTime3", "ItemTime4"].map(|time| (time, Value::Int(0))).into()),
        _ => return None,
    };
    let mut entity = new_block_entity(id, pos);
    if let Value::Compound(entries) = &mut entity {
        entries.extend(fields.into_iter().map(|(key, value)| (key.into(), value)));
    }
    Some(entity)
}

fn number(value: Option<&Value>) -> Option<i16> {
    match value? {
        Value::Byte(v) => Some((*v).into()),
        Value::Short(v) => Some(*v),
        Value::Int(v) => i16::try_from(*v).ok(),
        _ => None,
    }
}

/// The items a broken block's entity adds to what it leaves: a flower pot's plant, the food on
/// a campfire, and what an item frame or a decorated pot holds. Each by its name only: what else the stack carried is lost.
fn held_items(kept: &Value) -> Vec<Item> {
    let plant = match kept.get("PlantBlock").and_then(|plant| plant.get("name")) {
        Some(Value::String(name)) if sapling_data::is_item(name) => Some(Item::new(name.clone(), 1)),
        _ => None,
    };
    let food = ["Item1", "Item2", "Item3", "Item4", super::holders::FRAMED, super::holders::POTTED].into_iter().filter_map(|slot| sapling_world::ItemStack::parse(kept.get(slot)?));
    plant.into_iter().chain(food.map(|stack| Item { damage: stack.damage, ..Item::new(stack.name, stack.count) })).collect()
}

/// Gives `items`, which a broken block leaves, what its entity `kept` that they carry: a bed's
/// and a banner's colour, a pot's plant, a campfire's food.
pub(super) fn into_loot(kept: &Value, items: &mut Vec<Item>) {
    for item in items.iter_mut() {
        let field = KEPT_AS_DAMAGE.iter().find(|(name, _)| *name == item.name).and_then(|(_, field)| number(kept.get(field)));
        if let Some(damage) = field {
            item.damage = damage;
        }
    }
    items.extend(held_items(kept));
}
