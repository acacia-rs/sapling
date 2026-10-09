//! Lava meeting water: BDS `LiquidBlockBase::solidify` and the lava cases of `_tick`
//! (docs/research/water.md "Answers" 5).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::{Dice, level, send};
use super::{Cell, Liquid, cell};

const OBSIDIAN: &str = "minecraft:obsidian";
const COBBLESTONE: &str = "minecraft:cobblestone";
pub(super) const STONE: &str = "minecraft:stone";
const BASALT: &str = "minecraft:basalt";
const BLUE_ICE: &str = "minecraft:blue_ice";
const SOUL_SOIL: &str = "minecraft:soul_soil";
/// Lava hardens through its sides and its top, never from below.
const TOUCHING: [Face; 5] = [Face::South, Face::North, Face::East, Face::West, Face::Up];
/// The hiss's pitch x 1000, and how far each of its two throws moves it.
const FIZZ_PITCH: i32 = 2600;
const FIZZ_SPREAD: u32 = 800;

fn name_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<&'static str> {
    let level = level(world, layer)
        .filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    level
        .registry()
        .get(level.block(x, y, z))
        .map(|state| state.name)
}

/// The lava at `pos` hardens if water touches it: a source to obsidian, flowing lava to
/// cobblestone; beside blue ice and over soul soil, to basalt. Whether it did.
pub(super) fn meets_water(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(level) = level(world, layer)
        .filter(|level| level.in_height(pos[1]) && level.is_loaded(send::column(pos)))
    else {
        return false;
    };
    let Some(state) = level
        .registry()
        .get(level.block(pos[0], pos[1], pos[2]))
        .filter(|state| Liquid::of(state.name) == Some(Liquid::Lava))
    else {
        return false;
    };
    let is_source = state.liquid_depth == 0;
    let over_soul_soil = name_at(world, layer, Face::Down.neighbour(pos)) == Some(SOUL_SOIL);
    let into = TOUCHING
        .iter()
        .map(|face| face.neighbour(pos))
        .find_map(|near| match cell(world, layer, near) {
            Cell::Liquid(Liquid::Water, _) => Some(if is_source { OBSIDIAN } else { COBBLESTONE }),
            _ if over_soul_soil && name_at(world, layer, near) == Some(BLUE_ICE) => Some(BASALT),
            _ => None,
        });
    into.is_some_and(|into| turn(world, layer, pos, into))
}

/// Makes the cell the block `name` with a hiss.
pub(super) fn turn(world: &mut World, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let Some(id) = level(world, layer)
        .and_then(|level| level.registry().states_of(name).next())
        .map(|(id, _)| id)
    else {
        return false;
    };
    let turned = send::set(world, layer, pos, id);
    if turned {
        fizz(world, layer, pos);
    }
    turned
}

/// The hiss of lava meeting water or burning a block (BDS `LiquidBlock::_triggerFizzEffect`: the
/// pitch x 1000 is `((r - r) * 0.8 + 2.6) * 1000`; its smoke is the client's own).
pub(super) fn fizz(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let spread = dice.roll(FIZZ_SPREAD) as i32 - dice.roll(FIZZ_SPREAD) as i32;
    let hiss = sapling_net::encode(&LevelEvent {
        event: LevelEventEvent::SoundFizz,
        position: Vec3f { x, y, z },
        data: FIZZ_PITCH + spread,
    });
    send::to_viewers(world, layer, pos, &[hiss]);
}
