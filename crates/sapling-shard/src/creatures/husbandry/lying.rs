//! A lazy panda lying down (`panda_lazy`'s `behavior.lay_down`), as BDS's lay (research/captures/bds-x2panda.txt):
//! with nothing else to do it lies down at 1 in 400 a tick, `FlagsExtended` bit 0 on, and keeps
//! still until it gets up, also at 1 in 400 a tick. Six lie-downs of four pandas in 2900 standing
//! ticks fit the pack's 400 to start; their lengths (30, 66, 278, 697 ticks and two cut short) fit
//! 400 to stop, not the pack's `random_stop_interval` 2000. From the tick after either, its
//! `LayingAmount` goes up 0.15 a tick to 1 or down 0.19 to 0, `LayingAmountPrevious` a tick behind.

use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::shown;
use crate::blocks::Dice;
use crate::body::Body;
use crate::creatures::legs;

const PANDA: &str = "minecraft:panda";
const LIES_DOWN: f32 = 1.0 / 400.0;
const GETS_UP: f32 = 1.0 / 400.0;
const DOWN: f32 = 0.15;
const UP: f32 = 0.19;
/// `FlagsExtended` at rest, and the bit on while it lies.
const REST: i64 = 1 << 25;
const LYING: i64 = 1;

#[derive(Component, Clone, Copy)]
pub(super) struct Lying {
    down: bool,
    amount: f32,
    previous: f32,
}

fn float(key: Key, value: f32) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(value)) }
}

/// Before the move of a panda lying or getting up: its `LayingAmount` moves on a step, and one lying
/// keeps still or gets up. False for one not lying.
pub(super) fn keep_lying(world: &mut World, entity: Entity) -> bool {
    let Some(lying) = world.get::<Lying>(entity).copied() else { return false };
    let amount = if lying.down { (lying.amount + DOWN).min(1.0) } else { (lying.amount - UP).max(0.0) };
    let changed: Vec<MetadataDictionaryItem> = [(Key::LayingAmount, lying.amount, amount), (Key::LayingAmountPrevious, lying.previous, lying.amount)]
        .into_iter()
        .filter(|(_, was, now)| was != now)
        .map(|(key, _, now)| float(key, now))
        .collect();
    if !changed.is_empty() {
        shown::metadata(world, entity, changed);
    }
    let gets_up = lying.down && (crate::creatures::strolling::is_fleeing(world, entity) || world.get_resource_or_insert_with(Dice::default).fraction() < GETS_UP);
    let down = lying.down && !gets_up;
    if gets_up {
        shown::flags_extended(world, entity, REST);
    }
    if !down && amount == 0.0 && lying.amount == 0.0 {
        world.entity_mut(entity).remove::<Lying>();
    } else {
        world.entity_mut(entity).insert(Lying { down, amount, previous: lying.amount });
    }
    if down {
        legs::rest(world, entity);
    }
    down
}

/// Before the move of a panda with nothing else to do: a lazy one may lie down.
pub(super) fn start(world: &mut World, entity: Entity, kind: &str) -> bool {
    let lies = kind == PANDA && crate::creatures::genes::is_lazy(world, entity) && world.get::<Lying>(entity).is_none();
    let grounded = world.get::<Body>(entity).is_some_and(|body| body.on_ground);
    if !lies || !grounded || crate::creatures::strolling::is_fleeing(world, entity) || world.get_resource_or_insert_with(Dice::default).fraction() >= LIES_DOWN {
        return false;
    }
    world.entity_mut(entity).insert(Lying { down: true, amount: 0.0, previous: 0.0 });
    shown::flags_extended(world, entity, REST | LYING);
    legs::rest(world, entity);
    true
}
