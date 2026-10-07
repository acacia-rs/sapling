//! Stronghold starts (`StrongholdFeature::isFeatureChunk` 148373d00): three at village starts
//! found on a spiral around the origin (1483740e0), plus at most one per 200x200-chunk cell.

use crate::mth;
use crate::random::Mt19937;

/// The first three village starts met on BDS's spiral: from a random angle and a radius of
/// 40-55 chunks, each step scans 16x16 chunks around the point; a hit turns 0.6 pi and moves 8
/// chunks out, a miss turns pi/4 and moves 4 out.
pub fn spiral(level_seed: u32, is_village: &dyn Fn((i32, i32)) -> bool) -> Vec<(i32, i32)> {
    let mut random = Mt19937::new(level_seed);
    let mut angle = random.next_float() * 6.283_185_5;
    let mut radius = random.next_int(16) + 40;
    let mut found = Vec::new();
    for _ in 0..1000 {
        let x = (mth::cos(angle) * radius as f32).floor() as i32;
        let z = (radius as f32 * mth::sin(angle)).floor() as i32;
        let hit = (x - 8..=x + 7).flat_map(|cx| (z - 8..=z + 7).map(move |cz| (cx, cz))).find(|&chunk| is_village(chunk));
        let (step, turn) = match hit {
            Some(chunk) => {
                found.push(chunk);
                (8, 1.884_955_8)
            }
            None => (4, 0.785_398_2),
        };
        if found.len() > 2 {
            break;
        }
        radius += step;
        angle += turn;
    }
    found
}

/// The grid stronghold of a 200x200-chunk cell, if it has one: cells whose centre lies within
/// 10 chunks of the origin have none, nor do chunks that close themselves.
pub fn grid_start(level_seed: u32, (x, z): (i32, i32)) -> Option<(i32, i32)> {
    if x * x + z * z <= 99 {
        return None;
    }
    let (gx, gz) = ((x as f32 / 200.0).floor() as i32, (z as f32 / 200.0).floor() as i32);
    let (centre_x, centre_z) = (gx * 200 + 100, gz * 200 + 100);
    if centre_x * centre_x + centre_z * centre_z <= 99 {
        return None;
    }
    let seed = (centre_z.wrapping_mul(-0x4197_5147)).wrapping_add(centre_x.wrapping_mul(-0x6454_14af)) as u32;
    let mut random = Mt19937::new(seed.wrapping_add(level_seed).wrapping_add(0x05d5_34e7));
    let dx = (random.next_u32() % 100) as i32;
    let dz = random.next_int(100);
    let chance = random.next_float();
    (chance < 0.25).then_some((gx * 200 + dx + 50, gz * 200 + dz + 50))
}
