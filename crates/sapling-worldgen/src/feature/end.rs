//! The End's hardcoded decoration (`TheEndGenerator::decorationPostProcessChunk` 148245110): Java 1.12's End
//! decoration on the chunk's post-process generator where the biome rules' `math.random` left it: the spikes, then
//! beyond 64 chunks of the origin small islands, chorus plants and end gateways (docs/research/worldgen-bds-end.md).

use super::view::View;
use crate::block::Block;
use crate::end::Spike;
use crate::random::Mt19937;

/// `getIslandHeightValue(chunk x, chunk z, 1, 1)` (148244960).
pub type IslandHeight<'a> = &'a dyn Fn(i32, i32) -> f32;

pub fn decorate(random: &mut Mt19937, chunk: (i32, i32), view: &mut View, spikes: &[Spike; 10], island: IslandHeight) {
    let (x0, z0) = (chunk.0 * 16, chunk.1 * 16);
    for spike in spikes.iter().filter(|s| (s.x - s.radius) & !15 == x0 && (s.z - s.radius) & !15 == z0) {
        place_spike(view, spike);
    }
    if i64::from(chunk.0) * i64::from(chunk.0) + i64::from(chunk.1) * i64::from(chunk.1) <= 4096 {
        return;
    }
    if island(chunk.0, chunk.1) < -20.0 && random.next_u32() % 14 == 0 {
        let x = (random.next_u32() & 15) as i32 + x0 + 8;
        let y = (random.next_u32() & 15) as i32 + 55;
        let z = (random.next_u32() & 15) as i32 + z0 + 8;
        end_island(view, random, [x, y, z]);
        if random.next_u32() & 3 == 0 {
            end_island(view, random, [x, y, z]);
        }
    }
    if island(chunk.0, chunk.1) <= 40.0 {
        return;
    }
    let count = random.next_u32() % 5;
    let (ox, oz) = (x0 | 8, z0 | 8);
    let mut k = 0;
    // The x draw comes before the count test: the one that fails it is the gateway's.
    let last = loop {
        let dx = random.next_u32();
        if k >= count {
            break dx;
        }
        let dz = random.next_u32();
        let (x, z) = ((dx & 15) as i32 + ox, (dz & 15) as i32 + oz);
        let h = view.heightmap(x, z);
        if h > 0 && view.block(x, h, z) == Block::Air && view.block(x, h - 1, z) == Block::EndStone {
            view.set_block(x, h, z, Block::ChorusPlant);
            grow_chorus(view, random, [x, h, z], [x, h, z], 8, 0);
        }
        k += 1;
    };
    if last % 700 == 0 {
        let x = (random.next_u32() & 15) as i32 + ox;
        let z = (random.next_u32() & 15) as i32 + oz;
        let h = view.heightmap(x, z);
        if h > 0 {
            let y = (h + 3) as u16 as i32 + (random.next_u32() % 7) as i32;
            // BDS skips it beside a Loaded chunk, which none can be while this one decorates (worldgen-bds-end.md).
            end_gateway(view, [x, y, z]);
        }
    }
}

/// `SpikeFeature::place` (149d63340) with `clearAirAbove`: obsidian within the circle below the height, air
/// elsewhere in the box above y 65 up to ten over the top, then the cage. The bedrock and crystal on top belong to
/// `postProcessMobsAt` (slot 16), which a world without players never runs.
fn place_spike(view: &mut View, spike: &Spike) {
    let r = spike.radius;
    for x in spike.x - r..=spike.x + r {
        for y in 0..=spike.height + 10 {
            for z in spike.z - r..=spike.z + r {
                let d2 = (z - spike.z) * (z - spike.z) + (x - spike.x) * (x - spike.x);
                if d2 <= r * r + 1 && y < spike.height {
                    view.set_block(x, y, z, Block::Obsidian);
                } else if y > 65 {
                    view.set_block(x, y, z, Block::Air);
                }
            }
        }
    }
    if spike.guarded {
        for dx in -2..=2i32 {
            for dz in -2..=2i32 {
                for dy in 0..=3 {
                    if dx.abs() == 2 || dz.abs() == 2 || dy == 3 {
                        view.set_block(spike.x + dx, spike.height + dy, spike.z + dz, Block::IronBars0);
                    }
                }
            }
        }
    }
}

