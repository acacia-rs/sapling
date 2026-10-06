//! Java 1.18.1 `TerrainShaper.overworld(false)`, which BDS hardcodes (`TerrainShaper::buildOverworld`
//! 148068d20). Structure adapted from cubiomes (`biomenoise.c`) and xpple/cubiomes
//! (`terrainnoise.c`), MIT, Copyright (c) 2020 Cubitect.

use super::spline::{Coordinate, Spline, lerp, peaks_and_valleys};

pub fn offset_spline() -> Spline {
    let land1 = land_spline(-0.15, 0.00, 0.0, 0.1, 0.00, -0.03, false);
    let land2 = land_spline(-0.10, 0.03, 0.1, 0.1, 0.01, -0.03, false);
    let land3 = land_spline(-0.10, 0.03, 0.1, 0.7, 0.01, -0.03, true);
    let land4 = land_spline(-0.05, 0.03, 0.1, 1.0, 0.01, 0.01, true);
    Spline::builder(Coordinate::Continents)
        .point(-1.10, 0.044, 0.0)
        .point(-1.02, -0.2222, 0.0)
        .point(-0.51, -0.2222, 0.0)
        .point(-0.44, -0.12, 0.0)
        .point(-0.18, -0.12, 0.0)
        .point(-0.16, land1.clone(), 0.0)
        .point(-0.15, land1, 0.0)
        .point(-0.10, land2, 0.0)
        .point(0.25, land3, 0.0)
        .point(1.00, land4, 0.0)
        .build()
}

pub fn factor_spline() -> Spline {
    Spline::builder(Coordinate::Continents)
        .point(-0.19, 3.95, 0.0)
        .point(-0.15, erosion_factor(6.25, true), 0.0)
        .point(-0.1, erosion_factor(5.47, true), 0.0)
        .point(0.03, erosion_factor(5.08, true), 0.0)
        .point(0.06, erosion_factor(4.69, false), 0.0)
        .build()
}

pub fn jaggedness_spline() -> Spline {
    Spline::builder(Coordinate::Continents)
        .point(-0.11, 0.0, 0.0)
        .point(0.03, erosion_jaggedness(1.0, 0.5, 0.0, 0.0), 0.0)
        .point(0.65, erosion_jaggedness(1.0, 1.0, 1.0, 0.0), 0.0)
        .build()
}

/// Java `mountainContinentalness`.
fn mountain_continentalness(weirdness: f32, continentalness: f32, threshold: f32) -> f32 {
    let scale = 1.0 - (1.0 - continentalness) * 0.5;
    let shift = 0.5 * (1.0 - continentalness);
    let value = (weirdness + 1.17) * 0.460_829_47 * scale - shift;
    if weirdness < threshold { value.max(-0.2222) } else { value.max(0.0) }
}

fn mountain_ridge_spline(continentalness: f32, saddle: bool) -> Spline {
    let low = mountain_continentalness(-1.0, continentalness, -0.7);
    let high = mountain_continentalness(1.0, continentalness, -0.7);
    let scale = 1.0 - (1.0 - continentalness) * 0.5;
    let zero = 0.5 * (1.0 - continentalness) / (0.460_829_47 * scale) - 1.17;
    let builder = Spline::builder(Coordinate::Ridges);
    if -0.65 < zero && zero < 1.0 {
        let at_065 = mountain_continentalness(-0.65, continentalness, -0.7);
        let at_075 = mountain_continentalness(-0.75, continentalness, -0.7);
        let start_slope = (at_075 - low) / 0.25;
        let at_zero = mountain_continentalness(zero, continentalness, -0.7);
        let end_slope = (high - at_zero) / (1.0 - zero);
        builder
            .point(-1.0, low, start_slope)
            .point(-0.75, at_075, 0.0)
            .point(-0.65, at_065, 0.0)
            .point(zero - 0.01, at_zero, 0.0)
            .point(zero, at_zero, end_slope)
            .point(1.0, high, end_slope)
            .build()
    } else {
        let slope = (high - low) / 2.0;
        let builder = if saddle {
            builder.point(-1.0, low.max(0.2), 0.0).point(0.0, lerp(0.5, low, high), slope)
        } else {
            builder.point(-1.0, low, slope)
        };
        builder.point(1.0, high, slope).build()
    }
}

fn ridge_spline(f: f32, g: f32, h: f32, i: f32, j: f32, k: f32) -> Spline {
    let l = (0.5 * (g - f)).max(k);
    let m = 5.0 * (h - g);
    Spline::builder(Coordinate::Ridges)
        .point(-1.0, f, l)
        .point(-0.4, g, l.min(m))
        .point(0.0, h, m)
        .point(0.4, i, 2.0 * (i - h))
        .point(1.0, j, 0.7 * (j - i))
        .build()
}

