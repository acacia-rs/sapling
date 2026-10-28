//! Piglins' bartering, by the pack: an adult a player clicks holding a gold ingot (`minecraft:interact`
//! with `barter` and `admire`) takes one and admires it in its offhand for 8 seconds
//! (`admire_item`), then hands back a roll of `loot_tables/entities/piglin_barter.json`.
//! research/captures/bds-barter.txt: the ingot shown in the offhand (`MobEquipment` window
//! `Offhand`, slot 1) the tick it was taken, `admire` from its head a tick later, the hand emptied
//! 161 ticks on. Gold lying near it it picks up (pickup.rs).
//! What it holds is kept with the world (research/captures/bds-piglinkeep.txt, bds-piglinkeep2.txt):
//! a golden sword it admired went to its main hand as the offhand emptied, and after a restart it
//! came back holding the sword and the ingot it was admiring, shown as stored, and did not barter.
// TODO: BDS's handing back was not seen (its piglin in the drill picked a thrown ingot up and kept
// it); here the roll drops at the piglin as the hand empties. One that was hit still barters
// (`cooldown_after_being_attacked` 20).

mod pickup;

pub(super) use pickup::steer;

use acacia_proto::packets::MobEquipment;
use acacia_proto::types::WindowID;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{GameMode, Item};
use sapling_world::ItemStack;

use super::{Creature, Dying};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, NetId, Tick};
use crate::entities::{self, Kind};
use crate::inventory::{self, Inventory};
use crate::{drops, late_sounds};

const PIGLIN: &str = "minecraft:piglin";
const GOLD: &str = "minecraft:gold_ingot";
const BARTER: &str = "loot_tables/entities/piglin_barter.json";
const ADMIRES: u64 = 161;
/// `admire` is heard from this much of its height up.
const HEAD: f32 = 0.85;
const OFFHAND: u8 = 1;
const LIFT: f32 = 0.5;

/// The tick a piglin's admiring ends.
#[derive(Component)]
pub(super) struct Admiring {
    until: u64,
}

/// What a piglin holds, main hand and offhand.
#[derive(Component, Default)]
pub(super) struct Hands {
    main: Option<ItemStack>,
    off: Option<ItemStack>,
}

fn stack(name: &str, count: u8) -> ItemStack {
    ItemStack { name: name.to_owned(), count, damage: 0, block: None, tag: None }
}

fn data(stack: &ItemStack) -> sapling_data::Stack<'_> {
    sapling_data::Stack { name: &stack.name, count: stack.count, damage: stack.damage, block: stack.block, tag: stack.tag.as_ref() }
}

/// The offhand as BDS showed it changing while alive: slot 1.
fn offhand(id: u64, item: Option<&ItemStack>) -> MobEquipment {
    let item = item.and_then(|item| sapling_data::mob_hand(id, &data(item), true)).map_or_else(sapling_data::no_item, |held| held.item);
    MobEquipment { runtime_entity_id: id, item, slot: OFFHAND, selected_slot: 0, window_id: WindowID::Offhand }
}

fn main_hand(id: u64, item: &ItemStack) -> Option<Bytes> {
    sapling_data::mob_hand(id, &data(item), false).map(|held| sapling_net::encode(&held))
}

/// What shows what a piglin holds, as a viewer that comes is shown it.
pub(super) fn held(world: &World, piglin: Entity, id: u64) -> Vec<Bytes> {
    let Some(hands) = world.get::<Hands>(piglin) else { return Vec::new() };
    let off = hands.off.as_ref().and_then(|item| sapling_data::mob_hand(id, &data(item), true)).map(|held| sapling_net::encode(&held));
    hands.main.as_ref().and_then(|item| main_hand(id, item)).into_iter().chain(off).collect()
}

/// A stored piglin's hands become its own, no longer shown from its stored look.
pub(super) fn adopt(world: &mut World, piglin: Entity) {
    if world.get::<Kind>(piglin).is_none_or(|kind| kind.0 != PIGLIN) {
        return;
    }
    let Some(mut creature) = world.get_mut::<Creature>(piglin) else { return };
    let Some(look) = creature.look.as_mut() else { return };
    let hands = Hands { main: look.mainhand.take(), off: look.offhand.take() };
    world.entity_mut(piglin).insert(hands);
}

