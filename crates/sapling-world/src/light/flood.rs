//! Light spreading over a [`Grid`], one channel at a time, breadth first: each step loses what
//! `super::step` says. Light taken away is cleared from every cell it may have reached, which then
//! fill again from their own sources and from the brighter cells met at the edge of the clearing.

use std::collections::VecDeque;

use super::nibbles::Channel;
use super::{FULL_LIGHT, emission, filter, step};

pub(crate) type Pos = [i32; 3];

/// Cells the light can be in, with what they hold.
pub(crate) trait Grid {
    /// The cell's emission << 4 | filter; `None` where nothing is lit now (outside the world's
    /// height, or a column that is not loaded).
    fn props(&self, at: Pos) -> Option<u8>;
    /// `None` where [`Grid::props`] is.
    fn level(&self, channel: Channel, at: Pos) -> Option<u8>;
    fn set_level(&mut self, channel: Channel, at: Pos, level: u8);
    /// The first y over the world: the sky's light comes in through the layer below it.
    fn top(&self) -> i32;
}

/// The neighbours in order; the last is the one below.
const SIDES: [Pos; 6] = [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, 1, 0], [0, -1, 0]];
const DOWN: usize = 5;

fn beside(at: Pos, side: Pos) -> Pos {
    [at[0] + side[0], at[1] + side[1], at[2] + side[2]]
}

pub(crate) struct Flood {
    channel: Channel,
    raise: VecDeque<Pos>,
    lower: VecDeque<(Pos, u8)>,
}

impl Flood {
    pub fn new(channel: Channel) -> Self {
        Self { channel, raise: VecDeque::new(), lower: VecDeque::new() }
    }

    /// The light a cell makes by itself: its emission, or the sky's on the top layer.
    pub fn source(&self, grid: &impl Grid, at: Pos) -> u8 {
        let Some(props) = grid.props(at) else { return 0 };
        match self.channel {
            Channel::Block => emission(props),
            Channel::Sky if at[1] == grid.top() - 1 => step(FULL_LIGHT, filter(props), true),
            Channel::Sky => 0,
        }
    }

    /// Spreads from a cell as its level is now.
    pub fn spread_from(&mut self, at: Pos) {
        self.raise.push_back(at);
    }

    /// Sets a cell to `level` and spreads from it, where that is brighter than what it holds.
    pub fn raise(&mut self, grid: &mut impl Grid, at: Pos, level: u8) {
        if grid.level(self.channel, at).is_some_and(|was| level > was) {
            grid.set_level(self.channel, at, level);
            self.raise.push_back(at);
        }
    }

    /// The block in a cell changed: its light and all that came from it is worked out anew by
    /// [`Flood::run`], and its neighbours spread into it again.
    pub fn changed(&mut self, grid: &mut impl Grid, at: Pos) {
        let Some(was) = grid.level(self.channel, at) else { return };
        if was > 0 {
            grid.set_level(self.channel, at, 0);
            self.lower.push_back((at, was));
        }
        let own = self.source(grid, at);
        self.raise(grid, at, own);
        for side in SIDES {
            let next = beside(at, side);
            if grid.level(self.channel, next).is_some_and(|level| level > 0) {
                self.raise.push_back(next);
            }
        }
    }

    pub fn run(&mut self, grid: &mut impl Grid) {
        let sky = self.channel == Channel::Sky;
        while let Some((at, level)) = self.lower.pop_front() {
            for (way, side) in SIDES.iter().enumerate() {
                let next = beside(at, *side);
                let Some(held) = grid.level(self.channel, next).filter(|held| *held > 0) else { continue };
                if held < level || sky && way == DOWN && level == FULL_LIGHT {
                    grid.set_level(self.channel, next, 0);
                    self.lower.push_back((next, held));
                    let own = self.source(grid, next);
                    self.raise(grid, next, own);
                } else {
                    self.raise.push_back(next);
                }
            }
        }
        while let Some(at) = self.raise.pop_front() {
            let Some(level) = grid.level(self.channel, at).filter(|level| *level > 1) else { continue };
            for (way, side) in SIDES.iter().enumerate() {
                let next = beside(at, *side);
                let (Some(held), Some(props)) = (grid.level(self.channel, next), grid.props(next)) else { continue };
                let reached = step(level, filter(props), sky && way == DOWN);
                if reached > held {
                    grid.set_level(self.channel, next, reached);
                    self.raise.push_back(next);
                }
            }
        }
    }
}
