//! Powder snow's freeze on a player, as BDS's (research/captures/bds-freeze.txt, `bds_command_probe
//! freeze`): each tick its box is in powder snow the freeze climbs 1/140 to 1; out of it, or in
//! any piece of leather, it thaws 1/70. Full, it hurts 1 (freezing) that tick and every 40 after.
//! Each tick it changes the player and its viewers are told `FreezingEffectStrength`, with the
//! frozen flag the tick it fills or stops being full, and a tick later the walking speed it
//! leaves (effects/attributes.rs: 0.05 less at full).
// TODO: mobs do not freeze; the freeze is neither kept when the player leaves nor reset at death.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::effects::{Paced, tell_pace};
use super::tell;
use crate::components::{NetId, Outgoing, Tick};
use crate::inventory::Inventory;
use crate::movement::Motion;
use crate::visibility::ToViewers;

/// The boots' slot among the armour worn.
pub(crate) const FEET: usize = 3;
pub(super) const POWDER_SNOW: &[&str] = &["minecraft:powder_snow"];
const GAIN: f32 = 1.0 / 140.0;
const LOSS: f32 = 1.0 / 70.0;
const HURTS_EVERY: u32 = 40;
pub(super) const FREEZE_HARM: f32 = 1.0;

#[derive(Component, Default)]
pub(crate) struct Frost {
    level: f32,
    /// Ticks it has been full.
    full_for: u32,
}

/// Which pieces of the armour a player wears, helmet first, are leather.
pub(crate) fn leather(world: &World, player: Entity) -> [bool; 4] {
    let mut worn = [false; 4];
    if let Some(inventory) = world.get::<Inventory>(player) {
        for (slot, name) in worn.iter_mut().zip(inventory.worn_names()) {
            *slot = name.is_some_and(|name| name.starts_with("minecraft:leather_"));
        }
    }
    worn
}

/// A tick of the player's freeze, its box `in_snow` or not, told as it changes: whether it hurts
/// the player now.
pub(super) fn chill(world: &mut World, player: Entity, in_snow: bool) -> bool {
    let chilled = in_snow && !leather(world, player).contains(&true);
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return false };
    world.entity_mut(player).insert_if_new((Frost::default(), Paced::default()));
    let mut frost = world.get_mut::<Frost>(player).expect("put above");
    let was = frost.level;
    frost.level = if chilled { (was + GAIN).min(1.0) } else { (was - LOSS).max(0.0) };
    let level = frost.level;
    let hurts = level >= 1.0 && frost.full_for % HURTS_EVERY == 0;
    frost.full_for = if level >= 1.0 { frost.full_for + 1 } else { 0 };
    if level != was {
        tell_strength(world, player, id, level, (level >= 1.0) != (was >= 1.0));
    }
    // The walking speed BDS tells lags the strength by a tick.
    if world.get::<Paced>(player).is_some_and(|paced| paced.freeze != was) {
        world.get_mut::<Paced>(player).expect("checked above").freeze = was;
        tell_pace(world, player, id);
    }
    if let Some(mut simulated) = world.get_mut::<crate::simulation::Simulated>(player) {
        simulated.freeze(level);
    }
    hurts
}

/// The strength of the freeze's frost on the screen, and the frozen flag where it `turned`: the
/// player is told both at once, as BDS told it, its viewers the flag through `movement::announce`.
fn tell_strength(world: &mut World, player: Entity, id: u64, level: f32, turned: bool) {
    let tick = world.resource::<Tick>().0;
    let data = |metadata| SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick };
    let strength = MetadataDictionaryItem { key: Key::FreezingEffectStrength, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(level)) };
    let Some(mut motion) = world.get_mut::<Motion>(player).filter(|_| turned) else { return tell(world, player, &data(vec![strength])) };
    motion.pose.frozen = level >= 1.0;
    let mut metadata: Vec<_> = sapling_data::player_metadata("", motion.pose.flags()).into_iter().filter(|item| item.key == Key::Flags).collect();
    metadata.push(strength.clone());
    world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&data(metadata));
    world.resource_mut::<ToViewers>().push(player, &data(vec![strength]));
}
