//! A placed pool element (`PoolElementStructurePiece`, postProcess 141ef21e0; element placement
//! 145e1b280): jigsaws turn into their final states, then the template goes down with the
//! element's processors, drawing from a generator seeded with the chunk's structure seed.

use super::super::BoundingBox;
use super::super::canvas::Canvas;
use super::super::template::{Placement, Rotation};
use super::element::{Element, ElementKind, Projection};

#[derive(Clone)]
pub struct Piece {
    pub element: &'static Element,
    pub origin: (i32, i32, i32),
    pub rotation: Rotation,
    pub bb: BoundingBox,
    pub depth: i32,
}

impl Piece {
    pub fn new(element: &'static Element, origin: (i32, i32, i32), rotation: Rotation, depth: i32) -> Self {
        Self { element, origin, rotation, bb: element.bbox(origin, rotation), depth }
    }

    pub fn shift_y(&mut self, dy: i32) {
        self.origin.1 += dy;
        self.bb.move_y(dy);
    }

    /// The piece within the canvas's chunk; `seed` is the chunk's structure seed.
    pub fn post_process(&self, canvas: &mut Canvas, seed: u32) {
        let ElementKind::Template { template, rules, projection, .. } = &self.element.kind else {
            // TODO: feature elements (trees, piles, sculk patches) place features.
            return;
        };
        for jigsaw in self.element.jigsaws(self.origin, self.rotation) {
            let (x, y, z) = jigsaw.pos;
            if canvas.bb.contains(x, y, z) && *projection == Projection::Rigid {
                canvas.set(x, y, z, jigsaw.final_state);
            }
        }
        let mut placement = Placement::new(self.rotation, canvas.bb);
        placement.seed = seed;
        placement.rules = rules;
        let blocks: Vec<_> = {
            let world = |x: i32, y: i32, z: i32| canvas.get(x, y, z);
            placement.blocks_in(template, self.origin, &world).collect()
        };
        for ((x, y, z), block, _) in blocks {
            canvas.set(x, y, z, block);
        }
    }
}
