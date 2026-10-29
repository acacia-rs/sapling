//! What the workstations need to know of an item: which enchantments it takes (BDS's
//! `EnchantSlot` bits), how readily the enchanting table enchants it, how many uses it lasts and
//! what repairs it at an anvil (docs/research/workstations.md, "Items").

use acacia_world::Tool;

/// BDS's `EnchantSlot` bits: which items an enchantment goes on.
pub mod slot {
    pub const HEAD: u32 = 1 << 0;
    pub const TORSO: u32 = 1 << 1;
    pub const FEET: u32 = 1 << 2;
    pub const LEGS: u32 = 1 << 3;
    pub const SWORD: u32 = 1 << 4;
    pub const BOW: u32 = 1 << 5;
    pub const HOE: u32 = 1 << 6;
    pub const SHEARS: u32 = 1 << 7;
    pub const FLINT_AND_STEEL: u32 = 1 << 8;
    pub const AXE: u32 = 1 << 9;
    pub const PICKAXE: u32 = 1 << 10;
    pub const SHOVEL: u32 = 1 << 11;
    pub const FISHING_ROD: u32 = 1 << 12;
    pub const CARROT_STICK: u32 = 1 << 13;
    pub const ELYTRA: u32 = 1 << 14;
    pub const TRIDENT: u32 = 1 << 15;
    pub const CROSSBOW: u32 = 1 << 16;
    pub const SHIELD: u32 = 1 << 17;
    pub const COSMETIC_HEAD: u32 = 1 << 18;
    pub const COMPASS: u32 = 1 << 19;
    pub const FUNGUS_STICK: u32 = 1 << 20;
    pub const BRUSH: u32 = 1 << 21;
    pub const MACE: u32 = 1 << 22;
    pub const SPEAR: u32 = 1 << 23;
    pub const ARMOUR: u32 = HEAD | TORSO | FEET | LEGS;
    pub const DIGGING: u32 = HOE | AXE | PICKAXE | SHOVEL;
    pub const ALL: u32 = u32::MAX;
}

/// An item the workstations work on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gear {
    pub slots: u32,
    /// 0: the enchanting table does not enchant it.
    pub enchantability: u8,
    /// Uses it lasts; 0 for one that does not wear (a book).
    pub lasts: u16,
    /// What repairs it at an anvil: an item, or a tag (`#` first).
    pub repair: &'static [&'static str],
}

/// What a workstation makes: a stack's name and tag, its count and aux value the shard's to say.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    pub name: String,
    pub tag: Option<acacia_proto::nbt::Value>,
}

/// `tag` with `key` set to `value`, or gone for `None`; a tag left empty is no tag.
pub fn with_field(tag: Option<acacia_proto::nbt::Value>, key: &str, value: Option<acacia_proto::nbt::Value>) -> Option<acacia_proto::nbt::Value> {
    use acacia_proto::nbt::Value;
    let mut fields = match tag {
        Some(Value::Compound(fields)) => fields,
        _ => Vec::new(),
    };
    match (fields.iter_mut().find(|(name, _)| name == key), value) {
        (Some((_, old)), Some(value)) => *old = value,
        (None, Some(value)) => fields.push((key.into(), value)),
        (_, None) => fields.retain(|(name, _)| name != key),
    }
    (!fields.is_empty()).then_some(Value::Compound(fields))
}

const DAMAGE: &str = "Damage";

/// The uses a stack whose NBT is `tag` has been worn by.
pub fn damage(tag: Option<&acacia_proto::nbt::Value>) -> i32 {
    match tag.and_then(|tag| tag.get(DAMAGE)) {
        Some(acacia_proto::nbt::Value::Int(damage)) => *damage,
        _ => 0,
    }
}

/// `tag` worn by `damage` uses; unworn, without the field.
pub fn with_damage(tag: Option<acacia_proto::nbt::Value>, damage: i32) -> Option<acacia_proto::nbt::Value> {
    with_field(tag, DAMAGE, (damage > 0).then_some(acacia_proto::nbt::Value::Int(damage)))
}

pub const BOOK: &str = "minecraft:book";
pub const ENCHANTED_BOOK: &str = "minecraft:enchanted_book";

/// Tools and armour by material: the tools' and the armour's enchantability, what repairs them.
const MATERIALS: &[(&str, u8, u8, &[&str])] = &[
    ("wooden", 15, 0, &["#minecraft:planks"]),
    ("stone", 5, 0, &["minecraft:cobblestone", "minecraft:blackstone", "minecraft:cobbled_deepslate"]),
    ("copper", 13, 8, &["minecraft:copper_ingot"]),
    ("iron", 14, 9, &["minecraft:iron_ingot"]),
    ("golden", 22, 25, &["minecraft:gold_ingot"]),
    ("diamond", 10, 10, &["minecraft:diamond"]),
    ("netherite", 15, 15, &["minecraft:netherite_ingot"]),
    ("leather", 0, 15, &["minecraft:leather"]),
    ("chainmail", 0, 12, &["minecraft:iron_ingot"]),
];

