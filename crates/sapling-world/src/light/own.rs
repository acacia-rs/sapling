//! A column's light by itself, made on the I/O thread that loads it: the sky straight down each
//! block column, then both channels flooded within the column. What crosses into the neighbours
//! is added when it arrives (`World::relight`).

use acacia_world::{Chunk, SECTION_VOLUME};

use super::flood::{Flood, Grid, Pos};
use super::nibbles::{Channel, ColumnLight, index};
use super::{FULL_LIGHT, Props, emission, filter, step};

/// The props of one section: one value for a section of one block.
enum Cells {
    Uniform(u8),
    Mixed(Box<[u8; SECTION_VOLUME]>),
}

impl Cells {
    fn get(&self, at: usize) -> u8 {
        match self {
            Self::Uniform(props) => *props,
            Self::Mixed(props) => props[at],
        }
    }
}

struct Alone<'a> {
    cells: Vec<Cells>,
    light: &'a mut ColumnLight,
    /// The column's lowest x and z, and the dimension's floor.
    origin: [i32; 3],
    top: i32,
}

impl Alone<'_> {
    fn slot(&self, [x, y, z]: Pos) -> Option<usize> {
        let inside = (0..16).contains(&(x - self.origin[0])) && (0..16).contains(&(z - self.origin[2])) && (self.origin[1]..self.top).contains(&y);
        inside.then(|| ((y - self.origin[1]) >> 4) as usize)
    }
}

impl Grid for Alone<'_> {
    fn props(&self, at: Pos) -> Option<u8> {
        self.slot(at).map(|slot| self.cells[slot].get(index(at[0], at[1], at[2])))
    }

    fn level(&self, channel: Channel, at: Pos) -> Option<u8> {
        self.slot(at).map(|slot| self.light.section(channel, slot).get(index(at[0], at[1], at[2])))
    }

    fn set_level(&mut self, channel: Channel, at: Pos, level: u8) {
        if let Some(slot) = self.slot(at) {
            self.light.section_mut(channel, slot).set(index(at[0], at[1], at[2]), level);
        }
    }

    fn top(&self) -> i32 {
        self.top
    }
}

fn cells(chunk: &Chunk, props: &Props) -> Vec<Cells> {
    let air = chunk.dimension().air;
    let (mut blocks, mut liquid) = (Box::new([0; SECTION_VOLUME]), Box::new([0; SECTION_VOLUME]));
    (0..chunk.section_count())
        .map(|section| match chunk.section_uniform(section) {
            Some(id) => Cells::Uniform(props.cell(id, air, air)),
            None if chunk.copy_section(section, &mut blocks, &mut liquid) => Cells::Mixed(Box::new(std::array::from_fn(|at| props.cell(blocks[at], liquid[at], air)))),
            None => Cells::Uniform(props.cell(air, air, air)),
        })
        .collect()
}

/// The light of `chunk` as if nothing were around it.
pub(crate) fn light_column(chunk: &Chunk, props: &Props) -> ColumnLight {
    let dim = chunk.dimension();
    let mut light = ColumnLight::dark(dim);
    let origin = [chunk.x * 16, dim.min_y, chunk.z * 16];
    let mut grid = Alone { cells: cells(chunk, props), light: &mut light, origin, top: dim.min_y + dim.height as i32 };
    if dim.sky {
        sky(&mut grid);
    }
    shine(&mut grid);
    light.compact();
    light
}

/// Lights the sky down every block column, then spreads it sideways from where a neighbour's
/// shaft ends higher, and from what came through a filter.
fn sky(grid: &mut Alone) {
    let [ox, floor, oz] = grid.origin;
    // Sections of clear air up to the top are lit whole.
    let clear = grid.cells.iter().rposition(|cells| !matches!(cells, Cells::Uniform(props) if filter(*props) == 0)).map_or(0, |slot| slot + 1);
    for slot in clear..grid.cells.len() {
        *grid.light.section_mut(Channel::Sky, slot) = super::nibbles::Nibbles::Uniform(FULL_LIGHT);
    }
    let start = floor + 16 * clear as i32;
    // Per block column: the lowest y still at full light, and the lowest lit enough to spread.
    let mut full = [[start; 16]; 16];
    let mut lit = [[start; 16]; 16];
    for x in 0..16 {
        for z in 0..16 {
            let mut level = FULL_LIGHT;
            for y in (floor..start).rev() {
                let at = [ox + x, y, oz + z];
                level = step(level, filter(grid.props(at).unwrap_or(FULL_LIGHT)), true);
                if level == 0 {
                    break;
                }
                grid.set_level(Channel::Sky, at, level);
                if level == FULL_LIGHT {
                    full[x as usize][z as usize] = y;
                }
                if level > 1 {
                    lit[x as usize][z as usize] = y;
                }
            }
        }
    }
    let mut flood = Flood::new(Channel::Sky);
    for x in 0..16usize {
        for z in 0..16usize {
            let around = [(x.wrapping_sub(1), z), (x + 1, z), (x, z.wrapping_sub(1)), (x, z + 1)];
            let highest = around.iter().filter(|(nx, nz)| *nx < 16 && *nz < 16).map(|(nx, nz)| full[*nx][*nz]).max().unwrap_or(start);
            for y in lit[x][z]..highest.max(full[x][z]) {
                flood.spread_from([ox + x as i32, y, oz + z as i32]);
            }
        }
    }
    flood.run(grid);
}

/// Floods block light from every block that shines.
fn shine(grid: &mut Alone) {
    let [ox, floor, oz] = grid.origin;
    let mut flood = Flood::new(Channel::Block);
    for slot in 0..grid.cells.len() {
        if matches!(grid.cells[slot], Cells::Uniform(props) if emission(props) == 0) {
            continue;
        }
        for at in 0..SECTION_VOLUME {
            let shines = emission(grid.cells[slot].get(at));
            if shines > 0 {
                let cell = [ox + (at >> 8) as i32, floor + 16 * slot as i32 + (at & 15) as i32, oz + (at >> 4 & 15) as i32];
                flood.raise(grid, cell, shines);
            }
        }
    }
    flood.run(grid);
}
