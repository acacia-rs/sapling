//! Worldgen randomness: Xoroshiro128++ seeded from the world seed, forked per noise by the MD5 of
//! its name.

mod md5;
#[cfg(test)]
mod tests;
mod xoroshiro;

pub use md5::md5;
pub use xoroshiro::{PositionalFactory, Xoroshiro, position_seed};
