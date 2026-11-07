//! An entity shown in the tick it is made, before what its maker tells next: BDS announced a grown
//! turtle's scute, then the turtle's grown scale, then the scute's metadata (bds-ak1scute.txt).

use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::{Mob, SHOWN_WITHIN, SeesMobs};
use crate::components::{InLayer, NetId, Outgoing, Stage};
use crate::hidden::Hidden;
use crate::view::View;

/// The rest of an announcement begun by [`show_first`], for the players it was begun to.
pub(crate) struct Begun {
    viewers: Vec<Entity>,
    rest: Vec<Bytes>,
}

/// Tells the players who see the mob's column its first packet now and counts it as shown to them;
/// the rest of its announcement waits for [`finish`].
pub(crate) fn show_first(world: &mut World, mob: Entity) -> Option<Begun> {
    let (shown, id, layer) = world.query::<(&Mob, &NetId, &InLayer)>().get(world, mob).ok()?;
    let (home, id, layer, mut packets) = (shown.home, id.0, layer.0, shown.announcement.clone().into_iter());
    let first = packets.next()?;
    let mut viewers = Vec::new();
    for (viewer, stage, in_layer, view, mut sees, mut out, hidden) in world.query::<(Entity, &Stage, &InLayer, &View, &mut SeesMobs, &mut Outgoing, Option<&Hidden>)>().iter_mut(world) {
        let radius = SHOWN_WITHIN.min(view.radius());
        let near = view.center().is_some_and(|c| (home.x - c.x).pow(2) + (home.z - c.z).pow(2) <= radius * radius);
        if *stage != Stage::InWorld || in_layer.0 != layer || !near || Hidden::hides(hidden, mob) || sees.has(mob) {
            continue;
        }
        out.0.push(first.clone());
        sees.mobs.insert(mob, id);
        viewers.push(viewer);
    }
    Some(Begun { viewers, rest: packets.collect() })
}

/// Tells the players [`show_first`] began the announcement to the rest of it.
pub(crate) fn finish(world: &mut World, begun: Begun) {
    for viewer in begun.viewers {
        if let Some(mut out) = world.get_mut::<Outgoing>(viewer) {
            out.0.extend(begun.rest.iter().cloned());
        }
    }
}
