//! The draws of a world's portal making: BDS's `PortalForcer` takes `next & 3` from an MT19937
//! seeded with the world seed's low 32 bits, one draw per portal made (the directions 0, 2, 2, 0
//! of four made in a row, research/captures/bds-portal-makes.txt).

use std::collections::HashMap;

use bevy_ecs::prelude::*;

const N: usize = 624;

pub(crate) struct Mt19937 {
    state: [u32; N],
    index: usize,
}

impl Mt19937 {
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; N];
        state[0] = seed;
        for i in 1..N {
            state[i] = 1_812_433_253u32.wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30)).wrapping_add(i as u32);
        }
        Self { state, index: N }
    }

    pub fn next(&mut self) -> u32 {
        if self.index >= N {
            for i in 0..N {
                let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % N] & 0x7fff_ffff);
                self.state[i] = self.state[(i + 397) % N] ^ (y >> 1) ^ if y & 1 == 1 { 0x9908_b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }
}

/// Each world's portal forcer draws, made at its first portal from its overworld's seed.
#[derive(Resource, Default)]
pub(crate) struct Directions(HashMap<Entity, Mt19937>);

impl Directions {
    /// The direction to try first for the next portal made in the realm of `home`.
    pub fn draw(&mut self, home: Entity, seed: i64) -> i32 {
        (self.0.entry(home).or_insert_with(|| Mt19937::new(seed as u32)).next() & 3) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_draws_bds_made_its_portals_with() {
        let mut reference = Mt19937::new(5489);
        assert_eq!(reference.next(), 3_499_211_612, "the reference generator's first output");
        let mut drill = Mt19937::new(20_261_018);
        assert_eq!([0; 4].map(|_| drill.next() & 3), [0, 2, 2, 0]);
    }
}
