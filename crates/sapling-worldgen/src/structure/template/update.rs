//! The pass a template placement ends with (141f5c5d0) and the queue behind a `setBlock` that updates
//! neighbours (1415f6fd0): what gives fences, panes, bars, walls and stairs their neighbour-shaped
//! states. docs/research/worldgen-bds-templates.md, "The update pass".

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use super::connect::{SIDES, Shape, corner, joins, shape, with_states};
use super::wall;
use crate::block::Block;
use crate::structure::BoundingBox;
use crate::structure::canvas::{Canvas, Settled};

type Pos = (i32, i32, i32);

/// The six faces and the eight vertical diagonals: the neighbours the pass asks about.
const AROUND: [Pos; 14] =
    [(0, 0, -1), (0, 0, 1), (1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 1, -1), (0, -1, -1), (0, 1, 1), (0, -1, 1), (1, 1, 0), (1, -1, 0), (-1, 1, 0), (-1, -1, 0)];

/// What a canvas keeps for the pass.
#[derive(Default)]
pub struct Record<'a> {
    /// The blocks the template placement under way changed (BDS files a block only when its `setBlock`
    /// changed the cell); None outside a placement.
    placing: Option<HashMap<Pos, Block>>,
    /// Cells whose states a pass decided, across a replayed run's chunks; None runs no pass.
    pub settled: Option<&'a RefCell<Settled>>,
}

