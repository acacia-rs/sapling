//! A placed pool element (`PoolElementStructurePiece`, postProcess 141ef21e0; element placement
//! 145e1b280): rigid pieces first pass their class's placement check, jigsaws turn into their
//! final states, the template goes down with the element's processors (drawing from a generator
//! seeded with the chunk's structure seed), then the element's foundation flags apply.

use super::super::BoundingBox;
use super::super::adjust::Junction;
use super::super::canvas::Canvas;
use super::super::trace;
use super::super::template::{JavaPlacement, Placement, Rotation, Rule, apply, named, pass_cells};
use super::assemble::middle;
use super::element::{Element, ElementKind, Projection};
use crate::block::Block;
use crate::feature::element_random;
use crate::random::Mt19937;

#[derive(Clone)]
pub struct Piece {
    pub element: &'static Element,
    pub origin: (i32, i32, i32),
    pub rotation: Rotation,
    pub bb: BoundingBox,
    pub depth: i32,
    /// The terrain adjustment's ground above `bb.y0`: hardcoded placers pass 1 for the start and
    /// the attaching jigsaw's template y for children; v2 pieces keep Java's ground level delta.
    pub ground: i32,
    /// v2 junctions (Java's `JigsawJunction`s), which beard the terrain around connections.
    pub junctions: Vec<Junction>,
}

/// What the piece class overrides (`PoolElementStructurePiece` vt+0x70, +0x78, +0x88).
pub trait PieceClass {
    /// Rigid pieces skip placement when this says no (village: not floating, 141f7abb0).
    fn can_place(&self, _canvas: &Canvas, _bb: &BoundingBox) -> bool {
        true
    }

    /// What fills under a bottom-layer block (flag 1), if anything.
    fn fill_below(&self, _block: Block, _pos: (i32, i32, i32)) -> Option<Block> {
        None
    }

    /// What plugs the first hole under a solid block below the piece (flag 2).
    fn plug(&self, block: Block) -> Block;
}

/// The light heightmap BDS's terrain matching reads (LevelChunk +0xf80, saved before structures).
pub type Heights<'a> = &'a dyn Fn(i32, i32) -> i32;

impl Piece {
    pub fn new(element: &'static Element, origin: (i32, i32, i32), rotation: Rotation, depth: i32) -> Self {
        Self { element, origin, rotation, bb: element.bbox(origin, rotation), depth, ground: 1, junctions: Vec::new() }
    }

    pub fn is_rigid(&self) -> bool {
        self.element.projection() == Projection::Rigid
    }

    pub fn shift_y(&mut self, dy: i32) {
        self.origin.1 += dy;
        self.bb.move_y(dy);
    }

    /// The piece within the canvas's chunk; `seed` is the chunk's structure seed and `random` the
    /// chunk's structure generator, which the bottom fill draws from.
    pub fn post_process(&self, canvas: &mut Canvas, seed: u32, random: &mut Mt19937, class: &dyn PieceClass, heights: Heights) {
        if let ElementKind::Feature(name) = &self.element.kind {
            self.place_feature(canvas, name, random, class);
            return;
        }
        let ElementKind::Template { template, rules, projection, post, .. } = &self.element.kind else { return };
        if *projection == Projection::TerrainMatching {
            self.place_matching(canvas, seed, heights);
            return;
        }
        if !class.can_place(canvas, &self.bb) {
            return;
        }
        trace::label(|| format!("{} jigsaw", template.name));
        canvas.begin_template();
        for jigsaw in self.element.jigsaws(self.origin, self.rotation) {
            let (x, y, z) = jigsaw.pos;
            if canvas.bb.contains(x, y, z) {
                canvas.set(x, y, z, jigsaw_block(rules, jigsaw.final_state, canvas.get(x, y, z), seed));
            }
        }
        let mut placement = Placement::new(self.rotation, canvas.bb);
        placement.seed = seed;
        placement.rules = rules;
        let blocks: Vec<_> = {
            let world = |x: i32, y: i32, z: i32| canvas.get(x, y, z);
            placement.blocks_in(template, self.origin, &world).collect()
        };
        for ((x, y, z), block, index) in blocks {
            placement.label(template, index);
            canvas.set(x, y, z, block);
            // A container in the chunk takes one draw of the chunk's generator (its loot seed; ancient city
            // one-worker run, generator index per piece: worldgen-bds-order.md, "replay5").
            if matches!(block.name(), "chest" | "trapped_chest" | "barrel") {
                random.next_u32();
            }
        }
        placement.update_pass(canvas, template, self.origin);
        trace::label(|| format!("{} foundation", template.name));
        if post & 1 != 0 {
            self.fill_bottom(canvas, random, class);
        }
        if post & 2 != 0 {
            self.plug_holes(canvas, class);
        }
    }

