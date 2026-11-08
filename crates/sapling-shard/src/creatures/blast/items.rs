//! Items lying in a blast: hurt as a mob is (reached.rs), gone once their 5 health is, and heard hurt
//! either way, `entity_damage` from their cell the next tick (research/captures/bds-b1sculk.txt: stacks 4
//! from a TNT behind an obsidian wall took the 1 a hidden body takes, lay on and were heard).

use bevy_ecs::prelude::*;

use super::reached::{harm_by, impact};
use crate::blocks::GameEvent;
use crate::body::Body;
use crate::{drops, mobs};

const HEALTH: f32 = 5.0;

/// What blasts have taken of an item's health.
#[derive(Component)]
pub(super) struct Harmed(f32);

/// A blast at `at` reaching `reach` hurts the items lying in it.
pub(super) fn blast_items(world: &mut World, layer: Entity, at: [f32; 3], reach: f32) {
    let lying = drops::lying_in(world, layer, at.map(|c| c - reach), at.map(|c| c + reach));
    for (item, _) in lying {
        let Some(its) = world.get::<Body>(item).copied() else { continue };
        let Some(impact) = impact(world, layer, at, its.at, its.size, reach) else { continue };
        let taken = world.get::<Harmed>(item).map_or(0.0, |harmed| harmed.0) + harm_by(impact, reach);
        crate::blocks::vibrate_next_tick(world, layer, its.at.map(|c| c.floor() as i32), GameEvent::EntityDamage, None);
        if taken >= HEALTH {
            mobs::remove(world, item);
        } else {
            world.entity_mut(item).insert(Harmed(taken));
        }
    }
}
