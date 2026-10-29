//! Sponges (docs/research/world-ticks.md, "Sponge"): BDS's `SpongeBlock` subscribes to the place
//! and queued-tick events; the rule is the Java game's `SpongeBlock`. A dry sponge written, or one
//! beside a cell that changed, soaks up the water within 6 steps through water, up to 64 cells, and
//! turns wet; kelp and seagrass in it break. A wet sponge set in the Nether dries at once.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::queued::{self, Job};
use super::{level, liquid, send, updates};

const DRY: &str = "minecraft:sponge";
const WET: &str = "minecraft:wet_sponge";
const REACH: u32 = 6;
const MOST: usize = 64;
/// What stands in water and breaks when it is soaked up.
const PLANTS: [&str; 3] = ["minecraft:kelp", "minecraft:seagrass", "minecraft:tall_seagrass"];

type Level = sapling_world::World;

fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&'static str> {
    (level.in_height(y) && level.is_loaded(send::column([x, y, z]))).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

fn is_water(level: &Level, [x, y, z]: BlockPos) -> bool {
    let water = |id: u32| level.registry().get(id).is_some_and(|state| matches!(state.name, "minecraft:water" | "minecraft:flowing_water" | "minecraft:bubble_column"));
    name_at(level, [x, y, z]).is_some() && (water(level.block(x, y, z)) || water(level.liquid(x, y, z)))
}

/// After a write at `pos`: a dry sponge there or beside it soaks up water this tick; a wet sponge
/// written in the Nether dries.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    if name_at(level, pos) == Some(WET) && liquid::is_hot(world, layer) {
        queued::this_tick(world, layer, pos, Job::Sponge);
        return;
    }
    let sponges: Vec<BlockPos> = std::iter::once(pos).chain(Face::ALL.map(|face| face.neighbour(pos))).filter(|cell| name_at(level, *cell) == Some(DRY)).collect();
    for cell in sponges {
        queued::this_tick(world, layer, cell, Job::Sponge);
    }
}

/// The queued tick of the sponge at `pos`.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(here) = level(world, layer) else { return };
    match name_at(here, pos) {
        Some(WET) if liquid::is_hot(world, layer) => dry(world, layer, pos),
        Some(DRY) => {
            let soaked = soaked(here, pos);
            if soaked.is_empty() {
                return;
            }
            told_soaking(world, layer, pos);
            for cell in soaked {
                let plant = level(world, layer).and_then(|level| name_at(level, cell)).is_some_and(|name| PLANTS.contains(&name));
                if plant {
                    updates::pop(world, layer, cell);
                }
                liquid::take(world, layer, cell);
            }
            let Some(id) = level(world, layer).and_then(|level| level.registry().states_of(WET).next()).map(|(id, _)| id) else { return };
            send::set(world, layer, pos, id);
        }
        _ => {}
    }
}

/// What BDS sent with a sponge soaking: event 2021 with this data, then `sponge.absorb`, both
/// at the cell's corner (research/captures/bds-bkticks.txt).
const SOAK_DATA: i32 = 1_302_137;

fn told_soaking(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos.map(|c| c as f32);
    let event = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroyBlockNoSound, position: Vec3f { x, y, z }, data: SOAK_DATA });
    send::to_viewers(world, layer, pos, &[event, send::noise("sponge.absorb", Vec3f { x, y, z }, -1)]);
}

/// The cells of water the sponge at `pos` soaks up: breadth first through water, nearest first.
fn soaked(level: &Level, pos: BlockPos) -> Vec<BlockPos> {
    let mut found: Vec<BlockPos> = Vec::new();
    let mut ring = vec![pos];
    for _ in 0..REACH {
        let mut next = Vec::new();
        for from in ring {
            for cell in Face::ALL.map(|face| face.neighbour(from)) {
                if found.len() >= MOST {
                    return found;
                }
                if cell != pos && !found.contains(&cell) && is_water(level, cell) {
                    found.push(cell);
                    next.push(cell);
                }
            }
        }
        ring = next;
    }
    found
}

/// A wet sponge in the Nether becomes a dry one, its steam told first: event 3501 (the protocol
/// crate's `CauldronExplode`) 0.875 up the cell, as BDS (research/captures/bds-bknether.txt).
fn dry(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(id) = level(world, layer).and_then(|level| level.registry().states_of(DRY).next()).map(|(id, _)| id) else { return };
    let [x, z] = [pos[0], pos[2]].map(|c| c as f32 + 0.5);
    let steam = sapling_net::encode(&LevelEvent { event: LevelEventEvent::CauldronExplode, position: Vec3f { x, y: pos[1] as f32 + 0.875, z }, data: 0 });
    send::to_viewers(world, layer, pos, &[steam]);
    send::set(world, layer, pos, id);
}
