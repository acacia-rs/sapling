//! Maps, as BDS made and kept them (docs/research/maps.md; research/captures/bds-maps.txt). Using an
//! empty map makes a filled one centred on the 128-block square the user stands in, under a fresh id
//! that reserves the next four for its zoom levels and a level-wide `map_name_index`; a locator map
//! (damage 2) also gets `map_display_players`. The map in a player's hand ticks: every 15 to 20 ticks
//! it redraws the circle around its holder; a tick with changed pixels sends them, and every fifth
//! tick without sends the players on it, both to everyone in its world. A `MapInfoRequest` is
//! answered with the whole map, or, carrying pixels, has them painted in. Each realm keeps its
//! maps with its level (kept.rs).

mod kept;
mod marks;
mod render;

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelSoundEvent, MapInfoRequest};
use acacia_proto::types::{MapDecoration, TrackedObject, Vec3f};
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_world::ItemStack;

use crate::blocks::Dice;
use crate::components::{InLayer, Layer, NetId, Outgoing, PacketQueues, Position, Tick};
use crate::inventory::{self, CARTOGRAPHY_INPUT, Chart, Inventory, Place};

pub(crate) use kept::keep_all;
pub(crate) use render::Colours;

const EMPTY: &str = "minecraft:empty_map";
pub(crate) const FILLED: &str = "minecraft:filled_map";
const LOCATOR: i16 = 2;
/// Ticks between redraws: a map waits until its count reaches this, then restarts it at a roll below
/// [`REDRAW_ROLL`].
const REDRAW: u32 = 20;
const REDRAW_ROLL: u32 = 6;
/// Ticks between the packets of players on a map.
const MARK_EVERY: u32 = 5;
/// Ids a new map takes: its own and its four zoom levels'.
const IDS_PER_MAP: i64 = 5;
/// BDS's map ids are unique ids: the top half all ones.
const ID_BASE: i64 = -(1 << 32);

#[derive(Clone)]
pub(crate) struct MapData {
    pub id: i64,
    pub scale: u8,
    pub centre: [i32; 2],
    pub dimension: i32,
    pub layer: Entity,
    pub locked: bool,
    /// ABGR, rows of 128 from the north.
    pub pixels: Vec<u32>,
    /// The changed pixels not yet told: `[x0, z0, x1, z1]`, ends inclusive.
    changed: Option<[usize; 4]>,
    pub tracked: Vec<TrackedObject>,
    pub marks: Vec<MapDecoration>,
    redraw: u32,
    told: u32,
    born: u64,
}

impl MapData {
    /// A map as storage kept it; the next tick that ticks it shows it.
    fn kept(id: i64, scale: u8, centre: [i32; 2], dimension: i32, layer: Entity, locked: bool, pixels: Vec<u32>) -> Self {
        Self { id, scale, centre, dimension, layer, locked, pixels, changed: None, tracked: Vec::new(), marks: Vec::new(), redraw: 0, told: 0, born: u64::MAX }
    }

    pub(crate) fn paint(&mut self, x: usize, z: usize, pixel: u32) {
        let at = &mut self.pixels[z * render::SIZE as usize + x];
        if *at == pixel {
            return;
        }
        *at = pixel;
        self.changed = Some(match self.changed {
            Some([x0, z0, x1, z1]) => [x0.min(x), z0.min(z), x1.max(x), z1.max(z)],
            None => [x, z, x, z],
        });
    }
}

/// Every map of a realm, by id: on its home layer.
#[derive(Component, Default)]
pub(crate) struct Maps {
    pub maps: HashMap<i64, MapData>,
    made: i64,
    names: i32,
    colours: Colours,
}

impl Maps {
    /// The id the next new map takes.
    fn next_id(&self) -> i64 {
        ID_BASE + 1 + self.made * IDS_PER_MAP
    }
}

/// The centre of the square of `128 << scale` blocks that `at` lies in, on one axis (Java's
/// `MapItemSavedData.calculateCenter`).
pub(crate) fn centre(at: f32, scale: u8) -> i32 {
    let size = 128 << scale;
    ((at as f64 + 64.0) / size as f64).floor() as i32 * size + size / 2 - 64
}

/// The map id a filled map's tag names.
pub(crate) fn map_id(stack: &ItemStack) -> Option<i64> {
    match stack.tag.as_ref()?.get("map_uuid")? {
        Value::Long(id) => Some(*id),
        _ => None,
    }
}

/// A new blank map's data under a fresh id; returns the id and the next `map_name_index`.
pub(crate) fn make(world: &mut World, layer: Entity, dimension: i32, centre: [i32; 2], scale: u8, locked: bool) -> (i64, i32) {
    let born = world.resource::<Tick>().0;
    let mut maps = kept::maps_mut(world, layer);
    let id = maps.next_id();
    maps.made += 1;
    maps.names += 1;
    let size = render::SIZE as usize;
    let data = MapData {
        id, scale, centre, dimension, layer, locked,
        pixels: vec![0; size * size],
        changed: Some([0, 0, size - 1, size - 1]),
        tracked: Vec::new(),
        marks: Vec::new(),
        redraw: 0,
        told: 0,
        born,
    };
    maps.maps.insert(id, data);
    (id, maps.names)
}

/// Sends `packet` to every player in `layer`.
fn tell_layer<T: acacia_proto::Packet>(world: &mut World, layer: Entity, packet: &T) {
    let bytes = sapling_net::encode(packet);
    for (in_layer, mut out) in world.query::<(&InLayer, &mut Outgoing)>().iter_mut(world) {
        if in_layer.0 == layer {
            out.0.push(bytes.clone());
        }
    }
}

