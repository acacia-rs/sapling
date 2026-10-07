//! The mineshaft's room (`MineshaftRoom` 14831ca70/14831d010), crossings (1483205e0/148320950)
//! and stairs (148321790/148321870).

use super::{Builder, Kind, Materials};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::BoundingBox;
use crate::structure::canvas::Canvas;
use crate::structure::piece::{EAST, Frame, NORTH, SOUTH, WEST};

/// Children along each wall (north, south, west, east), at random gaps and heights, each
/// leaving an entrance box in the room; all of it only when a float clears the room chance.
pub fn room_children(builder: &mut Builder, index: usize, random: &mut Mt19937) {
    if random.next_float() > builder.materials.room_chance {
        return;
    }
    let (room, depth) = (builder.pieces[index].frame.bb, builder.pieces[index].frame.depth);
    let heights = (room.y1 - room.y0 + 1).max(5) - 4;
    let (x_span, z_span) = (room.x1 - room.x0 + 1, room.z1 - room.z0 + 1);
    for (span, direction) in [(x_span, NORTH), (x_span, SOUTH), (z_span, WEST), (z_span, EAST)] {
        let mut offset = 0;
        while offset < span {
            offset += random.next_int(span);
            if offset + 3 > span {
                break;
            }
            let y = room.y0 + random.next_int(heights) + 1;
            let at = match direction {
                NORTH => (room.x0 + offset, y, room.z0 - 1),
                SOUTH => (room.x0 + offset, y, room.z1 + 1),
                WEST => (room.x0 - 1, y, room.z0 + offset),
                _ => (room.x1 + 1, y, room.z0 + offset),
            };
            let before = builder.pieces.len();
            builder.add(random, at, direction, depth);
            if builder.pieces.len() > before {
                let child = builder.pieces[before].frame.bb;
                let entrance = match direction {
                    NORTH => BoundingBox::new(child.x0, child.y0, room.z0, child.x1, child.y1, room.z0 + 1),
                    SOUTH => BoundingBox::new(child.x0, child.y0, room.z1 - 1, child.x1, child.y1, room.z1),
                    WEST => BoundingBox::new(room.x0, child.y0, child.z0, room.x0 + 1, child.y1, child.z1),
                    _ => BoundingBox::new(room.x1 - 1, child.y0, child.z0, room.x1, child.y1, child.z1),
                };
                if let Kind::Room(entrances) = &mut builder.pieces[index].kind {
                    entrances.push(entrance);
                }
            }
            offset += 4;
        }
    }
}

/// A dirt floor under the room's air, the room hollowed up to 3 high, the entrances, and a
/// dome above.
pub fn room(f: &Frame, entrances: &[BoundingBox], canvas: &mut Canvas) {
    let (b, air) = (f.bb, Block::Air);
    f.generate_box(canvas, (b.x0, b.y0, b.z0), (b.x1, b.y0, b.z1), Block::Dirt, air, true);
    f.generate_box(canvas, (b.x0, b.y0 + 1, b.z0), (b.x1, (b.y0 + 3).min(b.y1), b.z1), air, air, false);
    for e in entrances {
        f.generate_box(canvas, (e.x0, e.y1 - 2, e.z0), (e.x1, e.y1, e.z1), air, air, false);
    }
    f.upper_half_sphere(canvas, (b.x0, b.y0 + 4, b.z0), (b.x1, b.y1, b.z1), air);
}

