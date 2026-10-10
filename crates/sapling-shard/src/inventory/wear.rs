//! Tools wear: a use is counted where the game keeps it, in the stack's `Damage` tag, and a tool
//! used once more than it lasts is gone. The rules are BDS's `ItemStackBase::hurtAndBreak`
//! (docs/research/block-rules.md, section 5).
// TODO: no break sound, and nothing but breaking blocks wears anything: armour does not wear.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_world::{Tool, ToolKind};

use super::{HOTBAR, Held, Inventory, Place, change};
use crate::blocks::Dice;

const DAMAGE: &str = "Damage";
/// The `ench` id of Unbreaking.
const UNBREAKING: i16 = 17;

impl Inventory {
    /// The points and toughness of the armour worn.
    pub(crate) fn armour(&self) -> [f32; 2] {
        let worn = self.armor.iter().flatten().filter_map(|held| sapling_data::armour(&held.stack.name));
        worn.fold([0.0; 2], |sum, piece| [sum[0] + piece[0], sum[1] + piece[1]])
    }
}

fn damage(tag: &Option<Value>) -> i32 {
    match tag.as_ref().and_then(|tag| tag.get(DAMAGE)) {
        Some(Value::Int(damage)) => *damage,
        _ => 0,
    }
}

fn set_damage(tag: &mut Option<Value>, damage: i32) {
    if let Value::Compound(fields) = tag.get_or_insert_with(|| Value::Compound(Vec::new())) {
        fields.retain(|(key, _)| key != DAMAGE);
        fields.push((DAMAGE.into(),Value::Int(damage)));
    }
}

/// The chance in a hundred that a use wears the stack: all of them, unless it is marked
/// `Unbreakable` (none) or has Unbreaking (100 over the level plus one).
fn wear_chance(held: &Held) -> u32 {
    let tag = held.stack.tag.as_ref();
    if tag.and_then(|tag| tag.get("Unbreakable")) == Some(&Value::Byte(1)) {
        return 0;
    }
    let level = match tag.and_then(|tag| tag.get("ench")) {
        Some(Value::List(list)) => list.items.iter().find(|enchantment| enchantment.get("id") == Some(&Value::Short(UNBREAKING))).and_then(|enchantment| enchantment.get("lvl")),
        _ => None,
    };
    if let Some(Value::Short(level)) = level { 100 / (u32::try_from(*level).unwrap_or(0) + 1) } else { 100 }
}

impl Inventory {
    /// One of the stack in the hand is gone: eaten (vitals/eating.rs).
    pub fn take_one_in_hand(&mut self) {
        let Some(slot) = self.slot_mut(Place::Main(usize::from(self.selected) % HOTBAR)) else { return };
        match slot.as_mut() {
            Some(held) if held.stack.count > 1 => held.stack.count -= 1,
            _ => *slot = None,
        }
        self.unsaved = true;
    }

    /// Counts `uses` against the stack in the hand, which lasts `lasts` of them.
    fn wear_hand(&mut self, uses: i32, lasts: u16) {
        let Some(slot) = self.slot_mut(Place::Main(usize::from(self.selected) % HOTBAR)) else { return };
        let Some(held) = slot.as_mut() else { return };
        let worn = damage(&held.stack.tag) + uses;
        match worn > i32::from(lasts) {
            true => *slot = None,
            false => set_damage(&mut held.stack.tag, worn),
        }
    }
}

/// Counts a broken block against the tool in a player's hand and shows the client the result;
/// a sword wears twice as fast, and a hand without a tool is left alone. One throw decides
/// whether the whole use counts.
pub(crate) fn wear_tool_in_hand(world: &mut World, player: Entity) {
    let Some(held) = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()) else { return };
    let (Some(tool), chance) = (Tool::from_identifier(&held.stack.name), wear_chance(held)) else { return };
    if chance < 100 && world.get_resource_or_insert_with(Dice::default).roll(100) >= chance {
        return;
    }
    let uses = if tool.kind == ToolKind::Sword { 2 } else { 1 };
    change(world, player, |inventory| inventory.wear_hand(uses, tool.max_durability()));
}
