//! A tamed cat's `behavior.pet_sleep_with_owner` and morning gift (its pack's `cat_tame`,
//! `cat_gift_for_owner`). As its owner lies down in a bed the cat, unless sitting, walks at 1.2 times
//! its pace onto the bed's foot and lies there (`RESTING`) till the owner gets up
//! (research/captures/bds-ar2cat.txt: it set off the tick the owner lay down, from 8 blocks off).
//! Waking with its owner in the morning (`on_wake_with_owner`), it gives a roll of
//! `cat_gift.json` 7 times in 10 (`drop_item_for`, morning only, its owner within 6).
// TODO: BDS's cat was put 4.7 blocks from the bed as it woke and gave nothing (the 3 in 10); where the
// drop goal moves a cat first is undrilled.

use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;
use sapling_api::Item;

use super::{Raised, distance, shown};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Position};
use crate::creatures::legs;

const CAT: &str = "minecraft:cat";
const SPEED: f32 = 1.2;
/// How far across and up or down a cat looks for its owner's bed.
const SEARCH: f32 = 16.0;
const SEARCH_UP: f32 = 10.0;
const ARRIVED: f32 = 0.3;
const GIFT: &str = "loot_tables/entities/cat_gift.json";
const GIFT_CHANCE: f32 = 0.7;
const GIFT_REACH: f32 = 6.0;
/// `time_of_day` 0.75 to 0.8: the day's first 1200 ticks.
const MORNING: i64 = 1200;
const DAY: i64 = 24000;

/// A cat on its way to its owner's bed, or lying on it.
#[derive(Component)]
pub(super) struct Bedtime {
    lying: bool,
}

/// Before the move of a tamed cat: it goes to its sleeping owner's bed and lies on it; as the owner gets
/// up it gets up too and may leave a gift. False when its owner is not in bed.
pub(super) fn lie(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    if kind != CAT {
        return false;
    }
    let Some(owner) = world.get::<Raised>(entity).filter(|raised| raised.is_tamed()).and_then(|raised| raised.kept.owner).and_then(|owner| shown::player_of(world, owner)) else { return false };
    let Some(body) = world.get::<Body>(entity).copied() else { return false };
    let bed = crate::sleep::bed_foot(world, owner).filter(|(on, [x, y, z])| *on == layer && (*y as f32 - body.at[1]).abs() <= SEARCH_UP && (*x as f32 + 0.5 - body.at[0]).abs() <= SEARCH && (*z as f32 + 0.5 - body.at[2]).abs() <= SEARCH);
    let Some((_, [x, _, z])) = bed else {
        if let Some(bedtime) = world.entity_mut(entity).take::<Bedtime>() {
            if bedtime.lying {
                shown::flag(world, entity, Flags::RESTING, false);
            }
            wake(world, layer, &body, owner);
        }
        return false;
    };
    let foot = [x as f32 + 0.5, z as f32 + 0.5];
    let lying = world.get::<Bedtime>(entity).is_some_and(|bedtime| bedtime.lying);
    if (foot[0] - body.at[0]).hypot(foot[1] - body.at[2]) > ARRIVED && !lying {
        let walks = sapling_data::husbandry(kind).map_or(0.3, |rules| rules.walks);
        legs::walk(world, entity, layer, &body, foot, walks * SPEED, 0.0);
        world.entity_mut(entity).insert(Bedtime { lying: false });
        return true;
    }
    legs::rest(world, entity);
    if !lying {
        world.entity_mut(entity).insert(Bedtime { lying: true });
        shown::flag(world, entity, Flags::RESTING, true);
    }
    true
}

/// The cat got up with its owner: in the morning, with the owner near, it may leave a gift.
fn wake(world: &mut World, layer: Entity, body: &Body, owner: Entity) {
    let morning = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().time.rem_euclid(DAY) < MORNING);
    let near = world.get::<Position>(owner).is_some_and(|position| distance(position.feet, body.at) <= GIFT_REACH);
    if !morning || !near || world.get_resource_or_insert_with(Dice::default).fraction() >= GIFT_CHANCE {
        return;
    }
    let loot = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::table_loot(GIFT, &mut || dice.fraction())
    };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return };
    let stacks: Vec<_> = loot.into_iter().filter_map(|(name, count)| crate::inventory::stored(&Item::new(name, count.min(64) as u8), registry)).collect();
    let centre = [body.at[0], body.at[1] + body.size[1] * 0.5, body.at[2]];
    for stack in stacks {
        crate::drops::hand_over(world, layer, stack, centre, [0.0; 3]);
    }
}