    /// A feature element (145e141d0): rigid, so the class's check first; the feature at the origin
    /// with the chunk's generator, then both foundation flags (the element's +0x121 is 3).
    fn place_feature(&self, canvas: &mut Canvas, name: &str, random: &mut Mt19937, class: &dyn PieceClass) {
        if !class.can_place(canvas, &self.bb) {
            return;
        }
        let (x, y, z) = self.origin;
        trace::label(|| format!("feature {name} {:?}", self.origin));
        canvas.place_feature(name, random, [x, y, z]);
        self.fill_bottom(canvas, random, class);
        self.plug_holes(canvas, class);
    }

    /// A v2 piece within the canvas's chunk (`template::JavaPlacement`); a feature element draws
    /// from its own generator (145e144c0).
    pub fn post_process_java(&self, canvas: &mut Canvas, level_seed: i64) {
        if let ElementKind::Feature(name) = &self.element.kind {
            let (x, y, z) = self.origin;
            trace::label(|| format!("feature {name} {:?}", self.origin));
            canvas.place_feature(name, &mut element_random([x, y, z]), [x, y, z]);
            return;
        }
        let ElementKind::Template { template, rules, java: Some(capped), .. } = &self.element.kind else { return };
        trace::label(|| format!("{} {:?} at {:?}", template.name, self.rotation, self.origin));
        let placement = JavaPlacement { rotation: self.rotation, rules, capped, level_seed };
        let blocks = placement.blocks(template, self.origin, &canvas.bb, &|x, y, z| canvas.get(x, y, z));
        canvas.begin_template();
        for ((x, y, z), block) in blocks {
            if canvas.bb.contains(x, y, z) {
                canvas.set(x, y, z, block);
            }
        }
        trace::label(|| format!("{} update pass", template.name));
        canvas.update_pass(pass_cells(self.rotation, false, template.size, self.origin), false);
    }

    /// Terrain matching (placeInWorld's +8 setting): every block at its column's height plus its
    /// template y, less one; jigsaws keep their final states the same way.
    fn place_matching(&self, canvas: &mut Canvas, seed: u32, heights: Heights) {
        let ElementKind::Template { template, rules, .. } = &self.element.kind else { return };
        let mut placement = Placement::new(self.rotation, BoundingBox::new(canvas.bb.x0, i32::MIN, canvas.bb.z0, canvas.bb.x1, i32::MAX, canvas.bb.z1));
        placement.seed = seed;
        placement.rules = rules;
        let base = self.origin.1;
        for jigsaw in self.element.jigsaws(self.origin, self.rotation) {
            let (x, y, z) = jigsaw.pos;
            if canvas.bb.contains(x, 0, z) {
                let at = heights(x, z) + y - base - 1;
                canvas.set(x, at, z, jigsaw_block(rules, jigsaw.final_state, canvas.get(x, at, z), seed));
            }
        }
        let blocks: Vec<_> = {
            let world = |x: i32, y: i32, z: i32| canvas.get(x, heights(x, z) + y - base - 1, z);
            placement.blocks_in(template, self.origin, &world).collect()
        };
        for ((x, y, z), block, index) in blocks {
            placement.label(template, index);
            canvas.set(x, heights(x, z) + y - base - 1, z, block);
        }
    }

