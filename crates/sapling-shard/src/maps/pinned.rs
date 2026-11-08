//! Explorer maps: a cartographer's map of the square its structure lies in, pinned on the
//! structure, its preview drawn (preview.rs) (research/captures/bds-at1maps.txt, bds-ax1maps.txt;
//! docs/research/villagers.md "Explorer maps"). The offer's map is made as offers roll and told no
//! one; a bought one is made again in its buyer's inventory (`regenerate`).

use acacia_proto::nbt::Value;
use acacia_proto::packets::InventorySlot;
use acacia_proto::types::{MapDecorationType, WindowIDVarint};
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::render::SIZE;
use super::{FILLED, kept, make, map_id, marks, tell_layer};
use crate::components::{InLayer, Layer, Outgoing, Position};
use crate::inventory::{Inventory, Place};

/// An explorer map's scale, its square 512 blocks.
pub(crate) const EXPLORER_SCALE: u8 = 2;
const REGENERATE: &str = "map_regenerate";
const MAIN_SLOTS: usize = 36;

/// The marker BDS drew for a destination; trial chambers take index 24, which the proto's enum
/// names `MarkerWhite2`.
fn marker(destination: &str) -> Option<MapDecorationType> {
    Some(match destination {
        "monument" => MapDecorationType::Monument,
        "mansion" => MapDecorationType::Mansion,
        "village_desert" => MapDecorationType::VillageDesert,
        "village_plains" => MapDecorationType::VillagePlains,
        "village_savanna" => MapDecorationType::VillageSavanna,
        "village_snowy" => MapDecorationType::VillageSnowy,
        "village_taiga" => MapDecorationType::VillageTaiga,
        "jungle_temple" => MapDecorationType::JungleTemple,
        "swamp_hut" => MapDecorationType::WitchHut,
        "trial_chambers" => MapDecorationType::MarkerWhite2,
        _ => return None,
    })
}

/// Makes the explorer map of `destination` (an explorer `filled_map`'s damage names it) as found
/// from `from`: pinned on the structure with its preview drawn, else blank on `from`'s square.
/// Returns the map's id; no one is told of it.
pub(crate) fn chart_explorer(world: &mut World, layer: Entity, damage: i16, from: [f32; 3]) -> Option<i64> {
    let dimension = world.get::<Layer>(layer)?.world.dimension();
    let destination = sapling_data::trades::explorer_destination(damage);
    let located = destination.and_then(|to| Some((to, world.get::<Layer>(layer)?.world.locate_structure(to, from[0].floor() as i32, from[2].floor() as i32)?)));
    let Some((to, at)) = located else {
        let centre = [from[0], from[2]].map(|axis| super::centre(axis, EXPLORER_SCALE));
        return Some(make(world, layer, dimension, centre, EXPLORER_SCALE, false));
    };
    let centre = at.map(|axis| super::centre(axis as f32, EXPLORER_SCALE));
    let id = make(world, layer, dimension, centre, EXPLORER_SCALE, false);
    let mut map = kept::maps_mut(world, layer).maps.get(&id).cloned()?;
    map.pin = marker(to).map(|r#type| marks::pin(&map, r#type, at));
    (map.tracked, map.marks) = map.pin.iter().cloned().unzip();
    if let Some(level) = world.get::<Layer>(layer) {
        super::preview::preview(&level.world, &mut map);
    }
    kept::maps_mut(world, layer).maps.insert(id, map);
    Some(id)
}

/// Tells the layer the whole of map `id`: its pixels, pin and markers.
fn tell_whole(world: &mut World, layer: Entity, id: i64) {
    let size = SIZE as usize;
    let mut maps = kept::maps_mut(world, layer);
    let Some(map) = maps.maps.get_mut(&id) else { return };
    map.changed = None;
    let whole = marks::update(map, Some([0, 0, size, size]));
    tell_layer(world, layer, &whole);
}

/// Each explorer map still marked `map_regenerate` in a player's inventory made again, as BDS's
/// `MapItem` does on the item's first inventory tick (141a64d60 into 14161b3e0, bds-at1maps.txt):
/// the offer's map told whole, a new map of its destination located from the player, the item
/// given the new id without the mark and its slot shown, then the new map told whole.
pub(crate) fn regenerate(world: &mut World) {
    let marked = |stack: &ItemStack| stack.name == FILLED && stack.tag.as_ref().is_some_and(|tag| tag.get(REGENERATE).is_some());
    let found: Vec<(Entity, usize, ItemStack)> = world
        .query::<(Entity, &Inventory)>()
        .iter(world)
        .flat_map(|(player, inventory)| (0..MAIN_SLOTS).filter_map(move |slot| Some((player, slot, inventory.slot(Place::Main(slot))?.as_ref()?.stack.clone()))))
        .filter(|(_, _, stack)| marked(stack))
        .collect();
    for (player, slot, mut stack) in found {
        let (Some(layer), Some(feet)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|at| at.feet)) else { continue };
        if let Some(offer) = map_id(&stack) {
            tell_whole(world, layer, offer);
        }
        let Some(id) = chart_explorer(world, layer, stack.damage, feet) else { continue };
        if let Some(Value::Compound(fields)) = stack.tag.as_mut() {
            fields.retain(|(name, _)| name != REGENERATE && name != "map_uuid");
            fields.push(("map_uuid".into(), Value::Long(id)));
        }
        let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
        if let Ok((mut inventory, mut out)) = query.get_mut(world, player) {
            inventory.put(Place::Main(slot), Some(stack));
            if inventory.sent {
                let item = crate::inventory::wire(inventory.slot(Place::Main(slot)).unwrap_or(&None), true);
                out.push(&InventorySlot { window_id: WindowIDVarint::Inventory, slot: slot as u32, container: None, storage_item: None, item });
            }
        }
        tell_whole(world, layer, id);
    }
}
