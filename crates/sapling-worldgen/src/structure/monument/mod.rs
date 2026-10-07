//! Ocean monuments (`OceanMonumentFeature::createStructureStart` 148332c00, `MonumentBuilding`
//! 148334260): Java 1.18's monument on BDS's MT, facing a random horizontal direction. Pieces
//! orient by Bedrock facing (2 north, 3 south, 4 west, 5 east); see docs/research/worldgen-bds-structures.md.

mod building;
mod building_walls;
mod graph;
mod rooms;
mod rooms_big;
mod wing;

use super::BoundingBox;
use super::canvas::Canvas;
use super::piece::{EAST, Frame, NORTH, SOUTH, WEST};
use crate::block::Block;
use crate::random::Mt19937;
use graph::{Graph, UP};

pub(super) const PRISMARINE: Block = Block::Prismarine;
pub(super) const BRICKS: Block = Block::PrismarineBricks;
pub(super) const DARK: Block = Block::DarkPrismarine;
pub(super) const LAMP: Block = Block::SeaLantern;
const SEA_LEVEL: i32 = 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    Entry,
    Core,
    Simple(i32),
    SimpleTop,
    DoubleX,
    DoubleY,
    DoubleZ,
    DoubleXY,
    DoubleYZ,
    Wing(i32),
    Penthouse,
}

/// What a piece reads of one grid room: its open faces and whether a room lies above it.
#[derive(Debug, Clone, Copy, Default)]
pub struct RoomInfo {
    pub open: [bool; 6],
    pub linked_up: bool,
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub frame: Frame,
    pub room: Room,
    pub index: i32,
    /// The piece's own room first, then the others it spans (see `fit`).
    pub rooms: Vec<RoomInfo>,
}

pub struct Monument {
    pub building: Frame,
    pub pieces: Vec<Piece>,
}

fn frame(bb: BoundingBox, facing: u8) -> Frame {
    // Bedrock facing to the legacy direction getWorldX/Z switch on (148333c30/148333c80).
    let orientation = match facing {
        2 => NORTH,
        3 => SOUTH,
        4 => WEST,
        _ => EAST,
    };
    Frame { bb, orientation: Some(orientation), depth: 0, keep: |_| false }
}

/// The monument starting in `chunk`: its own MT on the level seed and two floats, the facing,
/// then the building's room graph and pieces.
pub fn generate(level_seed: u32, (cx, cz): (i32, i32)) -> Monument {
    let mut random = Mt19937::new(level_seed);
    let (a, b) = (random.next_float(), random.next_float());
    let seed = (a * cx as f32) as i64 as u32 ^ (b * cz as f32) as i64 as u32 ^ level_seed;
    let mut random = Mt19937::new(seed);
    let facing = random.next_int(4) as u8 + 2;
    let (x, z) = (cx * 16 - 21, cz * 16 - 21);
    let building = frame(BoundingBox::new(x, SEA_LEVEL - 24, z, x + 57, SEA_LEVEL - 2, z + 57), facing);
    let mut graph = Graph::generate(&mut random);
    let mut pieces = Vec::new();
    let source = graph::index(2, 0, 0);
    graph.room_mut(source).claimed = true;
    pieces.push(room_piece(&graph, &[source], facing, Room::Entry, (1, 1, 1)));
    pieces.push(room_piece(&graph, &[graph.core], facing, Room::Core, (2, 2, 2)));
    for i in graph.order.clone() {
        let room = graph.room(i);
        if room.claimed || room.index >= 75 {
            continue;
        }
        pieces.push(fit(&mut graph, i, facing, &mut random));
    }
    let (dx, dy, dz) = (building.world_x(9, 22), building.bb.y0, building.world_z(9, 22));
    for piece in &mut pieces {
        piece.frame.bb.x0 += dx;
        piece.frame.bb.x1 += dx;
        piece.frame.bb.move_y(dy);
        piece.frame.bb.z0 += dz;
        piece.frame.bb.z1 += dz;
    }
    let corners = |a: (i32, i32, i32), b: (i32, i32, i32)| {
        let (p, q) = (building.world(a.0, a.1, a.2), building.world(b.0, b.1, b.2));
        BoundingBox::new(p.0.min(q.0), p.1.min(q.1), p.2.min(q.2), p.0.max(q.0), p.1.max(q.1), p.2.max(q.2))
    };
    let design = random.next_int_unbounded() & 1;
    for (bb, room) in [
        (corners((1, 1, 1), (23, 8, 21)), Room::Wing(design)),
        (corners((34, 1, 1), (56, 8, 21)), Room::Wing(design ^ 1)),
        (corners((22, 13, 22), (35, 17, 35)), Room::Penthouse),
    ] {
        pieces.push(Piece { frame: frame(bb, facing), room, index: -1, rooms: Vec::new() });
    }
    Monument { building, pieces }
}

