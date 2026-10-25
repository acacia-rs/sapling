//! Every enchantment as `/enchant` names it: its id, its word, the key of its name, its highest
//! level, what it goes on and what it cannot be with. The levels and the ones BDS refused are in
//! research/captures/bds-itemcmds.txt (sharpness at most 5, smite beside sharpness, protection
//! and knockback on a sword, sharpness on stone, fortune beside silk touch; any on a book);
//! the rest is the game's table as players know it, not read from BDS.

/// What an enchantment goes on, by the item's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gear {
    Head,
    Chest,
    Legs,
    Feet,
    Sword,
    Axe,
    Pickaxe,
    Shovel,
    Hoe,
    Bow,
    Crossbow,
    Trident,
    FishingRod,
    Mace,
    Spear,
    Shears,
    /// Other items that wear: a shield, elytra, flint and steel, a brush, the rods on a stick.
    Worn,
    /// What is worn on the head that does not wear: a pumpkin, a skull.
    Headwear,
}

use Gear::*;

const ARMOUR: &[Gear] = &[Head, Chest, Legs, Feet];
const BLADES: &[Gear] = &[Sword, Axe];
const DIGGERS: &[Gear] = &[Axe, Pickaxe, Shovel, Hoe];
const WEARING: &[Gear] = &[Head, Chest, Legs, Feet, Sword, Axe, Pickaxe, Shovel, Hoe, Bow, Crossbow, Trident, FishingRod, Mace, Spear, Shears, Worn];
const ALL: &[Gear] = &[Head, Chest, Legs, Feet, Sword, Axe, Pickaxe, Shovel, Hoe, Bow, Crossbow, Trident, FishingRod, Mace, Spear, Shears, Worn, Headwear];

#[derive(Debug, PartialEq, Eq)]
pub struct Kind {
    pub id: i16,
    /// The word `/enchant` takes.
    pub word: &'static str,
    /// The key of its name in the game's strings.
    pub key: &'static str,
    pub max: i16,
    pub goes_on: &'static [Gear],
    /// Enchantments of the same group but 0 cannot be on one item together.
    group: u8,
}

const fn kind(id: i16, word: &'static str, key: &'static str, max: i16, goes_on: &'static [Gear], group: u8) -> Kind {
    Kind { id, word, key, max, goes_on, group }
}

const PROTECTION: u8 = 1;
const DAMAGE: u8 = 2;
const DIGGING: u8 = 3;
const BOW_SUPPLY: u8 = 4;
const FEET_WATER: u8 = 5;
const CROSSBOW_SHOT: u8 = 6;

