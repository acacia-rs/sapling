//! Bedrock's `Random`: MT19937 with a 32-bit seed (genrand 141188be0, wrapper vtable 14a7125a0),
//! used outside the terrain-shape pipeline (bedrock floor, surface builders, carvers, decoration).
//! BDS initialises the state lazily; the outputs are standard MT19937.

const N: usize = 624;
const M: usize = 397;

#[derive(Clone)]
pub struct Mt19937 {
    seed: u32,
    state: [u32; N],
    index: usize,
    /// The second value of the last polar-method pair (`+0x9d4`, flag `+0x9e4`).
    gaussian: Option<f32>,
    draws: u32,
}

impl Mt19937 {
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; N];
        state[0] = seed;
        for i in 1..N {
            state[i] = (state[i - 1] ^ (state[i - 1] >> 30)).wrapping_mul(0x6C07_8965).wrapping_add(i as u32);
        }
        Self { seed, state, index: N, gaussian: None, draws: 0 }
    }

    /// Raw draws taken since seeding.
    pub fn draws(&self) -> u32 {
        self.draws
    }

    /// The seed this generator was last seeded with (BDS keeps it at `+8`).
    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        self.draws = self.draws.wrapping_add(1);
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C_5680;
        y ^= (y << 15) & 0xEFC6_0000;
        y ^ (y >> 18)
    }

    /// `nextInt(n)` (141188e50): the raw draw modulo n, 0 for n <= 0.
    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 { 0 } else { (self.next_u32() % bound as u32) as i32 }
    }

    /// `nextIntUnbounded` (140874140): the raw draw shifted right once, never negative.
    pub fn next_int_unbounded(&mut self) -> i32 {
        (self.next_u32() >> 1) as i32
    }

    /// `nextFloat` (141188db0).
    pub fn next_float(&mut self) -> f32 {
        unit(self.next_u32())
    }

    /// `nextDouble` (1408741f0): the raw draw times 2^-32.
    pub fn next_double(&mut self) -> f64 {
        self.next_u32() as f64 * 2.328_306_436_538_696_3e-10
    }

    /// The polar method in f32 (inlined in 1436d7cd0 and 149d61dc0): a pair per two calls, the
    /// first returned, the second kept for the next call.
    pub fn next_gaussian(&mut self) -> f32 {
        if let Some(cached) = self.gaussian.take() {
            return cached;
        }
        loop {
            let a = unit(self.next_u32()) * 2.0 - 1.0;
            let b = unit(self.next_u32()) * 2.0 - 1.0;
            let s = b * b + a * a;
            if s >= 1.0 || s == 0.0 {
                continue;
            }
            let multiplier = (s.ln() * -2.0 / s).sqrt();
            self.gaussian = Some(b * multiplier);
            return a * multiplier;
        }
    }

    /// `nextBoolean` (141188bc0): bit 27 of the raw draw.
    pub fn next_boolean(&mut self) -> bool {
        self.next_u32() & (1 << 27) != 0
    }

    /// `consumeCount` (140874390): skips `count` raw draws.
    pub fn consume_count(&mut self, count: u32) {
        for _ in 0..count {
            self.next_u32();
        }
    }

    fn twist(&mut self) {
        for i in 0..N {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % N] & 0x7FFF_FFFF);
            let mag = if y & 1 == 0 { 0 } else { 0x9908_B0DF };
            self.state[i] = self.state[(i + M) % N] ^ (y >> 1) ^ mag;
        }
        self.index = 0;
    }
}

/// A raw draw as BDS turns it into a float: times 2^-32 in f64, then rounded to f32.
pub fn unit(raw: u32) -> f32 {
    (raw as f64 * 2.328_306_436_538_696_3e-10) as f32
}

/// BDS's "force odd" on a `nextInt()` value: `v / 2 * 2 + 1` (14827f840, 148281d10). `nextInt()`
/// is never negative, so this equals `v | 1`.
pub fn force_odd(value: i32) -> i32 {
    value / 2 * 2 + 1
}

/// The per-chunk seed of legacy large features (carvers, canyons, decoration post-process): the
/// level-seeded generator's first two `nextInt()`s, forced odd, as x and z multipliers.
#[derive(Clone, Copy)]
pub struct LargeFeatureSeed {
    level_seed: u32,
    x_factor: i32,
    z_factor: i32,
}

impl LargeFeatureSeed {
    pub fn new(level_seed: u32) -> Self {
        let mut random = Mt19937::new(level_seed);
        let x_factor = force_odd(random.next_int_unbounded());
        let z_factor = force_odd(random.next_int_unbounded());
        Self { level_seed, x_factor, z_factor }
    }

    pub fn level_seed(&self) -> u32 {
        self.level_seed
    }

    pub fn chunk(&self, chunk_x: i32, chunk_z: i32) -> u32 {
        (chunk_z.wrapping_mul(self.z_factor).wrapping_add(chunk_x.wrapping_mul(self.x_factor))) as u32 ^ self.level_seed
    }
}
