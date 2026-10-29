//! What a block placed by a player holds from the start, as BDS writes it on placement, and what
//! its item gets back from it when it breaks (docs/research/block-entities.md).

use acacia_proto::nbt::{List, Str, Value};
use sapling_api::{BlockPos, Item};
use sapling_world::{ItemStack, new_block_entity};

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
/// banner's colour) and tag (a banner's patterns), the placer's yaw and the block's properties.
pub(super) struct Placing<'a> {
    pub damage: i16,
    pub tag: Option<&'a Value>,
    pub yaw: f32,
    pub properties: &'a str,
}

const PATTERNS: &str = "Patterns";
const BANNER_TYPE: &str = "Type";
/// BDS's `BannerBlockActor` reads at most this many of an item's patterns (141f0cad0).
const MAX_PATTERNS: usize = 6;

/// A banner's entity fields from the item placing it: its colour by damage, its kind (1 ominous)
/// and patterns by tag.
fn banner(placing: &Placing) -> Vec<(&'static str, Value)> {
    let kind = match placing.tag.and_then(|tag| tag.get(BANNER_TYPE)) {
        Some(Value::Int(kind)) => *kind,
        _ => 0,
    };
    let mut fields = vec![("Base", Value::Int(placing.damage.into())), (BANNER_TYPE, Value::Int(kind))];
    if let Some(Value::List(patterns)) = placing.tag.and_then(|tag| tag.get(PATTERNS)).filter(|patterns| matches!(patterns, Value::List(list) if !list.items.is_empty())) {
        fields.push((PATTERNS, Value::List(List { tag: COMPOUND, items: patterns.items.iter().take(MAX_PATTERNS).cloned().collect() })));
    }
    fields
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
        _ if BANNERS.contains(&block) => ("Banner", banner(placing)),
        "minecraft:flower_pot" => ("FlowerPot", Vec::new()),
        "minecraft:bell" => ("Bell", super::bells::UNRUNG.into()),
        "minecraft:chiseled_bookshelf" => ("ChiseledBookshelf", Vec::new()),
        "minecraft:beacon" => ("Beacon", vec![("primary", Value::Int(0)), ("secondary", Value::Int(0))]),
        "minecraft:conduit" => ("Conduit", vec![("Active", Value::Byte(0)), ("Target", Value::Long(-1))]),
        "minecraft:cauldron" => {
            let items = Value::List(List { tag: COMPOUND, items: Vec::new() });
            ("Cauldron", vec![("Items", items), ("PotionId", Value::Short(-1)), ("PotionType", Value::Short(-1))])
        }
        _ if CAMPFIRES.contains(&block) => ("Campfire", ["ItemTime1", "ItemTime2", "ItemTime3", "ItemTime4"].map(|time| (time, Value::Int(0))).into()),
        // Bare: spawners.rs reads BDS's defaults where a field is missing.
        "minecraft:mob_spawner" => ("MobSpawner", Vec::new()),
        "minecraft:trial_spawner" => ("TrialSpawner", Vec::new()),
        "minecraft:vault" => ("Vault", Vec::new()),
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

/// The stacks a broken block's entity adds to what it leaves, each whole: a flower pot's plant,
/// the food on a campfire, what an item frame, a decorated pot, a jukebox or a lectern holds and
/// a chiseled bookshelf's books.
fn held_items(kept: &Value) -> Vec<ItemStack> {
    let plant = match kept.get("PlantBlock").and_then(|plant| plant.get("name")) {
        Some(Value::String(name)) if sapling_data::is_item(name) => Some(ItemStack { name: name.to_string(), count: 1, damage: 0, block: None, tag: None }),
        _ => None,
    };
    let held = ["Item1", "Item2", "Item3", "Item4", super::holders::FRAMED, super::holders::POTTED, super::jukebox::RECORD, super::lectern::BOOK].into_iter().filter_map(|slot| ItemStack::parse(kept.get(slot)?));
    plant.into_iter().chain(held).chain(super::bookshelves::books(kept)).chain(pot_loot(kept)).collect()
}

/// A broken decorated pot's unrolled loot, rolled into its one slot as a container's is
/// (research/captures/bds-loot-probe.txt: BDS's pot left it beside itself).
fn pot_loot(kept: &Value) -> Vec<ItemStack> {
    let (Some(Value::String(id)), Some(Value::String(table))) = (kept.get("id"), kept.get("LootTable")) else { return Vec::new() };
    let seed = match kept.get("LootTableSeed") {
        Some(Value::Int(seed)) => *seed as u32,
        _ => 0,
    };
    if id != "DecoratedPot" {
        return Vec::new();
    }
    crate::containers::loot::rolled(table, seed, vec![0]).into_iter().map(|(_, (name, count))| ItemStack { name, count: count.min(64) as u8, damage: 0, block: None, tag: None }).collect()
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
    items.extend(held_items(kept).into_iter().map(|stack| Item { damage: stack.damage, ..Item::new(stack.name, stack.count) }));
}

/// The tags a broken block's entity `kept` gives the items it leaves, in their order, by item: a
/// banner's `{BlockEntityVersion, Patterns, Type}`, plain or not (research/captures/bds-banners.txt),
/// each held stack's own (a written book's pages, an item's enchantments), `None` for an untagged one.
pub(super) fn loot_tags(kept: &Value) -> Vec<(String, Option<Value>)> {
    let mut tags: Vec<_> = held_items(kept).into_iter().map(|stack| (stack.name, stack.tag)).collect();
    if kept.get("id") != Some(&Value::String("Banner".into())) {
        return tags;
    }
    let patterns = kept.get(PATTERNS).filter(|patterns| matches!(patterns, Value::List(list) if !list.items.is_empty()));
    let kind = kept.get(BANNER_TYPE).cloned().unwrap_or(Value::Int(0));
    let mut fields = vec![("BlockEntityVersion".into(), Value::Int(0))];
    fields.extend(patterns.map(|patterns| (PATTERNS.into(), patterns.clone())));
    fields.push((BANNER_TYPE.into(), kind));
    tags.insert(0, ("minecraft:banner".into(), Some(Value::Compound(fields))));
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_banner_takes_its_items_patterns_and_gives_them_back() {
        let layer = |pattern: &str| Value::Compound(vec![("Color".into(), Value::Int(1)), ("Pattern".into(), Value::String(pattern.into()))]);
        let patterns = Value::List(List { tag: COMPOUND, items: ["bo", "cr", "bl", "br", "tl", "tr", "ts"].map(layer).into() });
        let tag = Value::Compound(vec![(PATTERNS.into(), patterns), (BANNER_TYPE.into(), Value::Int(0))]);
        let placing = Placing { damage: 15, tag: Some(&tag), yaw: 0.0, properties: "" };
        let entity = placed("minecraft:standing_banner", [0, 64, 0], &placing).unwrap();
        let Some(Value::List(kept)) = entity.get(PATTERNS) else { panic!("{entity:?}") };
        assert_eq!(kept.items.len(), MAX_PATTERNS);
        let [(item, given)] = loot_tags(&entity).try_into().unwrap();
        let given = given.expect("the banner's tag");
        assert_eq!((item.as_str(), given.get(PATTERNS).is_some(), given.get(BANNER_TYPE)), ("minecraft:banner", true, Some(&Value::Int(0))));
        let plain = placed("minecraft:wall_banner", [0, 64, 0], &Placing { tag: None, ..placing }).unwrap();
        let tags: [(String, Option<Value>); 1] = loot_tags(&plain).try_into().unwrap();
        let [(_, Some(Value::Compound(given)))] = tags else { panic!("a plain banner's tag") };
        assert_eq!(given, vec![("BlockEntityVersion".into(), Value::Int(0)), (BANNER_TYPE.into(), Value::Int(0))]);
    }
}
