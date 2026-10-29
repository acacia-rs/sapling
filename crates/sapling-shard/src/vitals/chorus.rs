//! A chorus fruit eaten sends its eater somewhere near, as BDS did (research/captures/bds-foods2.txt):
//! a MovePlayer of cause `ChorusFruit` before the use is complete, then `teleport` heard at the
//! eyes where it was and where it came. The spot is the Java game's: 16 tries in the box 8 about
//! the feet, each let down onto the first block below.

use acacia_proto::packets::{LevelSoundEvent, MovePlayerTeleportCause};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use super::tell;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position};

const REACH: f32 = 8.0;
const TRIES: usize = 16;
const SIZE: [f32; 2] = [0.6, 1.8];
/// The source entity type BDS's MovePlayer named.
const SOURCE: i64 = 1;

/// The player ate a chorus fruit: it is put somewhere near, if a spot is found.
pub(super) fn scatter(world: &mut World, player: Entity) {
    let (Some(from), Some(layer)) = (world.get::<Position>(player).map(|position| position.feet), world.get::<InLayer>(player).map(|on| on.0)) else { return };
    world.get_resource_or_insert_with(Dice::default);
    let to = world.resource_scope(|world, mut dice: Mut<Dice>| {
        let level = &world.get::<Layer>(layer)?.world;
        let body = Body::new(from, [0.0; 3], SIZE);
        (0..TRIES).find_map(|_| {
            let mut off = || (dice.fraction() * 2.0 - 1.0) * REACH;
            let at = [from[0] + off(), from[1] + off(), from[2] + off()];
            crate::creatures::teleport_landing(level, &body, at)
        })
    });
    let (Some(to), Some(id)) = (to, world.get::<NetId>(player).map(|id| id.0)) else { return };
    crate::movement::teleport_as(world, player, to, MovePlayerTeleportCause::ChorusFruit, &[SOURCE]);
    for [x, y, z] in [from, to] {
        let position = Vec3f { x, y: y + EYE_HEIGHT, z };
        tell(world, player, &LevelSoundEvent { sound_id: "teleport".into(), position, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None });
    }
}
