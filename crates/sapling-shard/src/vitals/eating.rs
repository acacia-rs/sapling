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
/// An emptied bottle is heard from this far under the eyes and this far ahead of them.
const BOTTLE_BELOW: f32 = 0.2;
const BOTTLE_AHEAD: f32 = 0.2;
/// Chorus fruit's cooldown category, as BDS names it, and its length.
const CHORUS: &str = "minecraft:chorusfruit";
const CHORUS_COOLDOWN: u64 = 20;

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
    let chorus = holds_chorus(world, player);
    if chorus && crate::cooldowns::is_cooling(world, player, CHORUS) {
        return;
    }
    let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| vitals.meal.is_none() && (food.always || !vitals.hunger.is_full())) else { return };
    vitals.meal = Some(Meal { id, food, done: now + u64::from(food.ticks) });
    if chorus {
        crate::cooldowns::tell(world, player, CHORUS, CHORUS_COOLDOWN, false);
    }
    show(world, player, true);
}

/// Chorus fruit cools down once eaten; BDS told it as the eating started too (research/captures/bds-small.txt).
fn holds_chorus(world: &World, player: Entity) -> bool {
    world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).is_some_and(|held| held.stack.name == "minecraft:chorus_fruit")
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
        let Some(effect) = sapling_api::Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect).filter(|_| lucky) else { continue };
        // A stew's effect is told twice, Add then Update (bds-foods2.txt).
        let told = if food.leaves == Some("minecraft:bowl") { 2 } else { 1 };
        for _ in 0..told {
            effects::give(world, player, effect, gives.ticks, gives.amplifier, !food.ominous);
        }
    }
}

/// What held a food is left in its place.
fn leave(world: &mut World, player: Entity, food: &sapling_data::Food) {
    let registry = world.get::<InLayer>(player).and_then(|layer| world.get::<Layer>(layer.0)).map(|layer| layer.world.registry());
    if let Some(left) = food.leaves.zip(registry).and_then(|(name, registry)| inventory::stored(&sapling_api::Item::new(name, 1), registry)) {
        inventory::leave_in_hand(world, player, &left);
    }
}

/// A drink is gone: the bottle is heard emptied, under the eyes and ahead of them, as BDS's
/// (research/captures/bds-drink.txt); an ominous bottle as the omen taking hold and its own end
/// (bds-fx3.txt).
// TODO: only drunk facing south (yaw 0): the way ahead is taken to turn with the yaw.
fn drained(world: &mut World, player: Entity, ominous: bool) {
    let (Some(position), Some(id)) = (world.get::<Position>(player).copied(), world.get::<NetId>(player).map(|id| id.0)) else { return };
    let ([x, y, z], (sin, cos)) = (position.feet, position.yaw.to_radians().sin_cos());
    let position = Vec3f { x: x - sin * BOTTLE_AHEAD, y: y + EYE_HEIGHT - BOTTLE_BELOW, z: z + cos * BOTTLE_AHEAD };
    let heard = |sound_id: &str, entity_type: &str, entity_unique_id: i64| LevelSoundEvent {
        sound_id: sound_id.into(), position: position.clone(), extra_data: -1, entity_type: entity_type.into(), is_baby_mob: false, is_global: false, entity_unique_id, fire_at_position: None,
    };
    if ominous {
        tell(world, player, &heard("apply_effect.bad_omen", "minecraft:player", id as i64));
        tell(world, player, &heard("ominous_bottle.end_use", "", -1));
    } else {
        tell(world, player, &heard("bottle.empty", "", -1));
    }
}

/// The food is eaten: one less in the hand, the hunger fed, and a burp; a drink empties its bottle.
fn finish(world: &mut World, player: Entity, meal: Meal) {
    if !crate::life_events::may_consume(world, player) {
        return show(world, player, false);
    }
    let now = world.resource::<Tick>().0;
    let use_method = match (meal.food.drunk, meal.food.cures) {
        (true, true) => CompletedUsingItemUseMethod::Unknown(-1),
        (true, false) => CompletedUsingItemUseMethod::Consume,
        (false, _) => CompletedUsingItemUseMethod::Eat,
    };
    let used = CompletedUsingItem { used_item_id: meal.id, use_method };
    if holds_chorus(world, player) {
        crate::cooldowns::tell(world, player, CHORUS, CHORUS_COOLDOWN, true);
        super::chorus::scatter(world, player);
    }
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
        drained(world, player, meal.food.ominous);
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