/// A crossing's exits: ahead and to both sides of where it was entered, and on two-floored
/// crossings each upper exit with a coin flip.
pub fn crossing_children(builder: &mut Builder, index: usize, random: &mut Mt19937) {
    let piece = &builder.pieces[index];
    let (b, depth) = (piece.frame.bb, piece.frame.depth);
    let Kind::Crossing { direction, two_floors } = piece.kind else { return };
    let north = (b.x0 + 1, b.y0, b.z0 - 1);
    let south = (b.x0 + 1, b.y0, b.z1 + 1);
    let west = (b.x0 - 1, b.y0, b.z0 + 1);
    let east = (b.x1 + 1, b.y0, b.z0 + 1);
    let exits: [((i32, i32, i32), u8); 3] = match direction {
        SOUTH => [(south, SOUTH), (west, WEST), (east, EAST)],
        WEST => [(north, NORTH), (south, SOUTH), (west, WEST)],
        NORTH => [(north, NORTH), (west, WEST), (east, EAST)],
        _ => [(north, NORTH), (south, SOUTH), (east, EAST)],
    };
    for (at, direction) in exits {
        builder.add(random, at, direction, depth);
    }
    if two_floors {
        let up = b.y0 + 4;
        for (at, direction) in
            [((b.x0 + 1, up, b.z0 - 1), NORTH), ((b.x0 - 1, up, b.z0 + 1), WEST), ((b.x1 + 1, up, b.z0 + 1), EAST), ((b.x0 + 1, up, b.z1 + 1), SOUTH)]
        {
            if random.next_boolean() {
                builder.add(random, at, direction, depth);
            }
        }
    }
}

pub fn crossing(f: &Frame, two_floors: bool, materials: Materials, canvas: &mut Canvas) {
    let (b, air, planks) = (f.bb, Block::Air, materials.planks());
    if two_floors {
        f.generate_box(canvas, (b.x0 + 1, b.y0, b.z0), (b.x1 - 1, b.y0 + 2, b.z1), air, air, false);
        f.generate_box(canvas, (b.x0, b.y0, b.z0 + 1), (b.x1, b.y0 + 2, b.z1 - 1), air, air, false);
        f.generate_box(canvas, (b.x0 + 1, b.y1 - 2, b.z0), (b.x1 - 1, b.y1, b.z1), air, air, false);
        f.generate_box(canvas, (b.x0, b.y1 - 2, b.z0 + 1), (b.x1, b.y1, b.z1 - 1), air, air, false);
        f.generate_box(canvas, (b.x0 + 1, b.y0 + 3, b.z0 + 1), (b.x1 - 1, b.y0 + 3, b.z1 - 1), air, air, false);
    } else {
        f.generate_box(canvas, (b.x0 + 1, b.y0, b.z0), (b.x1 - 1, b.y1, b.z1), air, air, false);
        f.generate_box(canvas, (b.x0, b.y0, b.z0 + 1), (b.x1, b.y1, b.z1 - 1), air, air, false);
    }
    for (x, z) in [(b.x0 + 1, b.z0 + 1), (b.x0 + 1, b.z1 - 1), (b.x1 - 1, b.z0 + 1), (b.x1 - 1, b.z1 - 1)] {
        if f.get(canvas, x, b.y1 + 1, z) != air {
            f.generate_box(canvas, (x, b.y0, z), (x, b.y1, z), planks, air, false);
        }
    }
    for x in b.x0..=b.x1 {
        for z in b.z0..=b.z1 {
            if canvas.interior(x, b.y0, z) && matches!(canvas.get(x, b.y0 - 1, z), Block::Air | Block::IronChain) {
                canvas.set(x, b.y0 - 1, z, planks);
            }
        }
    }
}

/// The stairs' only exit, at their foot.
pub fn stairs_children(builder: &mut Builder, index: usize, random: &mut Mt19937) {
    let f = &builder.pieces[index].frame;
    let (b, depth) = (f.bb, f.depth);
    let (at, direction) = match f.orientation.unwrap_or(SOUTH) {
        SOUTH => ((b.x0, b.y0, b.z1 + 1), SOUTH),
        WEST => ((b.x0 - 1, b.y0, b.z0), WEST),
        NORTH => ((b.x0, b.y0, b.z0 - 1), NORTH),
        _ => ((b.x1 + 1, b.y0, b.z0), EAST),
    };
    builder.add(random, at, direction, depth);
}

pub fn stairs(f: &Frame, canvas: &mut Canvas) {
    let air = Block::Air;
    f.generate_box(canvas, (0, 5, 0), (2, 7, 1), air, air, false);
    f.generate_box(canvas, (0, 0, 7), (2, 2, 8), air, air, false);
    for i in 0..5 {
        f.generate_box(canvas, (0, 5 - i - i32::from(i < 4), 2 + i), (2, 7 - i, 2 + i), air, air, false);
    }
}
