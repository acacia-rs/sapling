//! Ocean ruins (`OceanRuinFeature::createStructureStart` 148351e10, `addPiece` 148353240, piece
//! 148354170 / postProcess 148354700 / data markers 148356170): Java 1.13's ruins with BDS's
//! own start offsets, cluster lists and chests. Pieces are template pieces: each chunk their box
//! meets places its part, with that chunk's generator seeding the integrity draws.

use super::BoundingBox;
use super::canvas::Canvas;
use super::template::{Placement, Rotation, Template, named};
use crate::block::Block;
use crate::random::{Mt19937, Xoroshiro, position_seed};

/// Warm and lukewarm oceans (deep ones too) build sandstone ruins; the rest stone brick ones.
pub const WARM_OCEANS: &[u32] = &[40, 41, 42, 43];
const LARGE_PROBABILITY: f32 = 0.3;
const CLUSTER_PROBABILITY: f32 = 0.9;

const WARM: [&str; 8] = ["ruin_warm1", "ruin_warm2", "ruin_warm3", "ruin_warm4", "ruin_warm5", "ruin_warm6", "ruin_warm7", "ruin_warm8"];
const BIG_WARM: [&str; 4] = ["big_ruin_warm4", "big_ruin_warm5", "big_ruin_warm6", "big_ruin_warm7"];
const COLD: [[&str; 3]; 9] = [
    ["ruin4", "ruin4", "ruin4"],
    ["ruin1_brick", "ruin1_cracked", "ruin1_mossy"],
    ["ruin2_brick", "ruin2_cracked", "ruin2_mossy"],
    ["ruin3_brick", "ruin3_cracked", "ruin3_mossy"],
    ["ruin4_brick", "ruin4_cracked", "ruin4_mossy"],
    ["ruin5_brick", "ruin5_cracked", "ruin5_mossy"],
    ["ruin6_brick", "ruin6_cracked", "ruin6_mossy"],
    ["ruin7_brick", "ruin7_cracked", "ruin7_mossy"],
    ["ruin8_brick", "ruin8_cracked", "ruin8_mossy"],
];
const BIG_COLD: [[&str; 3]; 4] = [
    ["big_ruin1_brick", "big_ruin1_cracked", "big_ruin1_mossy"],
    ["big_ruin2_brick", "big_ruin2_cracked", "big_ruin2_mossy"],
    ["big_ruin3_brick", "big_ruin3_cracked", "big_ruin3_mossy"],
    ["big_ruin8_brick", "big_ruin8_cracked", "big_ruin8_mossy"],
];

#[derive(Debug)]
pub struct RuinPiece {
    template: &'static Template,
    /// x and z of the template origin; y is found in each chunk's post-process.
    origin: (i32, i32),
    rotation: Rotation,
    integrity: f32,
    warm: bool,
    pub bb: BoundingBox,
}

#[derive(Debug)]
pub struct OceanRuin {
    pub pieces: Vec<RuinPiece>,
}

/// The ruins starting in chunk `start`, drawn from the start's blueprint generator as the start
/// check leaves it (`blueprint_random`).
pub fn generate(mut random: Mt19937, start: (i32, i32), warm: bool) -> OceanRuin {
    let rotation = Rotation::from_index(random.next_int(4));
    // Each rotation's origin keeps the 16x16 footprint inside the start chunk.
    let (dx, dz) = [(0, 0), (15, 0), (15, 15), (0, 15)][rotation as usize];
    let origin = (start.0 * 16 + dx, start.1 * 16 + dz);
    let mut ruin = OceanRuin { pieces: Vec::new() };
    let large = random.next_float() <= LARGE_PROBABILITY;
    ruin.add(&mut random, origin, rotation, large, if large { 0.9 } else { 0.8 }, warm);
    if large && random.next_float() <= CLUSTER_PROBABILITY {
        ruin.add_cluster(&mut random, origin, rotation, warm);
    }
    ruin
}

impl OceanRuin {
    fn add(&mut self, random: &mut Mt19937, origin: (i32, i32), rotation: Rotation, large: bool, integrity: f32, warm: bool) {
        let mut push = |name: &str, integrity: f32| {
            let template = Template::get(&format!("ruin/{name}"));
            // Not template::piece_box: the size + 1 box moved near origin's ruins (+171 wrong cells).
            let bb = footprint(origin, rotation, (template.size.0 - 1, template.size.2 - 1));
            self.pieces.push(RuinPiece { template, origin, rotation, integrity, warm, bb });
        };
        if warm {
            push(if large { BIG_WARM[random.next_int(4) as usize] } else { WARM[random.next_int(8) as usize] }, integrity);
            return;
        }
        let [brick, cracked, mossy] = if large { BIG_COLD[random.next_int(4) as usize] } else { COLD[random.next_int(9) as usize] };
        push(brick, integrity);
        push(cracked, 0.7);
        push(mossy, 0.5);
    }

