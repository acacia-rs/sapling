//! What a cartography table and a crafting grid ask of a map, and the maps a table makes.

use bevy_ecs::prelude::*;

use super::{Maps, MapData, centre, kept, map_id, marks, render, tell_layer};
use crate::components::{InLayer, Tick};
use crate::inventory::{self, CARTOGRAPHY_INPUT, Chart, Inventory, Place};

/// What a player's cartography table knows of the map in its input.
pub(crate) fn chart(world: &World, player: Entity) -> Chart {
    let maps = world.get::<InLayer>(player).and_then(|layer| world.get::<Maps>(crate::dimensions::home(world, layer.0)));
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.slot(Place::Ui(CARTOGRAPHY_INPUT))?.as_ref().and_then(|held| map_id(&held.stack)));
    let input = held.and_then(|id| maps?.maps.get(&id)).map(|map| (map.id, map.scale, map.locked));
    Chart { input, fresh: locked_id(world.resource::<Tick>().0, maps) }
}

/// What the level knows of the filled map in a player's crafting grid: id, scale, locked.
pub(crate) fn in_grid(world: &World, player: Entity) -> Option<(i64, u8, bool)> {
    let id = inventory::grid_map_id(world.get::<Inventory>(player)?)?;
    let home = crate::dimensions::home(world, world.get::<InLayer>(player)?.0);
    world.get::<Maps>(home)?.maps.get(&id).map(|map| (map.id, map.scale, map.locked))
}

/// The id a locked copy takes: BDS's look drawn, positive and 32 bits wide, apart from the
/// unique-id run new maps take (bds-maplock.txt: 1396458477, 2162056098, 3185328418).
fn locked_id(now: u64, maps: Option<&Maps>) -> i64 {
    use std::hash::BuildHasher;
    let hasher = std::hash::BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default();
    let taken = |id: &i64| maps.is_some_and(|maps| maps.maps.contains_key(id));
    (0u64..).map(|salt| i64::from(hasher.hash_one((now, maps.map_or(0, |maps| maps.maps.len()), salt)) as u32)).find(|id| *id > 0 && !taken(id)).unwrap_or(1)
}

/// Makes the map `to` a cartography table made of the map `from`: zoomed out (blank, a scale
/// up, centred anew around the old centre) or locked (a copy that never redraws), and tells
/// everyone in its world all of it. A zoom level made before is kept as it is.
pub(crate) fn charted(world: &mut World, layer: Entity, from: i64, to: i64, zoomed: bool) {
    let now = world.resource::<Tick>().0;
    let mut maps = kept::maps_mut(world, layer);
    let Some(old) = maps.maps.get(&from).filter(|_| !maps.maps.contains_key(&to)).cloned() else { return };
    let mut map = MapData { id: to, changed: None, tracked: Vec::new(), marks: Vec::new(), redraw: 0, told: 0, born: now.wrapping_sub(1), ..old.clone() };
    if zoomed {
        map.scale += 1;
        map.centre = old.centre.map(|axis| centre(axis as f32, map.scale));
        map.pixels.fill(0);
        if let Some((tracked, mark)) = map.pin.take() {
            let at = tracked.block_position.as_ref().map(|at| [at.x, at.z]).unwrap_or_default();
            map.pin = Some(marks::pin(&map, mark.r#type, at));
        }
    } else {
        map.locked = true;
    }
    if to == maps.next_id() {
        maps.made += 1;
    }
    let size = render::SIZE as usize;
    let whole = marks::update(&map, Some([0, 0, size, size]));
    let layer = map.layer;
    maps.maps.insert(to, map);
    tell_layer(world, layer, &whole);
}
