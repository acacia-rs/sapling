use super::*;
use crate::enchant::Draws;

/// Draws that count up from `at`, so that a test sees several of a group's trades.
struct Counting(i32);

impl Draws for Counting {
    fn int(&mut self, bound: i32) -> i32 {
        self.0 += 1;
        if bound < 1 { 0 } else { self.0 % bound }
    }

    fn float(&mut self) -> f32 {
        0.5
    }
}

const PROFESSIONS: [&str; 13] = [
    "farmer", "fisherman", "shepherd", "fletcher", "librarian", "cartographer", "cleric", "armorer", "weapon_smith", "tool_smith", "butcher", "leather_worker", "stone_mason",
];

#[test]
fn every_item_a_table_names_is_one_the_client_knows() {
    let mut unknown = Vec::new();
    for table in TABLES.values() {
        for trade in table.tiers.iter().flat_map(|tier| &tier.groups).flat_map(|group| &group.trades) {
            unknown.extend(trade.wants.iter().chain(&trade.gives).flatten().filter(|item| !crate::is_item(&item.name)).map(|item| item.name.clone()));
        }
    }
    assert!(unknown.is_empty(), "{unknown:?}");
}

#[test]
fn every_profession_has_five_tiers_from_no_experience_up() {
    for name in PROFESSIONS {
        let table = trade_table(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(table.exp_needed(), [0, 10, 70, 150, 250], "{name}");
    }
    assert!(trade_table("wandering_trader").is_some());
}

#[test]
fn a_novice_armorer_buys_coal_and_sells_one_piece_of_iron_armour() {
    let offers = roll(trade_table("armorer").unwrap(), 0, 0, &mut Counting(0));
    assert_eq!(offers.len(), 2, "{offers:?}");
    let coal = &offers[0];
    assert_eq!((coal.buy_a.name.as_str(), coal.buy_a.count, coal.sell.name.as_str(), coal.sell.count), ("minecraft:coal", 15, "minecraft:emerald", 1));
    assert_eq!((coal.trader_exp, coal.max_uses, coal.reward_exp, coal.buy_a.price_multiplier), (2, 16, true, 0.05));
    assert!(offers[1].sell.name.starts_with("minecraft:iron_"), "{offers:?}");
}

#[test]
fn a_librarians_book_is_enchanted_and_its_emeralds_are_its_price() {
    let table = trade_table("librarian").unwrap();
    let book = (0..20).flat_map(|seed| roll(table, 0, 0, &mut Counting(seed))).find(|offer| offer.sell.name == "minecraft:enchanted_book").expect("a book among twenty draws");
    assert!(crate::enchant::all(book.sell.tag.as_ref()).len() == 1, "{book:?}");
    assert_eq!(book.buy_a.name, "minecraft:emerald");
    assert!((5..=64).contains(&book.buy_a.count), "{book:?}");
    assert_eq!(book.buy_b.as_ref().map(|book| book.name.as_str()), Some("minecraft:book"));
}

#[test]
fn a_fisherman_wants_the_boat_of_its_biome() {
    let table = trade_table("fisherman").unwrap();
    let boats = |mark: i32| -> Vec<String> {
        (0..10).flat_map(|seed| roll(table, 4, mark, &mut Counting(seed))).filter_map(|offer| offer.buy_a.name.ends_with("_boat").then_some(offer.buy_a.name)).collect()
    };
    assert!(boats(0).iter().all(|boat| boat == "minecraft:oak_boat") && !boats(0).is_empty());
    assert!(boats(6).iter().all(|boat| boat == "minecraft:spruce_boat") && !boats(6).is_empty());
}

#[test]
fn a_wandering_trader_offers_from_each_of_its_groups() {
    let offers = roll(trade_table("wandering_trader").unwrap(), 0, 0, &mut Counting(0));
    assert!(offers.len() >= 6, "{offers:?}");
    assert!(roll(trade_table("wandering_trader").unwrap(), 1, 0, &mut Counting(0)).is_empty(), "one tier only");
}

/// Draws that hand out what they were given, in order.
struct Scripted(Vec<i32>, Vec<f32>);

impl Draws for Scripted {
    fn int(&mut self, _bound: i32) -> i32 {
        self.0.remove(0)
    }

    fn float(&mut self) -> f32 {
        self.1.remove(0)
    }
}

#[test]
fn random_dye_mixes_one_to_three_dyes_as_bds_does() {
    // Draw 3 lands on black; a second dye below 0.3, a third below 0.2.
    assert_eq!(roll::random_dye(&mut Scripted(vec![3], vec![0.5, 0.5])), 0xFF1D_1D21);
    assert_eq!(roll::random_dye(&mut Scripted(vec![0], vec![0.5, 0.5])), 0xFFF0_F0F0, "BDS's white");
    assert_eq!(roll::random_dye(&mut Scripted(vec![0, 3], vec![0.1, 0.5])), 0xFF86_8688, "white and black, averaged and cut");
    assert_eq!(roll::random_dye(&mut Scripted(vec![0, 0, 0], vec![0.1, 0.1])), 0xFFF0_F0F0);
}

#[test]
fn drawn_trades_come_in_the_order_drawn() {
    let table = trade_table("wandering_trader").unwrap();
    let group = table.tiers[0].groups.iter().find(|group| group.select > 1 && (group.select as usize) < group.trades.len()).expect("a group drawn from");
    let weights: i32 = group.trades.iter().map(|trade| trade.weight as i32).sum();
    // The first draw lands on the last trade, which then leads.
    let picked = roll::pick(&group.trades, group.select, &mut Scripted(vec![weights - 1; group.select as usize], Vec::new()));
    assert_eq!(picked[0], group.trades.len() - 1, "{picked:?}");
    assert_eq!(picked.len(), group.select as usize);
    assert_eq!(roll::pick(&group.trades, 0, &mut Scripted(Vec::new(), Vec::new())).len(), group.trades.len(), "select 0 keeps all");
}
