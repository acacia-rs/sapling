//! A villager's families by its profession: each profession's component group adds its
//! `minecraft:type_family` (BDS's `villager_v2` and `zombie_villager_v2` packs), so BDS's
//! `@e[family=armorer]` matched a villager that took up armoring (bds-s3poi.txt).

use bevy_ecs::prelude::*;

use super::conversion::Infected;
use super::jobs::{NITWIT, UNSKILLED};
use super::Trader;

/// Each profession's own family and its trade's; a zombie villager has no trade family.
const FAMILIES: [(i32, &str, Option<&str>); 15] = [
    (UNSKILLED, "unskilled", Some("peasant")),
    (1, "farmer", Some("peasant")),
    (2, "fisherman", Some("peasant")),
    (3, "shepherd", Some("peasant")),
    (4, "fletcher", Some("peasant")),
    (5, "librarian", None),
    (6, "cartographer", None),
    (7, "cleric", Some("priest")),
    (8, "armorer", Some("blacksmith")),
    (9, "weaponsmith", Some("blacksmith")),
    (10, "toolsmith", Some("blacksmith")),
    (11, "butcher", Some("artisan")),
    (12, "leatherworker", Some("artisan")),
    (13, "stone_mason", Some("artisan")),
    (NITWIT, "nitwit", Some("peasant")),
];

/// The families of a villager or zombie villager by its profession; `None` for any other mob (a
/// wandering trader too), whose kind's families stand.
pub(in crate::creatures) fn of(world: &World, mob: Entity) -> Option<Vec<&'static str>> {
    let (variant, zombie) = match (world.get::<Trader>(mob), world.get::<Infected>(mob)) {
        (Some(trader), _) if trader.leaves_at.is_none() => (trader.profession, false),
        (_, Some(infected)) => (infected.0.profession, true),
        _ => return None,
    };
    let &(_, own, trade) = FAMILIES.iter().find(|(known, ..)| *known == variant)?;
    Some(if zombie { vec![own, "zombie_villager", "zombie", "undead", "monster", "mob"] } else { ["villager"].into_iter().chain(trade).chain([own, "mob"]).collect() })
}
