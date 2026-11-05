//! The sulfur caves' springs: `sulfur_spring_trail_to_surface_feature` (Windows 149d660d0, Linux 0x49c5ee0) climbs
//! from a cave ceiling to the surface, places `random_sulfur_spring_with_tuff_feature` there and trails sulfur
//! down to where it started; the springs are `minecraft:structure_template_feature`s (`StructureTemplateFeature::place`
//! 143bbc130) of the sulfur_spring_* templates (docs/research/worldgen-bds-caves.md, "Sulfur springs").

use super::data::FEATURES;
use super::data::sulfur_springs::{KINDS, SpringTemplate, TEMPLATES};
use super::place::Ctx;
use super::view::View;
use crate::block::Block;

/// The trail's block list (its +0x48 on Linux): the types the sulfur column replaces.
const TRAIL_REPLACES: [&str; 32] = [
    "powder_snow", "snow", "sand", "gravel", "clay", "red_sand", "dirt", "mud", "moss_block", "grass_block", "stone", "granite",
    "diorite", "andesite", "tuff", "deepslate", "white_terracotta", "orange_terracotta", "magenta_terracotta",
    "light_blue_terracotta", "yellow_terracotta", "lime_terracotta", "pink_terracotta", "gray_terracotta",
    "light_gray_terracotta", "cyan_terracotta", "purple_terracotta", "blue_terracotta", "brown_terracotta",
    "green_terracotta", "red_terracotta", "black_terracotta",
];

/// `Material::isSolid` (its byte +5), as far as sapling's blocks go.
fn material_solid(block: Block) -> bool {
    block.is_solid() && !block.is_snow_layer()
}

/// Up the column from `at` to the heightmap (184 cells at most) to the first air cell on a solid block with
/// air or water in the two above, where the spring goes; once one places, sulfur along the way.
pub fn trail(ctx: &mut Ctx, [x, start, z]: [i32; 3], spring: usize) -> bool {
    let top = ctx.view.heightmap(x, z);
    let mut y = start;
    for _ in 0..184 {
        // BDS's cell test takes air or water, but its predicate (canBeBuiltOver, not liquid) already ruled water
        // out; the sink below a water cell it then tries can therefore never happen.
        let view = &*ctx.view;
        let open = |dy: i32| matches!(view.block(x, y + dy, z), b if b == Block::Air || b.is_water());
        let fits = material_solid(view.block(x, y - 1, z)) && view.block(x, y, z) == Block::Air && open(1) && open(2);
        if fits && FEATURES[spring].place(ctx, [x, y, z]).is_some() {
            sulfur(ctx, [x, start, z], y);
            return true;
        }
        if y >= top {
            break;
        }
        y += 1;
    }
    false
}

/// 20 tries per layer above `start` up to the spring, ±2 around the column, then a 20-step walk from `start`.
fn sulfur(ctx: &mut Ctx, [x, start, z]: [i32; 3], spring_y: i32) {
    for y in start + 1..=spring_y {
        for _ in 0..20 {
            let dx = ctx.random.next_int(3) - ctx.random.next_int(3);
            let dz = ctx.random.next_int(3) - ctx.random.next_int(3);
            if TRAIL_REPLACES.contains(&ctx.view.block(x + dx, y, z + dz).name()) {
                ctx.view.set_block(x + dx, y, z + dz, Block::Sulfur);
            }
        }
    }
    let mut at = [x, start, z];
    for _ in 0..20 {
        let dx = ctx.random.next_int(3) - ctx.random.next_int(3);
        let dz = ctx.random.next_int(3) - ctx.random.next_int(3);
        let dy = ctx.random.next_int(2) - ctx.random.next_int(2);
        at = [at[0] + dx, at[1] + dy, at[2] + dz];
        if ctx.view.block(at[0], at[1], at[2]) == Block::Air && material_solid(ctx.view.block(at[0], at[1] + 1, at[2])) {
            ctx.view.set_block(at[0], at[1], at[2], Block::Sulfur);
        }
    }
}