    /// Flag 1: under each bottom-layer block, through air and fluids, two to four of the class's
    /// fill block (`nextInt(3)` per filled column, drawn over the whole piece).
    fn fill_bottom(&self, canvas: &mut Canvas, random: &mut Mt19937, class: &dyn PieceClass) {
        let (b, y0) = (self.bb, self.bb.y0);
        for x in b.x0..=b.x1 {
            for z in b.z0..=b.z1 {
                let top = canvas.get(x, y0, z);
                if top == Block::Air {
                    continue;
                }
                let Some(fill) = class.fill_below(top, (x, y0, z)) else { continue };
                let depth = random.next_int(3);
                let mut y = y0 - 1;
                while y > y0 - depth - 3 {
                    let below = canvas.get(x, y, z);
                    if below != Block::Air && !below.is_fluid() || !canvas.bb.contains(x, y, z) {
                        break;
                    }
                    canvas.set(x, y, z, fill);
                    y -= 1;
                }
            }
        }
    }

    /// Flag 2: within 12 of the piece's middle, under each falling block (sand, gravel) just below
    /// the piece, the first block that does not fall, if air or water, becomes the class's plug.
    fn plug_holes(&self, canvas: &mut Canvas, class: &dyn PieceClass) {
        let (mx, mz, y) = (middle(self.bb.x0, self.bb.x1), middle(self.bb.z0, self.bb.z1), self.bb.y0 - 1);
        for x in mx - 12..=mx + 12 {
            for z in mz - 12..=mz + 12 {
                let top = canvas.get(x, y, z);
                if !top.falls() {
                    continue;
                }
                let plug = class.plug(top);
                let mut below = y - 1;
                while below > -64 {
                    let block = canvas.get(x, below, z);
                    if !block.falls() {
                        if block == Block::Air || block.name() == "water" {
                            canvas.set(x, below, z, plug);
                        }
                        break;
                    }
                    below -= 1;
                }
            }
        }
    }
}

/// A jigsaw's final state through the element's rules, each jigsaw on a generator of its own
/// seeded like the template's: the jigsaws of one chunk all draw the same floats
/// (docs/research/worldgen-bds-jigsaw.md, "Elements and processors").
fn jigsaw_block(rules: &[Rule], final_state: Block, world: Block, seed: u32) -> Block {
    apply(rules, final_state, world, &mut Mt19937::new(seed))
}

/// Ancient city pieces (141edf3a0 / 3b0): no fill, deepslate plugs.
pub struct AncientCityClass;

impl PieceClass for AncientCityClass {
    fn plug(&self, _block: Block) -> Block {
        Block::Deepslate
    }
}

/// Village pieces (141f7a9c0 / ab60 / abb0).
pub struct VillageClass<'a> {
    /// The biome's top material at a block.
    pub top_material: &'a dyn Fn(i32, i32, i32) -> Block,
}

impl PieceClass for VillageClass<'_> {
    /// Not when the middle two below and the four bottom corners one below are all air.
    fn can_place(&self, canvas: &Canvas, b: &BoundingBox) -> bool {
        if b.x0 >= b.x1 {
            return true;
        }
        let y = b.y0 - 1;
        let points = [(middle(b.x0, b.x1), y - 1, middle(b.z0, b.z1)), (b.x0, y, b.z0), (b.x0, y, b.z1), (b.x1, y, b.z1), (b.x1, y, b.z0)];
        !points.iter().all(|&(x, y, z)| canvas.get(x, y, z) == Block::Air)
    }

    /// Leaves get nothing; otherwise the biome's top material, or dirt where that is grass.
    fn fill_below(&self, block: Block, (x, y, z): (i32, i32, i32)) -> Option<Block> {
        if block.name().ends_with("leaves") {
            return None;
        }
        let top = (self.top_material)(x, y, z);
        Some(if top == Block::GrassBlock { Block::Dirt } else { top })
    }

    fn plug(&self, block: Block) -> Block {
        match block {
            Block::RedSand => named("red_sandstone", "", false),
            Block::Sand => Block::Sandstone,
            _ => Block::Stone,
        }
    }
}
