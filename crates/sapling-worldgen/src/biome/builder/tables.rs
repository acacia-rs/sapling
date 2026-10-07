//! The builder's parameter ranges (static initializers near 148068190; quantized from f32 through
//! f64, so -0.45 is -4499, unlike Java's -4500) and biome tables (u16 Bedrock ids on 148230a70's
//! stack, the 1.21.60+ plateau layout); 0 marks "none".

use super::{Range, join, span};

pub fn full() -> Range {
    span(-1.0, 1.0)
}

pub fn temperatures() -> [Range; 5] {
    [span(-1.0, -0.45), span(-0.45, -0.15), span(-0.15, 0.2), span(0.2, 0.55), span(0.55, 1.0)]
}

pub fn humidities() -> [Range; 5] {
    [span(-1.0, -0.35), span(-0.35, -0.1), span(-0.1, 0.1), span(0.1, 0.3), span(0.3, 1.0)]
}

pub fn erosions() -> [Range; 7] {
    [span(-1.0, -0.78), span(-0.78, -0.375), span(-0.375, -0.2225), span(-0.2225, 0.05), span(0.05, 0.45), span(0.45, 0.55), span(0.55, 1.0)]
}

/// Mushroom fields, deep ocean, ocean, coast, inland, near, mid and far inland.
pub struct Continents {
    pub mushroom: Range,
    pub deep_ocean: Range,
    pub ocean: Range,
    pub coast: Range,
    pub inland: Range,
    pub near: Range,
    pub mid: Range,
    pub far: Range,
}

pub fn continents() -> Continents {
    Continents {
        mushroom: span(-1.2, -1.05),
        deep_ocean: span(-1.05, -0.455),
        ocean: span(-0.455, -0.19),
        coast: span(-0.19, -0.11),
        inland: span(-0.11, 0.55),
        near: span(-0.11, 0.03),
        mid: span(0.03, 0.3),
        far: span(0.3, 1.0),
    }
}

pub fn frozen() -> Range {
    temperatures()[0]
}

pub fn unfrozen() -> Range {
    join(temperatures()[1], temperatures()[4])
}

/// The weirdness slices of `addInlandBiomes`, in order, with what each one adds.
pub fn weirdness_slices() -> [(Range, Slice); 13] {
    use Slice::*;
    [
        (span(-1.0, -0.933_333_3), Mid),
        (span(-0.933_333_3, -0.766_666_7), High),
        (span(-0.766_666_7, -0.566_666_6), Peaks),
        (span(-0.566_666_6, -0.4), High),
        (span(-0.4, -0.266_666_68), Mid),
        (span(-0.266_666_68, -0.05), Low),
        (span(-0.05, 0.05), Valleys),
        (span(0.05, 0.266_666_68), Low),
        (span(0.266_666_68, 0.4), Mid),
        (span(0.4, 0.566_666_6), High),
        (span(0.566_666_6, 0.766_666_7), Peaks),
        (span(0.766_666_7, 0.933_333_3), High),
        (span(0.933_333_3, 1.0), Mid),
    ]
}

#[derive(Clone, Copy)]
pub enum Slice {
    Mid,
    High,
    Peaks,
    Low,
    Valleys,
}

pub const MUSHROOM_FIELDS: u32 = 14;
pub const STONY_SHORE: u32 = 25;
pub const SWAMP: u32 = 6;
pub const MANGROVE_SWAMP: u32 = 191;
pub const RIVER: u32 = 7;
pub const FROZEN_RIVER: u32 = 11;
pub const BEACH: u32 = 16;
pub const SNOWY_BEACH: u32 = 26;
pub const DESERT: u32 = 2;
pub const BADLANDS: u32 = 37;
pub const ERODED_BADLANDS: u32 = 165;
pub const WOODED_BADLANDS: u32 = 38;
pub const WINDSWEPT_SAVANNA: u32 = 163;
pub const JAGGED_PEAKS: u32 = 182;
pub const FROZEN_PEAKS: u32 = 183;
pub const STONY_PEAKS: u32 = 189;
pub const SNOWY_SLOPES: u32 = 184;
pub const GROVE: u32 = 185;
pub const DRIPSTONE_CAVES: u32 = 188;
pub const LUSH_CAVES: u32 = 187;
pub const SULFUR_CAVES: u32 = 194;
pub const DEEP_DARK: u32 = 190;

pub const OCEANS: [[u32; 5]; 2] = [[47, 45, 24, 43, 40], [46, 44, 0, 42, 40]];
pub const MIDDLE: [[u32; 5]; 5] = [[12, 12, 12, 30, 5], [1, 1, 4, 5, 160], [132, 1, 4, 27, 29], [35, 35, 4, 21, 21], [2, 2, 2, 2, 2]];
/// [1][0] is 195 (dappled forest) in BDS's table; the biome is not registered without its
/// experiment, so the lookup yields none.
pub const MIDDLE_VARIANT: [[u32; 5]; 5] = [[140, 0, 30, 0, 0], [0, 0, 0, 0, 32], [129, 0, 0, 155, 0], [0, 0, 1, 23, 48], [0; 5]];
pub const PLATEAU: [[u32; 5]; 5] = [[12, 12, 12, 30, 30], [186, 186, 4, 5, 160], [186, 186, 186, 186, 193], [36, 36, 4, 4, 21], [37, 37, 37, 38, 38]];
pub const PLATEAU_VARIANT: [[u32; 5]; 5] = [[140, 0, 0, 0, 0], [192, 0, 186, 186, 32], [192, 192, 4, 27, 0], [0; 5], [165, 165, 0, 0, 0]];
pub const SHATTERED: [[u32; 5]; 5] = [[131, 131, 3, 34, 34], [131, 131, 3, 34, 34], [3, 3, 3, 34, 34], [0; 5], [0; 5]];
