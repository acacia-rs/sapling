//! Bedrock's `Mth::sin`/`Mth::cos`: a 65536-entry f32 table (0x14cc495f8), `table[i] = sinf(i / K)` with the
//! UCRT's `sinf`, indexed by `(int)(x * K) & 0xFFFF` with K = 10430.378f (65536/2pi in f32); cos adds 16384 before
//! truncating.

use std::sync::OnceLock;

const K: f32 = 10430.378;

/// Where the UCRT's `sinf` is not the correctly rounded sine: 85 entries one ulp off, read from a live Windows
/// BDS's table (all 65536 entries otherwise equal the rounded sine).
const UCRT: [(u16, u32); 85] = [
    (7079, 0x3f20_b5d3), (7153, 0x3f22_1eb4), (7287, 0x3f24_a6dd), (7326, 0x3f25_6234), (7378, 0x3f26_5b12), (7495, 0x3f28_8724),
    (7704, 0x3f2c_5ae8), (7720, 0x3f2c_a530), (7836, 0x3f2e_bc9b), (7859, 0x3f2f_261c), (7861, 0x3f2f_2f46), (7938, 0x3f30_8ed5),
    (7973, 0x3f31_2dd2), (8002, 0x3f31_b12a), (8036, 0x3f32_4ab5), (8049, 0x3f32_8549), (8108, 0x3f33_8e41), (8160, 0x3f34_7690),
    (8185, 0x3f34_e5d8), (8190, 0x3f34_fc11), (8289, 0x3f36_b1e6), (24594, 0x3f34_b4e7), (24647, 0x3f33_c86f), (24662, 0x3f33_854a),
    (24714, 0x3f32_9bcc), (24835, 0x3f30_7814), (24902, 0x3f2f_462b), (24920, 0x3f2e_f3ac), (24939, 0x3f2e_9c73), (25014, 0x3f2d_42ae),
    (25023, 0x3f2d_1909), (25296, 0x3f28_1a3e), (25301, 0x3f28_028d), (25341, 0x3f27_449b), (25458, 0x3f25_1568), (25536, 0x3f23_9da9),
    (25964, 0x3f1b_66b5), (26387, 0x3f13_067e), (26401, 0x3f12_be75), (26444, 0x3f11_e0bf), (27232, 0x3f01_9573), (39040, 0xbf10_d3cf),
    (39157, 0xbf13_2fa7), (39952, 0xbf22_b545), (40095, 0xbf25_6702), (40128, 0xbf26_050d), (40140, 0xbf26_3e69), (40211, 0xbf27_90ab),
    (40243, 0xbf28_2879), (40417, 0xbf2b_5ac9), (40431, 0xbf2b_9c17), (40487, 0xbf2c_a08d), (40555, 0xbf2d_db1e), (40566, 0xbf2e_0dd2),
    (40598, 0xbf2e_a10d), (40617, 0xbf2e_f844), (40718, 0xbf30_c568), (40846, 0xbf33_07b7), (40912, 0xbf34_2f36), (40923, 0xbf34_6047),
    (40924, 0xbf34_64bc), (40946, 0xbf34_c6b7), (40949, 0xbf34_d411), (57295, 0xbf35_de1f), (57458, 0xbf33_07b1), (57460, 0xbf32_feb6),
    (57543, 0xbf31_886e), (57549, 0xbf31_6d45), (57573, 0xbf31_006f), (57644, 0xbf2f_bd1a), (57751, 0xbf2d_d1df), (57802, 0xbf2c_e617),
    (57805, 0xbf2c_d831), (57826, 0xbf2c_76c1), (57831, 0xbf2c_5f8a), (57893, 0xbf2b_3ebe), (57926, 0xbf2a_a466), (58144, 0xbf26_9de3),
    (58156, 0xbf26_649c), (58316, 0xbf23_63a2), (58550, 0xbf1e_ed59), (58633, 0xbf1d_5334), (58707, 0xbf1b_e36a), (58854, 0xbf19_02d7),
    (60079, 0xbeff_d0d2),
];

fn table() -> &'static [f32] {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table: Vec<f32> = (0..65536).map(|i| ((i as f32 / K) as f64).sin() as f32).collect();
        UCRT.iter().for_each(|&(i, bits)| table[i as usize] = f32::from_bits(bits));
        table
    })
}

/// Truncates toward zero and saturates, as x64's `cvttss2si` does not (it yields i32::MIN); only
/// NaN or |x| > 2e5 radians differ, which no feature reaches.
fn at(index: f32) -> f32 {
    table()[(index as i32 as u32 & 0xFFFF) as usize]
}

pub fn sin(x: f32) -> f32 {
    at(x * K)
}

pub fn cos(x: f32) -> f32 {
    at(x * K + 16384.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn quarter_turns() {
        assert_eq!(super::sin(0.0), 0.0);
        assert!((super::sin(std::f32::consts::FRAC_PI_2) - 1.0).abs() < 1e-6);
        assert!((super::cos(0.0) - 1.0).abs() < 1e-6);
        assert!((super::sin(-1.0) + 0.841_470_96).abs() < 2e-4);
    }

    #[test]
    fn ucrt_entries_are_one_ulp_from_the_rounded_sine() {
        for &(i, bits) in &super::UCRT {
            let rounded = ((i as f32 / super::K) as f64).sin() as f32;
            assert_eq!(rounded.to_bits().abs_diff(bits), 1, "entry {i}");
        }
    }
}
