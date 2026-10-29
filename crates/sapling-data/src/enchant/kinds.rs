//! Every enchantment, one table for `/enchant`, the enchanting table, the anvil and the grindstone,
//! from BDS's `initEnchants` (141b5be40): its word and name key, its levels, how common it is, the
//! power each level wants, what it goes on and what it does not go with
//! (docs/research/workstations.md, "Enchantments"; `/enchant`'s refusals: research/captures/bds-itemcmds.txt).

use crate::gear::slot::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind {
    pub id: i16,
    /// The word `/enchant` takes.
    pub word: &'static str,
    /// The key of its name in the game's strings.
    pub key: &'static str,
    pub max: u8,
    /// BDS's `Frequency`: 30 common, 10 uncommon, 3 rare, 1 very rare.
    pub frequency: u8,
    /// The power level 1 wants, and each level more.
    base: i32,
    per: i32,
    /// BDS's `EnchantSlot` bits it goes on, first and as well; the table offers either.
    pub slots: u32,
    pub also: u32,
    /// Never from the table.
    pub treasure: bool,
    pub curse: bool,
    /// Two of one group (other than 0) do not go together.
    group: u8,
    /// Where it acts: worn, held, or on the item itself (the three lists of an offer).
    pub activation: Activation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    Equipped,
    Held,
    Itself,
}

mod group {
    pub const PROTECTION: u8 = 1;
    pub const DAMAGE: u8 = 2;
    pub const MINING: u8 = 3;
    pub const INFINITE: u8 = 4;
    pub const WATER_WALK: u8 = 5;
    pub const TRIDENT: u8 = 6;
}

/// Pairs that do not go together outside the groups: riptide and channeling, multishot and piercing.
const EXCLUSIVE: [(i16, i16); 2] = [(30, 32), (33, 34)];

struct Rarity(u8, i32, i32);

const fn kind(id: i16, word: &'static str, key: &'static str, max: u8, Rarity(frequency, base, per): Rarity, slots: u32, also: u32, group: u8, activation: Activation) -> Kind {
    Kind { id, word, key, max, frequency, base, per, slots, also, treasure: false, curse: false, group, activation }
}

const fn treasure(kind: Kind) -> Kind {
    Kind { treasure: true, ..kind }
}

const fn curse(kind: Kind) -> Kind {
    Kind { treasure: true, curse: true, ..kind }
}

const BLADE: u32 = SWORD | SPEAR;
const WORN_FIRST: u32 = 0x81_1E7F;
const WORN_ALSO: u32 = 0x72_E1C0;

use Activation::{Equipped, Held, Itself};

