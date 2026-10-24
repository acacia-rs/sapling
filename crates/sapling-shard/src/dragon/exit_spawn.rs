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

/// boost's hash_combine of each coordinate's `std::hash`, cut to the random's seed.
fn combined([x, y, z]: BlockPos, hash: fn(i32) -> u64) -> u32 {
    let join = |h: u64, v: i32| (hash(v).wrapping_add(0x9e37_79b9).wrapping_add(h << 6).wrapping_add(h >> 2)) ^ h;
    join(join(hash(x).wrapping_add(0x9e37_79b9), y), z) as u32
}

/// MSVC's `std::hash<int>`: FNV-1a over its four bytes.
fn msvc_hash(value: i32) -> u64 {
    value.to_le_bytes().iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

/// The seed Windows BDS hashes from the island's cell (seen live in 148274c80: 0xF92C2FAE at
/// (0, 75, -1120)). sapling builds worlds as Windows BDS does; Linux BDS's libstdc++ hashes an
/// int as itself and builds other islands (tests).
fn seed(at: BlockPos) -> u32 {
    combined(at, msvc_hash)
}

/// The end island BDS builds at (x, 75, z) where an exit finds no spawn: its cell and its end stone.
pub(super) fn island([x, _, z]: BlockPos) -> (BlockPos, Vec<BlockPos>) {
    let at = [x, ISLAND_Y, z];
    island_seeded(at, seed(at))
}

fn island_seeded(at @ [x, _, z]: BlockPos, seed: u32) -> (BlockPos, Vec<BlockPos>) {
    let mut random = Mt19937::new(seed);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The cells of a layer of radius `r` round the middle, as BDS's islands had them
    /// (research/captures/bds-exit-island*.txt: 145, 97, 45, 37 and 9 cells for 6, 4.5, 3, 2.5, 1).
    fn disc(r: f32) -> usize {
        let (lo, hi) = ((-r).floor() as i32, r.ceil() as i32);
        (lo..=hi).flat_map(|i| (lo..=hi).map(move |k| (i, k))).filter(|(i, k)| ((i * i + k * k) as f32) <= (r + 1.0) * (r + 1.0)).count()
    }

    fn layers((_, cells): (BlockPos, Vec<BlockPos>)) -> Vec<usize> {
        (60..=75).rev().map(|y| cells.iter().filter(|cell| cell[1] == y).count()).take_while(|&n| n > 0).collect()
    }

    #[test]
    fn the_island_is_seeded_as_windows_bds_seeds_it() {
        assert_eq!(seed([0, 75, -1120]), 0xF92C_2FAE);
        assert_eq!(layers(island([0, 0, -1120])), [5.0, 4.5, 3.0, 1.5, 1.0].map(disc));
    }

    #[test]
    fn linux_seeding_builds_the_islands_testbox_bds_built() {
        // Radii 6, 4.5, 3, 2.5, 1 (bds-exit-island.txt) and 6, 5.5, 4, 3.5, 2, 1.5 (bds-exit-island2.txt).
        let linux = |at: BlockPos| island_seeded(at, combined(at, |v| i64::from(v) as u64));
        assert_eq!(layers(linux([0, 75, -1120])), [6.0, 4.5, 3.0, 2.5, 1.0].map(disc));
        assert_eq!(layers(linux([602, 75, -829])), [6.0, 5.5, 4.0, 3.5, 2.0, 1.5].map(disc));
    }
}
