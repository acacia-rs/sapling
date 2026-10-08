//! The contribution kernels of `FeatureTerrainAdjustments`, computed as BDS fills its tables: the
//! legacy beard (145e28450, f32), the v2 beard (initBeardKernel 144cd3800, f64 stored as f32) and
//! the bury kernel (initBuryKernel 144cd3d40, f64 stored as f32).

use std::sync::OnceLock;

const SPAN: usize = 24;

fn index(a: i32, b: i32, c: i32) -> Option<usize> {
    let inside = |v: i32| (-12..12).contains(&v);
    (inside(a) && inside(b) && inside(c)).then(|| (a + 12) as usize * SPAN * SPAN + (b + 12) as usize * SPAN + (c + 12) as usize)
}

/// The legacy beard (145e28350) at (dx, dy, dz) outside a box, 0 beyond 12 blocks.
pub fn legacy_beard(dx: i32, dy: i32, dz: i32) -> f32 {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = vec![0.0; SPAN * SPAN * SPAN];
        for a in 0..SPAN as i32 {
            for b in 0..SPAN as i32 {
                let r = ((b - 12) * (b - 12) + (a - 12) * (a - 12)) as f32;
                let (r16, r2) = (r * 0.0625, r * 0.5);
                for c in 0..SPAN as i32 {
                    let d = ((c - 12) as f64 + 0.5) as f32;
                    let d2 = d * d;
                    let e = std::f32::consts::E.powf(-(d2 * 0.0625 + r16));
                    let q = d2 * 0.5 + r2;
                    let y = f32::from_bits(0x5f37_59df_u32.wrapping_sub(((q.to_bits() as i32) >> 1) as u32));
                    let y = (1.5 - q * 0.5 * y * y) * y;
                    table[index(a - 12, b - 12, c - 12).unwrap()] = -d * y * 0.5 * e;
                }
            }
        }
        table
    });
    index(dz, dx, dy).map_or(0.0, |i| table[i])
}

/// `exp(-((dy + 0.5)^2 + dx^2 + dz^2) / 16)` as the v2 kernel stores it.
fn v2_falloff(dx: i32, dy: i32, dz: i32) -> f32 {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = vec![0.0; SPAN * SPAN * SPAN];
        for a in -12..12 {
            for b in -12..12 {
                for c in -12..12 {
                    let d = c as f64 + 0.5;
                    let s = d * d + (b * b) as f64 + (a * a) as f64;
                    table[index(a, b, c).unwrap()] = std::f64::consts::E.powf(s * -0.0625) as f32;
                }
            }
        }
        table
    });
    table[index(dz, dx, dy).unwrap()]
}

/// The v2 beard (Java's `getBeardContribution`, with BDS's fast inverse square root constant) at
/// (dx, dy, dz), `to_ground` above the structure's ground; 0 beyond 12 blocks.
pub fn v2_beard(dx: i32, dy: i32, dz: i32, to_ground: i32) -> f32 {
    if index(dx, dy, dz).is_none() {
        return 0.0;
    }
    let d = to_ground as f64 + 0.5;
    let q = (d * d + (dx * dx) as f64 + (dz * dz) as f64) * 0.5;
    let y = f64::from_bits(0x5fe6_eb50_c7b5_37a9_u64.wrapping_sub(((q.to_bits() as i64) >> 1) as u64));
    let y = (1.5 - q * 0.5 * y * y) * y;
    (y * -d * 0.5 * v2_falloff(dx, dy, dz) as f64) as f32
}

/// The bury kernel at half-block offsets (a, b, c): `1 - |(a, b, c) / 2| / 6`, clamped.
pub fn bury(a: i32, b: i32, c: i32) -> f32 {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = vec![0.0; SPAN * SPAN * SPAN];
        for a in -12..12 {
            for b in -12..12 {
                for c in -12..12 {
                    let half = |v: i32| (v as f64 * 0.5) * (v as f64 * 0.5);
                    let distance = (half(b) + half(c) + half(a)).sqrt() * 0.166_666_666_666_666_66;
                    table[index(a, b, c).unwrap()] = (1.0 - distance).clamp(0.0, 1.0) as f32;
                }
            }
        }
        table
    });
    index(a, b, c).map_or(0.0, |i| table[i])
}
