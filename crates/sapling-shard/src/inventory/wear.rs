//! Tools wear: a use is counted where the game keeps it, in the stack's `Damage` tag, and a tool
//! used once more than it lasts is gone. The rules are BDS's `ItemStackBase::hurtAndBreak`
//! (docs/research/block-rules.md, section 5). Hits wear weapons too (creatures/blows.rs); armour:
//! armour_wear.rs.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_data::enchant::{self, Enchant};
use sapling_world::{Tool, ToolKind};

use super::{HOTBAR, Held, Inventory, Place, change};
use crate::blocks::Dice;

const DAMAGE: &str = "Damage";

impl Inventory {
    pub(crate) fn has_helmet(&self) -> bool {
        self.armor[0].is_some()
    }

    /// The names of the armour worn, helmet first.
    pub(crate) fn worn_names(&self) -> impl Iterator<Item = Option<&str>> {
        self.armor.iter().map(|slot| slot.as_ref().map(|held| held.stack.name.as_str()))
    }

    /// The points and toughness of the armour worn.
    pub(crate) fn armour(&self) -> [f32; 2] {
        let worn = self.armor.iter().flatten().filter_map(|held| sapling_data::armour(&held.stack.name));
        worn.fold([0.0; 2], |sum, piece| [sum[0] + piece[0], sum[1] + piece[1]])
    }
}

pub(super) fn damage(tag: &Option<Value>) -> i32 {
    match tag.as_ref().and_then(|tag| tag.get(DAMAGE)) {
        Some(Value::Int(damage)) => *damage,
        _ => 0,
    }
}

pub(super) fn set_damage(tag: &mut Option<Value>, damage: i32) {
    if let Value::Compound(fields) = tag.get_or_insert_with(|| Value::Compound(Vec::new())) {
        fields.retain(|(key, _)| key != DAMAGE);
        fields.push((DAMAGE.into(),Value::Int(damage)));
    }
}

/// The chance in a hundred that a use wears the stack, `armour` or not: all of them, unless it
/// is marked `Unbreakable` (none) or has Unbreaking.
pub(super) fn wear_chance(held: &Held, armour: bool) -> u32 {
    let tag = held.stack.tag.as_ref();
    if tag.and_then(|tag| tag.get("Unbreakable")) == Some(&Value::Byte(1)) {
        return 0;
    }
    enchant::wear_chance(enchant::level(tag, Enchant::Unbreaking), armour)
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

    /// Counts `uses` against the stack in the hand, which lasts `lasts` of them; whether it broke.
    fn wear_hand(&mut self, uses: i32, lasts: u16) -> bool {
        self.wear_at(Place::Main(usize::from(self.selected) % HOTBAR), uses, lasts)
    }

    /// Counts `uses` against the stack at `place`, which lasts `lasts` of them; whether it broke.
    fn wear_at(&mut self, place: Place, uses: i32, lasts: u16) -> bool {
        self.unsaved = true;
        let Some(slot) = self.slot_mut(place) else { return false };
        let Some(held) = slot.as_mut() else { return false };
        let worn = damage(&held.stack.tag) + uses;
        let broke = worn > i32::from(lasts);
        match broke {
            true => *slot = None,
            false => set_damage(&mut held.stack.tag, worn),
        }
        broke
    }
}

/// Counts `uses` against the stack at `place` of a player's, unless Unbreaking spares it; whether
/// it broke. The client is not shown it: the caller shows what BDS showed.
pub(crate) fn wear_at(world: &mut World, player: Entity, place: Place, uses: i32) -> bool {
    let Some(held) = world.get::<Inventory>(player).and_then(|inventory| inventory.slot(place)?.as_ref()) else { return false };
    let (chance, lasts, stack) = (wear_chance(held, false), sapling_data::gear(&held.stack.name).map_or(0, |gear| gear.lasts), held.stack.clone());
    if lasts == 0 || chance < 100 && world.get_resource_or_insert_with(Dice::default).roll(100) >= chance {
        return false;
    }
    let uses = crate::hand_events::wear(world, player, &stack, uses);
    uses > 0 && world.get_mut::<Inventory>(player).is_some_and(|mut inventory| inventory.wear_at(place, uses, lasts))
}

/// Counts a broken block against the tool in a player's hand and shows the client the result;
/// a sword wears twice as fast, and a hand without a tool is left alone. One throw decides
/// whether the whole use counts.
pub(crate) fn wear_tool_in_hand(world: &mut World, player: Entity) {
    let Some(tool) = world.get::<Inventory>(player).and_then(|inventory| Tool::from_identifier(&inventory.in_hand().as_ref()?.stack.name)) else { return };
    let uses = if tool.kind == ToolKind::Sword { 2 } else { 1 };
    wear_hand_by(world, player, uses, tool.max_durability());
}

/// Counts `uses` against what the player holds, which lasts `lasts` of them, unless Unbreaking
/// spares it; shows the client the result, and one that broke is heard.
pub(crate) fn wear_hand_by(world: &mut World, player: Entity, uses: i32, lasts: u16) {
    let Some(held) = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()) else { return };
    let (chance, stack) = (wear_chance(held, false), held.stack.clone());
    if chance < 100 && world.get_resource_or_insert_with(Dice::default).roll(100) >= chance {
        return;
    }
    let uses = crate::hand_events::wear(world, player, &stack, uses);
    if uses > 0 && change(world, player, |inventory| inventory.wear_hand(uses, lasts)) == Some(true) {
        sound_broken(world, player);
    }
}

/// What BDS let the player and those who see it hear of a stack of its that broke: `break` at its
/// eyes (research/captures/bds-swing-wear.txt).
pub(crate) fn sound_broken(world: &mut World, player: Entity) {
    let Some([x, y, z]) = world.get::<crate::components::Position>(player).map(|position| position.feet) else { return };
    let position = acacia_proto::types::Vec3f { x, y: y + sapling_data::EYE_HEIGHT, z };
    let sound = acacia_proto::packets::LevelSoundEvent { sound_id: "break".into(), position, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    crate::vitals::tell(world, player, &sound);
}
