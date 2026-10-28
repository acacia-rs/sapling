use super::*;

/// What BDS put in a chest of `simple_dungeon.json` seed 357601146 on first open, as (slot, item
/// network id, count) in the order it set them (research/captures/bds-loot-probe.txt, chests 0 and 4).
const BDS_DUNGEON: [(u8, i32, u32); 12] =
    [(18, 544, 1), (26, 329, 5), (1, 329, 2), (6, 418, 1), (3, 331, 2), (21, 293, 1), (10, 329, 1), (24, 304, 2), (20, 331, 2), (17, 363, 1), (15, 293, 1), (14, 304, 1)];

#[test]
fn a_seeded_chest_fills_the_slots_bds_fills() {
    let mut random = Mt19937::new(357_601_146);
    let rolled = sapling_data::drawn_table_loot("loot_tables/chests/simple_dungeon.json", &mut Bedrock(&mut random));
    let placed = fill(rolled, (0..27).collect(), &mut random);
    let bds: Vec<_> = BDS_DUNGEON.iter().map(|&(slot, id, count)| (slot, (sapling_data::item_name(id).unwrap_or("?").to_owned(), count))).collect();
    assert_eq!(placed, bds);
}