/// Using an empty map in the air fills it; false for a hand that holds none.
pub(crate) fn fill(world: &mut World, player: Entity) -> bool {
    let Some(held) = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().clone()).filter(|held| held.stack.name == EMPTY) else { return false };
    let (Some(layer), Some(position)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).copied()) else { return false };
    let Some(dimension) = world.get::<Layer>(layer).map(|level| level.world.dimension()) else { return false };
    let centre = [centre(position.feet[0], 0), centre(position.feet[2], 0)];
    let (id, name) = make(world, layer, dimension, centre, 0, false);
    let mut fields = vec![("map_name_index".into(), Value::Int(name)), ("map_uuid".into(), Value::Long(id))];
    if held.stack.damage == LOCATOR {
        fields.insert(0, ("map_display_players".into(), Value::Byte(1)));
    }
    let made = ItemStack { name: FILLED.to_owned(), count: 1, damage: 0, block: None, tag: Some(Value::Compound(fields)) };
    let creation = marks::creation(&kept::maps_mut(world, layer).maps[&id]);
    tell_layer(world, layer, &creation);
    inventory::swap_held_for(world, player, made);
    inventory::show_windows(world, player, true);
    let [x, y, z] = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]].map(|c| c.floor());
    let sound = LevelSoundEvent { sound_id: "ui.cartography_table.take_result".into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    crate::late_sounds::play_later(world, 1, layer, [x, y, z].map(|c| c as i32), sapling_net::encode(&sound));
    true
}

/// What a player's cartography table knows of the map in its input.
pub(crate) fn chart(world: &World, player: Entity) -> Chart {
    let maps = world.get::<InLayer>(player).and_then(|layer| world.get::<Maps>(crate::dimensions::home(world, layer.0)));
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.slot(Place::Ui(CARTOGRAPHY_INPUT))?.as_ref().and_then(|held| map_id(&held.stack)));
    let input = held.and_then(|id| maps?.maps.get(&id)).map(|map| (map.id, map.scale, map.locked));
    Chart { input, fresh: maps.map_or(ID_BASE + 1, Maps::next_id) }
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

/// Answers this tick's `MapInfoRequest`s: the whole map to the asker, or the pixels it sent painted in.
pub(crate) fn dispatch(world: &mut World) {
    kept::restore_all(world);
    let asked = world.resource_mut::<PacketQueues>().take::<MapInfoRequest>();
    let size = render::SIZE as usize;
    for (player, request) in asked {
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0) else { continue };
        let whole = {
            let mut maps = kept::maps_mut(world, layer);
            let Some(map) = maps.maps.get_mut(&request.map_id) else { continue };
            if !request.client_pixels.is_empty() {
                for pixel in request.client_pixels.iter().filter(|pixel| usize::from(pixel.index) < size * size) {
                    map.paint(usize::from(pixel.index) % size, usize::from(pixel.index) / size, pixel.rgba as u32);
                }
                continue;
            }
            [marks::creation(map), marks::update(map, Some([0, 0, size, size]))]
        };
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            whole.iter().for_each(|packet| out.push(packet));
        }
    }
}

/// Ticks each map in a player's hand.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let holders: Vec<(Entity, i64)> = crate::components::in_world(world)
        .into_iter()
        .filter_map(|player| Some((player, world.get::<Inventory>(player)?.in_hand().as_ref().filter(|held| held.stack.name == FILLED).and_then(|held| map_id(&held.stack))?)))
        .collect();
    let mut ticked = Vec::new();
    for (holder, id) in holders {
        if ticked.contains(&id) {
            continue;
        }
        ticked.push(id);
        let Some(home) = world.get::<InLayer>(holder).map(|layer| crate::dimensions::home(world, layer.0)) else { continue };
        let Some(mut maps) = world.entity_mut(home).take::<Maps>() else { continue };
        if let Some(map) = maps.maps.get_mut(&id).filter(|map| map.born != now) {
            tick_map(world, &mut maps.colours, map, holder);
        }
        world.entity_mut(home).insert(maps);
    }
}

fn tick_map(world: &mut World, colours: &mut Colours, map: &mut MapData, holder: Entity) {
    let Some(at) = world.get::<Position>(holder).copied() else { return };
    let here = world.get::<InLayer>(holder).is_some_and(|layer| layer.0 == map.layer);
    if map.redraw == REDRAW {
        map.redraw = world.get_resource_or_insert_with(Dice::default).roll(REDRAW_ROLL);
        if let Some(level) = world.get::<Layer>(map.layer).filter(|_| here && !map.locked) {
            render::render(&level.world, colours, map, at.feet);
        }
    } else {
        map.redraw += 1;
    }
    let packet = if let Some([x0, z0, x1, z1]) = map.changed.take() {
        marks::update(map, Some([x0, z0, x1 - x0 + 1, z1 - z0 + 1]))
    } else {
        map.told += 1;
        if (map.told - 1) % MARK_EVERY != 0 {
            return;
        }
        let seen = world
            .query_filtered::<(Entity, &InLayer, &Position, &NetId), With<Outgoing>>()
            .iter(world)
            .filter(|(_, layer, ..)| layer.0 == map.layer)
            .map(|(player, _, position, id)| marks::Seen { id: id.0 as i64, feet: position.feet, yaw: position.yaw, holder: player == holder })
            .collect();
        (map.tracked, map.marks) = marks::mark(map, seen);
        marks::update(map, None)
    };
    tell_layer(world, map.layer, &packet);
}
