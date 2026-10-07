//! cubiomes' overworld tree (Java's, MIT, Copyright (c) 2020 Cubitect), kept as a reference for
//! the BDS entry list in tests.rs.

mod nodes;
#[allow(dead_code)]
mod params;

use nodes::NODES;
use params::PARAMS;

/// Every leaf: cubiomes' biome id and its six quantized ranges.
pub fn leaves() -> Vec<(u8, [[i64; 2]; 6])> {
    NODES
        .iter()
        .filter(|&&node| node >> 56 == 0xFF)
        .map(|&node| (((node >> 48) & 0xFF) as u8, std::array::from_fn(|i| PARAMS[((node >> (8 * i)) & 0xFF) as usize])))
        .collect()
}
