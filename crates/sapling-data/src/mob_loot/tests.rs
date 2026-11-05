use super::*;

fn always(at: f32) -> impl FnMut() -> f32 {
    move || at
}

fn by_nobody(kind: &str, on_fire: bool, at: f32) -> Vec<(String, u32)> {
    mob_loot(kind, on_fire, Killer::None, &mut always(at))
}

fn names(loot: Vec<(String, u32)>) -> Vec<String> {
    loot.into_iter().map(|(name, _)| name).collect()
}

#[test]
fn every_item_a_table_leaves_is_one_the_client_knows() {
    let mut unknown = Vec::new();
    for table in LOOT["tables"].as_object().unwrap().values() {
        for entry in table["pools"].as_array().into_iter().flatten().flat_map(|pool| pool["entries"].as_array().into_iter().flatten()) {
            let name = entry["name"].as_str().unwrap_or_default();
            if entry["type"] == "item" && !crate::is_item(name) {
                unknown.push(name.to_owned());
            }
        }
    }
    assert!(unknown.is_empty(), "{unknown:?}");
}

#[test]
fn a_chicken_leaves_feathers_by_chance_and_its_meat() {
    assert_eq!(by_nobody("minecraft:chicken", false, 0.0), [("minecraft:chicken".to_owned(), 1)], "no feathers on the lowest roll");
    assert_eq!(by_nobody("minecraft:chicken", false, 0.999), [("minecraft:feather".to_owned(), 2), ("minecraft:chicken".to_owned(), 1)]);
}

#[test]
fn a_mob_on_fire_leaves_its_meat_cooked() {
    assert_eq!(by_nobody("minecraft:chicken", true, 0.0), [("minecraft:cooked_chicken".to_owned(), 1)]);
    assert_eq!(by_nobody("minecraft:pig", true, 0.0), [("minecraft:cooked_porkchop".to_owned(), 1)]);
}

#[test]
fn what_only_a_player_kill_leaves_is_not_left() {
    let zombie = names(by_nobody("minecraft:zombie", false, 0.0));
    assert!(!zombie.iter().any(|name| name == "minecraft:iron_ingot"), "{zombie:?}");
    assert!(by_nobody("minecraft:armor_stand", false, 0.5).is_empty() && by_nobody("minecraft:nothing", false, 0.5).is_empty());
    assert_eq!(by_nobody("minecraft:sheep", false, 0.0), [("minecraft:white_wool".to_owned(), 1), ("minecraft:mutton".to_owned(), 1)]);
}

#[test]
fn a_players_kill_leaves_the_rare_drops_and_looting_adds_to_them() {
    let zombie = names(mob_loot("minecraft:zombie", false, Killer::Player { looting: 0 }, &mut always(0.0)));
    assert!(zombie.iter().any(|name| name == "minecraft:iron_ingot"), "{zombie:?}");
    // The chicken's feathers: 0 to 2, and with Looting III up to 3 more on the highest roll.
    let feathers = |looting: u8| mob_loot("minecraft:chicken", false, Killer::Player { looting }, &mut always(0.999)).into_iter().find(|(name, _)| name == "minecraft:feather").map(|(_, count)| count);
    assert_eq!([feathers(0), feathers(3)], [Some(2), Some(2 + enchant::looted([0.0, 1.0], 3, &mut always(0.999)))]);
    assert!(feathers(3) > feathers(0));
}

#[test]
fn trial_tables_give_a_key_food_and_a_reward() {
    let key = |table: &str| table_loot(table, &mut always(0.5));
    assert_eq!(key("loot_tables/spawners/trial_chamber/key.json"), [("minecraft:trial_key".to_owned(), 1)]);
    assert_eq!(key("loot_tables/spawners/ominous/trial_chamber/key.json"), [("minecraft:ominous_trial_key".to_owned(), 1)]);
    // A count written `1.0` is one.
    assert_eq!(table_loot("loot_tables/spawners/trial_chamber/consumables.json", &mut always(0.0)), [("minecraft:cooked_chicken".to_owned(), 1)]);
    assert!(!table_loot("loot_tables/chests/trial_chambers/reward.json", &mut always(0.1)).is_empty());
    assert!(table_loot("loot_tables/nothing.json", &mut always(0.1)).is_empty());
}

#[test]
fn generated_containers_tables_are_kept() {
    for table in ["loot_tables/chests/simple_dungeon.json", "loot_tables/chests/village/village_plains_house.json", "loot_tables/dispensers/trial_chambers/chamber.json", "loot_tables/pots/trial_chambers/corridor.json", "loot_tables/entities/desert_pyramid_brushable_block.json"] {
        assert!(!table_loot(table, &mut always(0.3)).is_empty(), "{table}");
    }
}

/// `Level::getSpecialMultiplier` (Linux 0xc8a2090): by difficulty, the moon (full on day 0, new on
/// day 4) and the world's first ticks.
#[test]
fn the_special_multiplier_goes_with_difficulty_and_the_moon() {
    let noon = |day: i64| day * 24_000 + 6000;
    let at = |difficulty, time| (special_multiplier(difficulty, time) * 1000.0).round() / 1000.0;
    assert_eq!([0, 1, 2, 3].map(|difficulty| at(difficulty, noon(0))), [0.0, 0.0, 0.625, 1.0]);
    assert_eq!(at(2, noon(4)), 0.375, "normal under a new moon");
    assert_eq!(at(2, 0), 0.125, "normal at the world's first tick");
}

/// skeleton_gear.json drawing nothing but 0: its bow enchanted (`enchant_random_gear` 0.1), then the
/// armour pool on hard picks its tier 0 moved on by all three bonus rolls (chain), every piece's
/// next one after it; on easy no armour.
#[test]
fn a_hard_skeleton_drawing_low_wears_a_chain_set_and_holds_an_enchanted_bow() {
    let hard = gear_loot("loot_tables/entities/skeleton_gear.json", Regional::at(3, 6000), &mut always(0.0));
    let names: Vec<&str> = hard.iter().map(|looted| looted.name.as_str()).collect();
    assert_eq!(names, ["minecraft:bow", "minecraft:chainmail_helmet", "minecraft:chainmail_chestplate", "minecraft:chainmail_leggings", "minecraft:chainmail_boots"]);
    assert!(hard[0].tag.is_some(), "the bow enchanted");
    let easy = gear_loot("loot_tables/entities/skeleton_gear.json", Regional::at(1, 6000), &mut always(0.0));
    assert_eq!(easy.iter().map(|looted| looted.name.as_str()).collect::<Vec<_>>(), ["minecraft:bow"]);
    assert!(easy[0].tag.is_none(), "no multiplier, no enchantment");
}
