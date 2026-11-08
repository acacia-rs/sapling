//! Where bone meal on a grass block or a flower brings up plants: BDS's random walks, 48 of them, one to
//! three steps from the cell over the clicked block, each step over grass (docs/research/bone-meal.md
//! "Grass and flowers"). Grass: `GrassBlockBase::onFertilized` 147165f00, `GrassBlock::_shouldStopRandomWalk`
//! 147164d70 and `_plantGrass` 147164e10; flowers: `FlowerBlock::onFertilized` 14715ca10.

use sapling_api::BlockPos;
use sapling_worldgen::random::Mt19937;

use super::{FLOWERS, Level, state};

/// Walks `j` from 16 to 63 take `j / 16` steps.
const WALKS: std::ops::Range<i32> = 16..64;

/// The level with what this click set so far: later walks see earlier plants.
struct Planting<'a> {
    level: &'a Level,
    set: Vec<(BlockPos, u32)>,
}

impl Planting<'_> {
    fn id(&self, cell: BlockPos) -> Option<u32> {
        if let Some((_, id)) = self.set.iter().rev().find(|(at, _)| *at == cell) {
            return Some(*id);
        }
        self.level.in_height(cell[1]).then(|| self.level.block(cell[0], cell[1], cell[2]))
    }

    fn name(&self, cell: BlockPos) -> Option<&str> {
        let state = self.level.registry().get(self.id(cell)?)?;
        Some(state.name.strip_prefix("minecraft:").unwrap_or(state.name))
    }

    fn is_air(&self, cell: BlockPos) -> bool {
        self.id(cell).and_then(|id| self.level.registry().get(id)).is_some_and(|state| state.is_air())
    }

    /// Where a walk ends: a cell with no grass under it, or a solid one.
    fn stops(&self, [x, y, z]: BlockPos) -> bool {
        self.name([x, y - 1, z]) != Some("grass_block") || self.id([x, y, z]).and_then(|id| self.level.registry().get(id)).is_none_or(|state| state.is_solid())
    }
}

fn one_in(random: &mut Mt19937, n: i32) -> bool {
    random.next_int(n) == 0
}

/// Bone meal on the grass block at `pos` (its top free): each walk that ends on air plants short grass, a
/// fern one in eight, a flower of the biome one in eight; one that ends on short grass with air over it
/// grows it tall one in ten.
pub(super) fn grass(level: &Level, [x, y, z]: BlockPos, random: &mut Mt19937) -> Vec<(BlockPos, u32)> {
    let mut planting = Planting { level, set: Vec::new() };
    for walk in WALKS {
        let mut at = [x, y + 1, z];
        let mut stopped = false;
        for _ in 0..walk >> 4 {
            at[0] += random.next_int(3) - 1;
            at[1] += if one_in(random, 9) { -1 } else { one_in(random, 8) as i32 };
            at[2] += random.next_int(3) - 1;
            stopped = planting.stops(at);
            if stopped {
                break;
            }
        }
        if !stopped {
            plant_grass(&mut planting, at, random);
        }
    }
    planting.set
}

fn plant_grass(planting: &mut Planting, at: BlockPos, random: &mut Mt19937) {
    let level = planting.level;
    if planting.is_air(at) {
        let (name, growth) = if one_in(random, 8) {
            let biome = level.biome(at[0], at[1], at[2]);
            sapling_worldgen::meal_flower(&|tag| biome.is_some_and(|biome| sapling_data::biome_has_tag(biome, tag)), at, random)
        } else if one_in(random, 7) {
            ("fern", None)
        } else {
            ("short_grass", None)
        };
        if let Some(id) = state(level, name, &growth.map(|growth| format!("growth={growth}")).unwrap_or_default()) {
            planting.set.push((at, id));
        }
        return;
    }
    let above = [at[0], at[1] + 1, at[2]];
    if planting.name(at) == Some("short_grass") && planting.is_air(above) && one_in(random, 10) {
        let (Some(lower), Some(upper)) = (state(level, "tall_grass", "upper_block_bit=0"), state(level, "tall_grass", "upper_block_bit=1")) else { return };
        planting.set.extend([(at, lower), (above, upper)]);
    }
}

/// Bone meal on the flower `name` at `pos`: its own kind on each walk's air cell, a poppy and a
/// dandelion each other's one in eight. A walk starts a block over the flower, so its first step goes down.
pub(super) fn flower(level: &Level, [x, y, z]: BlockPos, name: &str, random: &mut Mt19937) -> Vec<(BlockPos, u32)> {
    let mut planting = Planting { level, set: Vec::new() };
    for walk in WALKS {
        let mut at = [x, y + 1, z];
        let mut stopped = false;
        for _ in 0..walk >> 4 {
            at[0] += random.next_int(3) - 1;
            let (up, by) = (random.next_int(3), random.next_int(3));
            at[1] += by * (up - 1) / 2;
            at[2] += random.next_int(3) - 1;
            stopped = planting.stops(at);
            if stopped {
                break;
            }
        }
        if stopped || !planting.is_air(at) {
            continue;
        }
        let swapped = match name {
            "poppy" if one_in(random, 8) => "dandelion",
            "dandelion" if one_in(random, 8) => "poppy",
            _ => name,
        };
        if let Some(id) = FLOWERS.contains(&swapped).then(|| state(level, swapped, "")).flatten() {
            planting.set.push((at, id));
        }
    }
    planting.set
}
