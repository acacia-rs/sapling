//! The adjustment entries reaching a chunk: every piece whose box comes within 12 blocks of it
//! (the legacy store files an entry under each chunk its box plus 12 meets, 144ccf020; the v2
//! collector tests the same margin), and v2 junctions within x and z of chunk start..start+27
//! (144cd4450 does not widen the lower bound).

use super::{Adaptation, Adjustments, Beard, Junction, Rigid};
use crate::density::Density;
use crate::structure::jigsaw::Piece;
use crate::structure::{BoundingBox, StructureKind, Structures};

const MARGIN: i32 = 12;
/// City pieces shave terrain up to their half size plus 1 below their ground and 5 above it.
const CITY_MARGINS: [f32; 2] = [1.0, 5.0];
/// Strongholds bury up to a chunk past their start's reach.
const STRONGHOLD_REACH: i32 = 10;

impl Structures {
    pub fn adjustments(&self, density: &Density, (cx, cz): (i32, i32)) -> Adjustments {
        let reach = BoundingBox::new(cx * 16 - MARGIN, i32::MIN, cz * 16 - MARGIN, cx * 16 + 15 + MARGIN, i32::MAX, cz * 16 + 15 + MARGIN);
        let near = |bb: &BoundingBox| bb.intersects(&reach);
        let mut adjust = Adjustments::default();
        for start in self.starts_near(density, StructureKind::Stronghold, (cx, cz), STRONGHOLD_REACH) {
            adjust.buries.extend(self.stronghold(start).pieces.iter().map(|p| p.frame.bb).filter(near));
        }
        for village in self.villages_reaching(density, (cx, cz), &near) {
            adjust.beards.extend(rigid(&village.pieces, &near).map(|p| Beard::new(p.bb, p.ground, [0.0; 2])));
        }
        for city in self.ancient_cities_reaching(density, (cx, cz), &near) {
            adjust.shavers.extend(rigid(&city.pieces, &near).map(|p| Beard::new(p.bb, p.ground, CITY_MARGINS)));
        }
        let (x0, z0) = (cx * 16, cz * 16);
        let close = |j: &&Junction| (x0..=x0 + 27).contains(&j.x) && (z0..=z0 + 27).contains(&j.z);
        for (kind, pieces) in self.v2_reaching(density, (cx, cz), &near) {
            let adaptation = match kind {
                StructureKind::TrailRuins => Adaptation::Bury,
                StructureKind::AbandonedCamp => Adaptation::BeardThin,
                _ => Adaptation::Encapsulate,
            };
            for piece in pieces.iter().filter(|p| near(&p.bb)) {
                adjust.junctions.extend(piece.junctions.iter().filter(close));
                if piece.is_rigid() {
                    adjust.rigids.push(Rigid { bb: piece.bb, adaptation, ground_delta: piece.ground });
                }
            }
        }
        if !adjust.is_empty() {
            adjust.finish((cx, cz));
        }
        adjust
    }
}

fn rigid<'a>(pieces: &'a [Piece], near: &'a dyn Fn(&BoundingBox) -> bool) -> impl Iterator<Item = &'a Piece> {
    pieces.iter().filter(move |p| p.is_rigid() && near(&p.bb))
}
