//! Where a structure may start: BDS's spaced grids. Legacy features seed an MT19937 per grid
//! region (`StructureFeature::getChunkPosInSpace` 141ef95f0); the data-driven ("v2") structure
//! sets use Java's `setLargeFeatureWithSalt` LCG (144cfea60).

use crate::random::{JavaRandom, Mt19937};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spread {
    Linear,
    /// Each axis the mean of two draws (`tiltedSpacing`).
    Triangular,
}

#[derive(Debug, Clone, Copy)]
pub struct Grid {
    pub spacing: i32,
    pub separation: i32,
    pub salt: i32,
    pub spread: Spread,
}

impl Grid {
    pub const fn new(spacing: i32, separation: i32, salt: i32, spread: Spread) -> Self {
        Self { spacing, separation, salt, spread }
    }

    /// The candidate chunk of the region holding `chunk` (legacy MT seeding, 32-bit seed).
    pub fn candidate(&self, level_seed: u32, chunk: (i32, i32)) -> (i32, i32) {
        self.candidate_random(level_seed, chunk).0
    }

    /// The candidate and the generator after its draws: BDS reseeds the blueprint generator it is
    /// given, so a start drawn after grid checks goes on from the last one.
    pub fn candidate_random(&self, level_seed: u32, chunk: (i32, i32)) -> ((i32, i32), Mt19937) {
        let (rx, rz) = (chunk.0.div_euclid(self.spacing), chunk.1.div_euclid(self.spacing));
        let seed = (rx.wrapping_mul(-0x66c6_0af8)).wrapping_add(rz.wrapping_mul(-0x0ea9_a42b)) as u32;
        let mut random = Mt19937::new(seed.wrapping_add(level_seed).wrapping_add(self.salt as u32));
        let range = self.spacing - self.separation;
        let mut draw = || match self.spread {
            Spread::Linear => random.next_int(range),
            Spread::Triangular => (random.next_int(range) + random.next_int(range)) / 2,
        };
        let x = draw();
        let z = draw();
        ((rx * self.spacing + x, rz * self.spacing + z), random)
    }

    /// The candidate chunk with Java's LCG on the full 64-bit seed (v2 `RandomSpreadPlacement`).
    pub fn java_candidate(&self, seed: i64, chunk: (i32, i32)) -> (i32, i32) {
        let (rx, rz) = (chunk.0.div_euclid(self.spacing), chunk.1.div_euclid(self.spacing));
        let mut random = JavaRandom::new(
            (rx as i64)
                .wrapping_mul(341_873_128_712)
                .wrapping_add((rz as i64).wrapping_mul(132_897_987_541))
                .wrapping_add(seed)
                .wrapping_add(self.salt as i64),
        );
        let range = self.spacing - self.separation;
        let mut draw = || match self.spread {
            Spread::Linear => random.next_int(range),
            Spread::Triangular => (random.next_int(range) + random.next_int(range)) / 2,
        };
        let x = draw();
        let z = draw();
        (rx * self.spacing + x, rz * self.spacing + z)
    }
}


/// Mineshafts (`MineshaftFeature::isFeatureChunk` 14831b070): a chunk seed from two `nextInt()`s
/// multiplied, not forced odd, and xored; then one skipped draw, `nextFloat() < 0.004` and
/// `nextInt(80) < max(|x|, |z|)`, both always drawn.
#[derive(Debug, Clone, Copy)]
pub struct MineshaftSeed {
    level_seed: u32,
    a: i32,
    b: i32,
}

impl MineshaftSeed {
    pub fn new(level_seed: u32) -> Self {
        let mut random = Mt19937::new(level_seed);
        let a = random.next_int_unbounded();
        let b = random.next_int_unbounded();
        Self { level_seed, a, b }
    }

    pub fn is_start(&self, chunk: (i32, i32)) -> bool {
        self.random(chunk).0
    }

    /// The mineshaft chance and the generator after its draws, which the start's pieces go on with.
    pub fn random(&self, (x, z): (i32, i32)) -> (bool, Mt19937) {
        let mut random = Mt19937::new((self.b.wrapping_mul(z) ^ self.a.wrapping_mul(x)) as u32 ^ self.level_seed);
        random.next_int_unbounded();
        let chance = random.next_float();
        let roll = random.next_int(80);
        (chance < 0.004 && roll < x.unsigned_abs().max(z.unsigned_abs()) as i32, random)
    }
}
