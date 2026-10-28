//! `SculkPatchFeature::place` (143ba12e0) and a sculk catalyst's spread share BDS's `SculkSpreader`:
//! its settings (config.rs), cursors (cursor.rs) and the sculk behaviours (behaviour.rs).

mod behaviour;
mod config;
mod cursor;

pub use config::{SculkWorld, Sound, Spreader};
pub use cursor::{Cursor, update_cursors};

use super::cave_spec::SculkPatch;
use super::place::Ctx;
use super::spreader::step;
use super::tree::spec::Range;
use crate::block::Block;

/// BDS caps a spreader at 32 cursors of at most 1000 charge each.
const MAX_CURSORS: usize = 32;
const MAX_CHARGE: i32 = 1000;

/// `SculkSpreader::addCursors` (inlined in 143ba12e0 and 144879f70): `charge` at `at` in parts of at
/// most 1000, each kept while there are fewer than 32 cursors.
pub fn add_cursors(cursors: &mut Vec<Cursor>, at: [i32; 3], mut charge: i32) {
    while charge > 0 {
        let part = charge.min(MAX_CHARGE);
        if cursors.len() < MAX_CURSORS {
            cursors.push(Cursor::new(at, part));
        }
        charge -= part;
    }
}

impl SculkPatch {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        if !self.can_start(ctx, at) {
            return None;
        }
        let mut cursors: Vec<Cursor> = Vec::new();
        for round in 0..self.growth_rounds + self.spread_rounds {
            for _ in 0..self.cursors {
                add_cursors(&mut cursors, at, self.charge);
            }
            for _ in 0..self.attempts {
                update_cursors(ctx.view, &Spreader::WORLDGEN, &mut cursors, at, ctx.random, round < self.spread_rounds);
            }
            cursors.clear();
        }
        let [x, y, z] = at;
        if ctx.random.next_float() <= self.catalyst_chance && ctx.view.block(x, y - 1, z).is_opaque_solid() {
            ctx.view.set_block(x, y, z, Block::SculkCatalyst);
        }
        let Range(lo, hi) = self.extra_growths;
        let count = if lo <= hi { lo + ctx.random.next_int(hi - lo + 1) } else { lo };
        for _ in 0..count {
            let dx = ctx.random.next_int(5) - 2;
            let dz = ctx.random.next_int(5) - 2;
            let p = [x + dx, y, z + dz];
            if ctx.view.block(p[0], p[1], p[2]) == Block::Air && ctx.view.block(p[0], p[1] - 1, p[2]).supports_multiface_plain(1) {
                ctx.view.set_block(p[0], p[1], p[2], Block::SculkShrieker);
            }
        }
        Some(at)
    }

    /// On sculk or a vein, or in air or a water source next to a block it may grow from.
    fn can_start(&self, ctx: &Ctx, at: [i32; 3]) -> bool {
        let block = ctx.view.block(at[0], at[1], at[2]);
        if block == Block::Sculk || behaviour::is_vein(block) {
            return true;
        }
        if block != Block::Air && block != Block::Water {
            return false;
        }
        [1, 0, 2, 3, 4, 5].into_iter().any(|face| {
            let [x, y, z] = step(at, face);
            let next = ctx.view.block(x, y, z);
            if self.can_place_on.is_empty() { next.is_opaque_solid() } else { self.can_place_on.contains(&next) }
        })
    }
}

/// `sculk_replaceable_world_gen` (built in 148b47d30); see `Spreader::replaceable`.
pub(super) fn replaceable(block: Block) -> bool {
    matches!(
        block.name(),
        "stone" | "granite" | "diorite" | "andesite" | "tuff" | "deepslate" | "grass_block" | "podzol" | "dirt"
            | "coarse_dirt" | "mycelium" | "dirt_with_roots" | "moss_block" | "pale_moss_block" | "hardened_clay"
            | "white_terracotta" | "orange_terracotta" | "magenta_terracotta" | "light_blue_terracotta"
            | "yellow_terracotta" | "lime_terracotta" | "pink_terracotta" | "gray_terracotta" | "light_gray_terracotta"
            | "cyan_terracotta" | "purple_terracotta" | "blue_terracotta" | "brown_terracotta" | "green_terracotta"
            | "red_terracotta" | "black_terracotta" | "crimson_nylium" | "warped_nylium" | "netherrack" | "basalt"
            | "blackstone" | "sand" | "red_sand" | "gravel" | "soul_sand" | "soul_soil" | "calcite" | "smooth_basalt"
            | "clay" | "dripstone_block" | "end_stone" | "sandstone" | "red_sandstone" | "mud" | "muddy_mangrove_roots"
            | "deepslate_bricks" | "deepslate_tiles" | "cobbled_deepslate" | "cracked_deepslate_bricks"
            | "cracked_deepslate_tiles" | "polished_deepslate" | "sulfur" | "cinnabar"
    )
}
