//! A world's light: what it holds, what waits to be worked out, and how it is asked for.

use std::collections::HashMap;
use std::sync::Arc;

use acacia_world::BlockRegistry;

use super::flood::Flood;
use super::grid::{WorldGrid, inherited};
use super::nibbles::{Channel, ColumnLight};
use super::{FULL_LIGHT, Props, filter};
use crate::{ChunkPos, World};

const SIDES: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

pub(crate) struct Lights {
    pub props: Arc<Props>,
    pub columns: HashMap<ChunkPos, ColumnLight>,
    /// Cells whose block lets through or gives other light than before the last relight.
    changed: Vec<[i32; 3]>,
    /// Columns loaded since the last relight, lit as if alone.
    arrived: Vec<ChunkPos>,
}

impl Lights {
    pub fn new(registry: &BlockRegistry) -> Self {
        Self::sharing(Arc::new(Props::of(registry)))
    }

    pub fn sharing(props: Arc<Props>) -> Self {
        Self { props, columns: HashMap::new(), changed: Vec::new(), arrived: Vec::new() }
    }

    /// A loaded column and the light it was loaded with.
    pub fn arrived(&mut self, pos: ChunkPos, light: ColumnLight) {
        self.columns.insert(pos, light);
        self.arrived.push(pos);
    }

    /// Forgets the light of the columns `keep` rejects, and all that waits to be worked out where
    /// `keep` rejects every column.
    pub fn retain(&mut self, keep: impl Fn(ChunkPos) -> bool) {
        self.columns.retain(|pos, _| keep(*pos));
        self.arrived.retain(|pos| keep(*pos));
        self.changed.retain(|[x, _, z]| keep(ChunkPos::of_block(*x, *z)));
    }
}

impl World {
    /// The props of the cell at a position, by what is in both its layers.
    pub(crate) fn light_props(&self, x: i32, y: i32, z: i32) -> u8 {
        self.light.props.cell(self.block(x, y, z), self.liquid(x, y, z), self.dim.air)
    }

    /// Notes a write that changed what a cell does to light, for the next [`World::relight`].
    pub(crate) fn light_written(&mut self, at: [i32; 3], before: u8) {
        if self.light_props(at[0], at[1], at[2]) != before {
            self.light.changed.push(at);
        }
    }

    /// The sky light and block light at a cell, 0 to 15 each, as of the last [`World::relight`];
    /// `None` where no column is loaded. Over the world the sky is open; under it, all is dark.
    pub fn light_at(&self, x: i32, y: i32, z: i32) -> Option<(u8, u8)> {
        let pos = ChunkPos::of_block(x, z);
        if !self.is_loaded(pos) {
            return None;
        }
        if !self.in_height(y) {
            let sky = if self.dim.sky && y >= self.heights().end { FULL_LIGHT } else { 0 };
            return Some((sky, 0));
        }
        Some(match self.light.columns.get(&pos).or_else(|| inherited(self, pos)) {
            Some(light) => (light.get(Channel::Sky, x, y, z).unwrap_or(0), light.get(Channel::Block, x, y, z).unwrap_or(0)),
            None => (if self.dim.sky { FULL_LIGHT } else { 0 }, 0),
        })
    }

    /// BDS's raw brightness at a cell (`BlockSource::getRawBrightness`): its sky light less
    /// `sky_darken` (the night's and the weather's share, `sapling-shard` `light`), or its block light
    /// where that is brighter; 0 in a block that stops all light. `None` where nothing is loaded.
    pub fn brightness_at(&self, x: i32, y: i32, z: i32, sky_darken: u8) -> Option<u8> {
        let (sky, block) = self.light_at(x, y, z)?;
        if self.in_height(y) && filter(self.light_props(x, y, z)) == FULL_LIGHT {
            return Some(0);
        }
        Some(sky.saturating_sub(sky_darken).max(block))
    }

    /// Whether blocks were written or columns loaded since the last [`World::relight`].
    pub fn needs_relight(&self) -> bool {
        !self.light.changed.is_empty() || !self.light.arrived.is_empty()
    }

    /// Joins the columns loaded since the last call to their loaded neighbours, then works the
    /// light out anew around every block written since, all in one flood per channel. Call once a
    /// tick and before light is asked for; a neighbour that is not loaded is joined when it is.
    pub fn relight(&mut self) {
        if !self.needs_relight() {
            return;
        }
        let (changed, arrived) = (std::mem::take(&mut self.light.changed), std::mem::take(&mut self.light.arrived));
        let mut columns = std::mem::take(&mut self.light.columns);
        let props = self.light.props.clone();
        let channels: &[Channel] = if self.dim.sky { &[Channel::Sky, Channel::Block] } else { &[Channel::Block] };
        let mut grid = WorldGrid { world: self, props: &props, columns: &mut columns };
        for &channel in channels {
            let mut flood = Flood::new(channel);
            arrived.iter().for_each(|pos| join(&mut flood, &grid, *pos, channel));
            flood.run(&mut grid);
            changed.iter().for_each(|at| flood.changed(&mut grid, *at));
            flood.run(&mut grid);
        }
        self.light.columns = columns;
    }
}

/// Spreads from the cells along each side of a column that faces a loaded neighbour, on both
/// sides, except where both sections are at one and the same level.
fn join(flood: &mut Flood, grid: &WorldGrid, pos: ChunkPos, channel: Channel) {
    let Some(own) = grid.columns.get(&pos) else { return };
    let floor = grid.world.heights().start;
    for (dx, dz) in SIDES {
        let Some(other) = grid.columns.get(&ChunkPos::new(pos.x + dx, pos.z + dz)) else { continue };
        for slot in 0..own.sections() {
            let level = own.section(channel, slot).uniform();
            if level.is_some() && level == other.section(channel, slot).uniform() {
                continue;
            }
            for along in 0..16 {
                let edge = |d: i32| match d {
                    1 => 15,
                    -1 => 0,
                    _ => along,
                };
                let (x, z) = (pos.x * 16 + edge(dx), pos.z * 16 + edge(dz));
                for y in floor + 16 * slot as i32..floor + 16 * slot as i32 + 16 {
                    flood.spread_from([x, y, z]);
                    flood.spread_from([x + dx, y, z + dz]);
                }
            }
        }
    }
}
