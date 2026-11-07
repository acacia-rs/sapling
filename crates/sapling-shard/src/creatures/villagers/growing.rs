//! A baby villager growing up (its pack's `minecraft:ageable_grow_up`): it stays unskilled, or one
//! in ten becomes a nitwit (the event's weights 90 and 10). Its age and box are husbandry's.

use acacia_proto::types::MetadataDictionaryItemKey as Key;
use bevy_ecs::prelude::*;

use super::jobs::{self, NITWIT};
use super::{Trader, int, tell};
use crate::blocks::Dice;

const NITWIT_ODDS: u32 = 10;

/// The mob `entity` grew up; for a villager, its profession may change.
pub(in crate::creatures) fn grew_up(world: &mut World, entity: Entity) {
    let Some(mut trader) = world.get::<Trader>(entity).filter(|trader| trader.leaves_at.is_none()).cloned() else { return };
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    let nitwit = dice.roll(NITWIT_ODDS) == 0;
    if nitwit {
        jobs::take_up(&mut trader, NITWIT, &mut dice);
    }
    world.insert_resource(dice);
    if nitwit {
        let changed = vec![int(Key::Variant, NITWIT), int(Key::MaxTradeTier, trader.max_tier())];
        world.entity_mut(entity).insert(trader);
        tell(world, entity, changed);
    }
}
