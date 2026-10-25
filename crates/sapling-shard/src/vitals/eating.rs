//! Eating and drinking: use held on a food until the shard takes it, as BDS does
//! (docs/research/player-damage.md, "Eating"; food-effects.md).
// TODO: BDS also sends every attribute as the use starts; only survival and adventure players
// eat here.

use acacia_proto::packets::{CompletedUsingItem, CompletedUsingItemUseMethod, EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use super::{Vitals, effects, is_mortal, tell};
use crate::blocks::Dice;
use crate::components::{InLayer, Layer, NetId, Outgoing, PacketQueues, Position, Tick};
use crate::inventory::{self, Inventory};
use crate::movement::{self, Motion};

/// A burp is heard from this far under the eyes.
const MOUTH: f32 = 0.1;

/// A food being eaten: its item's network id, what it feeds, and the tick it is eaten in.
#[derive(Clone, Copy)]
pub(super) struct Meal {
    id: i16,
    food: sapling_data::Food,
    done: u64,
}

fn held(world: &World, player: Entity) -> Option<(i16, sapling_data::Food)> {
    let hand = world.get::<Inventory>(player)?.in_hand();
    let stack = &hand.as_ref()?.stack;
    Some((inventory::wire(hand, false).network_id, sapling_data::consumable(&stack.name, stack.damage)?))
}

/// Shows the player and, through `movement::announce`, its viewers whether it is using an item.
pub(crate) fn show(world: &mut World, player: Entity, using: bool) {
    let tick = world.resource::<Tick>().0;
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return };
    let Some(mut motion) = world.get_mut::<Motion>(player).filter(|motion| motion.pose.using != using) else { return };
    motion.pose.using = using;
    let shown = movement::pose_packet(id, motion.pose, tick);
    world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&shown);
}

/// A click in the air: a hungry player starts on the food in its hand. A click while it eats is
/// the client's own end of the use, which the shard times itself.
pub(crate) fn start(world: &mut World, player: Entity) {
    let now = world.resource::<Tick>().0;
    let Some((id, food)) = held(world, player).filter(|_| is_mortal(world, player)) else { return };
    let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| vitals.meal.is_none() && (food.always || !vitals.hunger.is_full())) else { return };
    vitals.meal = Some(Meal { id, food, done: now + u64::from(food.ticks) });
    show(world, player, true);
}

/// Use let go before the food was eaten.
/// A bite of what is not held, worth `[hunger, saturation]`: false for a survival player who is
/// full.
pub(crate) fn bite(world: &mut World, player: Entity, [hunger, saturation]: [f32; 2]) -> bool {
    let is_full = super::is_mortal(world, player) && world.get::<super::Vitals>(player).is_some_and(|vitals| vitals.hunger.is_full());
    if !is_full {
        super::feed(world, player, |eater| eater.feed(hunger, saturation));
    }
    !is_full
}

pub(crate) fn stop(world: &mut World, player: Entity) {
    if world.get_mut::<Vitals>(player).is_some_and(|mut vitals| vitals.meal.take().is_some()) {
        show(world, player, false);
    }
}

/// What a food does besides feeding: the effects it gives, each as often as it does; milk ends
/// them all.
fn afflict(world: &mut World, player: Entity, food: &sapling_data::Food) {
    if food.cures {
        effects::clear(world, player);
    }
    if food.ends_poison {
        effects::end(world, player, sapling_api::Effect::Poison);
    }
    for gives in food.gives {
        let lucky = gives.chance >= 1.0 || world.get_resource_or_insert_with(Dice::default).fraction() < gives.chance;
        if let Some(effect) = sapling_api::Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect).filter(|_| lucky) {
            effects::give(world, player, effect, gives.ticks, gives.amplifier, true);
        }
    }
}

/// What held a food is left in its place.
fn leave(world: &mut World, player: Entity, food: &sapling_data::Food) {
    let registry = world.get::<InLayer>(player).and_then(|layer| world.get::<Layer>(layer.0)).map(|layer| layer.world.registry());
    if let Some(left) = food.leaves.zip(registry).and_then(|(name, registry)| inventory::stored(&sapling_api::Item::new(name, 1), registry)) {
        inventory::give_and_pop(world, player, &left);
    }
}

/// The food is eaten: one less in the hand, the hunger fed, and a burp; a drink is not heard.
fn finish(world: &mut World, player: Entity, meal: Meal) {
    let now = world.resource::<Tick>().0;
    let use_method = match (meal.food.drunk, meal.food.cures) {
        (true, true) => CompletedUsingItemUseMethod::Unknown(-1),
        (true, false) => CompletedUsingItemUseMethod::Consume,
        (false, _) => CompletedUsingItemUseMethod::Eat,
    };
    let used = CompletedUsingItem { used_item_id: meal.id, use_method };
    // BDS tells the effects first.
    afflict(world, player, &meal.food);
    world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&used);
    inventory::change(world, player, Inventory::take_one_in_hand);
    leave(world, player, &meal.food);
    let Some(mut vitals) = world.get_mut::<Vitals>(player) else { return };
    vitals.hunger.feed(meal.food.hunger, meal.food.saturation);
    let hunger = vitals.hunger;
    let (Some(id), Some([x, y, z])) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet)) else { return };
    tell(world, player, &hunger.told(id, now));
    show(world, player, false);
    if meal.food.drunk {
        return;
    }
    let position = Vec3f { x, y: y + EYE_HEIGHT - MOUTH, z };
    tell(world, player, &LevelSoundEvent { sound_id: "burp".into(), position, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None });
}

/// A tick of every meal: the client's bites are shown to those who see it, a meal whose food
/// left the hand or whose eater died is over, and one whose time has come is eaten.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let bites = world.resource_mut::<PacketQueues>().take::<EntityEvent>();
    for (player, bite) in bites.into_iter().filter(|(_, event)| event.event_id == EntityEventEventId::EatingItem) {
        let (Some(id), true) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Vitals>(player).is_some_and(|vitals| vitals.meal.is_some())) else { continue };
        tell(world, player, &EntityEvent { runtime_entity_id: id, ..bite });
    }
    let mut eating = world.query::<(Entity, &Vitals)>();
    let meals: Vec<(Entity, Meal)> = eating.iter(world).filter_map(|(player, vitals)| Some((player, vitals.meal?))).collect();
    for (player, meal) in meals {
        if !is_mortal(world, player) || held(world, player).is_none_or(|(id, _)| id != meal.id) {
            stop(world, player);
        } else if now >= meal.done {
            world.get_mut::<Vitals>(player).expect("queried above").meal = None;
            finish(world, player, meal);
        }
    }
}