/// By id, as a stack's `ench` list numbers them.
pub const KINDS: [Kind; 42] = [
    kind(0, "protection", "enchantment.protect.all", 4, Rarity(30, 1, 11), ARMOUR, 0, group::PROTECTION, Equipped),
    kind(1, "fire_protection", "enchantment.protect.fire", 4, Rarity(10, 10, 8), ARMOUR, 0, group::PROTECTION, Equipped),
    kind(2, "feather_falling", "enchantment.protect.fall", 4, Rarity(10, 5, 6), FEET, 0, 0, Equipped),
    kind(3, "blast_protection", "enchantment.protect.explosion", 4, Rarity(3, 5, 8), ARMOUR, 0, group::PROTECTION, Equipped),
    kind(4, "projectile_protection", "enchantment.protect.projectile", 4, Rarity(10, 3, 6), ARMOUR, 0, group::PROTECTION, Equipped),
    kind(5, "thorns", "enchantment.thorns", 3, Rarity(1, 10, 20), TORSO, HEAD | FEET | LEGS, 0, Equipped),
    kind(6, "respiration", "enchantment.oxygen", 3, Rarity(3, 10, 10), HEAD, 0, 0, Equipped),
    kind(7, "depth_strider", "enchantment.waterWalker", 3, Rarity(3, 10, 10), FEET, 0, group::WATER_WALK, Equipped),
    kind(8, "aqua_affinity", "enchantment.waterWorker", 1, Rarity(3, 1, 0), HEAD, 0, 0, Equipped),
    kind(9, "sharpness", "enchantment.damage.all", 5, Rarity(30, 1, 11), BLADE, AXE, group::DAMAGE, Held),
    kind(10, "smite", "enchantment.damage.undead", 5, Rarity(10, 5, 8), BLADE | MACE, AXE, group::DAMAGE, Held),
    kind(11, "bane_of_arthropods", "enchantment.damage.arthropods", 5, Rarity(10, 5, 8), BLADE | MACE, AXE, group::DAMAGE, Held),
    // Knockback, unbreaking, power and piercing sit where BDS listed them (research/captures/bds-enchants.txt).
    kind(12, "knockback", "enchantment.knockback", 2, Rarity(10, 5, 20), BLADE, 0, 0, Itself),
    kind(13, "fire_aspect", "enchantment.fire", 2, Rarity(3, 10, 20), BLADE | MACE, 0, 0, Held),
    kind(14, "looting", "enchantment.lootBonus", 3, Rarity(3, 15, 9), BLADE, 0, 0, Held),
    kind(15, "efficiency", "enchantment.digging", 5, Rarity(30, 1, 10), DIGGING, SHEARS, 0, Held),
    kind(16, "silk_touch", "enchantment.untouching", 1, Rarity(1, 15, 0), DIGGING, SHEARS, group::MINING, Held),
    kind(17, "unbreaking", "enchantment.durability", 3, Rarity(10, 5, 8), WORN_FIRST, WORN_ALSO, 0, Held),
    kind(18, "fortune", "enchantment.lootBonusDigger", 3, Rarity(3, 15, 9), DIGGING, 0, group::MINING, Held),
    kind(19, "power", "enchantment.arrowDamage", 5, Rarity(30, 1, 10), BOW, 0, 0, Itself),
    kind(20, "punch", "enchantment.arrowKnockback", 2, Rarity(3, 12, 20), BOW, 0, 0, Held),
    kind(21, "flame", "enchantment.arrowFire", 1, Rarity(3, 20, 0), BOW, 0, 0, Held),
    kind(22, "infinity", "enchantment.arrowInfinite", 1, Rarity(1, 20, 0), BOW, 0, group::INFINITE, Held),
    kind(23, "luck_of_the_sea", "enchantment.lootBonusFishing", 3, Rarity(3, 15, 9), FISHING_ROD, 0, 0, Held),
    kind(24, "lure", "enchantment.fishingSpeed", 3, Rarity(3, 15, 9), FISHING_ROD, 0, 0, Held),
    treasure(kind(25, "frost_walker", "enchantment.frostwalker", 2, Rarity(3, 10, 10), FEET, 0, group::WATER_WALK, Equipped)),
    treasure(kind(26, "mending", "enchantment.mending", 1, Rarity(3, 25, 25), WORN_FIRST, WORN_ALSO, group::INFINITE, Itself)),
    curse(kind(27, "binding", "enchantment.curse.binding", 1, Rarity(1, 25, 0), ARMOUR | ELYTRA | COSMETIC_HEAD, 0, 0, Itself)),
    curse(kind(28, "vanishing", "enchantment.curse.vanishing", 1, Rarity(1, 25, 0), ALL, 0, 0, Itself)),
    kind(29, "impaling", "enchantment.tridentImpaling", 5, Rarity(10, 1, 8), TRIDENT, 0, 0, Held),
    kind(30, "riptide", "enchantment.tridentRiptide", 3, Rarity(3, 17, 7), TRIDENT, 0, group::TRIDENT, Held),
    kind(31, "loyalty", "enchantment.tridentLoyalty", 3, Rarity(30, 12, 7), TRIDENT, 0, group::TRIDENT, Held),
    kind(32, "channeling", "enchantment.tridentChanneling", 1, Rarity(1, 25, 0), TRIDENT, 0, 0, Held),
    kind(33, "multishot", "enchantment.crossbowMultishot", 1, Rarity(3, 20, 0), CROSSBOW, 0, 0, Held),
    kind(34, "piercing", "enchantment.crossbowPiercing", 4, Rarity(30, 1, 10), CROSSBOW, 0, 0, Itself),
    kind(35, "quick_charge", "enchantment.crossbowQuickCharge", 3, Rarity(10, 12, 20), CROSSBOW, 0, 0, Held),
    treasure(kind(36, "soul_speed", "enchantment.soul_speed", 3, Rarity(1, 10, 10), FEET, 0, 0, Equipped)),
    treasure(kind(37, "swift_sneak", "enchantment.swift_sneak", 3, Rarity(1, 10, 10), LEGS, 0, 0, Equipped)),
    treasure(kind(38, "wind_burst", "enchantment.heavy_weapon.windburst", 3, Rarity(3, 15, 9), MACE, 0, 0, Held)),
    kind(39, "density", "enchantment.heavy_weapon.density", 5, Rarity(10, 5, 8), MACE, 0, group::DAMAGE, Held),
    kind(40, "breach", "enchantment.heavy_weapon.breach", 4, Rarity(3, 15, 9), MACE, 0, group::DAMAGE, Held),
    kind(41, "lunge", "enchantment.lunge", 3, Rarity(10, 5, 8), SPEAR, 0, 0, Held),
];

