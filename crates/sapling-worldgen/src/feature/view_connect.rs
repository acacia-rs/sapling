//! A feature's write is a `setBlock` with flag 3: the fences, panes and bars beside the cell redo their side
//! toward it, and each that changes tells its own neighbours (docs/research/worldgen-bds-templates.md, "The update
//! pass", item 3). Liquids hear nothing of it.

use super::view::View;
use crate::structure::template::side_toward;

impl View<'_> {
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: crate::block::Block) -> bool {
        let changed = self.set_block_alone(x, y, z, block);
        if changed {
            let mut queue = vec![(x, y, z)];
            while let Some((cx, cy, cz)) = queue.pop() {
                let here = self.block(cx, cy, cz);
                for (dx, dz) in [(1, 0), (0, -1), (0, 1), (-1, 0)] {
                    let (bx, bz) = (cx + dx, cz + dz);
                    if let Some(new) = side_toward(self.block(bx, cy, bz), here, (-dx, -dz))
                        && self.set_block_alone(bx, cy, bz, new)
                    {
                        queue.push((bx, cy, bz));
                    }
                }
            }
        }
        changed
    }
}