fn land_spline(f: f32, g: f32, h: f32, i: f32, j: f32, k: f32, extended: bool) -> Spline {
    let s1 = mountain_ridge_spline(lerp(i, 0.6, 1.5), extended);
    let s2 = mountain_ridge_spline(lerp(i, 0.6, 1.0), extended);
    let s3 = mountain_ridge_spline(i, extended);
    let half = 0.5 * i;
    let s4 = ridge_spline(f - 0.15, half, half, half, i * 0.6, 0.5);
    let s5 = ridge_spline(f, j * i, g * i, half, i * 0.6, 0.5);
    let s6 = ridge_spline(f, j, j, g, h, 0.5);
    let s7 = ridge_spline(f, j, j, g, h, 0.5);
    let s8 = Spline::builder(Coordinate::Ridges).point(-1.0, f, 0.0).point(-0.4, s6.clone(), 0.0).point(0.0, h + 0.07, 0.0).build();
    let s9 = ridge_spline(-0.02, k, k, g, h, 0.0);
    let mut builder = Spline::builder(Coordinate::Erosion)
        .point(-0.85, s1, 0.0)
        .point(-0.7, s2, 0.0)
        .point(-0.4, s3, 0.0)
        .point(-0.35, s4, 0.0)
        .point(-0.1, s5, 0.0)
        .point(0.2, s6, 0.0);
    if extended {
        builder = builder.point(0.4, s7.clone(), 0.0).point(0.45, s8.clone(), 0.0).point(0.55, s8, 0.0).point(0.58, s7, 0.0);
    }
    builder.point(0.7, s9, 0.0).build()
}

/// Java `getErosionFactor`.
fn erosion_factor(value: f32, higher: bool) -> Spline {
    let base = Spline::builder(Coordinate::Weirdness).point(-0.2, 6.3, 0.0).point(0.2, value, 0.0).build();
    let dip = Spline::builder(Coordinate::Weirdness).point(-0.05, 6.3, 0.0).point(0.05, 2.67, 0.0).build();
    let rise = Spline::builder(Coordinate::Weirdness).point(-0.05, 2.67, 0.0).point(0.05, 6.3, 0.0).build();
    let builder = Spline::builder(Coordinate::Erosion)
        .point(-0.6, base.clone(), 0.0)
        .point(-0.5, dip, 0.0)
        .point(-0.35, base.clone(), 0.0)
        .point(-0.25, base.clone(), 0.0)
        .point(-0.1, rise, 0.0)
        .point(0.03, base.clone(), 0.0);
    if higher {
        let weird = Spline::builder(Coordinate::Weirdness).point(0.0, value, 0.0).point(0.1, 0.625, 0.0).build();
        let ridge = Spline::builder(Coordinate::Ridges).point(-0.9, value, 0.0).point(-0.69, weird, 0.0).build();
        builder.point(0.35, value, 0.0).point(0.45, ridge.clone(), 0.0).point(0.55, ridge, 0.0).point(0.62, value, 0.0).build()
    } else {
        let low = Spline::builder(Coordinate::Ridges).point(-0.7, base.clone(), 0.0).point(-0.15, 1.37, 0.0).build();
        let high = Spline::builder(Coordinate::Ridges).point(0.45, base, 0.0).point(0.7, 1.56, 0.0).build();
        builder.point(0.05, high.clone(), 0.0).point(0.4, high, 0.0).point(0.45, low.clone(), 0.0).point(0.55, low, 0.0).point(0.58, value, 0.0).build()
    }
}

fn erosion_jaggedness(high_erosion_high: f32, low_erosion_high: f32, high_erosion_mid: f32, low_erosion_mid: f32) -> Spline {
    let low = ridge_jaggedness(low_erosion_high, low_erosion_mid);
    Spline::builder(Coordinate::Erosion)
        .point(-1.0, ridge_jaggedness(high_erosion_high, high_erosion_mid), 0.0)
        .point(-0.78, low.clone(), 0.0)
        .point(-0.5775, low, 0.0)
        .point(-0.375, 0.0, 0.0)
        .build()
}

fn ridge_jaggedness(high: f32, mid: f32) -> Spline {
    let start = peaks_and_valleys(0.4);
    let end = peaks_and_valleys(0.566_666_66);
    let middle = (start + end) / 2.0;
    let mid_value = if mid > 0.0 { weirdness_jaggedness(mid) } else { Spline::Constant(0.0) };
    let high_value = if high > 0.0 { weirdness_jaggedness(high) } else { Spline::Constant(0.0) };
    Spline::builder(Coordinate::Ridges).point(start, 0.0, 0.0).point(middle, mid_value, 0.0).point(1.0, high_value, 0.0).build()
}

fn weirdness_jaggedness(magnitude: f32) -> Spline {
    Spline::builder(Coordinate::Weirdness).point(-0.01, 0.63 * magnitude, 0.0).point(0.01, 0.3 * magnitude, 0.0).build()
}
