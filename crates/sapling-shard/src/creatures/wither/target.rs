//! The wither's prey, as its pack's `nearest_attackable_target` keeps it (bds-ab1wither2.txt,
//! bds-ah1wither-gdb2.txt): it looks on the hunters' beat, while charging too, takes the nearest as
//! [`heads::prey`] has it and keeps it while it lives within 70 in its layer. Each take and loss is told
//! as `TargetEid`, and a take is heard `mob.warning` a tick on, as any hunter's growl. Its heads and
//! flight follow the prey a tick late: BDS's `WitherTarget1` came a tick after its `TargetEid`.

use bevy_ecs::prelude::*;

use super::super::{hunting, is_alive};
use super::{KIND, Wither, heads};
use crate::body::Body;
use crate::components::{InLayer, NetId, Position, Tick};
use crate::{entities, late_sounds, vitals};

/// Its growl comes from 0.85 of its pack's collision height (3) up, not of its 3.5 box: 2.55 over its
/// feet at the summon in both captures.
const PACK_HEIGHT: f32 = 3.0;

/// Its prey as the last tick left it, and where it stands.
pub(super) fn held(world: &World, wither: Entity) -> Option<(Entity, [f32; 3])> {
    let prey = world.get::<Wither>(wither)?.prey?;
    Some((prey, hunting::feet_of(world, prey)?))
}

/// Where `prey` stands if the wither may keep it: alive (a player mortal), in its layer, within its sight.
fn keeps(world: &World, prey: Entity, layer: Entity, body: &Body) -> Option<[f32; 3]> {
    let feet = hunting::feet_of(world, prey)?;
    let alive = if world.get::<Position>(prey).is_some() { vitals::is_mortal(world, prey) } else { is_alive(world, prey) };
    (alive && world.get::<InLayer>(prey)?.0 == layer && hunting::distance(feet, body.at) <= heads::SIGHT).then_some(feet)
}

/// Lets go of a prey it may no longer keep, or on its beat takes the nearest, and tells the change.
pub(super) fn look(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    let (Some(had), Some(id)) = (world.get::<Wither>(wither).map(|kept| kept.prey), world.get::<NetId>(wither).map(|id| id.0)) else { return };
    let now = world.resource::<Tick>().0;
    let prey = match had {
        Some(prey) => keeps(world, prey, layer, body).map(|_| prey),
        None if (now + id).is_multiple_of(hunting::LOOKS) => heads::prey(world, wither, layer, body).map(|(prey, _)| prey),
        None => None,
    };
    if prey == had {
        return;
    }
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.prey = prey;
    }
    let target = prey.and_then(|prey| world.get::<NetId>(prey)).map_or(0, |prey| prey.0 as i64);
    let cell = body.at.map(|c| c.floor() as i32);
    entities::tell_viewers(world, wither, layer, cell, vec![hunting::targets(id, target)]);
    if prey.is_some() {
        let up = hunting::HEAD * PACK_HEIGHT / body.size[1];
        late_sounds::play_later(world, 1, layer, cell, hunting::sound("mob.warning", id, KIND, body, up));
    }
}
