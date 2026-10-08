//! The monster rooms of `decorationPostProcessChunk` (1482143a0), after the lava lake and before
//! the feature rules: 4 tries in the deepslate layer, then 10 anywhere below y 320, each
//! `MonsterRoomFeature::place` (149e9dae0), Java 1.12's dungeon with its chest-facing rule.

use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::MIN_Y;

/// `OverworldGenerator`'s generation height (144a184d0).
const GENERATION_HEIGHT: i32 = 320;

/// `random` is the chunk's post-process generator after its lava lake.
pub fn monster_rooms(mut random: Mt19937, chunk: (i32, i32), view: &mut View) {
    let (x0, z0) = (chunk.0 * 16 | 8, chunk.1 * 16 | 8);
    let deep = (MIN_Y + 6).abs();
    for _ in 0..4 {
        let x = random.next_int(16) + x0;
        let y = random.next_int(deep) + MIN_Y + 6;
        let z = random.next_int(16) + z0;
        room(view, &mut random, [x, y, z]);
    }
    for _ in 0..10 {
        let x = random.next_int(16) + x0;
        let y = random.next_int(GENERATION_HEIGHT);
        let z = random.next_int(16) + z0;
        room(view, &mut random, [x, y, z]);
    }
}

fn solid(view: &View, [x, y, z]: [i32; 3]) -> bool {
    view.block(x, y, z).is_solid()
}

fn air(view: &View, [x, y, z]: [i32; 3]) -> bool {
    view.block(x, y, z) == Block::Air
}

fn room(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) -> bool {
    let (rx, rz) = (random.next_int(2), random.next_int(2));
    let (x0, x1, z0, z1) = (x - rx - 3, x + rx + 3, z - rz - 3, z + rz + 3);
    let mut openings = 0;
    for px in x0..=x1 {
        for py in y - 1..=y + 4 {
            for pz in z0..=z1 {
                let p = [px, py, pz];
                if (py == y - 1 || py == y + 4) && !solid(view, p) {
                    return false;
                }
                let wall = px == x0 || px == x1 || pz == z0 || pz == z1;
                if wall && py == y && air(view, p) && air(view, [px, py + 1, pz]) {
                    openings += 1;
                }
            }
        }
    }
    if !(1..=5).contains(&openings) {
        return false;
    }
    for px in x0..=x1 {
        for py in (y - 1..=y + 3).rev() {
            for pz in z0..=z1 {
                let p = [px, py, pz];
                let shell = px == x0 || px == x1 || pz == z0 || pz == z1 || py == y - 1;
                if !shell || (py >= MIN_Y && !solid(view, [px, py - 1, pz])) {
                    view.set_block(px, py, pz, Block::Air);
                } else if solid(view, p) {
                    let mossy = py == y - 1 && random.next_int(4) != 0;
                    view.set_block(px, py, pz, if mossy { Block::MossyCobblestone } else { Block::Cobblestone });
                }
            }
        }
    }
    chests(view, random, [x, y, z], rx + 2, rz + 2);
    view.set_block(x, y, z, Block::MobSpawner);
    random.next_int(4);
    true
}

/// Two rounds of up to three tries for an air cell with exactly one solid side.
fn chests(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], rx: i32, rz: i32) {
    for _ in 0..2 {
        for _ in 0..3 {
            let dz = random.next_int(rz * 2 + 1) - rz;
            let dx = random.next_int(rx * 2 + 1) - rx;
            let p = [x + dx, y, z + dz];
            if !air(view, p) {
                continue;
            }
            let sides = [[-1, 0], [1, 0], [0, -1], [0, 1]].iter().filter(|[ox, oz]| solid(view, [p[0] + ox, y, p[2] + oz])).count();
            if sides == 1 {
                let facing = chest_facing(view, p, dx, dz);
                view.set_block(p[0], y, p[2], Block::CHESTS[facing]);
                random.next_u32();
                break;
            }
        }
    }
}

/// The chest's facing (0..4) from its offset in the room and the air beside it, as 149e9dae0
/// derives it.
fn chest_facing(view: &View, [x, y, z]: [i32; 3], dx: i32, dz: i32) -> usize {
    let open = |ox: i32, oz: i32| air(view, [x + ox, y, z + oz]);
    let behind = match dz.signum() {
        -1 if !open(0, -1) => return 0,
        -1 => true,
        1 if !open(0, 1) => return 2,
        _ => false,
    };
    let toward = || if !behind && dz <= 0 { if dx > 0 { 1 } else { 2 } } else { usize::from(dz > 0) * 2 };
    match dx.signum() {
        -1 => {
            let blocked = !open(-1, 0);
            if behind || dz > 0 || blocked {
                if blocked { 3 } else { usize::from(dz > 0) * 2 }
            } else {
                3
            }
        }
        1 if !open(1, 0) => 1,
        _ => toward(),
    }
}
