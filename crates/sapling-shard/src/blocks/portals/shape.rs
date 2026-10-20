//! A Nether portal's frame: obsidian round an inside of 2 to 21 wide and 3 to 21 high, the
//! corners left out or not (research/captures/bds-portal-frames.txt).

use sapling_api::BlockPos;

pub(crate) const MIN: [i32; 2] = [2, 3];
pub(crate) const MAX: i32 = 21;

/// What a cell is to a portal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cell {
    Frame,
    /// Air, fire or a portal block: what an inside may hold.
    Open,
    Other,
}

/// The direction a portal's inside runs in, as `portal_axis` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    X,
    Z,
}

impl Axis {
    pub fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Z => "z",
        }
    }

    fn step(self, [x, y, z]: BlockPos, by: i32) -> BlockPos {
        match self {
            Self::X => [x + by, y, z],
            Self::Z => [x, y, z + by],
        }
    }
}

/// An inside with a whole frame round it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shape {
    pub axis: Axis,
    /// The inside's lowest cell at its start along the axis.
    pub corner: BlockPos,
    pub width: i32,
    pub height: i32,
}

impl Shape {
    /// The inside's cells, along the axis first and each column from the top: the order BDS
    /// lists a lit portal's blocks in.
    pub fn cells(&self) -> Vec<BlockPos> {
        let column = |along: i32| (0..self.height).rev().map(move |up| self.axis.step(above(self.corner, up), along));
        (0..self.width).flat_map(column).collect()
    }

    pub fn contains(&self, pos: BlockPos) -> bool {
        self.cells().contains(&pos)
    }
}

fn above([x, y, z]: BlockPos, by: i32) -> BlockPos {
    [x, y + by, z]
}

/// The frame an open cell is inside, along x if it has one both ways (the Java game's order).
pub(crate) fn find(at: BlockPos, cell: &dyn Fn(BlockPos) -> Cell) -> Option<Shape> {
    [Axis::X, Axis::Z].into_iter().find_map(|axis| find_along(at, axis, cell))
}

/// The frame round `at` with its inside running along `axis`.
pub(crate) fn find_along(at: BlockPos, axis: Axis, cell: &dyn Fn(BlockPos) -> Cell) -> Option<Shape> {
    if cell(at) != Cell::Open {
        return None;
    }
    let mut bottom = at;
    while at[1] - bottom[1] < MAX && cell(above(bottom, -1)) == Cell::Open {
        bottom = above(bottom, -1);
    }
    // Back along the axis over a floor of frame, to the wall.
    let floored = |pos: BlockPos| cell(pos) == Cell::Open && cell(above(pos, -1)) == Cell::Frame;
    let back = (1..=MAX).find(|&by| !floored(axis.step(bottom, -by)))?;
    if cell(axis.step(bottom, -back)) != Cell::Frame {
        return None;
    }
    let corner = axis.step(bottom, 1 - back);
    let width = (0..=MAX).find(|&by| !floored(axis.step(corner, by)))?;
    if !(MIN[0]..=MAX).contains(&width) || cell(axis.step(corner, width)) != Cell::Frame {
        return None;
    }
    let row = |up: i32, wanted: Cell| (0..width).all(|along| cell(axis.step(above(corner, up), along)) == wanted);
    let walled = |up: i32| cell(axis.step(above(corner, up), -1)) == Cell::Frame && cell(axis.step(above(corner, up), width)) == Cell::Frame;
    let height = (0..=MAX).find(|&up| !(row(up, Cell::Open) && walled(up)))?;
    ((MIN[1]..=MAX).contains(&height) && row(height, Cell::Frame)).then_some(Shape { axis, corner, width, height })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Obsidian round an inside of `width` x `height` along `axis` from `corner`, corners kept or not.
    fn frame(axis: Axis, corner: BlockPos, width: i32, height: i32, corners: bool) -> HashSet<BlockPos> {
        let mut obsidian = HashSet::new();
        for along in -1..=width {
            for up in -1..=height {
                let edge = along == -1 || along == width || up == -1 || up == height;
                let corner_cell = (along == -1 || along == width) && (up == -1 || up == height);
                if edge && (corners || !corner_cell) {
                    obsidian.insert(axis.step(above(corner, up), along));
                }
            }
        }
        obsidian
    }

    fn cells(obsidian: HashSet<BlockPos>, stone: &[BlockPos]) -> impl Fn(BlockPos) -> Cell {
        let stone: HashSet<BlockPos> = stone.iter().copied().collect();
        move |pos| if obsidian.contains(&pos) { Cell::Frame } else if stone.contains(&pos) { Cell::Other } else { Cell::Open }
    }

    #[test]
    fn the_sizes_bds_lit_and_those_it_did_not() {
        let lit = |width, height| find([4, -59, 5], &cells(frame(Axis::X, [4, -59, 5], width, height, true), &[])).map(|shape| (shape.width, shape.height));
        assert_eq!([lit(2, 3), lit(21, 21), lit(3, 4)], [Some((2, 3)), Some((21, 21)), Some((3, 4))]);
        assert_eq!([lit(1, 2), lit(2, 2), lit(22, 3), lit(3, 22)], [None, None, None, None]);
    }

    #[test]
    fn a_frame_along_z_without_corners_lights_from_any_inside_cell() {
        let around = cells(frame(Axis::Z, [60, -59, 11], 2, 3, false), &[]);
        let shape = find([60, -58, 12], &around).unwrap();
        assert_eq!((shape.axis, shape.corner, shape.width, shape.height), (Axis::Z, [60, -59, 11], 2, 3));
        assert_eq!(shape.cells(), [[60, -57, 11], [60, -58, 11], [60, -59, 11], [60, -57, 12], [60, -58, 12], [60, -59, 12]]);
        assert!(find_along([60, -58, 12], Axis::X, &around).is_none());
    }

    #[test]
    fn a_gap_or_a_block_inside_spoils_the_frame() {
        let mut gap = frame(Axis::X, [0, 0, 0], 2, 3, true);
        gap.remove(&[-1, 1, 0]);
        assert!(find([0, 0, 0], &cells(gap, &[])).is_none());
        assert!(find([0, 0, 0], &cells(frame(Axis::X, [0, 0, 0], 2, 3, true), &[[1, 2, 0]])).is_none());
        assert!(find([0, 5, 0], &cells(frame(Axis::X, [0, 0, 0], 2, 3, true), &[])).is_none(), "outside the frame");
    }
}
