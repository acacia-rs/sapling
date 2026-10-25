//! v2 template placement (`StructurePoolElement` vt+0x38 145e1c040 -> 141ee0510), Java's
//! `processBlockInfos`: every block of the template, structure blocks dropped and jigsaws turned
//! into their final states, through the rule processors, then each capped processor (145942bd0)
//! applying its rules to at most `limit` blocks, picked in an order shuffled by the level seed
//! forked at the template origin. Clipping to a chunk comes after, so pieces agree across chunks.

use super::java_states::v2_block;
use super::rules::{Rule, apply_first};
use super::{Kind, Rotation, Template, named};
use crate::block::Block;
use crate::block_entity::{BlockEntity, cell_seed};
use crate::random::JavaRandom;
use crate::structure::BoundingBox;

#[derive(Debug, Clone)]
pub struct Capped {
    pub limit: usize,
    pub rules: &'static [Rule],
    /// The loot table its rules' `append_loot` gives what they change (archaeology).
    pub loot: Option<&'static str>,
}

pub struct JavaPlacement<'a> {
    pub rotation: Rotation,
    pub rules: &'a [Rule],
    pub capped: &'a [Capped],
    pub level_seed: i64,
}

impl JavaPlacement<'_> {
    /// The processed blocks of `template` at `origin` within `clip`, in placement order, with their
    /// block entities; `world` is what the world holds where a rule's location predicate looks.
    /// Without capped processors a block's result depends on nothing outside it, so the rest is skipped.
    pub fn blocks(&self, template: &Template, origin: (i32, i32, i32), clip: &BoundingBox, world: &dyn Fn(i32, i32, i32) -> Block) -> Vec<((i32, i32, i32), Block, Option<BlockEntity>)> {
        let mut out = Vec::with_capacity(template.blocks.len());
        for (index, b) in template.blocks.iter().enumerate() {
            let mut block = match b.state.kind() {
                Kind::StructureBlock => continue,
                Kind::Jigsaw => match template.jigsaws.iter().find(|j| j.block == index) {
                    Some(j) => j.final_state.block(self.rotation),
                    None => continue,
                },
                Kind::Block if template.ceiling_vines.contains(&index) => named("vine", "vine_direction_bits=15", false),
                Kind::Block => v2_block(b.state, self.rotation),
            };
            if block.name() == "structure_void" {
                continue;
            }
            let (dx, dy, dz) = self.rotation.apply(b.pos);
            let at = (origin.0 + dx, origin.1 + dy, origin.2 + dz);
            if self.capped.is_empty() && !clip.contains(at.0, at.1, at.2) {
                continue;
            }
            if let Some(output) = apply_first(self.rules, block, world(at.0, at.1, at.2), at) {
                block = output;
            }
            out.push((at, block, template.entity(index, || cell_seed([at.0, at.1, at.2]))));
        }
        for capped in self.capped {
            let mut random = JavaRandom::new(self.level_seed).fork_at(origin.0, origin.1, origin.2);
            let mut order: Vec<usize> = (0..out.len()).collect();
            random.shuffle(&mut order);
            let limit = capped.limit.min(out.len());
            let mut changed = 0;
            for k in order {
                if changed == limit {
                    break;
                }
                let (at, block, _) = out[k];
                if let Some(output) = apply_first(capped.rules, block, world(at.0, at.1, at.2), at)
                    && output != block
                {
                    out[k].1 = output;
                    if let Some(table) = capped.loot {
                        out[k].2 = Some(BlockEntity::Loot { table, seed: cell_seed([at.0, at.1, at.2]) });
                    }
                    changed += 1;
                }
            }
        }
        out
    }
}
