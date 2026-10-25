//! Lightning rods and copper, as BDS's `LightningRodBlock::onLightningHit` and the bolt's copper
//! walk: a storm's bolt goes to the nearest rod within 64 under open sky and rain; a rod struck
//! is powered 8 ticks with a spark, and the copper it stands on (or under the bolt) is scraped a
//! stage, three to five times, each time walking up to eight steps of copper about it.
// TODO: the redstone a powered rod gives is the redstone side's (blocks/redstone); an oxidised
// unwaxed rod is scraped instead of powered, as BDS's copper vtable reads, not tried live.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ChunkPos;

use crate::blocks::{self, Dice};
use crate::components::{Layer, Tick};

const ROD: &str = "lightning_rod";
const POWERED: &str = "powered_bit";
const FACING: &str = "facing_direction";
/// A rod draws a bolt from this far across and up or down, and stays powered this many ticks.
const REACH: i32 = 64;
const POWERED_FOR: u64 = 8;
/// A strike scrapes the copper this many times and more by a roll, each walking up to this many
/// steps, each step trying this many cells about.
const WALKS: u32 = 3;
const MORE_WALKS: u32 = 3;
const STEPS: u32 = 8;
const TRIES: usize = 10;
const STAGES: [&str; 3] = ["exposed_", "weathered_", "oxidized_"];

/// Rods struck, and the tick each goes dark.
#[derive(Resource, Default)]
struct Lit(Vec<(u64, Entity, BlockPos)>);

fn is_rod(name: &str) -> bool {
    name.ends_with(ROD)
}

fn state_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&'static str, &'static str, u32)> {
    let level = &world.get::<Layer>(layer)?.world;
    let id = level.in_height(y).then(|| level.block(x, y, z))?;
    level.registry().get(id).map(|state| (state.name, state.properties, id))
}

/// Where a bolt meant for `picked` goes instead: on the nearest rod within reach whose column is
/// open over it and that rain falls on.
pub(super) fn drawn_to(world: &World, layer: Entity, picked: BlockPos) -> Option<[f32; 3]> {
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = picked;
    let (chunks, home) = (REACH / 16 + 1, ChunkPos::of_block(x, z));
    let columns = (-chunks..=chunks).flat_map(|dx| (-chunks..=chunks).map(move |dz| ChunkPos::new(home.x + dx, home.z + dz))).filter(|pos| level.is_loaded(*pos));
    let rods = columns.flat_map(|pos| level.find_blocks(pos, |id| level.registry().get(id).is_some_and(|state| is_rod(state.name))));
    let near = rods.filter(|[rx, ry, rz]| (rx - x).abs() <= REACH && (rz - z).abs() <= REACH && (ry - y).abs() <= REACH);
    let open = near.filter(|&[rx, ry, rz]| super::storms::top(level, rx, rz) <= ry + 1 && super::storms::rains_on(level, [rx, ry + 1, rz]));
    let far = |[rx, ry, rz]: &[i32; 3]| (rx - x).pow(2) + (ry - y).pow(2) + (rz - z).pow(2);
    open.min_by_key(far).map(|cell| cell.map(|c| c as f32))
}

/// One stage less weathered of the unwaxed copper `name`: `exposed_copper` is `copper_block`.
fn scraped(name: &str) -> Option<String> {
    let bare = name.strip_prefix("minecraft:")?;
    let (stage, rest) = STAGES.iter().enumerate().find_map(|(stage, prefix)| bare.strip_prefix(prefix).map(|rest| (stage, rest)))?;
    Some(match stage {
        0 if rest == "copper" => "minecraft:copper_block".to_owned(),
        0 => format!("minecraft:{rest}"),
        _ => format!("minecraft:{}{rest}", STAGES[stage - 1]),
    })
}

fn spark(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    let spark = LevelEvent { event: LevelEventEvent::ParticleElectricSpark, position: Vec3f { x, y, z }, data: 0 };
    blocks::to_viewers(world, layer, pos, &[sapling_net::encode(&spark)]);
}