impl Kind {
    /// The least power the enchanting table gives `level` at (BDS's `getMinCost`).
    pub fn min_power(&self, level: u8) -> i32 {
        self.base + self.per * (i32::from(level) - 1)
    }

    /// How often the table picks it, against the others it could.
    pub fn weight(&self) -> u32 {
        match self.frequency {
            30 => 10,
            10 => 5,
            3 => 2,
            _ => 1,
        }
    }

    /// Whether it goes on an item of these `EnchantSlot` bits (`ALL`: a book).
    pub fn goes_on(&self, slots: u32) -> bool {
        slots == ALL || (self.slots | self.also) & slots != 0
    }
}

/// The enchantment numbered `id`.
pub fn kind_of(id: i16) -> Option<&'static Kind> {
    KINDS.get(usize::try_from(id).ok()?)
}

/// The enchantment `/enchant` calls `word`, in any case.
pub fn named(word: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|kind| kind.word.eq_ignore_ascii_case(word))
}

/// Whether two enchantments can be on one item: one goes with itself.
pub fn compatible(a: &Kind, b: &Kind) -> bool {
    let paired = EXCLUSIVE.iter().any(|pair| *pair == (a.id, b.id) || *pair == (b.id, a.id));
    a.id == b.id || !(paired || (a.group != 0 && a.group == b.group))
}

/// Whether the enchantments `a` and `b`, two of them, go on one item together.
pub(super) fn go_together(a: i16, b: i16) -> bool {
    let (Some(first), Some(second)) = (kind_of(a), kind_of(b)) else { return false };
    a != b && compatible(first, second)
}

/// The `EnchantSlot` bits of the item named `name` (namespaced): `ALL` for a book, a head's
/// cosmetic slot, 0 for an item no enchantment goes on.
pub fn slots_of(name: &str) -> u32 {
    match name {
        crate::gear::BOOK | crate::gear::ENCHANTED_BOOK => ALL,
        _ if name.ends_with("_skull") || name.ends_with("_head") => COSMETIC_HEAD,
        _ => crate::gear::gear(name).map_or(0, |gear| gear.slots),
    }
}

/// Whether `kind` goes on the item named `name` (namespaced): any goes on a book.
pub fn goes_on(kind: &Kind, name: &str) -> bool {
    let slots = slots_of(name);
    slots != 0 && kind.goes_on(slots)
}
