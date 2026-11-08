//! A mob moved by a command or plugin into a player's view is shown before the creatures' tick:
//! BDS answered the `/tp`, announced the poisoned mob it brought back whole, then hurt it, all in
//! one tick (research/captures/bds-tickcircle6.txt; mob-damage.md "view circle").

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ChunkPos;

use super::{Mob, SHOWN_WITHIN, Viewer, follow, placed};
use crate::components::{InLayer, NetId, Stage};
use crate::hidden::Hidden;
use crate::labels::Labels;
use crate::view::View;

/// The mobs moved since the last [`show_moved`].
#[derive(Resource, Default)]
pub(crate) struct Moved(Vec<Entity>);

pub(crate) fn moved(world: &mut World, mob: Entity) {
    if world.get::<Mob>(mob).is_some() {
        world.get_resource_or_insert_with(Moved::default).0.push(mob);
    }
}

/// Files each moved mob under the column it is in and announces it there to the players whose
/// view covers that column and who do not see it yet; [`super::update`] takes it as seen.
pub(crate) fn show_moved(world: &mut World) {
    let Some(moved) = world.get_resource_mut::<Moved>().map(|mut moved| std::mem::take(&mut moved.0)) else { return };
    for mob in moved {
        if let Some(feet) = world.get::<crate::body::Body>(mob).map(|body| body.at) {
            show(world, mob, feet);
        }
    }
}

fn show(world: &mut World, mob: Entity, feet: [f32; 3]) {
    follow(world, mob, feet);
    let Some(layer) = world.get::<InLayer>(mob).map(|in_layer| in_layer.0) else { return };
    let Some(id) = world.get::<NetId>(mob).map(|id| id.0) else { return };
    let labels = crate::labels::packet(world.get::<Labels>(mob), id);
    let Some(mut shown) = world.get_mut::<Mob>(mob) else { return };
    if let Some(first) = shown.announcement.first_mut() {
        *first = placed(first, feet);
    }
    let home = shown.home;
    let (announcement, after) = (shown.announcement.clone(), shown.after.clone());
    let announced = super::live::as_now(announcement, world.get::<crate::sync::Told>(mob).filter(|_| !crate::entities::is_new(world, mob)));
    let packets: Vec<Bytes> = announced.into_iter().chain(labels).chain(after).collect();
    for (stage, in_layer, view, mut sees, mut out, hidden) in world.query::<Viewer>().iter_mut(world) {
        if *stage != Stage::InWorld || in_layer.0 != layer || sees.mobs.contains_key(&mob) || Hidden::hides(hidden, mob) || !shows(view, home) {
            continue;
        }
        out.0.extend_from_slice(&packets);
        sees.mobs.insert(mob, id);
    }
}

fn shows(view: &View, home: ChunkPos) -> bool {
    let radius = SHOWN_WITHIN.min(view.radius());
    view.center().is_some_and(|center| (home.x - center.x).pow(2) + (home.z - center.z).pow(2) <= radius * radius)
}
