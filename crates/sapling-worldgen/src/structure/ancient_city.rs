//! Ancient cities (`createStructureStart` 1483117d0 -> 141edd690): a rotation and a city centre
//! from the start's generator, then the queued jigsaw placer (depth 7, within 128 of the centre)
//! from the `minecraft:city_anchor` jigsaw at the start column, anchored at y -28 (the piece
//! class's height, 141edf3a0, is -27).

use std::sync::Arc;

use super::canvas::Canvas;
use super::jigsaw::{AncientCityClass, Assembler, Piece, Variant, data};
use super::template::Rotation;
use super::{BoundingBox, StructureKind, Structures};
use crate::density::Density;

/// Cities stay within 128 blocks of their centre piece plus a piece.
const REACH: i32 = 10;
const HEIGHT: i32 = -27;

pub struct AncientCity {
    pub pieces: Vec<Piece>,
}

pub fn generate(random: &mut crate::random::Mt19937, start: (i32, i32)) -> AncientCity {
    let rotation = Rotation::from_index(random.next_int(4));
    let pool = data().pool("ancient_city/city_center").expect("ancient city centres");
    let element = pool.list[random.next_int(pool.list.len() as i32) as usize];
    let height = |_: i32, _: i32| HEIGHT;
    let mut assembler = Assembler::new(Variant::Queued, 7, random, &height);
    assembler.start_queued(element, (start.0 * 16, 0, start.1 * 16), rotation, "minecraft:city_anchor", 128);
    AncientCity { pieces: assembler.pieces }
}

impl Structures {
    /// Ancient cities with a piece meeting `chunk`.
    pub(super) fn ancient_cities_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<Arc<AncientCity>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        self.ancient_cities_reaching(density, chunk, &|bb| bb.intersects(&column))
    }

    /// Ancient cities around `chunk` with a piece that `near` accepts.
    pub(super) fn ancient_cities_reaching(&self, density: &Density, chunk: (i32, i32), near: &dyn Fn(&BoundingBox) -> bool) -> Vec<Arc<AncientCity>> {
        let starts = self.starts_near(density, StructureKind::AncientCity, chunk, REACH + 1);
        let grid = StructureKind::AncientCity.grid().unwrap();
        let mut cache = self.templates.ancient_cities.lock().unwrap();
        starts
            .into_iter()
            .map(|start| cache.entry(start).or_insert_with(|| Arc::new(generate(&mut grid.candidate_random(self.level_seed, start).1, start))).clone())
            .filter(|c| c.pieces.iter().any(|p| near(&p.bb)))
            .collect()
    }

    /// Every city piece meeting the canvas's chunk, in assembly order.
    pub(super) fn place_ancient_cities(canvas: &mut Canvas, cities: &[Arc<AncientCity>], seed: u32) {
        let column = canvas.bb;
        let mut random = crate::random::Mt19937::new(seed);
        for piece in cities.iter().flat_map(|c| c.pieces.iter()).filter(|p| p.bb.intersects(&column)) {
            piece.post_process(canvas, seed, &mut random, &AncientCityClass, &|_, _| 0);
        }
    }
}

