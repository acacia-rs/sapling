//! Entities shown a tick after they are made, as BDS showed a dispensed shot: unseen the tick it
//! leaves, then announced as it stands, already a move along, before its second move, with what
//! follows it (an arrow's click) after (research/captures/bds-p3disp.txt, bds-p2shoot.txt).

use acacia_proto::RawPacket;
use acacia_proto::packets::AddEntity;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::{MobGrid, SHOWN_WITHIN, Viewer, announced, reannounce};
use crate::components::{InLayer, NetId, Stage};
use crate::hidden::Hidden;

/// Kept off every screen until [`show_late`]; `after` is told right after its announcement.
#[derive(Component)]
pub(crate) struct ShownLate {
    pub after: Vec<Bytes>,
}

/// Shows `mob`, if it waits to be shown, to every player who sees its column now: its announcement
/// with its body at `at` moving `velocity`, then what was kept for after it.
pub(crate) fn show_late(world: &mut World, mob: Entity, at: [f32; 3], velocity: [f32; 3]) {
    let Some(ShownLate { after }) = world.entity_mut(mob).take::<ShownLate>() else { return };
    let (Some(mut shown), Some(id), Some(layer)) = (announced(world, mob), world.get::<NetId>(mob).map(|id| id.0), world.get::<InLayer>(mob).map(|layer| layer.0)) else { return };
    if let Some(mut added) = shown.first().and_then(|first| RawPacket::parse(first.clone()).ok()?.decode::<AddEntity>().ok()) {
        added.position = Vec3f { x: at[0], y: at[1], z: at[2] };
        added.velocity = Vec3f { x: velocity[0], y: velocity[1], z: velocity[2] };
        shown[0] = sapling_net::encode(&added);
        reannounce(world, mob, shown.clone());
    }
    world.resource_scope(|world, grid: Mut<MobGrid>| {
        for (stage, in_layer, view, mut sees, mut out, hidden) in world.query::<Viewer>().iter_mut(world) {
            let covers = grid.grid.within(layer, view, SHOWN_WITHIN).any(|(seen, _)| *seen == mob);
            if *stage != Stage::InWorld || in_layer.0 != layer || !covers || Hidden::hides(hidden, mob) || sees.has(mob) {
                continue;
            }
            out.0.extend(shown.iter().cloned().chain(after.iter().cloned()));
            sees.mobs.insert(mob, id);
        }
    });
}
