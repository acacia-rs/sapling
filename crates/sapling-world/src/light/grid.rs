//! A world's loaded columns as one [`Grid`]: blocks read from the world, light from the columns
//! it lit. An arena reads its template's light until it changes a column's (as for blocks), and a
//! world without storage has open sky in every column it holds no light for.

use std::collections::HashMap;

use super::flood::{Grid, Pos};
use super::nibbles::{Channel, ColumnLight};
use super::{FULL_LIGHT, Props};
use crate::{ChunkPos, World};

pub(super) struct WorldGrid<'a> {
    pub world: &'a World,
    pub props: &'a Props,
    /// The world's own light, taken out of it while the grid works on it.
    pub columns: &'a mut HashMap<ChunkPos, ColumnLight>,
}

/// The light a column has where the world holds none of its own: its template's, else open sky.
pub(super) fn inherited<'a>(world: &'a World, pos: ChunkPos) -> Option<&'a ColumnLight> {
    world.base.as_ref().and_then(|template| template.light(pos))
}

impl WorldGrid<'_> {
    fn reaches(&self, [x, y, z]: Pos) -> Option<ChunkPos> {
        let pos = ChunkPos::of_block(x, z);
        (self.world.in_height(y) && self.world.is_loaded(pos)).then_some(pos)
    }
}

impl Grid for WorldGrid<'_> {
    fn props(&self, at: Pos) -> Option<u8> {
        let pos = self.reaches(at)?;
        let [x, y, z] = at;
        let air = self.world.dim.air;
        Some(self.world.column(pos).map_or(self.props.cell(air, air, air), |column| self.props.cell(column.block(x, y, z), column.liquid(x, y, z), air)))
    }

    fn level(&self, channel: Channel, at: Pos) -> Option<u8> {
        let pos = self.reaches(at)?;
        let [x, y, z] = at;
        match self.columns.get(&pos).or_else(|| inherited(self.world, pos)) {
            Some(light) => light.get(channel, x, y, z),
            None => Some(if channel == Channel::Sky && self.world.dim.sky { FULL_LIGHT } else { 0 }),
        }
    }

    fn set_level(&mut self, channel: Channel, at: Pos, level: u8) {
        let Some(pos) = self.reaches(at) else { return };
        let (world, [x, y, z]) = (self.world, at);
        let light = self.columns.entry(pos).or_insert_with(|| inherited(world, pos).cloned().unwrap_or_else(|| ColumnLight::open(world.dim)));
        light.set(channel, x, y, z, level);
    }

    fn top(&self) -> i32 {
        self.world.heights().end
    }
}
