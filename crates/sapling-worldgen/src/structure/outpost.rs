//! Pillager outposts (`createStructureStart` 148357080, piece 1483587c0, postProcess 148358d80):
//! a watchtower, its overgrowth at 5% integrity, and up to four feature templates around it,
//! each a template piece that finds its own height when first post-processed.

use super::BoundingBox;
use super::canvas::Canvas;
use super::template::{FULL_CUBE, Placement, Rotation, Template, flags, named, piece_box};
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::random::Mt19937;

const MIN_Y: i32 = -64;
/// Feature spots around the tower's middle, each moved by up to 4 (14a927870).
const SPOTS: [(i32, i32); 4] = [(16, 16), (-16, 16), (16, -16), (-16, -16)];
const FEATURES: [&str; 7] = [
    "feature_cage1",
    "feature_cage2",
    "feature_cage_with_allays",
    "feature_logs",
    "feature_targets",
    "feature_tent1",
    "feature_tent2",
];

/// The markers' mobs in spawning order: the cage's, the pillagers', the captains', the allays'.
const MOBS: [(&str, &str); 4] = [
    ("minecraft:iron_golem", "minecraft:entity_spawned"),
    ("minecraft:pillager", "minecraft:entity_spawned"),
    ("minecraft:pillager", "minecraft:spawn_as_illager_captain"),
    ("minecraft:allay", "minecraft:entity_spawned"),
];

pub struct Piece {
    template: &'static Template,
    origin: (i32, i32),
    rotation: Rotation,
    integrity: f32,
    /// Feature pieces stand at their middle's height and only on even ground.
    feature: bool,
    pub bb: BoundingBox,
}

pub struct Outpost {
    pub pieces: Vec<Piece>,
}

impl Outpost {
    /// The start's box as its spawn areas give it: the pieces' columns, from 63 up (read off BDS saves).
    pub fn whole_box(&self) -> BoundingBox {
        let union = self.pieces.iter().map(|p| p.bb).reduce(|a, b| a.union(&b)).expect("an outpost has its tower");
        BoundingBox { y0: 63, ..union }
    }
}

impl Piece {
    fn new(name: &str, origin: (i32, i32), rotation: Rotation, integrity: f32, feature: bool) -> Self {
        let template = Template::get(&format!("pillageroutpost/{name}"));
        let bb = piece_box(origin, rotation, template.size);
        Self { template, origin, rotation, integrity, feature, bb }
    }

    /// The piece's y from the light heightmap at its middle and four edge points; `None` where a
    /// feature's ground is too uneven or has a liquid top (148358d80).
    pub fn height(&self, canvas: &Canvas) -> Option<i32> {
        let b = &self.bb;
        let (hx, hz) = ((b.x1 - b.x0 + 1) >> 1, (b.z1 - b.z0 + 1) >> 1);
        let (mx, mz) = (b.x0 + hx, b.z0 + hz);
        let points = [(mx, mz), (mx + hx, mz), (mx, mz + hz), (b.x0, mz), (mx, b.z0)];
        let heights = points.map(|(x, z)| canvas.height(x, z));
        let high = *heights.iter().max().unwrap();
        let low = (*heights.iter().min().unwrap()).max(MIN_Y);
        let wet = || points.iter().zip(heights).any(|(&(x, z), h)| canvas.get(x, h - 1, z).is_fluid());
        match (self.feature, high - low < 9) {
            (true, true) if !wet() => Some(heights[0]),
            (true, _) => None,
            (false, true) => Some((low + high) / 2),
            (false, false) => Some(high - 4),
        }
    }

    /// Its box once placed at `y`.
    pub fn placed_box(&self, y: i32) -> BoundingBox {
        BoundingBox { y0: y, y1: y + self.template.size.1, ..self.bb }
    }

