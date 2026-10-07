//! Java vectors, from deepslate's tests (which were taken from Java 1.18).

use super::*;

#[test]
fn md5_of_dummy() {
    assert_eq!(md5(b"dummy"), [39, 88, 118, 227, 76, 246, 9, 219, 17, 143, 61, 132, 183, 153, 167, 144]);
    assert_eq!(md5(b""), [0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8, 0x42, 0x7e]);
}

#[test]
fn next_long_matches_java() {
    let mut random = Xoroshiro::from_seed(1);
    let expected: [i64; 10] = [
        -1033667707219518978,
        6451672561743293322,
        -1821890263888393630,
        890086654470169703,
        8094835630745194324,
        2779418831538184155,
        -2153570570747265786,
        2631759950516672506,
        1341645417244425603,
        -2886123833362855573,
    ];
    for value in expected {
        assert_eq!(random.next_long() as i64, value);
    }
}

#[test]
fn next_int_matches_java() {
    let mut random = Xoroshiro::from_seed(1);
    let expected = [1734564350, 836234122, 825264738, -1425890201, 767430484, -2015535141, -606094074, 950360058, 224558467, 916343147];
    for value in expected {
        assert_eq!(random.next_int(), value);
    }
    let mut random = Xoroshiro::from_seed(1);
    assert_eq!(random.next_int_bounded(25), 10);
    assert_eq!(random.next_int_bounded(256), 49);
    assert_eq!(random.next_int_bounded(255), 48);
    assert_eq!(random.next_int_bounded(254), 169);
    assert_eq!(random.next_int_bounded(0x7FFF_FFFF), 383715241);
}

#[test]
fn next_double_and_float_match_java() {
    let mut random = Xoroshiro::from_seed(1);
    let doubles = [0.9439647613102243, 0.34974587038035987, 0.9012351308931007, 0.048251694223845565, 0.4388219188383503];
    for value in doubles {
        assert_eq!(random.next_double(), value);
    }
    let mut random = Xoroshiro::from_seed(1);
    let floats = [0.9439647f32, 0.34974587, 0.9012351, 0.04825169, 0.4388219];
    for value in floats {
        assert_eq!(random.next_float(), value);
    }
}

#[test]
fn fork_positional_matches_java_fork() {
    // Java fork() takes the same two longs that forkPositional() keeps.
    let mut random = Xoroshiro::from_seed(1);
    let factory = random.fork_positional();
    assert_eq!(format!("{factory:?}"), "PositionalFactory { lo: 17413076366490032638, hi: 6451672561743293322 }");
}

#[test]
fn position_seed_matches_mth_get_seed() {
    assert_eq!(position_seed(0, 0, 0), 0);
    // (1*3129871)^0^0 = 3129871; l*l*42317861 + l*11, >> 16
    let l: i64 = 3129871;
    assert_eq!(position_seed(1, 0, 0), l.wrapping_mul(l).wrapping_mul(42317861).wrapping_add(l * 11) >> 16);
}

#[test]
fn mt19937_matches_reference_output() {
    let mut random = Mt19937::new(5489);
    assert_eq!(random.next_u32(), 3_499_211_612);
    let mut random = Mt19937::new(5489);
    assert_eq!((0..10_000).map(|_| random.next_u32()).last(), Some(4_123_659_995));
}
