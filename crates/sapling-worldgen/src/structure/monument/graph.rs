//! The monument's room graph (inlined in `MonumentBuilding`'s constructor 148334260): Java's
//! 5x5x3 grid of room definitions, the core room, then random openings closed while every room
//! still reaches the entrance. BDS shuffles with a fresh MT on the start's seed, drawing
//! `nextInt(i)` for i from n-1 down (never swapping an element with itself).

use crate::random::Mt19937;

/// Face indices (Java's 3D data values).
pub const DOWN: usize = 0;
pub const UP: usize = 1;
pub const NORTH: usize = 2;
pub const SOUTH: usize = 3;
pub const WEST: usize = 4;
pub const EAST: usize = 5;
const OPPOSITE: [usize; 6] = [UP, DOWN, SOUTH, NORTH, EAST, WEST];
/// (dx, dy, dz) per face.
const STEP: [(i32, i32, i32); 6] = [(0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)];

pub const TOP: usize = 75;
pub const LEFT_WING: usize = 76;
pub const RIGHT_WING: usize = 77;

#[derive(Debug, Clone, Default)]
pub struct Room {
    /// Grid index (y * 25 + z * 5 + x), or 1001..1003 for the wings and the top.
    pub index: i32,
    pub connections: [Option<usize>; 6],
    pub open: [bool; 6],
    pub claimed: bool,
    pub source: bool,
    scan: i32,
}

pub struct Graph {
    /// Grid rooms by index 0..75 (None where the grid has no room), then the top and wings.
    pub rooms: Vec<Option<Room>>,
    pub core: usize,
    /// The rooms in the order the fitters see them: the shuffled grid, then top, wings.
    pub order: Vec<usize>,
}

pub fn index(x: i32, y: i32, z: i32) -> usize {
    (y * 25 + z * 5 + x) as usize
}

impl Graph {
    pub fn room(&self, i: usize) -> &Room {
        self.rooms[i].as_ref().expect("room")
    }

    pub fn room_mut(&mut self, i: usize) -> &mut Room {
        self.rooms[i].as_mut().expect("room")
    }

    /// The room through face `face` of room `i`.
    pub fn next(&self, i: usize, face: usize) -> usize {
        self.room(i).connections[face].expect("connection")
    }

    pub fn generate(random: &mut Mt19937) -> Self {
        let mut rooms: Vec<Option<Room>> = vec![None; 78];
        let mut add = |x: i32, y: i32, z: i32| {
            let i = index(x, y, z);
            rooms[i] = Some(Room { index: i as i32, ..Room::default() });
        };
        for x in 0..5 {
            for z in 0..4 {
                add(x, 0, z);
            }
        }
        for x in 0..5 {
            for z in 0..4 {
                add(x, 1, z);
            }
        }
        for x in 1..4 {
            for z in 0..2 {
                add(x, 2, z);
            }
        }
        let source = index(2, 0, 0);
        for x in 0..5 {
            for z in 0..5 {
                for y in 0..3 {
                    let i = index(x, y, z);
                    if rooms[i].is_none() {
                        continue;
                    }
                    for (face, (dx, dy, dz)) in STEP.iter().enumerate() {
                        let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                        if !(0..5).contains(&nx) || !(0..5).contains(&nz) || !(0..3).contains(&ny) || rooms[index(nx, ny, nz)].is_none() {
                            continue;
                        }
                        // Java's quirk: links along z are stored under the opposite face.
                        let face = if nz == z { face } else { OPPOSITE[face] };
                        connect(&mut rooms, i, face, index(nx, ny, nz));
                    }
                }
            }
        }
        for (i, id) in [(TOP, 1003), (LEFT_WING, 1001), (RIGHT_WING, 1002)] {
            rooms[i] = Some(Room { index: id, claimed: true, ..Room::default() });
        }
        connect(&mut rooms, index(2, 2, 0), UP, TOP);
        connect(&mut rooms, index(0, 1, 0), SOUTH, LEFT_WING);
        connect(&mut rooms, index(4, 1, 0), SOUTH, RIGHT_WING);
        let mut graph = Graph { rooms, core: 0, order: Vec::new() };
        graph.room_mut(source).source = true;
        graph.core = index(random.next_int(4), 0, 2);
        let core = graph.core;
        let (east, north, up) = (graph.next(core, EAST), graph.next(core, NORTH), graph.next(core, UP));
        let east_north = graph.next(east, NORTH);
        for i in [core, east, north, east_north, up, graph.next(east, UP), graph.next(north, UP), graph.next(east_north, UP)] {
            graph.room_mut(i).claimed = true;
        }
        let mut list: Vec<usize> = (0..TOP).filter(|&i| graph.rooms[i].is_some()).collect();
        for i in list.iter().copied().chain([TOP]) {
            let room = graph.room_mut(i);
            room.open = room.connections.map(|c| c.is_some());
        }
        let mut shuffle = Mt19937::new(random.seed());
        for i in (1..list.len()).rev() {
            let j = shuffle.next_int(i as i32) as usize;
            list.swap(i, j);
        }
        let mut scan = 1;
        for &i in &list {
            let (mut closed, mut tries) = (0, 0);
            while closed < 2 && tries < 5 {
                tries += 1;
                let face = random.next_int(6) as usize;
                if !graph.room(i).open[face] {
                    continue;
                }
                let other = graph.next(i, face);
                graph.room_mut(i).open[face] = false;
                graph.room_mut(other).open[OPPOSITE[face]] = false;
                // Each search takes the next scan mark; the second only runs after the first succeeds.
                let mut search = |graph: &mut Graph, room: usize| {
                    scan += 1;
                    graph.find_source(room, scan - 1)
                };
                if search(&mut graph, i) && search(&mut graph, other) {
                    closed += 1;
                    continue;
                }
                graph.room_mut(i).open[face] = true;
                graph.room_mut(other).open[OPPOSITE[face]] = true;
            }
        }
        list.extend([TOP, LEFT_WING, RIGHT_WING]);
        graph.order = list;
        graph
    }

    /// Java's `findSource`: depth-first through open faces, marking rooms with `scan`.
    fn find_source(&mut self, i: usize, scan: i32) -> bool {
        if self.room(i).source {
            return true;
        }
        self.room_mut(i).scan = scan;
        for face in 0..6 {
            let room = self.room(i);
            if let Some(next) = room.connections[face]
                && room.open[face]
                && self.room(next).scan != scan
                && self.find_source(next, scan)
            {
                return true;
            }
        }
        false
    }
}

/// `setConnection`: both ways.
fn connect(rooms: &mut [Option<Room>], a: usize, face: usize, b: usize) {
    rooms[a].as_mut().expect("room").connections[face] = Some(b);
    rooms[b].as_mut().expect("room").connections[OPPOSITE[face]] = Some(a);
}
