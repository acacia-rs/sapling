//! An enderman stared at, as BDS's was (research/captures/bds-stare-findings.md): a survival
//! player within 64 whose look is within the Java game's `1 - 0.025 / d` of its head for 5 ticks
//! is taken as its prey, with `stare`, and it teleports.
// TODO: a carved pumpkin worn does not hide the look; the 4.9 to 9.4 degrees drilled at 6 off do
// not tell the Java game's rule from a field of view scaled by distance.

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::{late_sounds, vitals};
use super::{hunting, teleports};

const KIND: &str = "minecraft:enderman";
/// Its head is this far up it, and it is looked at from up to this far.
const HEAD: f32 = 2.465;
const SEES: f32 = 64.0;
/// A look is its own if within `1 - SLACK / d` of it, held for this many ticks.
const SLACK: f32 = 0.025;
const HELD: u64 = 5;

#[derive(Component, Default)]
pub(super) struct Stared {
    by: Option<(Entity, u64)>,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        world.entity_mut(entity).insert(Stared::default());
    }
}

/// Whether the player at `position` looks at the head at `head`.
fn looks_at(position: &Position, head: [f32; 3]) -> bool {
    let eyes = [position.feet[0], position.feet[1] + sapling_data::EYE_HEIGHT, position.feet[2]];
    let towards: [f32; 3] = std::array::from_fn(|axis| head[axis] - eyes[axis]);
    let far = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
    let (yaw, pitch) = (position.yaw.to_radians(), position.pitch.to_radians());
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    far > 1e-4 && far <= SEES && (0..3).map(|axis| look[axis] * towards[axis] / far).sum::<f32>() > 1.0 - SLACK / far
}

/// Before its move: an enderman with no prey notes who stares at it, and takes one who held the
/// stare long enough.
pub(super) fn watch(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if world.get::<Stared>(entity).is_none() || hunting::prey_at(world, entity).is_some() {
        return;
    }
    let now = world.resource::<Tick>().0;
    let head = [body.at[0], body.at[1] + HEAD, body.at[2]];
    let starer = in_world(world).into_iter().find(|player| {
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer)
            && vitals::is_mortal(world, *player)
            && world.get::<Position>(*player).is_some_and(|position| looks_at(position, head) && hunting::in_sight(world, layer, body, position.feet))
    });
    let held = world.get::<Stared>(entity).and_then(|stared| stared.by).filter(|(by, _)| Some(*by) == starer);
    let since = held.map_or(now, |(_, since)| since);
    if let Some(mut stared) = world.get_mut::<Stared>(entity) {
        stared.by = starer.map(|by| (by, since));
    }
    let Some(player) = starer.filter(|_| now >= since + HELD) else { return };
    hunting::provoke(world, entity, layer, id, kind, body, player);
    late_sounds::play_later(world, 1, layer, body.at.map(|c| c.floor() as i32), hunting::sound("stare", id, kind, body, HEAD / body.size[1]));
    teleports::startle(world, entity);
}
