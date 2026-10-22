//! BDS's searches round a gateway's exit (research/dragon-extras-decompile.md 1-3): `findValidSpawn`
//! 1487fde00, `findTallestBlock` 1487fdbb0 and the end island built where no spawn is found
//! (148274c80). A block "with a box" stands for BDS's solid-blocking material.

use sapling_api::BlockPos;
use sapling_world::World as Level;
use sapling_worldgen::random::Mt19937;

const SCAN_FROM: i32 = 127;
const ISLAND_Y: i32 = 75;
/// The End's height: BDS reads from one under it down.
const TOP: i32 = 256;
const NEIGHBOURS: [[i32; 2]; 8] = [[1, 0], [-1, 0], [0, -1], [0, 1], [1, -1], [1, 1], [-1, -1], [-1, 1]];

fn named(level: &Level, [x, y, z]: BlockPos) -> Option<&'static str> {
    level.registry().get(level.block(x, y, z)).map(|state| state.name)
}

fn solid(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid())
}

fn is_air(level: &Level, [x, y, z]: BlockPos) -> bool {
    !level.in_height(y) || level.registry().get(level.block(x, y, z)).is_none_or(|state| state.is_air())
}

/// The first block with a box under the first air at or below `y` in the column (141603330).
fn top_solid_under(level: &Level, x: i32, mut y: i32, z: i32) -> Option<i32> {
    while !is_air(level, [x, y, z]) {
        y -= 1;
        if !level.in_height(y) {
            return None;
        }
    }
    (0..y).rev().take_while(|&y| level.in_height(y)).find(|&y| solid(level, [x, y, z]))
}

/// `findValidSpawn(region, pos, endStoneOnly, radius, 0)`: the column top within `radius` chunks
/// nearest `pos`, one with no air round it first; `None` where there is none.
pub(super) fn valid_spawn(level: &Level, pos: BlockPos, end_stone_only: bool, radius: i32) -> Option<BlockPos> {
    let reach = radius * 16;
    let (mut any, mut clear): (Option<(f32, BlockPos)>, Option<(f32, BlockPos)>) = (None, None);
    for x in pos[0] - reach..=pos[0] + reach {
        for z in pos[2] - reach..=pos[2] + reach {
            let Some(mut y) = top_solid_under(level, x, SCAN_FROM, z) else { continue };
            let roomy = |y: i32| !solid(level, [x, y + 1, z]) && !solid(level, [x, y + 2, z]);
            let tests_clear = if named(level, [x, y, z]) == Some("minecraft:bedrock") && named(level, [x, y - 2, z]) == Some("minecraft:end_gateway") {
                let Some(under) = top_solid_under(level, x, y - 2, z) else { continue };
                y = under;
                if !roomy(y) {
                    continue;
                }
                true
            } else if end_stone_only {
                if named(level, [x, y, z]) != Some("minecraft:end_stone") {
                    continue;
                }
                true
            } else {
                if named(level, [x, y, z]) == Some("minecraft:bedrock") || !roomy(y) {
                    continue;
                }
                false
            };
            let is_clear = tests_clear && !level.registry().get(level.block(x, y + 1, z)).is_some_and(|state| state.is_liquid()) && NEIGHBOURS.iter().all(|[dx, dz]| !is_air(level, [x + dx, y, z + dz]));
            let d = [(y, pos[1]), (x, pos[0]), (z, pos[2])].iter().map(|&(a, b)| ((a as f32 + 0.5) - (b as f32 + 0.5)).powi(2)).fold(0.0, |sum, term| sum + term);
            let better = |best: &Option<(f32, BlockPos)>| best.is_none_or(|(least, _)| d < least);
            if better(&any) {
                any = Some((d, [x, y, z]));
            }
            if is_clear && better(&clear) {
                clear = Some((d, [x, y, z]));
            }
        }
    }
    clear.or(any).map(|(_, at)| at)
}

/// `findTallestBlock`: the highest block with a box within `reach` across, the first column on a
/// tie; without bedrock, neither bedrock nor `pos`'s own column counts. `pos` where none does.
pub(super) fn tallest(level: &Level, pos: BlockPos, reach: i32, with_bedrock: bool) -> BlockPos {
    let mut best: Option<BlockPos> = None;
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            if dx == 0 && dz == 0 && !with_bedrock {
                continue;
            }
            let (x, z, floor) = (pos[0] + dx, pos[2] + dz, best.map_or(0, |[_, y, _]| y));
            let counts = |y: i32| solid(level, [x, y, z]) && (with_bedrock || named(level, [x, y, z]) != Some("minecraft:bedrock"));
            if let Some(y) = (floor + 1..TOP).rev().find(|&y| counts(y)) {
                best = Some([x, y, z]);
            }
        }
    }
    best.unwrap_or(pos)
}

fn fnv(value: i32) -> u64 {
    value.to_le_bytes().iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

/// The seed BDS hashes from the island's cell: FNV-1a per coordinate, joined as boost's hash_combine.
fn seed([x, y, z]: BlockPos) -> u32 {
    let join = |h: u64, v: i32| (fnv(v).wrapping_add(0x9e37_79b9).wrapping_add(h << 6).wrapping_add(h >> 2)) ^ h;
    let first = fnv(x).wrapping_add(0x9e37_79b9);
    join(join(first, y), z) as u32
}

/// The end island BDS builds at (x, 75, z) where an exit finds no spawn: its cell and its end stone.
pub(super) fn island([x, _, z]: BlockPos) -> (BlockPos, Vec<BlockPos>) {
    let at = [x, ISLAND_Y, z];
    let mut random = Mt19937::new(seed(at));
    let mut r = random.next_int(3) as f32 + 4.0;
    let (mut dy, mut cells) = (0, Vec::new());
    while r > 0.5 {
        let (lo, hi) = ((-r).floor() as i32, r.ceil() as i32);
        for i in lo..=hi {
            for k in lo..=hi {
                if ((i * i + k * k) as f32) <= (r + 1.0) * (r + 1.0) {
                    cells.push([x + i, ISLAND_Y + dy, z + k]);
                }
            }
        }
        r -= random.next_int(2) as f32 + 0.5;
        dy -= 1;
    }
    (at, cells)
}
