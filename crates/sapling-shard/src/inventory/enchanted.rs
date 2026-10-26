//! What enchantments do to the stacks that carry them: Mending mends one with experience, Curse
//! of Vanishing is gone at death, Curse of Binding keeps a piece on. The rules:
//! `sapling_data::enchant`.

use acacia_proto::nbt::Value;
use sapling_data::enchant::{self, Enchant};

use super::wear::{damage, set_damage};
use super::{HOTBAR, Held, Inventory};

fn level(held: &Held, enchant: Enchant) -> u8 {
    enchant::level(held.stack.tag.as_ref(), enchant)
}

/// Whether the stack goes, rather than drops, when its player dies.
pub(super) fn vanishes(held: &Held) -> bool {
    level(held, Enchant::Vanishing) > 0
}

impl Inventory {
    /// The NBT of each armour piece, helmet first; `None` for an empty slot or a plain piece.
    pub(crate) fn worn_tags(&self) -> [Option<&Value>; 4] {
        std::array::from_fn(|piece| self.armor.get(piece).and_then(Option::as_ref).and_then(|held| held.stack.tag.as_ref()))
    }

    /// Mends one worn stack with Mending, of the armour, the hand and the offhand, picked by
    /// `pick` (given the count, a number below it), with an orb worth `value`; what is left of it.
    pub(crate) fn mend(&mut self, value: i32, pick: impl FnOnce(u32) -> u32) -> i32 {
        let hand = usize::from(self.selected) % HOTBAR;
        let mut worn: Vec<&mut Held> = self.armor.iter_mut().chain([&mut self.main[hand], &mut self.offhand]).flatten().filter(|held| level(held, Enchant::Mending) > 0 && damage(&held.stack.tag) > 0).collect();
        if worn.is_empty() {
            return value;
        }
        let picked = pick(worn.len() as u32) as usize % worn.len();
        let held = &mut worn[picked];
        let worn_by = damage(&held.stack.tag);
        let (mended, left) = enchant::mend(worn_by, value);
        set_damage(&mut held.stack.tag, worn_by - mended);
        self.unsaved = true;
        left
    }

    /// The ids of the armour pieces with Curse of Binding, by slot.
    pub(super) fn bound(&self) -> [Option<i32>; 4] {
        std::array::from_fn(|piece| self.armor.get(piece).and_then(Option::as_ref).filter(|held| level(held, Enchant::Binding) > 0).map(|held| held.id))
    }

    /// Whether every piece `bound` named is still where it was.
    pub(super) fn keeps(&self, bound: &[Option<i32>; 4]) -> bool {
        bound.iter().enumerate().all(|(piece, id)| id.is_none_or(|id| self.armor[piece].as_ref().is_some_and(|held| held.id == id)))
    }
}