/// `EndIslandFeature::place` (144c12ac0): discs of end stone shrinking downward.
fn end_island(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) {
    let mut r = random.next_int(3) as f32 + 4.0;
    let mut dy = 0;
    while r > 0.5 {
        let (lo, hi) = ((-r).floor() as i32, r.ceil() as i32);
        let rr = (r + 1.0) * (r + 1.0);
        for i in lo..=hi {
            for k in lo..=hi {
                if ((i * i + k * k) as f32) <= rr {
                    view.set_block(x + i, y + dy, z + k, Block::EndStone);
                }
            }
        }
        r -= random.next_int(2) as f32 + 0.5;
        dy -= 1;
    }
}

/// North, east, south, west (0x14a681437), as offsets.
const CHORUS_SIDES: [[i32; 2]; 4] = [[0, -1], [1, 0], [0, 1], [-1, 0]];

/// Air on every horizontal side but `except` (148b29240), tested north, east, south, west.
fn sides_empty(view: &View, [x, y, z]: [i32; 3], except: Option<usize>) -> bool {
    CHORUS_SIDES.iter().enumerate().filter(|&(i, _)| Some(i) != except).all(|(_, [dx, dz])| view.block(x + dx, y, z + dz) == Block::Air)
}

/// `ChorusFlowerBlock::growTreeRecursive` (148b29a60): a stem, branches into empty air within `reach` of the root,
/// a flower of age 5 where no branch grew.
fn grow_chorus(view: &mut View, random: &mut Mt19937, at: [i32; 3], root: [i32; 3], reach: u32, depth: i32) {
    let first = i32::from(depth == 0);
    let n = random.next_int(4) + 1 + first;
    for j in 1..=n {
        let p = [at[0], at[1] + j, at[2]];
        if !sides_empty(view, p, None) {
            return;
        }
        view.set_block(p[0], p[1], p[2], Block::ChorusPlant);
    }
    let top = [at[0], at[1] + n, at[2]];
    let mut branched = false;
    if depth < 4 {
        let m = random.next_int(4) + first;
        for _ in 0..m {
            let side = random.next_int(4) as usize;
            let [dx, dz] = CHORUS_SIDES[side];
            let b = [top[0] + dx, top[1], top[2] + dz];
            let near = b[0].abs_diff(root[0]) < reach && b[2].abs_diff(root[2]) < reach;
            if near && view.block(b[0], b[1], b[2]) == Block::Air && view.block(b[0], b[1] - 1, b[2]) == Block::Air && sides_empty(view, b, Some((side + 2) % 4)) {
                branched = true;
                view.set_block(b[0], b[1], b[2], Block::ChorusPlant);
                grow_chorus(view, random, b, root, reach, depth + 1);
            }
        }
    }
    if !branched {
        view.set_block(top[0], top[1], top[2], Block::ChorusFlower5);
    }
}

/// `EndGatewayFeature::place` (144c120f0): the gateway between two infiniburn bedrock caps, air around.
fn end_gateway(view: &mut View, [x, y, z]: [i32; 3]) {
    for dx in -1..=1i32 {
        for dy in -2..=2i32 {
            for dz in -1..=1i32 {
                let centre = dx == 0 && dz == 0;
                let block = match dy.abs() {
                    0 if centre => Block::EndGateway,
                    2 if centre => Block::InfiniburnBedrock,
                    1 if dx == 0 || dz == 0 => Block::InfiniburnBedrock,
                    _ => Block::Air,
                };
                view.set_block(x + dx, y + dy, z + dz, block);
            }
        }
    }
}
