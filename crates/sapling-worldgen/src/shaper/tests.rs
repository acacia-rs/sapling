use super::*;

#[test]
fn peaks_and_valleys_matches_java() {
    // Java values from deepslate's TerrainShaper test.
    for (weirdness, expected) in [(-1.1, -0.29999998), (-0.7, 0.9000001), (0.0, -1.0), (0.1, -0.6999999), (0.4, 0.19999999), (0.6, 0.8000001)] {
        assert!((peaks_and_valleys(weirdness) - expected).abs() < 1e-6, "{weirdness}");
    }
}

#[test]
fn shaper_hits_spline_knots() {
    let shaper = TerrainShaper::overworld();
    // Deep ocean plateau: continentalness -0.51 is a knot at -0.2222.
    let point = ShaperPoint::new(-0.51, 0.0, 0.0);
    assert!((shaper.raw_offset(&point) - -0.2222).abs() < 1e-6);
    assert_eq!(shaper.factor(&ShaperPoint::new(-0.5, 0.0, 0.0)), 3.95 * 128.0);
    assert_eq!(shaper.jaggedness(&ShaperPoint::new(-0.5, 0.0, 0.0)), 0.0);
}