/// A room piece over `(w, h, d)` grid cells from the first of `rooms` (148333960).
fn room_piece(graph: &Graph, rooms: &[usize], facing: u8, room: Room, (w, h, d): (i32, i32, i32)) -> Piece {
    let info = |i: usize| RoomInfo { open: graph.room(i).open, linked_up: graph.room(i).connections[UP].is_some() };
    let index = graph.room(rooms[0]).index;
    let (x, z, y) = (index % 5, index / 5 % 5, index / 25);
    let (sx, sz) = if facing == 2 || facing == 3 { (w, d) } else { (d, w) };
    let (x0, z0) = match facing {
        4 => (-(z + d) * 8 + 1, x * 8),
        3 => (x * 8, z * 8),
        2 => (x * 8, -(z + d) * 8 + 1),
        _ => (z * 8, x * 8),
    };
    let bb = BoundingBox::new(x0, y * 4, z0, x0 + sx * 8 - 1, y * 4 + h * 4 - 1, z0 + sz * 8 - 1);
    Piece { frame: frame(bb, facing), room, index, rooms: rooms.iter().map(|&i| info(i)).collect() }
}

/// The first of BDS's fitters that takes room `i` (vtables 14a928f10..14a928fd0), claiming the
/// rooms it covers.
fn fit(graph: &mut Graph, i: usize, facing: u8, random: &mut Mt19937) -> Piece {
    use graph::{EAST as E, NORTH as N, SOUTH as S, UP as U};
    let free = |g: &Graph, i: usize, face: usize| g.room(i).open[face] && !g.room(g.next(i, face)).claimed;
    let claim = |g: &mut Graph, rooms: &[usize]| rooms.iter().for_each(|&r| g.room_mut(r).claimed = true);
    let (piece, rooms, size) = if free(graph, i, E) && free(graph, i, U) && free(graph, graph.next(i, E), U) {
        let (e, u) = (graph.next(i, E), graph.next(i, U));
        (Room::DoubleXY, vec![i, e, u, graph.next(e, U)], (2, 2, 1))
    } else if free(graph, i, N) && free(graph, i, U) && free(graph, graph.next(i, N), U) {
        let (n, u) = (graph.next(i, N), graph.next(i, U));
        (Room::DoubleYZ, vec![i, n, u, graph.next(n, U)], (1, 2, 2))
    } else if free(graph, i, N) {
        // BDS's DoubleZ creator re-checks and falls back to the room south, as Java does.
        let room = if free(graph, i, N) { i } else { graph.next(i, S) };
        (Room::DoubleZ, vec![room, graph.next(room, N)], (1, 1, 2))
    } else if free(graph, i, E) {
        (Room::DoubleX, vec![i, graph.next(i, E)], (2, 1, 1))
    } else if free(graph, i, U) {
        (Room::DoubleY, vec![i, graph.next(i, U)], (1, 2, 1))
    } else if graph.room(i).open[1..].iter().all(|open| !open) {
        (Room::SimpleTop, vec![i], (1, 1, 1))
    } else {
        (Room::Simple(random.next_int(3)), vec![i], (1, 1, 1))
    };
    claim(graph, &rooms);
    room_piece(graph, &rooms, facing, piece, size)
}