/// Writes what each kept piglin holds into its stored compound.
pub(super) fn keep(world: &mut World) {
    let held: Vec<(Entity, [Option<ItemStack>; 2])> =
        world.query_filtered::<(Entity, &Hands), With<entities::Stored>>().iter(world).map(|(piglin, hands)| (piglin, [hands.main.clone(), hands.off.clone()])).collect();
    for (piglin, hands) in held {
        entities::restore(world, piglin, |compound, registry| {
            let stored = sapling_world::stored_actors(compound).ok()?.into_iter().next()?.0?;
            let armor = std::array::from_fn(|slot| stored.armor.get(slot).cloned().flatten());
            sapling_world::geared_actor(compound, &armor, &hands, sapling_world::actor_pose(compound).unwrap_or(0), registry).ok()
        });
    }
}

/// The piglin starts admiring `item` in its offhand, shown `delay` ticks on and heard a tick later.
fn admire(world: &mut World, piglin: Entity, layer: Entity, item: ItemStack, delay: u64) {
    let (Some(id), Some(body)) = (world.get::<NetId>(piglin).map(|id| id.0), world.get::<Body>(piglin).copied()) else { return };
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    let shown = sapling_net::encode(&offhand(id, Some(&item)));
    let main = world.get_mut::<Hands>(piglin).and_then(|mut hands| hands.main.take());
    world.entity_mut(piglin).insert((Admiring { until: now + ADMIRES }, Hands { main, off: Some(item) }));
    if delay == 0 {
        entities::tell_viewers(world, piglin, layer, cell, vec![shown]);
    } else {
        late_sounds::play_later(world, delay, layer, cell, shown);
    }
    late_sounds::play_later(world, delay + 1, layer, cell, super::hunting::sound("admire", id, PIGLIN, &body, HEAD));
}

/// The player clicked `piglin`: false unless it is a grown one, not admiring, and the player holds gold.
pub(crate) fn barter(world: &mut World, player: Entity, piglin: Entity) -> bool {
    let is_piglin = world.get::<Kind>(piglin).is_some_and(|kind| kind.0 == PIGLIN) && world.get::<Dying>(piglin).is_none();
    let holds_gold = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name == GOLD)).unwrap_or(false);
    if !is_piglin || !holds_gold || super::is_baby(world, piglin) || world.get::<Admiring>(piglin).is_some() {
        return false;
    }
    let Some(layer) = world.get::<InLayer>(piglin).map(|layer| layer.0) else { return false };
    if world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative) {
        inventory::use_up_held(world, player);
    }
    admire(world, piglin, layer, stack(GOLD, 1), 0);
    true
}

/// Piglins done admiring empty their offhand: an ingot is bartered, anything else goes to the main hand.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let done: Vec<(Entity, Entity, u64, Body)> = world
        .query::<(Entity, &Admiring, &InLayer, &NetId, &Body)>()
        .iter(world)
        .filter(|(_, admiring, ..)| now >= admiring.until)
        .map(|(piglin, _, layer, id, body)| (piglin, layer.0, id.0, *body))
        .collect();
    for (piglin, layer, id, body) in done {
        world.entity_mut(piglin).remove::<Admiring>();
        let Some(mut hands) = world.get_mut::<Hands>(piglin) else { continue };
        let Some(admired) = hands.off.take() else { continue };
        let barters = admired.name == GOLD;
        let mut told: Vec<Bytes> = Vec::new();
        if !barters {
            told.extend(main_hand(id, &admired));
            hands.main = Some(admired);
        }
        told.push(sapling_net::encode(&offhand(id, None)));
        entities::tell_viewers(world, piglin, layer, body.at.map(|c| c.floor() as i32), told);
        if barters {
            hand_back(world, layer, &body);
        }
    }
}

fn hand_back(world: &mut World, layer: Entity, body: &Body) {
    let loot = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::table_loot(BARTER, &mut || dice.fraction())
    };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return };
    let stacks: Vec<_> = loot.into_iter().filter_map(|(name, count)| inventory::stored(&Item::new(name, count.min(64) as u8), registry)).collect();
    for stack in stacks {
        drops::toss(world, layer, stack, [body.at[0], body.at[1] + LIFT, body.at[2]]);
    }
}