const PIECES: [(&str, u32); 4] = [("helmet", slot::HEAD), ("chestplate", slot::TORSO), ("leggings", slot::LEGS), ("boots", slot::FEET)];

/// Items that are neither tiered tools nor armour of a material.
const OTHERS: &[(&str, u32, u8, u16, &[&str])] = &[
    ("minecraft:book", slot::ALL, 1, 0, &[]),
    ("minecraft:enchanted_book", slot::ALL, 0, 0, &[]),
    ("minecraft:bow", slot::BOW, 1, 384, &[]),
    ("minecraft:crossbow", slot::CROSSBOW, 1, 465, &[]),
    ("minecraft:trident", slot::TRIDENT, 1, 250, &[]),
    ("minecraft:fishing_rod", slot::FISHING_ROD, 1, 384, &[]),
    ("minecraft:shears", slot::SHEARS, 0, 238, &[]),
    ("minecraft:flint_and_steel", slot::FLINT_AND_STEEL, 0, 64, &[]),
    ("minecraft:shield", slot::SHIELD, 0, 336, &["#minecraft:planks"]),
    ("minecraft:elytra", slot::ELYTRA, 0, 432, &["minecraft:phantom_membrane"]),
    ("minecraft:carrot_on_a_stick", slot::CARROT_STICK, 1, 25, &[]),
    ("minecraft:warped_fungus_on_a_stick", slot::FUNGUS_STICK, 1, 100, &[]),
    ("minecraft:mace", slot::MACE, 15, 500, &["minecraft:breeze_rod"]),
    ("minecraft:brush", slot::BRUSH, 0, 64, &[]),
    ("minecraft:turtle_helmet", slot::HEAD, 9, 275, &["minecraft:turtle_scute"]),
    ("minecraft:compass", slot::COMPASS, 0, 0, &[]),
    ("minecraft:recovery_compass", slot::COMPASS, 0, 0, &[]),
    ("minecraft:carved_pumpkin", slot::COSMETIC_HEAD, 0, 0, &[]),
];

fn tool_slot(tool: Tool) -> u32 {
    use acacia_world::ToolKind;
    match tool.kind {
        ToolKind::Sword => slot::SWORD,
        ToolKind::Shovel => slot::SHOVEL,
        ToolKind::Pickaxe => slot::PICKAXE,
        ToolKind::Axe => slot::AXE,
        ToolKind::Hoe => slot::HOE,
        ToolKind::Shears => slot::SHEARS,
    }
}

/// What the workstations know of the item `name` (namespaced); `None` for one they do nothing with.
pub fn gear(name: &str) -> Option<Gear> {
    if let Some(custom) = crate::custom_items::by_name(name) {
        const WORN: [u32; 4] = [slot::HEAD, slot::TORSO, slot::LEGS, slot::FEET];
        let slots = custom.armour.map_or(0, |(at, _)| WORN[usize::from(at.min(3))]);
        return custom.durability.map(|lasts| Gear { slots, enchantability: 0, lasts, repair: &[] });
    }
    if let Some((_, slots, enchantability, lasts, repair)) = OTHERS.iter().find(|(known, ..)| *known == name) {
        return Some(Gear { slots: *slots, enchantability: *enchantability, lasts: *lasts, repair });
    }
    let (material, piece) = name.strip_prefix("minecraft:")?.rsplit_once('_')?;
    let (_, tools, armour, repair) = MATERIALS.iter().find(|(known, ..)| *known == material)?;
    if let Some(tool) = Tool::from_identifier(name).filter(|tool| tool.tier.is_some()) {
        return Some(Gear { slots: tool_slot(tool), enchantability: *tools, lasts: tool.max_durability(), repair });
    }
    let (_, slots) = PIECES.iter().find(|(known, _)| *known == piece)?;
    Some(Gear { slots: *slots, enchantability: *armour, lasts: crate::armour_lasts(name)?, repair })
}

impl Gear {
    /// Whether `item` repairs it at an anvil.
    pub fn is_repaired_by(&self, item: &str) -> bool {
        self.repair.iter().any(|by| by.strip_prefix('#').map_or(*by == item, |tag| crate::has_tag(item, tag)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gear_is_known_by_name() {
        let sword = gear("minecraft:diamond_sword").unwrap();
        assert_eq!((sword.slots, sword.enchantability, sword.lasts), (slot::SWORD, 10, 1561));
        assert!(sword.is_repaired_by("minecraft:diamond") && !sword.is_repaired_by("minecraft:iron_ingot"));
        let helmet = gear("minecraft:golden_helmet").unwrap();
        assert_eq!((helmet.slots, helmet.enchantability, helmet.lasts), (slot::HEAD, 25, 77));
        assert!(gear("minecraft:wooden_pickaxe").unwrap().is_repaired_by("minecraft:birch_planks"));
        assert_eq!(gear("minecraft:book").map(|book| book.enchantability), Some(1));
        assert_eq!([gear("minecraft:stone"), gear("minecraft:iron_ingot"), gear("minecraft:leather_sword")], [None; 3]);
    }
}
