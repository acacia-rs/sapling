use acacia_proto::nbt::Value;

use super::anvil::{self, Repair};
use super::grind::grind;
use super::table::{self, costs, offers};
use super::*;
use crate::items::Stack;

/// Draws that give back a script, then zeros.
struct Script(Vec<i32>);

impl Draws for Script {
    fn int(&mut self, bound: i32) -> i32 {
        if self.0.is_empty() { 0 } else { self.0.remove(0).min(bound - 1).max(0) }
    }

    fn float(&mut self) -> f32 {
        0.5
    }
}

fn stack<'a>(name: &'a str, tag: Option<&'a Value>) -> Stack<'a> {
    Stack { name, count: 1, damage: 0, block: None, tag }
}

fn tagged(levels: &[(i16, u8)], fields: &[(&str, i32)]) -> Value {
    let base = Value::Compound(fields.iter().map(|(key, value)| ((*key).into(), Value::Int(*value))).collect());
    with_all(Some(base), levels).unwrap_or(Value::Compound(Vec::new()))
}

#[test]
fn the_tables_three_costs_follow_the_shelves() {
    // base = draw(8) + 1 + shelves / 2 + draw(shelves + 1)
    assert_eq!(costs(15, &mut Script(vec![7, 15])), [10, 21, 30]);
    assert_eq!(costs(0, &mut Script(Vec::new())), [1, 1, 1], "each at least 1");
    assert_eq!(costs(40, &mut Script(Vec::new())), [2, 6, 30], "fifteen count at most");
}

#[test]
fn an_offer_is_named_with_three_or_four_words() {
    let offered = offers(0, &stack("minecraft:iron_pickaxe", None), &mut Script(Vec::new()));
    assert!(offered.iter().all(|offer| offer.name.split(' ').count() == 3 && offer.name.starts_with("the the")), "{offered:?}");
    assert!(offers(0, &stack("minecraft:shears", None), &mut Script(Vec::new())).is_empty(), "shears take nothing from the table");
}

#[test]
fn the_table_offers_for_a_sword_and_not_for_an_enchanted_one() {
    let offered = offers(15, &stack("minecraft:diamond_sword", None), &mut Script(vec![7, 15, 7, 15, 7, 15]));
    assert_eq!(offered.iter().map(|offer| offer.cost).collect::<Vec<_>>(), [10, 21, 30]);
    assert!(offered.iter().all(|offer| offer.enchants.iter().all(|(id, _)| kinds::kind_of(*id).is_some_and(|kind| kind.goes_on(crate::gear::slot::SWORD)))));
    let sharp = tagged(&[(9, 1)], &[]);
    assert!(offers(15, &stack("minecraft:diamond_sword", Some(&sharp)), &mut Script(Vec::new())).is_empty());
    let book = table::enchanted(&stack("minecraft:book", None), &offered[0]).unwrap();
    assert_eq!(book.name, "minecraft:enchanted_book");
    assert_eq!([table::lapis(2), table::levels_taken(2) as u8], [3, 3]);
}

#[test]
fn shelves_count_in_a_ring_two_out_on_two_layers() {
    let spots: Vec<_> = table::shelf_spots().collect();
    assert_eq!(spots.len(), 32);
    assert!(spots.contains(&([2, 1, -1], [[1, 0, -1], [1, 1, -1]])) && spots.contains(&([-2, 0, 0], [[-1, 0, 0], [-1, 1, 0]])));
}

#[test]
fn an_anvil_repairs_a_quarter_a_unit_a_level_each() {
    let worn = tagged(&[], &[("Damage", 200)]);
    let ingots = Stack { count: 9, ..stack("minecraft:iron_ingot", None) };
    let Repair { made, cost, material } = anvil::repair(&stack("minecraft:iron_pickaxe", Some(&worn)), Some(&ingots), None).unwrap();
    assert_eq!((cost, material), (4, 4));
    assert_eq!((made.tag.as_ref().and_then(|tag| tag.get("Damage")), made.tag.as_ref().and_then(|tag| tag.get("RepairCost"))), (None, Some(&Value::Int(1))));
    assert!(anvil::repair(&stack("minecraft:iron_pickaxe", None), Some(&ingots), None).is_none(), "nothing to mend");
    assert!(anvil::repair(&stack("minecraft:iron_pickaxe", Some(&worn)), Some(&stack("minecraft:diamond", None)), None).is_none());
}

