use super::EndGenerator;

const SEED: i64 = 20261003;

/// BDS 1.26.5x `/locate structure end_city` in the End of seed 20261003: from (x, z), the block answered.
const CITY_ANSWERS: [(i32, i32, i32, i32); 6] =
    [(0, 0, -888, -888), (1000, 0, 1352, 88), (-1000, 0, -1192, 56), (0, 1000, -216, 1016), (0, -1000, -264, -1224), (1500, 1500, 664, 1640)];

#[test]
fn locate_city_matches_bds() {
    let generator = EndGenerator::new(SEED);
    for (x, z, bx, bz) in CITY_ANSWERS {
        let chunk = generator.locate_city(x, z);
        assert_eq!(chunk.map(|(cx, cz)| (cx * 16 + 8, cz * 16 + 8)), Some((bx, bz)), "from {x} {z}");
    }
}

/// The centres on the radius-42 circle truncate toward zero (Java floors them).
#[test]
fn spike_centres() {
    let centres: Vec<(i32, i32)> = EndGenerator::new(SEED).spikes().iter().map(|s| (s.x, s.z)).collect();
    assert_eq!(centres, [(42, 0), (33, 24), (12, 39), (-12, 39), (-33, 24), (-42, 0), (-33, -24), (-12, -39), (12, -39), (33, -24)]);
}
