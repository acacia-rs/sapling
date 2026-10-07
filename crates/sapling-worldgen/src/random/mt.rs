//! Bedrock's `Random`: MT19937 with a 32-bit seed, used outside the terrain-shape pipeline
//! (bedrock floor, decoration, legacy carvers).

const N: usize = 624;
const M: usize = 397;

#[derive(Clone)]
pub struct Mt19937 {
    state: [u32; N],
    index: usize,
}

impl Mt19937 {
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; N];
        state[0] = seed;
        for i in 1..N {
            state[i] = (state[i - 1] ^ (state[i - 1] >> 30)).wrapping_mul(0x6C07_8965).wrapping_add(i as u32);
        }
        Self { state, index: N }
    }

    fn twist(&mut self) {
        for i in 0..N {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % N] & 0x7FFF_FFFF);
            let mut next = self.state[(i + M) % N] ^ (y >> 1);
            if y & 1 != 0 {
                next ^= 0x9908_B0DF;
            }
            self.state[i] = next;
        }
        self.index = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C_5680;
        y ^= (y << 15) & 0xEFC6_0000;
        y ^ (y >> 18)
    }

    /// The raw draw times 2^-32 in f64, rounded to f32 (141188db0).
    pub fn next_float(&mut self) -> f32 {
        (self.next_u32() as f64 * 2.328_306_436_538_696_3e-10) as f32
    }

    /// `Random::nextInt(n)`: the raw draw modulo n, 0 for n <= 0.
    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 { 0 } else { (self.next_u32() % bound as u32) as i32 }
    }
}