/// Scrapes the copper at `pos` a stage, keeping its other states, with a spark; false where it
/// is no weathered unwaxed copper.
fn scrape(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(cleaner) = state_at(world, layer, pos).and_then(|(name, properties, _)| Some((scraped(name)?, properties))) else { return false };
    let Some(id) = world.get::<Layer>(layer).and_then(|level| level.world.registry().find(&cleaner.0, cleaner.1).or_else(|| level.world.registry().states_of(&cleaner.0).next().map(|(id, _)| id))) else { return false };
    let done = blocks::set(world, layer, pos, id);
    if done {
        spark(world, layer, pos);
    }
    done
}

/// Sets the rod at `pos` powered or not.
fn power(world: &mut World, layer: Entity, pos: BlockPos, on: bool) {
    let Some((_, _, id)) = state_at(world, layer, pos) else { return };
    let powered = world.get::<Layer>(layer).and_then(|level| blocks::with_property(level.world.registry(), id, POWERED, if on { "1" } else { "0" }));
    if let Some(powered) = powered.filter(|powered| *powered != id) {
        blocks::set(world, layer, pos, powered);
    }
}

/// The cell a rod at `pos` stands on, by the way it faces.
fn stands_on(properties: &str, pos: BlockPos) -> BlockPos {
    let facing = properties.split(',').find_map(|pair| pair.strip_prefix(FACING)?.strip_prefix('=')?.parse().ok()).and_then(Face::from_index).unwrap_or(Face::Up);
    let [dx, dy, dz] = facing.offset();
    [pos[0] - dx, pos[1] - dy, pos[2] - dz]
}

/// A bolt came down in the cell `pos`: a rod there is powered (or scraped), and the copper under
/// it is scraped clean about.
pub(super) fn struck(world: &mut World, layer: Entity, pos: BlockPos) {
    let below = [pos[0], pos[1] - 1, pos[2]];
    let start = match state_at(world, layer, pos).filter(|(name, ..)| is_rod(name)) {
        Some((_, properties, _)) => {
            if !scrape(world, layer, pos) {
                power(world, layer, pos, true);
                spark(world, layer, pos);
                let off = world.resource::<Tick>().0 + POWERED_FOR;
                let mut lit = world.get_resource_or_insert_with(Lit::default);
                lit.0.retain(|(_, on, at)| (*on, *at) != (layer, pos));
                lit.0.push((off, layer, pos));
            }
            stands_on(properties, pos)
        }
        None => below,
    };
    if !state_at(world, layer, start).is_some_and(|(name, ..)| scraped(name).is_some()) {
        return;
    }
    let walks = WALKS + world.get_resource_or_insert_with(Dice::default).roll(MORE_WALKS);
    for _ in 0..walks {
        scrape(world, layer, start);
        let steps = 1 + world.get_resource_or_insert_with(Dice::default).roll(STEPS);
        let mut at = start;
        for _ in 0..steps {
            let next = (0..TRIES).find_map(|_| {
                let step: [i32; 3] = {
                    let mut dice = world.get_resource_or_insert_with(Dice::default);
                    std::array::from_fn(|_| dice.roll(3) as i32 - 1)
                };
                let next: BlockPos = std::array::from_fn(|axis| at[axis] + step[axis]);
                scrape(world, layer, next).then_some(next)
            });
            let Some(next) = next else { break };
            at = next;
        }
    }
}

/// Rods whose time is up go dark.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(due) = world.get_resource_mut::<Lit>().map(|mut lit| {
        let (due, left): (Vec<_>, Vec<_>) = lit.0.drain(..).partition(|(off, ..)| *off <= now);
        lit.0 = left;
        due
    }) else { return };
    for (_, layer, pos) in due {
        power(world, layer, pos, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copper_is_scraped_a_stage_at_a_time() {
        assert_eq!(scraped("minecraft:oxidized_cut_copper").as_deref(), Some("minecraft:weathered_cut_copper"));
        assert_eq!(scraped("minecraft:exposed_copper").as_deref(), Some("minecraft:copper_block"));
        assert_eq!(scraped("minecraft:exposed_lightning_rod").as_deref(), Some("minecraft:lightning_rod"));
        assert_eq!([scraped("minecraft:copper_block"), scraped("minecraft:waxed_oxidized_copper")], [None, None]);
    }

    #[test]
    fn a_rod_stands_on_the_block_behind_its_point() {
        assert_eq!(stands_on("facing_direction=1", [0, 64, 0]), [0, 63, 0]);
        assert_eq!(stands_on("facing_direction=5", [0, 64, 0]), [-1, 64, 0]);
    }
}
