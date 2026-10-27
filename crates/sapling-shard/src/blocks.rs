//! Breaking, placing and clicking blocks: what a client asks for is checked, offered to the
//! handlers and then either done for everyone who sees the place or undone on the client that
//! predicted it (docs/DESIGN.md "Step 6 as built"; request.rs). Blocks that change on their own:
//! falling, liquids, plates, updates.

mod after_break;
mod bonemeal;
mod blown;
mod bubbles;
mod buckets;
mod bulbs;
mod cauldrons;
mod composter;
mod copper;
mod crafter;
mod daylight;
mod dispensers;
mod doors;
mod drips;
pub(crate) mod end_portal;
mod entities;
mod facing;
mod fields;
mod fire;
mod falling;
mod frost;
mod ground;
mod growth;
mod handled;
mod holders;
mod items;
mod jukebox;
mod leaves;
mod lectern;
mod liquid;
mod loot;
mod mining;
mod notes;
mod ores;
mod pending;
mod pistons;
mod place;
mod plates;
pub(crate) mod portals;
mod pots;
mod queued;
mod rails;
mod rods;
mod saplings;
mod scaffolds;
mod sculk;
mod redstone;
mod request;
mod send;
mod shapes;
mod sign_ink;
mod signs;
mod stacking;
mod summoning;
mod target;
mod tools;
mod tripwire;
mod updates;
mod waterlog;

pub(crate) use blown::blow_out;
pub(crate) use falling::from_storage as falling_from_storage;
pub(crate) use daylight::tick as read_daylight;
pub(crate) use fire::{light as light_fire, tick as burn_fires};
pub(crate) use frost::{frost_walk, tick as melt_frost};
pub(crate) use growth::{arrived as plants_arrived, tick as random_ticks, trample};
pub(crate) use ores::{light as light_ore, tick as dim_ores, tread as tread_ore};
pub(crate) use scaffolds::tick as settle_scaffolds;
pub(crate) use doors::{bed_head, release_buttons, with as with_property};
pub(crate) use holders::spend as spend_held;
pub(crate) use pistons::{at_place as block_entity_at, tick as move_pistons};
pub(crate) use plates::press_plates;
pub(crate) use redstone::{apply as apply_redstone, arrived as circuits_arrived, container_changed, evaluate as evaluate_redstone, power_at};
pub(crate) use signs::dispatch_edits as dispatch_sign_edits;
pub(crate) use loot::{Dice, mined};
pub(crate) use liquid::tick as flow_liquids;
pub(crate) use mining::{Breaking, sound_hits};
pub(crate) use request::{BlockInputs, dispatch};
use request::{Request, Site, site};
pub(crate) use send::{batched, block_at, hold, keep_entity, noise, put_block, resolve, set, set_liquid, set_many, set_network_only, set_quietly, set_sectioned, show_held, show_whole, sound, to_viewers};
pub(crate) use liquid::neighbours_changed;
pub(crate) use updates::break_block;
pub(crate) use crafter::{crafter_slot, dispatch_toggles as dispatch_crafter_toggles};
pub(crate) use queued::tick as run_queued;
pub(crate) use lectern::turn_pages as turn_lectern_pages;
pub(crate) use pending::{arrived as ticks_arrived, write_back as write_back_ticks, write_back_before_saving as write_back_ticks_before_saving};
pub(crate) use rods::struck as lightning_struck;
pub(crate) use sculk::{GameEvent, tick as hear_vibrations, vibrate, vibrate_at, walked};
pub(crate) use target::shot_stopped;
pub(crate) use tripwire::tick as trip_wires;

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face, GameMode};
use sapling_data::EYE_HEIGHT;

use crate::components::{Layer, Mode, Position};

/// Blocks from the eye to the middle of a block: BDS took a click at 6.4 and 12.2 (creative) and
/// refused 7.3 and 13.2.
const REACH: f32 = 7.0;
const CREATIVE_REACH: f32 = 13.0;

fn level(world: &World, layer: Entity) -> Option<&sapling_world::World> {
    world.get::<Layer>(layer).map(|layer| &layer.world)
}

fn opposite(face: Face) -> Face {
    match face {
        Face::Down => Face::Up,
        Face::Up => Face::Down,
        Face::North => Face::South,
        Face::South => Face::North,
        Face::West => Face::East,
        Face::East => Face::West,
    }
}

/// Whether the middle of the block at `pos` is within the player's reach of its eyes.
pub(crate) fn reaches(world: &World, player: Entity, pos: BlockPos) -> bool {
    let (Some(position), Some(mode)) = (world.get::<Position>(player), world.get::<Mode>(player)) else { return false };
    let eye = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    let distance: f32 = (0..3).map(|axis| (pos[axis] as f32 + 0.5 - eye[axis]).powi(2)).sum();
    let reach = if mode.0 == GameMode::Creative { CREATIVE_REACH } else { REACH };
    distance <= reach * reach
}

/// Sets a block for a handler, as a player's place puts it (concrete powder by water as concrete),
/// and settles what is around it; a cell that already held it changes nothing. False where it
/// could not be set.
pub(crate) fn update_block(world: &mut World, layer: Entity, pos: BlockPos, block: &Block) -> bool {
    let registry = level(world, layer).map(|level| level.registry());
    let put_as = registry.and_then(|registry| send::resolve(registry, block)).map(|id| (falling::as_put(world, layer, pos, id), id));
    let swapped = match (registry, put_as) {
        (Some(registry), Some((put, id))) if put != id => Some(send::describe(registry, put)),
        _ => None,
    };
    let door = put_as.and_then(|(put, _)| doors::put_whole(world, layer, pos, put));
    let put = door.or_else(|| put_block(world, layer, pos, swapped.as_ref().unwrap_or(block)));
    if put == Some(true) {
        updates::settle(world, layer, pos);
    }
    put.is_some()
}

/// Moves falling blocks and starts the falls that are due, then settles what they changed.
pub(crate) fn fall_blocks(world: &mut World) {
    for (layer, pos) in falling::tick(world) {
        updates::settle(world, layer, pos);
    }
}