pub const KINDS: [Kind; 42] = [
    kind(0, "protection", "enchantment.protect.all", 4, ARMOUR, PROTECTION),
    kind(1, "fire_protection", "enchantment.protect.fire", 4, ARMOUR, PROTECTION),
    kind(2, "feather_falling", "enchantment.protect.fall", 4, &[Feet], 0),
    kind(3, "blast_protection", "enchantment.protect.explosion", 4, ARMOUR, PROTECTION),
    kind(4, "projectile_protection", "enchantment.protect.projectile", 4, ARMOUR, PROTECTION),
    kind(5, "thorns", "enchantment.thorns", 3, ARMOUR, 0),
    kind(6, "respiration", "enchantment.oxygen", 3, &[Head], 0),
    kind(7, "depth_strider", "enchantment.waterWalker", 3, &[Feet], FEET_WATER),
    kind(8, "aqua_affinity", "enchantment.waterWorker", 1, &[Head], 0),
    kind(9, "sharpness", "enchantment.damage.all", 5, BLADES, DAMAGE),
    kind(10, "smite", "enchantment.damage.undead", 5, &[Sword, Axe, Mace], DAMAGE),
    kind(11, "bane_of_arthropods", "enchantment.damage.arthropods", 5, &[Sword, Axe, Mace], DAMAGE),
    kind(12, "knockback", "enchantment.knockback", 2, &[Sword], 0),
    kind(13, "fire_aspect", "enchantment.fire", 2, &[Sword, Mace], 0),
    kind(14, "looting", "enchantment.lootBonus", 3, &[Sword], 0),
    kind(15, "efficiency", "enchantment.digging", 5, &[Axe, Pickaxe, Shovel, Hoe, Shears], 0),
    kind(16, "silk_touch", "enchantment.untouching", 1, DIGGERS, DIGGING),
    kind(17, "unbreaking", "enchantment.durability", 3, WEARING, 0),
    kind(18, "fortune", "enchantment.lootBonusDigger", 3, DIGGERS, DIGGING),
    kind(19, "power", "enchantment.arrowDamage", 5, &[Bow], 0),
    kind(20, "punch", "enchantment.arrowKnockback", 2, &[Bow], 0),
    kind(21, "flame", "enchantment.arrowFire", 1, &[Bow], 0),
    kind(22, "infinity", "enchantment.arrowInfinite", 1, &[Bow], BOW_SUPPLY),
    kind(23, "luck_of_the_sea", "enchantment.lootBonusFishing", 3, &[FishingRod], 0),
    kind(24, "lure", "enchantment.fishingSpeed", 3, &[FishingRod], 0),
    kind(25, "frost_walker", "enchantment.frostwalker", 2, &[Feet], FEET_WATER),
    kind(26, "mending", "enchantment.mending", 1, WEARING, BOW_SUPPLY),
    kind(27, "binding", "enchantment.curse.binding", 1, &[Head, Chest, Legs, Feet, Worn, Headwear], 0),
    kind(28, "vanishing", "enchantment.curse.vanishing", 1, ALL, 0),
    kind(29, "impaling", "enchantment.tridentImpaling", 5, &[Trident], 0),
    kind(30, "riptide", "enchantment.tridentRiptide", 3, &[Trident], 0),
    kind(31, "loyalty", "enchantment.tridentLoyalty", 3, &[Trident], 0),
    kind(32, "channeling", "enchantment.tridentChanneling", 1, &[Trident], 0),
    kind(33, "multishot", "enchantment.crossbowMultishot", 1, &[Crossbow], CROSSBOW_SHOT),
    kind(34, "piercing", "enchantment.crossbowPiercing", 4, &[Crossbow], CROSSBOW_SHOT),
    kind(35, "quick_charge", "enchantment.crossbowQuickCharge", 3, &[Crossbow], 0),
    kind(36, "soul_speed", "enchantment.soul_speed", 3, &[Feet], 0),
    kind(37, "swift_sneak", "enchantment.swift_sneak", 3, &[Legs], 0),
    kind(38, "wind_burst", "enchantment.heavy_weapon.windburst", 3, &[Mace], 0),
    kind(39, "density", "enchantment.heavy_weapon.density", 5, &[Mace], DAMAGE),
    kind(40, "breach", "enchantment.heavy_weapon.breach", 4, &[Mace], DAMAGE),
    kind(41, "lunge", "enchantment.lunge", 3, &[Spear], 0),
];

/// Riptide is with neither Loyalty nor Channeling, which are with each other.
const RIPTIDE: i16 = 30;

pub fn kind_of(id: i16) -> Option<&'static Kind> {
    KINDS.iter().find(|kind| kind.id == id)
}

pub fn named(word: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|kind| kind.word.eq_ignore_ascii_case(word))
}

/// Whether two enchantments can be on one item.
pub fn compatible(a: &Kind, b: &Kind) -> bool {
    let riptide = |x: &Kind, y: &Kind| x.id == RIPTIDE && [31, 32].contains(&y.id);
    a.id == b.id || !((a.group != 0 && a.group == b.group) || riptide(a, b) || riptide(b, a))
}

/// What the item named `name` (namespaced) is for enchanting; `None` for an item none goes on.
pub fn gear(name: &str) -> Option<Gear> {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    // `_pickaxe` before `_axe`, which it ends with too.
    const ENDINGS: [(&str, Gear); 12] = [
        ("_helmet", Head), ("_chestplate", Chest), ("_leggings", Legs), ("_boots", Feet), ("_sword", Sword), ("_pickaxe", Pickaxe), ("_axe", Axe), ("_shovel", Shovel), ("_hoe", Hoe),
        ("_spear", Spear), ("_skull", Headwear), ("_head", Headwear),
    ];
    let named = match name {
        "bow" => Some(Bow),
        "crossbow" => Some(Crossbow),
        "trident" => Some(Trident),
        "fishing_rod" => Some(FishingRod),
        "mace" => Some(Mace),
        "shears" => Some(Shears),
        "turtle_helmet" => Some(Head),
        "shield" | "elytra" | "flint_and_steel" | "brush" | "carrot_on_a_stick" | "warped_fungus_on_a_stick" => Some(Worn),
        "carved_pumpkin" => Some(Headwear),
        _ => None,
    };
    named.or_else(|| ENDINGS.iter().find(|(ending, _)| name.ends_with(ending)).map(|(_, gear)| *gear))
}

/// Whether `kind` goes on the item named `name`: any goes on a book.
pub fn goes_on(kind: &Kind, name: &str) -> bool {
    matches!(name.strip_prefix("minecraft:").unwrap_or(name), "book" | "enchanted_book") || gear(name).is_some_and(|gear| kind.goes_on.contains(&gear))
}
