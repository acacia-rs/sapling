//! The four newest effects on a player, done as on a mob (creatures/omens.rs) from where BDS puts
//! a player, its eyes, with a dead player's box 0.2 high (research/captures/bds-player-omens-findings.md).

use bevy_ecs::prelude::*;
use sapling_api::Effect;
use sapling_data::EYE_HEIGHT;

use super::{SIZE, effects};
use crate::components::{InLayer, Position, Tick};
use crate::creatures::{self, Omens};

/// A dead player's box, and its burst's particle data.
const DEAD_TALL: f32 = 0.2;
const FLASH: i32 = 3;

fn eyes(world: &World, player: Entity) -> Option<(Entity, [f32; 3], f32)> {
    let layer = world.get::<InLayer>(player)?.0;
    let position = world.get::<Position>(player)?;
    let [x, y, z] = position.feet;
    Some((layer, [x, y + EYE_HEIGHT, z], position.yaw))
}

/// What a dead player's effects do, and the tick they do it in.
#[derive(Component)]
pub(super) struct Due {
    omens: Omens,
    layer: Entity,
    at: [f32; 3],
    tick: u64,
}

/// The player died: its wind charged, weaving and oozing do what they do the tick after, as BDS's
/// burst, webs and slimes came (research/captures/bds-player-omens.txt).
pub(super) fn died(world: &mut World, player: Entity) {
    let has = |effect| effects::level(world, player, effect).is_some();
    let omens = Omens { wind: has(Effect::WindCharged), weave: has(Effect::Weaving), ooze: has(Effect::Oozing) };
    let tick = world.resource::<Tick>().0 + 1;
    if let Some((layer, at, _)) = eyes(world, player).filter(|_| omens.wind || omens.weave || omens.ooze) {
        world.entity_mut(player).insert(Due { omens, layer, at, tick });
    }
}

/// Does the omens due by this tick.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, Omens, Entity, [f32; 3])> = world.query::<(Entity, &Due)>().iter(world).filter(|(_, due)| due.tick <= now).map(|(player, due)| (player, due.omens, due.layer, due.at)).collect();
    for (player, omens, layer, at) in due {
        world.entity_mut(player).remove::<Due>();
        creatures::befall(world, layer, omens, at, DEAD_TALL, FLASH);
    }
}

/// The player was hurt and lives: infested may let out silverfish.
pub(super) fn hurt(world: &mut World, player: Entity) {
    if effects::level(world, player, Effect::Infested).is_none() {
        return;
    }
    if let Some((layer, [x, y, z], facing)) = eyes(world, player) {
        creatures::infested_hurt(world, layer, [x, y + SIZE[1] / 2.0, z], facing);
    }
}
