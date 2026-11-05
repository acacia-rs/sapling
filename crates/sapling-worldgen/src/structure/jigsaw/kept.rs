//! A piece's jigsaw cells: their final states, and the `JigsawBlock`s that `/place`'s
//! `keepJigsaws` keeps there (`BlockEntity::Jigsaw`, filed only when the canvas keeps jigsaws).

use super::super::canvas::Canvas;
use super::super::template::{Rule, apply};
use super::element::JigsawInfo;
use super::piece::Piece;
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::random::Mt19937;

impl Piece {
    /// A jigsaw's final state through the element's rules, each jigsaw on a generator of its own
    /// seeded like the template's: the jigsaws of one chunk all draw the same floats
    /// (docs/research/worldgen-bds-jigsaw.md, "Elements and processors").
    pub(super) fn set_jigsaw(&self, canvas: &mut Canvas, rules: &[Rule], jigsaw: &JigsawInfo, (x, y, z): (i32, i32, i32), seed: u32) {
        let block = apply(rules, jigsaw.final_state, canvas.get(x, y, z), self.from_pivot((x, y, z)), &mut Mt19937::new(seed));
        canvas.set(x, y, z, block);
        if canvas.keep_jigsaws {
            canvas.set_entity(x, y, z, kept(jigsaw, block));
        }
    }

    /// A v2 placement's jigsaws, filed over what the template left in their cells.
    pub(super) fn keep_jigsaws(&self, canvas: &mut Canvas) {
        for jigsaw in self.element.jigsaws(self.origin, self.rotation) {
            let (x, y, z) = jigsaw.pos;
            if canvas.bb.contains(x, y, z) {
                let over = canvas.get(x, y, z);
                canvas.set_entity(x, y, z, kept(&jigsaw, over));
            }
        }
    }
}

fn kept(jigsaw: &JigsawInfo, over: Block) -> BlockEntity {
    BlockEntity::Jigsaw {
        facing: jigsaw.front as u8,
        name: jigsaw.name,
        target: jigsaw.target,
        pool: jigsaw.pool,
        rollable: !jigsaw.aligned,
        final_state: jigsaw.final_state,
        over,
        selection_priority: jigsaw.selection_priority,
        placement_priority: jigsaw.placement_priority,
    }
}
