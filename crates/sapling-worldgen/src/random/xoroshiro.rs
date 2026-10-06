//! Xoroshiro128++ as Java 1.18 `XoroshiroRandomSource` and BDS (14821fa49) use it, with the
//! positional factory that seeds every named noise.

use super::md5::md5;

const GOLDEN_RATIO_64: u64 = 0x9E37_79B9_7F4A_7C15;
const SILVER_RATIO_64: u64 = 0x6A09_E667_F3BC_C909;

fn mix_stafford13(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[derive(Debug, Clone)]
pub struct Xoroshiro {
    lo: u64,
    hi: u64,
}

impl Xoroshiro {
    /// A zero state is replaced by the golden/silver pair, as Java's constructor does.
    pub fn new(lo: u64, hi: u64) -> Self {
        if lo == 0 && hi == 0 { Self { lo: GOLDEN_RATIO_64, hi: SILVER_RATIO_64 } } else { Self { lo, hi } }
    }

    /// Java `RandomSupport.upgradeSeedTo128bit`.
    pub fn from_seed(seed: i64) -> Self {
        let lo = seed as u64 ^ SILVER_RATIO_64;
        let hi = lo.wrapping_add(GOLDEN_RATIO_64);
        Self::new(mix_stafford13(lo), mix_stafford13(hi))
    }

    pub fn next_long(&mut self) -> u64 {
        let (lo, mut hi) = (self.lo, self.hi);
        let value = lo.wrapping_add(hi).rotate_left(17).wrapping_add(lo);
        hi ^= lo;
        self.lo = lo.rotate_left(49) ^ hi ^ (hi << 21);
        self.hi = hi.rotate_left(28);
        value
    }

    pub fn next_int(&mut self) -> i32 {
        self.next_long() as i32
    }

    /// Lemire's bounded draw, Java `XoroshiroRandomSource.nextInt(int)`.
    pub fn next_int_bounded(&mut self, bound: u32) -> i32 {
        let bound = bound as u64;
        let mut product = (self.next_long() & 0xFFFF_FFFF) * bound;
        let mut low = product & 0xFFFF_FFFF;
        if low < bound {
            let threshold = (!bound).wrapping_add(1) as u32 % bound as u32;
            while low < threshold as u64 {
                product = (self.next_long() & 0xFFFF_FFFF) * bound;
                low = product & 0xFFFF_FFFF;
            }
        }
        (product >> 32) as i32
    }

    pub fn next_double(&mut self) -> f64 {
        (self.next_long() >> 11) as f64 * 1.1102230246251565e-16
    }

    /// 24 bits scaled by 2^-24 (BDS multiplies by 5.9604645e-08 in `_prepareHeights`).
    pub fn next_float(&mut self) -> f32 {
        (self.next_long() >> 40) as f32 * 5.960_464_5e-8
    }

    pub fn fork_positional(&mut self) -> PositionalFactory {
        PositionalFactory { lo: self.next_long(), hi: self.next_long() }
    }
}

/// Java `XoroshiroPositionalRandomFactory`.
#[derive(Debug, Clone, Copy)]
pub struct PositionalFactory {
    lo: u64,
    hi: u64,
}

impl PositionalFactory {
    /// Java `fromHashOf`: the MD5 digest read as two big-endian longs, xored into the state.
    pub fn from_hash_of(&self, name: &str) -> Xoroshiro {
        let digest = md5(name.as_bytes());
        let lo = u64::from_be_bytes(digest[..8].try_into().unwrap());
        let hi = u64::from_be_bytes(digest[8..].try_into().unwrap());
        Xoroshiro::new(lo ^ self.lo, hi ^ self.hi)
    }

    pub fn at(&self, x: i32, y: i32, z: i32) -> Xoroshiro {
        Xoroshiro::new(position_seed(x, y, z) as u64 ^ self.lo, self.hi)
    }
}

/// Java `Mth.getSeed(x, y, z)`.
pub fn position_seed(x: i32, y: i32, z: i32) -> i64 {
    // Java multiplies x in int (wrapping), z in long.
    let mut seed = x.wrapping_mul(3_129_871) as i64 ^ (z as i64).wrapping_mul(116_129_781) ^ y as i64;
    seed = seed.wrapping_mul(seed).wrapping_mul(42_317_861).wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}