#[test]
fn an_anvil_adds_prior_work_and_caps_a_rename() {
    let worked = tagged(&[], &[("RepairCost", 3)]);
    let renamed = anvil::repair(&stack("minecraft:iron_sword", Some(&worked)), None, Some("Edge")).unwrap();
    assert_eq!(renamed.cost, 4);
    assert_eq!(anvil::custom_name(renamed.made.tag.as_ref()), Some("Edge"));
    assert_eq!(renamed.made.tag.as_ref().and_then(|tag| tag.get("RepairCost")), Some(&Value::Int(3)), "a rename alone adds no work");
    let much = tagged(&[], &[("RepairCost", 63)]);
    assert_eq!(anvil::repair(&stack("minecraft:iron_sword", Some(&much)), None, Some("Edge")).unwrap().cost, 39);
    assert!(anvil::repair(&stack("minecraft:iron_sword", None), None, None).is_none(), "nothing to do");
    assert_eq!([anvil::more_work(0), anvil::more_work(1), anvil::more_work(3)], [1, 3, 7]);
    assert!(anvil::allowed(39, false) && !anvil::allowed(40, false) && anvil::allowed(40, true));
}

#[test]
fn an_anvil_raises_a_level_only_below_its_most_and_will_not_enchant_a_stack() {
    let [sharp_five, book] = [tagged(&[(9, 5)], &[]), tagged(&[(9, 5)], &[])];
    let input = stack("minecraft:iron_sword", Some(&sharp_five));
    assert!(anvil::repair(&input, Some(&stack("minecraft:enchanted_book", Some(&book))), None).is_none(), "nothing gained");
    let books = Stack { count: 2, ..stack("minecraft:book", None) };
    let unbreaking = tagged(&[(17, 1)], &[]);
    assert_eq!(anvil::repair(&books, Some(&stack("minecraft:enchanted_book", Some(&unbreaking))), None).unwrap().cost, anvil::TOO_EXPENSIVE);
}

#[test]
fn an_anvil_merges_enchantments_and_charges_by_rarity() {
    let sharp = tagged(&[(9, 3)], &[]);
    let book = tagged(&[(9, 3), (17, 2), (10, 1)], &[]);
    let repair = anvil::repair(&stack("minecraft:iron_sword", Some(&sharp)), Some(&stack("minecraft:enchanted_book", Some(&book))), None).unwrap();
    // Only what is gained: Sharpness III to IV at 1, Unbreaking II at 2 halved off a book; Smite clashes, free.
    assert_eq!(repair.cost, 1 + 2);
    assert_eq!(all(repair.made.tag.as_ref()), [(9, 4), (17, 2)]);
}

#[test]
fn a_grindstone_keeps_curses_and_gives_back_their_power() {
    let sword = tagged(&[(9, 3), (28, 1)], &[("RepairCost", 3)]);
    let ground = grind(Some(stack("minecraft:diamond_sword", Some(&sword))), None).unwrap();
    assert_eq!(all(ground.made.tag.as_ref()), [(28, 1)]);
    assert_eq!((ground.work, ground.made.tag.as_ref().and_then(|tag| tag.get("RepairCost"))), (1, Some(&Value::Int(1))), "a curse kept is work done");
    // Sharpness III wants 23: from 12 to 23 back.
    assert_eq!(ground.experience, [12, 23]);
    let book = tagged(&[(17, 1)], &[]);
    assert_eq!(grind(Some(stack("minecraft:enchanted_book", Some(&book))), None).unwrap().made.name, "minecraft:book");
    assert!(grind(Some(stack("minecraft:diamond_sword", None)), None).is_none(), "nothing to grind");
    let [a, b] = [tagged(&[], &[("Damage", 1000)]), tagged(&[], &[("Damage", 1000)])];
    let joined = grind(Some(stack("minecraft:diamond_sword", Some(&a))), Some(stack("minecraft:diamond_sword", Some(&b)))).unwrap();
    // 561 + 561 + 78 of 1561 left.
    assert_eq!(joined.made.tag.as_ref().and_then(|tag| tag.get("Damage")), Some(&Value::Int(361)));
    assert!(grind(Some(stack("minecraft:diamond_sword", None)), Some(stack("minecraft:iron_sword", None))).is_none());
}
