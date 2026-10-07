//! Shipwrecks (`ShipwreckFeature::createStructureStart` 1483719b0, piece postProcess 148372130):
//! BDS's own piece. Only the chunk at the piece's corner post-processes it, placing the whole
//! template through the 3x3 block source; rotation, template, woods and chests come from that
//! chunk's structure generator.

use super::BoundingBox;
use super::canvas::Canvas;
use super::template::{Placement, Rotation, Template};
use crate::block::Block;
use crate::random::Mt19937;

const SEA_LEVEL: i32 = 63;
/// Beach, snowy beach, mushroom shore: shipwrecks there are beached (`0x4018000 >> biome`).
pub const BEACHES: &[u32] = &[15, 16, 26];

/// The first 14 can lie beached.
const TEMPLATES: [&str; 20] = [
    "swwithmast",
    "swwithmastdegraded",
    "swsidewaysfull",
    "swsidewaysfronthalf",
    "swsidewaysbackhalf",
    "swsidewaysfulldegraded",
    "swsidewaysfronthalfdegraded",
    "swsidewaysbackhalfdegraded",
    "swrightsideupfull",
    "swrightsideupfronthalf",
    "swrightsideupbackhalf",
    "swrightsideupfulldegraded",
    "swrightsideupfronthalfdegraded",
    "swrightsideupbackhalfdegraded",
    "swupsidedownfull",
    "swupsidedownfronthalf",
    "swupsidedownbackhalf",
    "swupsidedownfulldegraded",
    "swupsidedownfronthalfdegraded",
    "swupsidedownbackhalfdegraded",
];

/// The woods replacing oak and dark oak (14a927de0).
const WOODS: [(u8, u8); 9] = [(0, 1), (3, 1), (5, 3), (5, 1), (1, 0), (1, 3), (1, 5), (0, 2), (1, 4)];

/// The shipwreck of start chunk `start`, written where it reaches `canvas`'s chunk; `random` is
/// the start chunk's structure generator, freshly seeded for the shipwreck feature, and
/// `beached` whether the biome at the piece's corner is a beach.
pub fn place(canvas: &mut Canvas, random: &mut Mt19937, start: (i32, i32), beached: bool) {
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let rotation = Rotation::from_index(random.next_int(4));
    let mut index = random.next_int(20);
    if beached {
        index = random.next_int(14);
    }
    let template = Template::get(&format!("shipwreck/{}", TEMPLATES[index as usize]));
    let (oak, dark_oak) = WOODS[random.next_int(9) as usize];
    // The rotated template keeps to the chunk's 16x16 corner as ocean ruins do.
    let (dx, dz) = [(0, 0), (15, 0), (15, 15), (0, 15)][rotation as usize];
    let corner = (x0 + dx, z0 + dz);
    let mut y = average_floor(canvas, template, corner, rotation) + if index < 14 { -2 } else { -4 };
    if beached && y < SEA_LEVEL {
        y = SEA_LEVEL;
    }
    let origin = (corner.0, y, corner.1);
    let mut placement = Placement::new(rotation, BoundingBox::new(i32::MIN, -64, i32::MIN, i32::MAX, 319, i32::MAX));
    placement.flood = !beached;
    placement.wood = Some([Some(oak), None, None, None, None, Some(dark_oak)]);
    let chests = placement.place(canvas, template, origin);
    random.consume_count(chests as u32);
    let water_or_air = |y: i32| if y < SEA_LEVEL { Block::Water } else { Block::Air };
    for ((x, y, z), marker) in template.markers_at(origin, rotation) {
        canvas.set(x, y, z, water_or_air(y));
        // Half the treasure and supply chests are taken out again; a kept chest draws its loot seed.
        let removed = matches!(marker, "treasureChest" | "supplyChest") && random.next_int(10) < 5;
        if removed {
            canvas.set(x, y - 1, z, if SEA_LEVEL < y { Block::Air } else { Block::Water });
        } else if matches!(marker, "treasureChest" | "supplyChest" | "mapChest") {
            random.next_u32();
        }
    }
}

/// The mean floor height over the middle half of the turned template's footprint.
fn average_floor(canvas: &Canvas, template: &Template, (cx, cz): (i32, i32), rotation: Rotation) -> i32 {
    let (sx, sz) = (template.size.0, template.size.2);
    let (ax, _, az) = rotation.apply((sx / 4, 0, sz / 4));
    let (bx, _, bz) = rotation.apply((sx * 3 / 4, 0, sz * 3 / 4));
    let (mut sum, mut count) = (0, 0);
    for x in cx + ax.min(bx)..=cx + ax.max(bx) {
        for z in cz + az.min(bz)..=cz + az.max(bz) {
            sum += canvas.floor_height(x, z);
            count += 1;
        }
    }
    sum / count
}
