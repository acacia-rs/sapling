//! `ImprovedNoise::sample_offset_lanes` with AVX2 gathers, chosen at run time; the build stays
//! baseline x86-64. Each lane runs the scalar path's IEEE operations in its order (`round_ps` floors
//! exactly as `f32::floor`, no FMA: the feature is not enabled), so lanes are bit-identical to it
//! (noise/tests.rs).

use std::arch::x86_64::*;
use std::sync::OnceLock;

use super::ImprovedNoise;
use super::improved::{GRADIENT, LANES, Lanes};

/// Per axis, bit i set where gradient i's component is 0, and where it is -1.
const ZERO: [i32; 3] = [gradient_bits(0, 0.0), gradient_bits(1, 0.0), gradient_bits(2, 0.0)];
const NEGATIVE: [i32; 3] = [gradient_bits(0, -1.0), gradient_bits(1, -1.0), gradient_bits(2, -1.0)];

const fn gradient_bits(axis: usize, value: f32) -> i32 {
    let mut bits = 0;
    let mut i = 0;
    while i < 16 {
        if GRADIENT[i][axis] == value {
            bits |= 1 << i;
        }
        i += 1;
    }
    bits
}

/// None without AVX2, or for a coordinate past what an i32 floor holds (the scalar cast saturates).
pub fn sample_offset(noise: &ImprovedNoise, x: &Lanes, y: &Lanes, z: &Lanes) -> Option<Lanes> {
    static AVX2: OnceLock<bool> = OnceLock::new();
    let fits = |v: &Lanes| v.iter().all(|c| c.abs() < 2.0e9);
    // SAFETY: the CPU has AVX2.
    (*AVX2.get_or_init(|| is_x86_feature_detected!("avx2")) && fits(x) && fits(y) && fits(z)).then(|| unsafe { lanes(noise, x, y, z) })
}

#[target_feature(enable = "avx2")]
fn lanes(noise: &ImprovedNoise, x: &Lanes, y: &Lanes, z: &Lanes) -> Lanes {
    let load = |v: &Lanes| unsafe { _mm256_loadu_ps(v.as_ptr()) };
    let (x, y, z) = (load(x), load(y), load(z));
    let floor = |v: __m256| _mm256_cvttps_epi32(_mm256_round_ps::<{ _MM_FROUND_TO_NEG_INF | _MM_FROUND_NO_EXC }>(v));
    let (ix, iy, iz) = (floor(x), floor(y), floor(z));
    let (dx, dy, dz) = (_mm256_sub_ps(x, _mm256_cvtepi32_ps(ix)), _mm256_sub_ps(y, _mm256_cvtepi32_ps(iy)), _mm256_sub_ps(z, _mm256_cvtepi32_ps(iz)));
    let one_i = _mm256_set1_epi32(1);
    // One gather gives hash(i) and hash(i + 1) (`pairs`). SAFETY: indices are masked to 0..256.
    let byte = _mm256_set1_epi32(0xFF);
    let pair = |i: __m256i| {
        let q = unsafe { _mm256_i32gather_epi32::<4>(noise.pairs.as_ptr(), _mm256_and_si256(i, byte)) };
        (_mm256_and_si256(q, byte), _mm256_and_si256(_mm256_srli_epi32::<8>(q), byte))
    };
    let (ha, hb) = pair(ix);
    let (a, b) = (_mm256_add_epi32(ha, iy), _mm256_add_epi32(hb, iy));
    let (aa, ab) = pair(a);
    let (ba, bb) = pair(b);
    let one = _mm256_set1_ps(1.0);
    let (dx1, dy1, dz1) = (_mm256_sub_ps(dx, one), _mm256_sub_ps(dy, one), _mm256_sub_ps(dz, one));
    // A gradient component times a finite p, bit for bit: p, -p (sign flipped), or 0 * p (p's sign alone).
    let sign = _mm256_set1_epi32(i32::MIN);
    let times = |g: __m256i, axis: usize, p: __m256| {
        let flag = |bits: i32| _mm256_cmpeq_epi32(_mm256_and_si256(_mm256_srlv_epi32(_mm256_set1_epi32(bits), g), one_i), one_i);
        let p = _mm256_castps_si256(p);
        let signed = _mm256_xor_si256(p, _mm256_and_si256(flag(NEGATIVE[axis]), sign));
        _mm256_castsi256_ps(_mm256_blendv_epi8(signed, _mm256_and_si256(p, sign), flag(ZERO[axis])))
    };
    let dot = |hash: __m256i, px: __m256, py: __m256, pz: __m256| {
        let g = _mm256_and_si256(hash, _mm256_set1_epi32(15));
        _mm256_add_ps(_mm256_add_ps(times(g, 0, px), times(g, 1, py)), times(g, 2, pz))
    };
    let (h000, h001) = pair(_mm256_add_epi32(aa, iz));
    let (h100, h101) = pair(_mm256_add_epi32(ba, iz));
    let (h010, h011) = pair(_mm256_add_epi32(ab, iz));
    let (h110, h111) = pair(_mm256_add_epi32(bb, iz));
    let d000 = dot(h000, dx, dy, dz);
    let d100 = dot(h100, dx1, dy, dz);
    let d010 = dot(h010, dx, dy1, dz);
    let d110 = dot(h110, dx1, dy1, dz);
    let d001 = dot(h001, dx, dy, dz1);
    let d101 = dot(h101, dx1, dy, dz1);
    let d011 = dot(h011, dx, dy1, dz1);
    let d111 = dot(h111, dx1, dy1, dz1);
    let smoothstep = |t: __m256| {
        let cube = _mm256_mul_ps(_mm256_mul_ps(t, t), t);
        let inner = _mm256_add_ps(_mm256_mul_ps(_mm256_sub_ps(_mm256_mul_ps(t, _mm256_set1_ps(6.0)), _mm256_set1_ps(15.0)), t), _mm256_set1_ps(10.0));
        _mm256_mul_ps(inner, cube)
    };
    let lerp = |t: __m256, a: __m256, b: __m256| _mm256_add_ps(a, _mm256_mul_ps(t, _mm256_sub_ps(b, a)));
    let (sx, sy, sz) = (smoothstep(dx), smoothstep(dy), smoothstep(dz));
    let (x00, x10) = (lerp(sx, d000, d100), lerp(sx, d010, d110));
    let (x01, x11) = (lerp(sx, d001, d101), lerp(sx, d011, d111));
    let out = lerp(sz, lerp(sy, x00, x10), lerp(sy, x01, x11));
    let mut lanes = [0.0f32; LANES];
    // SAFETY: `lanes` holds eight f32.
    unsafe { _mm256_storeu_ps(lanes.as_mut_ptr(), out) };
    lanes
}
