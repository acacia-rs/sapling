//! What the vanilla packs say of raising each kind (`data/husbandry.tsv`, README "Husbandry"): what
//! it breeds on and with, how a baby grows and what speeds it, what tames it or calms it, what heals
//! it, what it lays, whether it follows its owner, sits and defends it, takes a lead or follows food,
//! and the component groups its events swap.

use std::collections::HashMap;
use std::sync::LazyLock;

/// The component groups an event takes away and gives, BDS's `definitions` entries without their `+`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Swap {
    pub removed: Vec<&'static str>,
    pub added: Vec<&'static str>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Husbandry {
    /// Items that put a grown one in love (`minecraft:breedable`).
    pub breed: Vec<&'static str>,
    /// The kinds it breeds with.
    pub mates: Vec<&'static str>,
    /// It breeds only tamed, and only at full health.
    pub breed_tame: bool,
    pub breed_healthy: bool,
    /// The ticks a baby takes to grow up, and what feeding it each item takes off, as a share of them.
    pub grows: Option<u32>,
    pub grow: Vec<(&'static str, f32)>,
    /// What tames a wild one (`minecraft:tameable`), and the chance each feeding does.
    pub tame: Vec<&'static str>,
    pub tame_chance: f32,
    /// A tamed one's full health and its bite, where its tamed groups change them.
    pub tame_health: Option<f32>,
    pub tame_attack: Option<f32>,
    /// A mount's temper (`minecraft:tamemount`): the least and most, and what each item adds.
    pub temper: Option<(i32, i32)>,
    pub temper_feed: Vec<(&'static str, i32)>,
    /// What heals it and by how much (`minecraft:healable`).
    pub heal: Vec<(&'static str, f32)>,
    /// What a grown one drops now and then (`minecraft:spawn_entity`), the least and most ticks apart.
    pub lays: Option<(&'static str, u32, u32)>,
    /// A tamed one follows its owner from this far until it is this near (`behavior.follow_owner`).
    pub follows: Option<(f32, f32)>,
    pub sits: bool,
    /// A tamed one fights what hurts its owner and what its owner hits.
    pub defends: bool,
    pub leashable: bool,
    /// Items it follows a player holding (`behavior.tempt`), and its speed then, times [`Husbandry::walks`].
    pub tempt: Vec<&'static str>,
    pub tempt_speed: f32,
    /// Its `minecraft:movement`, the middle of a range a kind rolls in.
    pub walks: f32,
    pub on_born: Swap,
    pub on_grow: Swap,
    pub on_tame: Swap,
    pub on_sheared: Swap,
    pub on_regrow: Swap,
}

static TABLE: LazyLock<HashMap<&'static str, Husbandry>> =
    LazyLock::new(|| include_str!("../data/husbandry.tsv").lines().filter_map(|row| row.split_once('\t')).map(|(kind, fields)| (kind, parse(fields))).collect());

fn list(value: &'static str) -> Vec<&'static str> {
    value.split(',').filter(|item| !item.is_empty()).collect()
}

/// `name:number` pairs; an item's own name holds a colon, so the number is after the last.
fn pairs<T: std::str::FromStr>(value: &'static str) -> Vec<(&'static str, T)> {
    list(value).into_iter().filter_map(|pair| pair.rsplit_once(':')).filter_map(|(name, number)| Some((name, number.parse().ok()?))).collect()
}

fn range(value: &str) -> Option<(i32, i32)> {
    let (least, most) = value.split_once('-')?;
    Some((least.parse().ok()?, most.parse().ok()?))
}

fn swap(value: &'static str) -> Swap {
    let groups = list(value);
    Swap { removed: groups.iter().filter_map(|group| group.strip_prefix('-')).collect(), added: groups.iter().filter_map(|group| group.strip_prefix('+')).collect() }
}

fn parse(fields: &'static str) -> Husbandry {
    let mut found = Husbandry::default();
    for field in fields.split(' ') {
        let (key, value) = field.split_once('=').unwrap_or((field, ""));
        match key {
            "breed" => found.breed = list(value),
            "mates" => found.mates = list(value),
            "breed_tame" => found.breed_tame = true,
            "breed_healthy" => found.breed_healthy = true,
            "grows" => found.grows = value.parse().ok(),
            "grow" => found.grow = pairs(value),
            "tame" => found.tame = list(value),
            "tame_chance" => found.tame_chance = value.parse().unwrap_or(1.0),
            "tame_health" => found.tame_health = value.parse().ok(),
            "tame_attack" => found.tame_attack = value.parse().ok(),
            "temper" => found.temper = range(value),
            "temper_feed" => found.temper_feed = pairs(value),
            "heal" => found.heal = pairs(value),
            "lays" => found.lays = value.rsplit_once(':').and_then(|(item, ticks)| range(ticks).map(|(least, most)| (item, least as u32, most as u32))),
            "follows" => found.follows = range(value).map(|(from, to)| (from as f32, to as f32)),
            "sits" => found.sits = true,
            "defends" => found.defends = true,
            "leashable" => found.leashable = true,
            "tempt" => found.tempt = list(value),
            "tempt_speed" => found.tempt_speed = value.parse().unwrap_or(1.0),
            "walks" => found.walks = value.parse().unwrap_or(0.0),
            "on_born" => found.on_born = swap(value),
            "on_grow" => found.on_grow = swap(value),
            "on_tame" => found.on_tame = swap(value),
            "on_sheared" => found.on_sheared = swap(value),
            "on_regrow" => found.on_regrow = swap(value),
            _ => {}
        }
    }
    found
}

/// What the packs say of raising `kind`; `None` for a kind they give nothing of it.
pub fn husbandry(kind: &str) -> Option<&'static Husbandry> {
    TABLE.get(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cow_breeds_on_wheat_and_a_wolf_is_tamed_with_a_bone() {
        let cow = husbandry("minecraft:cow").unwrap();
        assert_eq!((cow.breed.as_slice(), cow.mates.as_slice(), cow.grows, cow.breed_tame), (&["minecraft:wheat"][..], &["minecraft:cow"][..], Some(24_000), false));
        assert_eq!(cow.on_grow, Swap { removed: vec!["minecraft:cow_baby"], added: vec!["minecraft:cow_adult"] });
        let wolf = husbandry("minecraft:wolf").unwrap();
        assert_eq!((wolf.tame.as_slice(), wolf.tame_chance, wolf.follows, wolf.sits, wolf.defends), (&["minecraft:bone"][..], 0.33, Some((10.0, 2.0)), true, true));
        assert!(wolf.breed_tame && wolf.breed_healthy && wolf.on_born.added == ["minecraft:wolf_baby"]);
        assert_eq!((wolf.tame_health, wolf.tame_attack, wolf.walks), (Some(40.0), Some(4.0), 0.3));
        let horse = husbandry("minecraft:horse").unwrap();
        assert_eq!((horse.temper, horse.temper_feed.first().copied()), (Some((0, 100)), Some(("minecraft:wheat", 3))));
        assert!(horse.mates.contains(&"minecraft:donkey") && horse.grow.contains(&("minecraft:hay_block", 0.15)));
        assert_eq!(husbandry("minecraft:chicken").unwrap().lays, Some(("minecraft:egg", 6000, 12_000)));
        assert!(husbandry("minecraft:sheep").unwrap().on_sheared.added.contains(&"minecraft:sheep_sheared"));
        assert!(husbandry("minecraft:zombie").is_none());
    }

    #[test]
    fn every_item_the_table_names_is_one_a_client_knows() {
        for (kind, raised) in TABLE.iter() {
            let named = raised.breed.iter().chain(&raised.tame).chain(&raised.tempt).copied().chain(raised.grow.iter().map(|(item, _)| *item)).chain(raised.temper_feed.iter().map(|(item, _)| *item));
            let named = named.chain(raised.heal.iter().map(|(item, _)| *item)).chain(raised.lays.map(|(item, ..)| item));
            for item in named.filter(|item| item.starts_with("minecraft:")) {
                assert!(crate::is_item(item), "{kind}: {item}");
            }
        }
    }
}