    /// Places the piece at `y` within the canvas's chunk; `seed` is the chunk's structure seed and
    /// `random` its structure generator, which the template's chests and then the markers draw from
    /// (handler 148359310): `topChest` re-seeds the chest below it, a group of allays draws its
    /// size. The markers' mobs follow (`postProcessMobsAt` 1483599e0): golems, pillagers, captains,
    /// allays, each at its marker's corner, persistent.
    /// No block is ignored (settings +0x10 is null): air is placed and takes an integrity draw.
    pub fn post_process(&self, canvas: &mut Canvas, y: i32, seed: u32, random: &mut Mt19937) {
        let mut placement = Placement::new(self.rotation, canvas.bb);
        placement.integrity = self.integrity;
        placement.seed = seed;
        let origin = (self.origin.0, y, self.origin.1);
        placement.place(canvas, self.template, origin).draw_seeds(canvas, random);
        let mut mobs: [Vec<(i32, i32, i32)>; 4] = Default::default();
        // File order, not the map's (markers_at): the tower's nine markers rehash it, and only the spawn order would differ.
        for (index, marker) in &self.template.markers {
            let (dx, dy, dz) = self.rotation.apply(self.template.blocks[*index].pos);
            let (x, y, z) = (origin.0 + dx, origin.1 + dy, origin.2 + dz);
            if !canvas.bb.contains(x, y, z) {
                continue;
            }
            let list = match marker.as_str() {
                "cage" => 0,
                "pillager" => 1,
                "captain" => 2,
                "Group of Allays" => 3,
                "topChest" if canvas.get(x, y - 1, z).name() == "chest" => {
                    canvas.set_entity(x, y - 1, z, BlockEntity::Loot { table: "loot_tables/chests/pillager_outpost.json", seed: random.next_int_unbounded() });
                    continue;
                }
                _ => continue,
            };
            canvas.set(x, y, z, Block::Air);
            // The cage's golem stands a cell east in a piece turned once (the list's x + 1).
            let cell = (x + (list == 0 && self.rotation == Rotation::Clockwise90) as i32, y, z);
            let count = if list == 3 { random.next_int(3) as usize + 1 } else { 1 };
            mobs[list].extend(std::iter::repeat_n(cell, count));
        }
        for (cells, (kind, event)) in mobs.into_iter().zip(MOBS) {
            cells.into_iter().for_each(|cell| canvas.add_actor(kind, event, cell));
        }
    }
}

impl Piece {
    /// The start's foundation pass (148358490): under each block of the piece's bottom layer, down
    /// through air and fluids, the block itself if a full cube, else cobblestone.
    pub fn fill_below(&self, canvas: &mut Canvas, y: i32) {
        let (a, c) = (self.bb, canvas.bb);
        super::trace::label(|| format!("{} foundation", self.template.name));
        for x in a.x0.max(c.x0)..=a.x1.min(c.x1) {
            for z in a.z0.max(c.z0)..=a.z1.min(c.z1) {
                let top = canvas.get(x, y, z);
                if top == Block::Air {
                    continue;
                }
                let fill = if flags(top) & FULL_CUBE != 0 { top } else { named("cobblestone", "", false) };
                let mut below = y - 1;
                while below >= 2 && matches!(canvas.get(x, below, z), Block::Air | Block::Water | Block::Lava) {
                    canvas.set(x, below, z, fill);
                    below -= 1;
                }
            }
        }
    }
}

/// The outpost of start chunk `start`, drawn from the start's generator.
pub fn generate(random: &mut Mt19937, start: (i32, i32)) -> Outpost {
    let rotation_index = random.next_int(4);
    let rotation = Rotation::from_index(rotation_index);
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let mut pieces = vec![
        Piece::new("watchtower", (x0, z0), rotation, 1.0, false),
        Piece::new("watchtower_overgrown", (x0, z0), rotation, 0.05, false),
    ];
    let (dx, dz) = match rotation_index {
        1 => (-15, 15),
        2 => (-15, -15),
        3 => (15, -15),
        _ => (15, 15),
    };
    let (cx, cz) = (x0.min(x0 + dx) + 8, z0.min(z0 + dz) + 8);
    let spots: Vec<(i32, i32)> = SPOTS
        .iter()
        .map(|&(sx, sz)| {
            let x = random.next_int(8) + sx - 4;
            let z = random.next_int(8) + sz - 4;
            (x, z)
        })
        .collect();
    for (sx, sz) in spots {
        if random.next_float() >= 0.9 {
            continue;
        }
        let rotation = Rotation::from_index(random.next_int(4));
        let name = FEATURES[random.next_int(7) as usize];
        pieces.push(Piece::new(name, (cx + sx, cz + sz), rotation, 1.0, true));
    }
    Outpost { pieces }
}
