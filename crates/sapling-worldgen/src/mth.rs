//! Bedrock's `Mth::sin`/`Mth::cos`: a 65536-entry f32 table, `table[i] = sinf(i / K)`, indexed by
//! `(int)(x * K) & 0xFFFF` with K = 10430.378f (65536/2pi in f32); cos adds 16384 before truncating.

use std::sync::OnceLock;

const K: f32 = 10430.378;

fn table() -> &'static [f32] {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    TABLE.get_or_init(|| (0..65536).map(|i| ((i as f32 / K) as f64).sin() as f32).collect())
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
}
