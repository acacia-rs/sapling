//! A bat hangs from the block over its head (the Java game's `Bat::customServerAiStep`), as BDS's
//! did (research/captures/bds-g14bat.txt, bds-g14fly.txt): one born under a ceiling hangs from the
//! start, its `AddEntity` showing `RESTING`; one flying under one takes hold at 1 in 100 a tick
//! (BDS's took 5 s). It hangs still, its head against the ceiling, and lets go the tick the block
//! over it goes.
// TODO: a player within 4 waking it (the Java game's) is not done: BDS's actor stood 4.65 off.

use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Layer;
use crate::entities;

const KIND: &str = "minecraft:bat";
const TAKES_HOLD: f32 = 0.01;

#[derive(Component)]
pub(super) struct Hanging;

/// The flags a bat of `kind` born as `body` is shown with: `RESTING` under a ceiling.
pub(super) fn born_flags(level: &Level, kind: &str, body: &Body) -> Flags {
    if kind == KIND && ceiling(level, body) { Flags::RESTING } else { Flags(0) }
}

/// The flags the mob shows as it is announced again: `RESTING` while it hangs.
pub(super) fn hanging_flags(world: &World, entity: Entity) -> Flags {
    if world.get::<Hanging>(entity).is_some() { Flags::RESTING } else { Flags(0) }
}

/// A bat born hanging holds on.
pub(super) fn arm(world: &mut World, entity: Entity, layer: Entity, kind: &str) {
    let (Some(body), Some(level)) = (world.get::<Body>(entity).copied(), world.get::<Layer>(layer).map(|layer| &layer.world)) else { return };
    if born_flags(level, kind, &body) != Flags(0) {
        world.entity_mut(entity).insert(Hanging);
        entities::mob_flags(world, entity, kind, Flags::RESTING, true);
        hold(world, entity, &body);
    }
}

fn ceiling(level: &Level, body: &Body) -> bool {
    let [x, y, z] = body.at.map(|c| c.floor() as i32);
    level.in_height(y + 1) && super::harm::is_full_block(level, [x, y + 1, z])
}

/// Before its move: a hanging bat holds on, or lets go as its ceiling goes; a flying one under a
/// ceiling may take hold. False for a bat flying on, or another kind.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (true, Some(body), Some(level)) = (kind == KIND, world.get::<Body>(entity).copied(), world.get::<Layer>(layer).map(|layer| &layer.world)) else { return false };
    let under = ceiling(level, &body);
    let hanging = world.get::<Hanging>(entity).is_some();
    let takes_hold = !hanging && under && world.get_resource_or_insert_with(Dice::default).fraction() < TAKES_HOLD;
    if (hanging && !under) || takes_hold {
        if takes_hold {
            world.entity_mut(entity).insert(Hanging);
        } else {
            world.entity_mut(entity).remove::<Hanging>();
        }
        super::husbandry::show_flag(world, entity, Flags::RESTING, takes_hold);
    }
    let holds = world.get::<Hanging>(entity).is_some();
    if holds {
        hold(world, entity, &body);
    }
    holds
}

fn hold(world: &mut World, entity: Entity, body: &Body) {
    let top = body.at[1].floor() + 1.0 - body.size[1];
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        body.at[1] = top;
        body.fling([0.0; 3]);
    }
}
