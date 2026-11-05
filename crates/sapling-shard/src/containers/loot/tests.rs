use super::*;

/// What BDS put in a chest of `simple_dungeon.json` seed 357601146 on first open, as (slot, item
/// network id, count) in the order it set them (research/captures/bds-loot-probe.txt, chests 0 and 4).
const BDS_DUNGEON: [(u8, i32, u32); 12] =
    [(18, 544, 1), (26, 329, 5), (1, 329, 2), (6, 418, 1), (3, 331, 2), (21, 293, 1), (10, 329, 1), (24, 304, 2), (20, 331, 2), (17, 363, 1), (15, 293, 1), (14, 304, 1)];

#[test]
fn a_seeded_chest_fills_the_slots_bds_fills() {
    let mut random = Mt19937::new(357_601_146);
    let rolled = sapling_data::drawn_table_loot("loot_tables/chests/simple_dungeon.json", &mut Bedrock(&mut random));
    let placed: Vec<_> = fill(rolled, (0..27).collect(), &mut random).into_iter().map(|(slot, looted)| (slot, (looted.name, looted.count))).collect();
    let bds: Vec<_> = BDS_DUNGEON.iter().map(|&(slot, id, count)| (slot, (sapling_data::item_name(id).unwrap_or("?").to_owned(), count))).collect();
    assert_eq!(placed, bds);
}

/// BDS's slots, item ids and counts for dungeon chests of seeds 5 and 73, each with an enchanted
/// book, rolled third and first (research/captures/bds-loot-books.txt, chests 0 and 4).
const BDS_BOOKS: [(u32, &[(u8, i32, u32)]); 2] = [
    (5, &[(25, 304, 1), (10, 277, 1), (6, 277, 4), (20, 542, 1), (23, 558, 1), (9, 331, 1), (26, 329, 1), (2, 531, 1)]),
    (
        73,
        &[(2, 376, 1), (26, 531, 1), (13, 295, 1), (20, 329, 1), (17, 384, 1), (11, 384, 4), (23, 331, 1), (24, 331, 2), (8, 331, 1), (16, 295, 1), (22, 295, 1), (1, 277, 2), (18, 363, 1), (10, 295, 1), (4, 277, 4), (3, 329, 3)],
    ),
];

#[test]
fn a_dungeon_book_draws_its_cost_and_the_chest_stays_in_step() {
    for (seed, bds) in BDS_BOOKS {
        let placed = rolled("loot_tables/chests/simple_dungeon.json", seed, (0..27).collect());
        let book = placed.iter().find(|(_, looted)| looted.name == "minecraft:enchanted_book").expect("a book");
        assert!(book.1.tag.as_ref().is_some_and(|tag| tag.get("ench").is_some()), "{book:?}");
        let placed: Vec<_> = placed.into_iter().map(|(slot, looted)| (slot, (looted.name, looted.count))).collect();
        let bds: Vec<_> = bds.iter().map(|&(slot, id, count)| (slot, (sapling_data::item_name(id).unwrap_or("?").to_owned(), count))).collect();
        assert_eq!(placed, bds, "seed {seed}");
    }
}

#[test]
fn a_seeded_pot_leaves_what_bds_left() {
    // The pot of the probe's row, broken in survival: BDS dropped the pot and item 307, one of it.
    let left: Vec<_> = rolled("loot_tables/pots/trial_chambers/corridor.json", 1_434_522_187, vec![0]).into_iter().map(|(slot, looted)| (slot, (looted.name, looted.count))).collect();
    assert_eq!(left, [(0, (sapling_data::item_name(307).unwrap_or("?").to_owned(), 1))]);
}
