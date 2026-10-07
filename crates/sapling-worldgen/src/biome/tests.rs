//! BDS's entry list against cubiomes' Java tree: the same leaves, up to BDS's f64 quantization.

use super::btree;
use super::builder::overworld_entries;
use super::ids::bedrock_id;

#[test]
fn entries_are_the_java_leaves() {
    let mut ours: Vec<(u32, [[i64; 2]; 6])> = overworld_entries()
        .iter()
        .map(|e| (e.biome, std::array::from_fn(|i| [e.space[i].0, e.space[i].1])))
        .collect();
    // cubiomes quantizes in f32 (-0.45 to -4500), BDS through f64 (-4499): compare to the nearest 5.
    let snap = |v: i64| -> i64 { (v as f64 / 5.0).round() as i64 * 5 };
    for (_, space) in ours.iter_mut() {
        for range in space.iter_mut() {
            *range = range.map(snap);
        }
    }
    let mut java: Vec<(u32, [[i64; 2]; 6])> =
        btree::leaves().into_iter().map(|(b, s)| (bedrock_id(b).unwrap_or(9999), s.map(|r| r.map(snap)))).collect();
    ours.sort();
    java.sort();
    let missing: Vec<_> = java.iter().filter(|l| ours.binary_search(l).is_err()).take(10).collect();
    let extra: Vec<_> = ours.iter().filter(|l| java.binary_search(l).is_err()).take(10).collect();
    assert!(missing.is_empty() && extra.is_empty() && ours.len() == java.len(), "{} vs {}\nmissing {missing:?}\nextra {extra:?}", ours.len(), java.len());
}