impl Monument {
    /// The building, then every piece reaching the chunk, on the chunk's monument generator.
    pub fn post_process(&self, canvas: &mut Canvas, random: &mut Mt19937) {
        building::post_process(&self.building, canvas);
        let chunk = canvas.bb;
        for piece in self.pieces.iter().filter(|p| p.frame.bb.intersects(&chunk)) {
            let f = &piece.frame;
            match piece.room {
                Room::Entry => rooms::entry(f, piece, canvas),
                Room::Simple(design) => rooms::simple(f, piece, design, canvas, random),
                Room::SimpleTop => rooms::simple_top(f, piece, canvas, random),
                Room::DoubleY => rooms::double_y(f, piece, canvas),
                Room::DoubleX => rooms::double_x(f, piece, canvas),
                Room::DoubleZ => rooms_big::double_z(f, piece, canvas),
                Room::DoubleXY => rooms_big::double_xy(f, piece, canvas),
                Room::DoubleYZ => rooms_big::double_yz(f, piece, canvas),
                Room::Core => rooms_big::core(f, canvas),
                Room::Wing(design) => wing::wing(f, design, canvas),
                Room::Penthouse => rooms_big::penthouse(f, canvas),
            }
        }
    }
}

/// Sea water below the sea level and air above, over a box (`generateWaterBox`, inlined).
pub(super) fn water_box(f: &Frame, canvas: &mut Canvas, (x0, y0, z0): (i32, i32, i32), (x1, y1, z1): (i32, i32, i32)) {
    for y in y0..=y1 {
        let block = if f.world_y(y) < SEA_LEVEL { Block::Water } else { Block::Air };
        for x in x0..=x1 {
            for z in z0..=z1 {
                f.place(canvas, block, x, y, z);
            }
        }
    }
}

/// `generateBoxOnFillOnly`: `block` only where the chunk holds water.
pub(super) fn box_on_water(f: &Frame, canvas: &mut Canvas, (x0, y0, z0): (i32, i32, i32), (x1, y1, z1): (i32, i32, i32), block: Block) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                if f.get(canvas, x, y, z) == Block::Water {
                    f.place(canvas, block, x, y, z);
                }
            }
        }
    }
}

/// `generateDefaultFloor` (148333cb0): solid, or with a cross-shaped opening down.
pub(super) fn default_floor(f: &Frame, canvas: &mut Canvas, x: i32, z: i32, open_down: bool) {
    let mut put =
        |a: (i32, i32), b: (i32, i32), block: Block| f.generate_box(canvas, (x + a.0, 0, z + a.1), (x + b.0, 0, z + b.1), block, block, false);
    if !open_down {
        put((0, 0), (7, 7), PRISMARINE);
        return;
    }
    put((0, 0), (2, 7), PRISMARINE);
    put((5, 0), (7, 7), PRISMARINE);
    put((3, 0), (4, 2), PRISMARINE);
    put((3, 5), (4, 7), PRISMARINE);
    put((3, 2), (4, 2), BRICKS);
    put((3, 5), (4, 5), BRICKS);
    put((2, 3), (2, 4), BRICKS);
    put((5, 3), (5, 4), BRICKS);
}

/// Java's `chunkIntersects`: the piece's (x0, z0)..(x1, z1) columns against the chunk.
pub(super) fn chunk_intersects(f: &Frame, canvas: &Canvas, (x0, z0): (i32, i32), (x1, z1): (i32, i32)) -> bool {
    let (ax, az, bx, bz) = (f.world_x(x0, z0), f.world_z(x0, z0), f.world_x(x1, z1), f.world_z(x1, z1));
    let c = canvas.bb;
    ax.min(bx) <= c.x1 && ax.max(bx) >= c.x0 && az.min(bz) <= c.z1 && az.max(bz) >= c.z0
}
