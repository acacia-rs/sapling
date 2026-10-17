//! What a dying player lets go of (vitals/death.rs).

use sapling_world::ItemStack;

use super::Inventory;
use crate::experience::Experience;

/// Experience a level leaves, and the most a player leaves, as the Java game.
const PER_LEVEL: i32 = 7;
const MOST: i32 = 100;

impl Inventory {
    /// Empties the player's own slots and its experience: the stacks, and the experience to leave.
    pub fn let_go(&mut self) -> (Vec<ItemStack>, i32) {
        let slots = self.main.iter_mut().chain(&mut self.armor).chain([&mut self.offhand, &mut self.cursor]);
        let stacks = slots.filter_map(Option::take).map(|held| held.stack).collect();
        let points = (self.experience.level * PER_LEVEL).min(MOST);
        (self.experience, self.unsaved) = (Experience::default(), true);
        (stacks, points)
    }
}