/// `minecraft:structure_template_feature` with `adjustment_radius` 0 and a random rotation.
#[derive(Debug)]
pub struct TemplateFeature {
    /// Index into `TEMPLATES`.
    pub template: usize,
    pub ground_level: i32,
    pub rotate_around_center: bool,
    pub leveled: bool,
    pub unburied: bool,
    /// `block_intersection`'s allowlist (every non-void cell's; motion-blocking-only is not ported).
    pub intersection: Option<&'static [Block]>,
}

/// Bedrock's `Rotation` 0..3 on a horizontal offset.
fn turn([x, z]: [i32; 2], rotation: i32) -> [i32; 2] {
    match rotation {
        1 => [-z, x],
        2 => [-x, -z],
        3 => [z, -x],
        _ => [x, z],
    }
}

impl TemplateFeature {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let template = &TEMPLATES[self.template].1;
        let rotation = ctx.random.next_int(4);
        let [sx, sy, sz] = template.size;
        let half = |size: i32| -((size as f32 * 0.5).floor() as i32);
        let offset = if self.rotate_around_center { [half(sx), half(sz)] } else { [0, 0] };
        let ground = self.ground_level.clamp(0, sy.max(1) - 1);
        // The constraints see the template unlowered, one cell list each (their constructors d281340, d281a60, d280350).
        let cell = |(cx, cy, cz): (i32, i32, i32)| {
            let [dx, dz] = turn([cx + offset[0], cz + offset[1]], rotation);
            [at[0] + dx, at[1] + cy, at[2] + dz]
        };
        let cells = || template.cells.iter().map(|&(x, y, z, kind)| ((x as i32, y as i32, z as i32), KINDS[kind as usize]));
        let view = &*ctx.view;
        let world = |p: (i32, i32, i32)| {
            let [x, y, z] = cell(p);
            view.block(x, y, z)
        };
        let leveled = || {
            cells().filter(|&((_, y, _), block)| y == ground && block != Block::Air).all(|((x, _, z), _)| ground_near(view, cell((x, -1, z))))
        };
        let unburied = || cells().filter(|&((_, y, _), block)| y == sy - 1 && block != Block::Air).all(|((x, _, z), _)| world((x, sy, z)) == Block::Air);
        let intersects = |allow: &[Block]| cells().all(|(p, _)| allow.contains(&world(p)));
        if !(!self.leveled || leveled()) || !(!self.unburied || unburied()) || !self.intersection.is_none_or(intersects) {
            return None;
        }
        let [ox, oz] = turn(offset, rotation);
        let origin = [at[0] + ox, at[1] - ground, at[2] + oz];
        place_cells(ctx.view, template, origin, rotation);
        Some(at)
    }
}

/// `LeveledConstraint` (Linux d2818d0): a solid cell with a non-solid one above it, at most two from `at` either way.
fn ground_near(view: &View, [x, y, z]: [i32; 3]) -> bool {
    let solid = |y: i32| {
        let block = view.block(x, y, z);
        block.is_solid_blocking() && block.material_blocks_motion()
    };
    (y - 2..=y + 2).any(|y| solid(y) && !solid(y + 1))
}

/// `StructureTemplate::placeInWorld` (141f52300) with the feature's settings: every non-void cell, air too, then
/// each `tick_queue_data` tick on its cell's chunk, kept for the game.
fn place_cells(view: &mut View, template: &SpringTemplate, [x, y, z]: [i32; 3], rotation: i32) {
    let at = |cx: u8, cy: u8, cz: u8| {
        let [dx, dz] = turn([cx as i32, cz as i32], rotation);
        [x + dx, y + cy as i32, z + dz]
    };
    for &(cx, cy, cz, kind) in template.cells {
        let [x, y, z] = at(cx, cy, cz);
        view.set_block(x, y, z, KINDS[kind as usize]);
    }
    for &(cx, cy, cz, delays) in template.ticks {
        let cell = at(cx, cy, cz);
        let block = view.block(cell[0], cell[1], cell[2]);
        delays.iter().for_each(|&delay| view.keep_tick(cell, block, delay.into()));
    }
}