impl Record<'_> {
    /// A piece's write of `block` over `before`.
    pub fn note(&mut self, at: Pos, before: Block, block: Block) {
        match &mut self.placing {
            Some(placing) if before != block => drop(placing.insert(at, block)),
            Some(_) => {}
            None => {
                if let Some(settled) = self.settled {
                    settled.borrow_mut().remove(&at);
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Tagged: every side, whatever the template placed around it.
    Stairs,
    /// The neighbour-update component: the sides in the mask.
    Joiner(Shape),
    /// `onStructureBlockPlace`: all of it, from the world.
    Wall,
}

fn class(block: Block) -> Option<Class> {
    if wall::is_wall(block) {
        return Some(Class::Wall);
    }
    shape(block).map(|kind| if kind == Shape::Stairs { Class::Stairs } else { Class::Joiner(kind) })
}

fn offset((x, y, z): Pos, (dx, dy, dz): Pos) -> Pos {
    (x + dx, y + dy, z + dz)
}

impl Canvas<'_> {
    /// Starts a template placement: its writes are filed for `update_pass`.
    pub fn begin_template(&mut self) {
        if self.record.settled.is_some() {
            self.record.placing = Some(HashMap::new());
        }
    }

    /// Ends a template placement with the pass over `cells`, the template's box grown by one in its own
    /// x, y, z order, as world positions. A `legacy` template's blocks came through the loader's old
    /// converter, which files fences and panes for every side and walls for a look whatever is around.
    pub fn update_pass(&mut self, cells: impl Iterator<Item = Pos>, legacy: bool) {
        let Some(placed) = self.record.placing.take() else { return };
        let Some(first) = placed.keys().next() else { return };
        let grow = |(lo, hi): (Pos, Pos), p: &Pos| ((lo.0.min(p.0 - 1), lo.1.min(p.1 - 1), lo.2.min(p.2 - 1)), (hi.0.max(p.0 + 1), hi.1.max(p.1 + 1), hi.2.max(p.2 + 1)));
        let (lo, hi) = placed.keys().fold((*first, *first), grow);
        let reached = |p: &Pos| (lo.0..=hi.0).contains(&p.0) && (lo.1..=hi.1).contains(&p.1) && (lo.2..=hi.2).contains(&p.2);
        for at in cells.filter(reached) {
            let near = |by: &dyn Fn(Pos) -> bool| AROUND.iter().any(|&d| by(offset(at, d)));
            let sides = |by: &dyn Fn(Pos) -> bool| -> Vec<usize> { (0..4).filter(|&i| by(offset(at, (SIDES[i].1, 0, SIDES[i].2)))).collect() };
            let is_placed = |p: Pos| placed.contains_key(&p);
            let not_placed = |p: Pos| !placed.contains_key(&p);
            match placed.get(&at) {
                Some(&recorded) => match class(recorded).inspect(|_| self.settle(at)) {
                    Some(Class::Stairs) => self.update(at, recorded, &[0, 1, 2, 3]),
                    Some(Class::Joiner(_)) if legacy => self.update(at, recorded, &[0, 1, 2, 3]),
                    Some(Class::Joiner(_)) => self.update(at, recorded, &sides(&not_placed)),
                    Some(Class::Wall) if legacy || near(&not_placed) => self.update_wall(at),
                    _ => {}
                },
                None if near(&is_placed) => {
                    let block = self.get(at.0, at.1, at.2);
                    match class(block) {
                        Some(Class::Wall) => self.update_wall(at),
                        Some(_) => self.update(at, block, &sides(&is_placed)),
                        None => {}
                    }
                }
                None => {}
            }
        }
    }

    /// `upgradeStructureConnectionBlocks` (inlined at 141f07a80): once a start's pieces are placed in a
    /// chunk, every fence, pane, bar and stair inside the start's box, in the chunk and the columns
    /// around it, from the world's floor to the box's top, looks at all its sides again.
    pub fn upgrade(&mut self, start: &BoundingBox) {
        let (chunk, top) = (self.bb, start.y1.min(self.bb.y1));
        let connecting = |canvas: &Self, x: i32, y: i32, z: i32| matches!(class(canvas.get(x, y, z)), Some(Class::Stairs | Class::Joiner(_)));
        // Nothing runs unless the chunk itself holds such a block: a post stays joined to the stone
        // beyond the border that a neighbour without one clears afterwards.
        let any = || (chunk.x0..=chunk.x1).any(|x| (chunk.z0..=chunk.z1).any(|z| (chunk.y0..=chunk.y1).any(|y| connecting(self, x, y, z))));
        if self.record.settled.is_none() || !any() {
            return;
        }
        for x in start.x0.max(chunk.x0 - 1)..=start.x1.min(chunk.x1 + 1) {
            for z in start.z0.max(chunk.z0 - 1)..=start.z1.min(chunk.z1 + 1) {
                for y in chunk.y0..=top {
                    if connecting(self, x, y, z) {
                        self.update((x, y, z), self.get(x, y, z), &[0, 1, 2, 3]);
                        self.settle((x, y, z));
                    }
                }
            }
        }
        // A fence that joined something is no longer the block a mineshaft piece keeps (canvas.rs).
        let joined: Vec<Pos> = self.shaft_fences.iter().copied().filter(|&(x, y, z)| self.get(x, y, z).states().contains("=1")).collect();
        joined.iter().for_each(|at| drop(self.shaft_fences.remove(at)));
    }

    /// A placed block keeps the state the pass leaves it with.
    fn settle(&self, at: Pos) {
        if let Some(settled) = self.record.settled {
            settled.borrow_mut().insert(at);
        }
    }

    /// `from` at `at` with its `sides` (indices into `SIDES`) recomputed, written if that changes it; a
    /// change is set with the neighbour flag, so the blocks beside it look at their side toward it in turn.
    fn update(&mut self, at: Pos, from: Block, sides: &[usize]) {
        let mut queue = VecDeque::new();
        if self.update_one(at, from, sides) {
            queue.push_back(at);
        }
        while let Some(changed) = queue.pop_front() {
            for &(_, dx, dz) in &SIDES {
                let beside = offset(changed, (dx, 0, dz));
                let block = self.get(beside.0, beside.1, beside.2);
                // The side of `beside` that faces the changed cell.
                let toward = SIDES.iter().position(|&(_, sx, sz)| (sx, sz) == (-dx, -dz)).expect("an opposite side");
                if matches!(class(block), Some(Class::Stairs | Class::Joiner(_))) && self.update_one(beside, block, &[toward]) {
                    queue.push_back(beside);
                }
            }
        }
    }

    fn update_one(&mut self, at: Pos, from: Block, sides: &[usize]) -> bool {
        if sides.is_empty() {
            return false;
        }
        let (x, y, z) = at;
        let changes: Vec<(String, String)> = match class(from) {
            Some(Class::Stairs) => vec![("minecraft:corner".into(), corner(self, from, at).into())],
            Some(Class::Joiner(kind)) => sides
                .iter()
                .map(|&i| {
                    let (side, dx, dz) = SIDES[i];
                    (format!("minecraft:connection_{side}"), u8::from(joins(kind, self.get(x + dx, y, z + dz), (dx, dz))).to_string())
                })
                .collect(),
            _ => return false,
        };
        let new = with_states(from, &changes);
        // A placed block starts from the state it was filed with, whatever the cell holds by now.
        if new == from || new == self.get(x, y, z) {
            return false;
        }
        self.set_in_view(x, y, z, new);
        true
    }

    /// A wall's sides and post from the blocks around it now (1425e4e50); it tells no neighbour.
    fn update_wall(&mut self, at: Pos) {
        let (x, y, z) = at;
        let block = self.get(x, y, z);
        let new = with_states(block, &wall::changes(self, at));
        if new != block {
            self.set_in_view(x, y, z, new);
        }
    }
}