    /// BDS's take on Java's `addClusterRuins`: up to eight spots around the big ruin, 4 to 7 of
    /// them tried, each kept when its 6x7 footprint misses the big ruin's 16x16.
    fn add_cluster(&mut self, random: &mut Mt19937, origin: (i32, i32), rotation: Rotation, warm: bool) {
        let big = footprint(origin, rotation, (15, 15));
        let (x, z) = (big.x0, big.z0);
        let mut spot = |dx: i32, nx: i32, dz: i32, nz: i32| {
            let sx = x + dx + random.next_int(nx);
            (sx, z + dz + random.next_int(nz))
        };
        let mut spots = vec![
            spot(-15, 7, 17, 6),
            spot(-15, 7, 1, 6),
            spot(-15, 7, -12, 4),
            spot(1, 6, 17, 6),
            spot(1, 6, -12, 2),
            spot(17, 6, 19, 5),
            spot(17, 6, 1, 6),
            spot(17, 6, -12, 4),
        ];
        let count = random.next_int(4) + 4;
        for _ in 0..count {
            let at = spots.remove(random.next_int(spots.len() as i32) as usize);
            let rotation = Rotation::from_index(random.next_int(4));
            let small = footprint(at, rotation, (5, 6));
            if !(small.x1 >= big.x0 && small.x0 <= big.x1 && small.z1 >= big.z0 && small.z0 <= big.z1) {
                self.add(random, at, rotation, false, 0.8, warm);
            }
        }
    }
}

/// The archaeology processor (a capped rule built in 148354700, finished by 145e12a20 from
/// 141eedbb0): the piece's placed sand or gravel cells in this chunk, shuffled by a Xoroshiro
/// seeded from the template origin, and the first five still sand or gravel made suspicious.
fn bury_loot(canvas: &mut Canvas, origin: (i32, i32, i32), mut cells: Vec<(i32, i32, i32)>) {
    // BDS keeps only the low 32 bits of Java's position seed, sign-extended (141eedbb0).
    let seed = (position_seed(origin.0, origin.1, origin.2) as i32) as i64;
    let mut random = Xoroshiro::from_seed(seed);
    for i in 1..cells.len() {
        let j = random.next_int_bounded(i as u32 + 1) as usize;
        cells.swap(i, j);
    }
    let mut buried = 0;
    for (x, y, z) in cells {
        let suspicious = match canvas.get(x, y, z) {
            Block::Gravel => named("suspicious_gravel", "brushed_progress=0,hanging=1", false),
            Block::Sand => named("suspicious_sand", "brushed_progress=0,hanging=1", false),
            _ => continue,
        };
        canvas.set(x, y, z, suspicious);
        buried += 1;
        if buried == 5 {
            break;
        }
    }
}

/// The x/z box from `origin` to its corner `(cx, cz)` turned by `rotation`, all heights.
fn footprint(origin: (i32, i32), rotation: Rotation, (cx, cz): (i32, i32)) -> BoundingBox {
    let (rx, _, rz) = rotation.apply((cx, 0, cz));
    let (x1, z1) = (origin.0 + rx, origin.1 + rz);
    BoundingBox::new(origin.0.min(x1), -64, origin.1.min(z1), origin.0.max(x1), 319, origin.1.max(z1))
}

impl RuinPiece {
    /// The piece's part in `canvas`'s chunk; `seed` is that chunk's structure seed.
    pub fn post_process(&self, canvas: &mut Canvas, seed: u32) {
        let y = self.floor_y(canvas);
        let origin = (self.origin.0, y, self.origin.1);
        let mut placement = Placement::new(self.rotation, canvas.bb);
        placement.integrity = self.integrity.clamp(0.0, 1.0);
        placement.seed = seed;
        placement.flood = true;
        placement.ignore = Some("air");
        let floor = if self.warm { Block::Sand } else { Block::Gravel };
        let mut buried = Vec::new();
        for ((x, y, z), block, _) in placement.blocks(self.template, origin) {
            canvas.set(x, y, z, block);
            if block == floor {
                buried.push((x, y, z));
            }
        }
        bury_loot(canvas, origin, buried);
        for (index, _) in self.template.markers.iter().filter(|(_, m)| m == "chest") {
            let (dx, dy, dz) = self.rotation.apply(self.template.blocks[*index].pos);
            let (x, y, z) = (origin.0 + dx, origin.1 + dy, origin.2 + dz);
            if canvas.bb.contains(x, y, z) {
                canvas.set(x, y, z, self.marker_filler());
                canvas.set(x, y - 1, z, named("chest", "minecraft:cardinal_direction=east", y <= 63));
            }
        }
    }

    /// What a chest marker becomes: the ruin's floor material by template.
    fn marker_filler(&self) -> Block {
        match self.template.name.as_str() {
            _ if self.warm => Block::Sandstone,
            "ruin/ruin3_brick" => named("stone_bricks", "", false),
            "ruin/ruin2_brick" | "ruin/big_ruin1_brick" => Block::Gravel,
            _ => Block::Water,
        }
    }

    /// The template's y (148354700): from the heightmap at its origin, down to the lowest floor
    /// under its footprint when most columns there sink more than three blocks lower.
    fn floor_y(&self, canvas: &Canvas) -> i32 {
        let height = canvas.height(self.origin.0, self.origin.1);
        let (mut lowest, mut sunk) = (512, 0);
        for x in self.bb.x0..=self.bb.x1 {
            for z in self.bb.z0..=self.bb.z1 {
                let mut y = height - 1;
                while matches!(canvas.get(x, y, z), Block::Air | Block::Water) && y > -63 {
                    y -= 1;
                }
                lowest = lowest.min(y);
                sunk += i32::from(y < height - 3);
            }
        }
        let width = self.bb.x1 - self.bb.x0;
        if height - lowest > 3 && sunk > width - 2 { lowest } else { height - 1 }
    }
}
