//! Each enchantment as the enchanting table and the anvil see it, from BDS's `initEnchants`:
//! its levels, how common it is, the power each level wants, what it goes on, and what it does
//! not go with (docs/research/workstations.md, "Enchantments").

use crate::gear::slot::*;

#[derive(Clone, Copy)]
pub(super) struct Kind {
    pub max: u8,
    /// BDS's `Frequency`: 30 common, 10 uncommon, 3 rare, 1 very rare.
    pub frequency: u8,
    /// The power level 1 wants, and each level more.
    pub base: i32,
    pub per: i32,
    /// What it goes on, first and as well; the table offers either.
    pub slots: u32,
    pub also: u32,
    /// Never from the table.
    pub treasure: bool,
    pub curse: bool,
    /// Two of one group (other than 0) do not go together.
    pub group: u8,
    /// Where it acts: worn, held, or on the item itself (the three lists of an offer).
    pub activation: Activation,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

const fn kind(max: u8, frequency: u8, base: i32, per: i32, slots: u32, also: u32, group: u8, activation: Activation) -> Kind {
    Kind { max, frequency, base, per, slots, also, treasure: false, curse: false, group, activation }
}

const fn treasure(kind: Kind) -> Kind {
    Kind { treasure: true, ..kind }
}

const fn curse(kind: Kind) -> Kind {
    Kind { treasure: true, curse: true, ..kind }
}

const BLADE: u32 = SWORD | SPEAR;
const ROD: u32 = FISHING_ROD;
const WORN_FIRST: u32 = 0x81_1E7F;
const WORN_ALSO: u32 = 0x72_E1C0;

use Activation::{Equipped, Held, Itself};

/// By id, as a stack's `ench` list numbers them.
pub(super) const KINDS: [Kind; 42] = [
    kind(4, 30, 1, 11, ARMOUR, 0, group::PROTECTION, Equipped),
    kind(4, 10, 10, 8, ARMOUR, 0, group::PROTECTION, Equipped),
    kind(4, 10, 5, 6, FEET, 0, 0, Equipped),
    kind(4, 3, 5, 8, ARMOUR, 0, group::PROTECTION, Equipped),
    kind(4, 10, 3, 6, ARMOUR, 0, group::PROTECTION, Equipped),
    kind(3, 1, 10, 20, TORSO, HEAD | FEET | LEGS, 0, Equipped),
    kind(3, 3, 10, 10, HEAD, 0, 0, Equipped),
    kind(3, 3, 10, 10, FEET, 0, group::WATER_WALK, Equipped),
    kind(1, 3, 1, 0, HEAD, 0, 0, Equipped),
    kind(5, 30, 1, 11, BLADE, AXE, group::DAMAGE, Held),
    kind(5, 10, 5, 8, BLADE | MACE, AXE, group::DAMAGE, Held),
    kind(5, 10, 5, 8, BLADE | MACE, AXE, group::DAMAGE, Held),
    kind(2, 10, 5, 20, BLADE, 0, 0, Held),
    kind(2, 3, 10, 20, BLADE | MACE, 0, 0, Held),
    kind(3, 3, 15, 9, BLADE, 0, 0, Held),
    kind(5, 30, 1, 10, DIGGING, SHEARS, 0, Held),
    kind(1, 1, 15, 0, DIGGING, SHEARS, group::MINING, Held),
    kind(3, 10, 5, 8, WORN_FIRST, WORN_ALSO, 0, Itself),
    kind(3, 3, 15, 9, DIGGING, 0, group::MINING, Held),
    kind(5, 30, 1, 10, BOW, 0, 0, Held),
    kind(2, 3, 12, 20, BOW, 0, 0, Held),
    kind(1, 3, 20, 0, BOW, 0, 0, Held),
    kind(1, 1, 20, 0, BOW, 0, group::INFINITE, Held),
    kind(3, 3, 15, 9, ROD, 0, 0, Held),
    kind(3, 3, 15, 9, ROD, 0, 0, Held),
    treasure(kind(2, 3, 10, 10, FEET, 0, group::WATER_WALK, Equipped)),
    treasure(kind(1, 3, 25, 25, WORN_FIRST, WORN_ALSO, group::INFINITE, Itself)),
    curse(kind(1, 1, 25, 0, ARMOUR | ELYTRA | COSMETIC_HEAD, 0, 0, Itself)),
    curse(kind(1, 1, 25, 0, ALL, 0, 0, Itself)),
    kind(5, 10, 1, 8, TRIDENT, 0, 0, Held),
    kind(3, 3, 17, 7, TRIDENT, 0, group::TRIDENT, Held),
    kind(3, 30, 12, 7, TRIDENT, 0, group::TRIDENT, Held),
    kind(1, 1, 25, 0, TRIDENT, 0, 0, Held),
    kind(1, 3, 20, 0, CROSSBOW, 0, 0, Held),
    kind(4, 30, 1, 10, CROSSBOW, 0, 0, Held),
    kind(3, 10, 12, 20, CROSSBOW, 0, 0, Held),
    treasure(kind(3, 1, 10, 10, FEET, 0, 0, Equipped)),
    treasure(kind(3, 1, 10, 10, LEGS, 0, 0, Equipped)),
    treasure(kind(3, 3, 15, 9, MACE, 0, 0, Held)),
    kind(5, 10, 5, 8, MACE, 0, group::DAMAGE, Held),
    kind(4, 3, 15, 9, MACE, 0, group::DAMAGE, Held),
    kind(3, 10, 5, 8, SPEAR, 0, 0, Held),
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

    pub fn goes_on(&self, slots: u32) -> bool {
        slots == ALL || (self.slots | self.also) & slots != 0
    }
}

/// The enchantment numbered `id`.
pub(super) fn of(id: i16) -> Option<&'static Kind> {
    KINDS.get(usize::try_from(id).ok()?)
}

/// Whether the enchantments `a` and `b` go on one item together.
pub(super) fn go_together(a: i16, b: i16) -> bool {
    let (Some(first), Some(second)) = (of(a), of(b)) else { return false };
    let paired = EXCLUSIVE.iter().any(|pair| *pair == (a, b) || *pair == (b, a));
    a != b && !paired && (first.group == 0 || first.group != second.group)
}
