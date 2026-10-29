//! BDS's offers for one player's seed, as research/captures/bds-enchants.txt recorded them: the
//! seed (1893008896) was found by searching every 31-bit seed for the fishing rod's draws.

use sapling_data::enchant::table;
use sapling_world::ItemStack;
use sapling_worldgen::random::Mt19937;

use super::{Bedrock, wire};
use crate::inventory::stack;

const SEED: u32 = 1_893_008_896;

/// Per shelves and item: each offer as `cost slot_flags [worn] [held] [itself] "name"`.
const SEEN: [(u32, &str, [&str; 3]); 16] = [
    (2, "minecraft:diamond_sword", [r#"3 16 [] [(9, 1)] [] "fiddle twist shrink ""#, r#"7 16 [] [(9, 2), (17, 1)] [(12, 1)] "snuff destroy dry ""#, r#"10 16 [] [(17, 2)] [] "stretch elder fiddle earth ""#]),
    (0, "minecraft:diamond_sword", [r#"2 16 [] [(9, 1)] [] "fiddle twist shrink ""#, r#"5 16 [] [(9, 1), (17, 1)] [(12, 1)] "snuff destroy dry ""#, r#"7 16 [] [(9, 2)] [] "stretch elder fiddle earth ""#]),
    (2, "minecraft:bow", [r#"3 32 [] [] [(19, 1)] "fiddle twist shrink ""#, r#"7 32 [] [(17, 1)] [(19, 1)] "shorten ball snuff ""#, r#"10 32 [] [] [(19, 2)] "embiggen ignite stretch elder ""#]),
    (0, "minecraft:bow", [r#"2 32 [] [] [(19, 1)] "fiddle twist shrink ""#, r#"5 32 [] [(17, 1)] [(19, 1)] "shorten ball snuff ""#, r#"7 32 [] [] [(19, 1)] "embiggen ignite stretch elder ""#]),
    (2, "minecraft:crossbow", [r#"3 65536 [] [] [(34, 1)] "fiddle twist shrink ""#, r#"7 65536 [] [(17, 1)] [(34, 1)] "shorten ball snuff ""#, r#"10 65536 [] [] [(34, 2)] "embiggen ignite stretch elder ""#]),
    (0, "minecraft:crossbow", [r#"2 65536 [] [] [(34, 1)] "fiddle twist shrink ""#, r#"5 65536 [] [(17, 1)] [(34, 1)] "shorten ball snuff ""#, r#"7 65536 [] [] [(34, 1)] "embiggen ignite stretch elder ""#]),
    (2, "minecraft:book", [r#"3 -1 [] [(9, 1)] [] "fiddle twist shrink ""#, r#"7 -1 [] [(9, 1), (29, 1)] [(19, 1)] "snuff destroy dry ""#, r#"10 -1 [(1, 1)] [] [] "stretch elder fiddle earth ""#]),
    (0, "minecraft:book", [r#"2 -1 [] [(9, 1)] [] "fiddle twist shrink ""#, r#"5 -1 [] [(9, 1), (29, 1)] [(19, 1)] "snuff destroy dry ""#, r#"7 -1 [(2, 1)] [] [] "stretch elder fiddle earth ""#]),
    (2, "minecraft:mace", [r#"3 4194304 [] [(10, 1)] [] "fiddle twist shrink ""#, r#"7 4194304 [] [(13, 1), (39, 1), (17, 1)] [] "snuff destroy dry ""#, r#"10 4194304 [] [(13, 1)] [] "stretch elder fiddle earth ""#]),
    (0, "minecraft:mace", [r#"2 4194304 [] [(10, 1)] [] "fiddle twist shrink ""#, r#"5 4194304 [] [(13, 1), (39, 1), (17, 1)] [] "snuff destroy dry ""#, r#"7 4194304 [] [(13, 1)] [] "stretch elder fiddle earth ""#]),
    (2, "minecraft:fishing_rod", [r#"3 4096 [] [] [] "darkness range fiddle ""#, r#"7 4096 [] [(17, 1)] [] "of the imbue the ""#, r#"10 4096 [] [(17, 1)] [] "fresh darkness twist ""#]),
    (0, "minecraft:fishing_rod", [r#"2 4096 [] [] [] "darkness range fiddle ""#, r#"5 4096 [] [(17, 1)] [] "of the imbue the ""#, r#"7 4096 [] [(17, 1)] [] "fresh darkness twist ""#]),
    (2, "minecraft:wooden_spear", [r#"3 8388608 [] [(9, 1)] [] "fiddle twist shrink ""#, r#"7 8388608 [] [(10, 1)] [] "snuff destroy dry ""#, r#"10 8388608 [] [(10, 2)] [] "stretch elder fiddle earth ""#]),
    (0, "minecraft:wooden_spear", [r#"2 8388608 [] [(9, 1)] [] "fiddle twist shrink ""#, r#"5 8388608 [] [(10, 1)] [] "snuff destroy dry ""#, r#"7 8388608 [] [(10, 1)] [] "stretch elder fiddle earth ""#]),
    (2, "minecraft:golden_helmet", [r#"3 1 [(4, 1)] [] [] "fiddle twist shrink ""#, r#"7 1 [(0, 1)] [(17, 1)] [] "snuff destroy dry ""#, r#"10 1 [(0, 2)] [] [] "stretch elder fiddle earth ""#]),
    (0, "minecraft:golden_helmet", [r#"2 1 [(4, 1)] [] [] "fiddle twist shrink ""#, r#"5 1 [(0, 1)] [(17, 1)] [] "snuff destroy dry ""#, r#"7 1 [(0, 2)] [] [] "stretch elder fiddle earth ""#]),
];

#[test]
fn offers_match_what_bds_drew_from_the_same_seed() {
    let mut wrong = Vec::new();
    for (shelves, name, seen) in SEEN {
        let item = ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag: None };
        let offers = table::offers(shelves, &stack(&item), &mut Bedrock(Mt19937::new(SEED)));
        let made: Vec<String> = offers
            .iter()
            .map(|offer| {
                let option = wire(0, offer);
                let pairs = |list: &[acacia_proto::types::Enchant]| list.iter().map(|enchant| (enchant.id, enchant.level)).collect::<Vec<_>>();
                format!("{} {} {:?} {:?} {:?} {:?}", option.cost, option.slot_flags, pairs(&option.equip_enchants), pairs(&option.held_enchants), pairs(&option.self_enchants), option.name)
            })
            .collect();
        if made != seen {
            wrong.push(format!("{name} with {shelves} shelves: {made:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
