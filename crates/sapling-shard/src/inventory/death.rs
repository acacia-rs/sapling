//! What a dying player lets go of (vitals/death.rs).

use acacia_proto::nbt::Value;
use sapling_world::ItemStack;

use super::Inventory;
use super::enchanted::vanishes;
use super::plugin::KEEP_ON_DEATH;
use crate::experience::Experience;

/// Experience a level leaves, and the most a player leaves, as the Java game.
const PER_LEVEL: i32 = 7;
const MOST: i32 = 100;

impl Inventory {
    /// Empties the player's own slots and its experience: the stacks, and the experience to leave.
    /// A stack with Curse of Vanishing is gone instead; one with `minecraft:keep_on_death` stays.
    pub fn let_go(&mut self) -> (Vec<ItemStack>, i32) {
        let slots = self.main.iter_mut().chain(&mut self.armor).chain([&mut self.offhand, &mut self.cursor]);
        let kept = |stack: &ItemStack| matches!(stack.tag.as_ref().and_then(|tag| tag.get(KEEP_ON_DEATH)), Some(Value::Byte(1)));
        let let_go = slots.filter(|slot| slot.as_ref().is_some_and(|held| !kept(&held.stack)));
        let stacks = let_go.filter_map(Option::take).filter(|held| !vanishes(held)).map(|held| held.stack).collect();
        let points = (self.experience.level * PER_LEVEL).min(MOST);
        (self.experience, self.unsaved) = (Experience::default(), true);
        (stacks, points)
    }
}
