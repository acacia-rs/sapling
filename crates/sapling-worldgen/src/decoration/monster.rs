//! Monster rooms (`MonsterRoomFeature::place` 149e9dae0), tried after the lava lake on the same
//! generator: 4 origins below y 0, then 10 anywhere, each in the 16x16 square 8 blocks into the chunk.

use std::collections::HashMap;

use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::MIN_Y;

const DEEP_TRIES: i32 = 4;
const TRIES: i32 = 10;
/// The overworld's height (vtable 0x1c0), the bound of the upper tries' y.
const HEIGHT: i32 = 320;

/// The blocks one chunk's rooms write, in order, across chunk borders.
pub type Writes = Vec<((i32, i32, i32), Block)>;

/// The world as the rooms see it: `base` with the writes so far on top.
struct World<'a, F: FnMut(i32, i32, i32) -> Block> {
    base: &'a mut F,
    written: HashMap<(i32, i32, i32), Block>,
    writes: Writes,
}

impl<F: FnMut(i32, i32, i32) -> Block> World<'_, F> {
    fn get(&mut self, x: i32, y: i32, z: i32) -> Block {
        match self.written.get(&(x, y, z)) {
            Some(&block) => block,
            None => (self.base)(x, y, z),
        }
    }

    /// Bedrock's `Material::isSolid`.
    fn solid(&mut self, x: i32, y: i32, z: i32) -> bool {
        self.get(x, y, z).is_solid()
    }

    fn set(&mut self, x: i32, y: i32, z: i32, block: Block) {
        self.written.insert((x, y, z), block);
        self.writes.push(((x, y, z), block));
    }
}

/// Every room of a chunk; `get` reads the world around it.
pub fn monster_rooms(random: &mut Mt19937, (cx, cz): (i32, i32), get: &mut impl FnMut(i32, i32, i32) -> Block) -> Writes {
    let mut world = World { base: get, written: HashMap::new(), writes: Vec::new() };
    let (x0, z0) = (cx * 16 + 8, cz * 16 + 8);
    let deep = -(MIN_Y + 6);
    for _ in 0..DEEP_TRIES {
        let x = x0 + random.next_int(16);
        let y = random.next_int(deep) + MIN_Y + 6;
        let z = z0 + random.next_int(16);
        place(&mut world, random, (x, y, z));
    }
    for _ in 0..TRIES {
        let x = x0 + random.next_int(16);
        let y = random.next_int(HEIGHT);
        let z = z0 + random.next_int(16);
        place(&mut world, random, (x, y, z));
    }
    world.writes
}

fn place<F: FnMut(i32, i32, i32) -> Block>(world: &mut World<F>, random: &mut Mt19937, (x, y, z): (i32, i32, i32)) {
    let rx = random.next_int(2) + 2;
    let rz = random.next_int(2) + 2;
    let (xa, xb, za, zb) = (x - rx - 1, x + rx + 1, z - rz - 1, z + rz + 1);
    let mut openings = 0;
    for wx in xa..=xb {
        for wy in y - 1..=y + 4 {
            for wz in za..=zb {
                if (wy == y - 1 || wy == y + 4) && !world.solid(wx, wy, wz) {
                    return;
                }
                let wall = wx == xa || wx == xb || wz == za || wz == zb;
                if wall && wy == y && world.get(wx, wy, wz) == Block::Air && world.get(wx, wy + 1, wz) == Block::Air {
                    openings += 1;
                }
            }
        }
    }
    if !(1..=5).contains(&openings) {
        return;
    }
    for wx in xa..=xb {
        for wy in (y - 1..=y + 3).rev() {
            for wz in za..=zb {
                let shell = wx == xa || wz == za || wy == y - 1 || wz == zb || wx == xb;
                if !shell {
                    world.set(wx, wy, wz, Block::Air);
                } else if wy >= MIN_Y && !world.solid(wx, wy - 1, wz) {
                    world.set(wx, wy, wz, Block::Air);
                } else if world.solid(wx, wy, wz) {
                    let mossy = wy == y - 1 && random.next_int(4) != 0;
                    world.set(wx, wy, wz, if mossy { Block::MossyCobblestone } else { Block::Cobblestone });
                }
            }
        }
    }
    for _ in 0..2 {
        for _ in 0..3 {
            // C++ evaluates the z draw first.
            let dz = random.next_int(rz * 2 + 1) - rz;
            let dx = random.next_int(rx * 2 + 1) - rx;
            let (cx, cz) = (x + dx, z + dz);
            if world.get(cx, y, cz) != Block::Air {
                continue;
            }
            let walls = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().filter(|(ox, oz)| world.solid(cx + ox, y, cz + oz)).count();
            if walls != 1 {
                continue;
            }
            let mut open = |ox: i32, oz: i32| world.get(cx + ox, y, cz + oz) == Block::Air;
            let facing = chest_facing(dx, dz, &mut open);
            world.set(cx, y, cz, [Block::ChestSouth, Block::ChestWest, Block::Chest, Block::ChestEast][facing as usize]);
            random.next_int_unbounded();
            break;
        }
    }
    world.set(x, y, z, Block::MobSpawner);
    random.next_int(4);
}

/// The chest's cardinal direction (south, west, north, east) away from the room's far walls, as
/// BDS's branches have it.
fn chest_facing(dx: i32, dz: i32, open: &mut impl FnMut(i32, i32) -> bool) -> u8 {
    let toward_north;
    if dz >= 1 {
        if !open(0, 1) {
            return 2;
        }
        toward_north = false;
    } else if dz <= -1 {
        if !open(0, -1) {
            return 0;
        }
        toward_north = true;
    } else {
        toward_north = false;
    }
    let along_z = if dz > 0 { 2 } else { 0 };
    if dx >= 1 {
        if !open(1, 0) {
            return 1;
        }
    } else if dx <= -1 {
        if !open(-1, 0) {
            return 3;
        }
        return if toward_north || dz > 0 { along_z } else { 3 };
    }
    if !toward_north && dz <= 0 { if dx > 0 { 1 } else { 2 } } else { along_z }
}
