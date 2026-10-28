//! Professions and job sites: each profession's block, table and screen title (BDS's
//! `villager_v2` behaviour pack); a villager's type by the biome it was born in; a villager
//! without a job taking the nearest free job site and the profession it stands for, and losing
//! it with its site if it never traded; going to its site in working hours and restocking there,
//! twice a day at most (docs/research/villagers.md, "Professions").

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::{Trader, int, tell};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Layer;
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key};

pub(super) struct Profession {
    variant: i32,
    /// Its component group, which `minecraft:become_<group>` gives.
    group: &'static str,
    pub table: &'static str,
    site: &'static str,
    pub title: &'static str,
}

const fn job(variant: i32, group: &'static str, table: &'static str, site: &'static str, title: &'static str) -> Profession {
    Profession { variant, group, table, site, title }
}

const PROFESSIONS: [Profession; 13] = [
    job(1, "farmer", "farmer", "minecraft:composter", "entity.villager.farmer"),
    job(2, "fisherman", "fisherman", "minecraft:barrel", "entity.villager.fisherman"),
    job(3, "shepherd", "shepherd", "minecraft:loom", "entity.villager.shepherd"),
    job(4, "fletcher", "fletcher", "minecraft:fletching_table", "entity.villager.fletcher"),
    job(5, "librarian", "librarian", "minecraft:lectern", "entity.villager.librarian"),
    job(6, "cartographer", "cartographer", "minecraft:cartography_table", "entity.villager.cartographer"),
    job(7, "cleric", "cleric", "minecraft:brewing_stand", "entity.villager.cleric"),
    job(8, "armorer", "armorer", "minecraft:blast_furnace", "entity.villager.armor"),
    job(9, "weaponsmith", "weapon_smith", "minecraft:grindstone", "entity.villager.weapon"),
    job(10, "toolsmith", "tool_smith", "minecraft:smithing_table", "entity.villager.tool"),
    job(11, "butcher", "butcher", "minecraft:smoker", "entity.villager.butcher"),
    job(12, "leatherworker", "leather_worker", "minecraft:cauldron", "entity.villager.leather"),
    job(13, "mason", "stone_mason", "minecraft:stonecutter_block", "entity.villager.mason"),
];
pub(super) const UNSKILLED: i32 = 0;
pub(super) const NITWIT: i32 = 14;

/// Ticks between a villager's looks for a job site or at its own: BDS's dweller update interval,
/// 60 and up to 40 more.
const LOOK_EVERY: u64 = 60;
pub(super) const LOOK_SPREAD: u32 = 40;
/// How far a villager looks for a job site, across and up or down.
// TODO: BDS's POI search range was not decompiled.
const REACH: i32 = 10;
const REACH_UP: i32 = 4;
/// Job site searches per tick, shard-wide: each looks at about 4000 blocks.
const SEARCHES_PER_TICK: u32 = 1;
/// How near its site a villager works.
const AT_WORK: f32 = 1.5;
const WALK: f32 = 0.5;
const DAY: i64 = 24_000;

/// The job sites and beds villagers took, by world and cell, with who took each.
#[derive(Resource, Default)]
pub(super) struct Claimed {
    pub sites: HashMap<(Entity, [i32; 3]), Entity>,
    /// The tick of the last searches for job sites and for beds, and how many it made.
    searched: [(u64, u32); 2],
}

/// What a search is for: each has its own searches a tick.
pub(super) const FOR_JOBS: usize = 0;
pub(super) const FOR_BEDS: usize = 1;

impl Claimed {
    /// Whether a search `kind` may be made at `now`; counts it if so.
    pub fn may_search(&mut self, now: u64, kind: usize) -> bool {
        let searched = &mut self.searched[kind];
        if *searched == (now, SEARCHES_PER_TICK) {
            return false;
        }
        *searched = if searched.0 == now { (now, searched.1 + 1) } else { (now, 1) };
        true
    }
}

pub(super) fn of(variant: i32) -> Option<&'static Profession> {
    PROFESSIONS.iter().find(|profession| profession.variant == variant)
}

/// The profession a new villager has: as `event` makes it, or BDS's `minecraft:entity_spawned`,
/// one of the thirteen or a nitwit, each as likely.
pub(super) fn first_profession(event: Option<&str>, dice: &mut Dice) -> i32 {
    match event.and_then(|event| event.strip_prefix("minecraft:")) {
        Some("become_unskilled" | "entity_born") => UNSKILLED,
        Some("become_nitwit") => NITWIT,
        // 90% unskilled, 10% nitwit (the pack's `spawn_from_village`).
        Some("spawn_from_village") => if dice.roll(10) == 0 { NITWIT } else { UNSKILLED },
        Some(event) => event.strip_prefix("become_").and_then(|group| PROFESSIONS.iter().find(|p| p.group == group)).map_or(UNSKILLED, |p| p.variant),
        None => 1 + dice.roll(PROFESSIONS.len() as u32 + 1) as i32,
    }
}

/// The trader takes up the profession `variant` with its offers.
pub(super) fn take_up(trader: &mut Trader, variant: i32, dice: &mut Dice) {
    trader.profession = variant;
    trader.stock(of(variant).map(|profession| profession.table), dice);
}

/// A villager's type by the biome at `feet`, the pack's filters in their order, a later winning:
/// desert or mesa, jungle, savanna, cold and not ocean or frozen, swamp, taiga; plains else.
pub(super) fn mark_at(level: &Level, feet: [f32; 3]) -> i32 {
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    let Some(biome) = level.biome(x, y, z) else { return 0 };
    let has = |tag: &str| sapling_data::biome_has_tag(biome, tag);
    let marks = [
        (1, has("desert") || has("mesa")),
        (2, has("jungle")),
        (3, has("savanna")),
        (4, (has("cold") && !has("ocean")) || has("frozen")),
        (5, has("swamp") || has("mangrove_swamp")),
        (6, has("taiga")),
    ];
    marks.into_iter().filter(|(_, held)| *held).map(|(mark, _)| mark).last().unwrap_or(0)
}

pub(super) fn block_at(level: &Level, [x, y, z]: [i32; 3]) -> Option<&'static str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z)).map(|state| state.name)).flatten()
}

/// The nearest free cell around `feet` that `fits`, and what it gives for it.
pub(super) fn nearest(level: &Level, claimed: &Claimed, layer: Entity, feet: [i32; 3], fits: impl Fn([i32; 3], &'static str) -> Option<i32>) -> Option<([i32; 3], i32)> {
    let mut best: Option<(i32, [i32; 3], i32)> = None;
    for dy in -REACH_UP..=REACH_UP {
        for dz in -REACH..=REACH {
            for dx in -REACH..=REACH {
                let cell = [feet[0] + dx, feet[1] + dy, feet[2] + dz];
                let Some(given) = block_at(level, cell).and_then(|name| fits(cell, name)) else { continue };
                let distance = dx * dx + dy * dy + dz * dz;
                if !claimed.sites.contains_key(&(layer, cell)) && best.is_none_or(|(least, ..)| distance < least) {
                    best = Some((distance, cell, given));
                }
            }
        }
    }
    best.map(|(_, cell, given)| (cell, given))
}

/// A villager's look at its job and bed (beds.rs): whether its site still stands, or for a free
/// one, which it can take (a few searches a tick, shard-wide).
pub(super) fn look(world: &mut World, entity: Entity, layer: Entity, now: u64) {
    let Some(mut trader) = world.get::<Trader>(entity).cloned() else { return };
    let feet = world.get::<Body>(entity).map_or([0; 3], |body| body.at.map(|c| c.floor() as i32));
    let mut claimed = world.remove_resource::<Claimed>().unwrap_or_default();
    // A villager that went leaves its site free.
    claimed.sites.retain(|_, owner| world.get::<Trader>(*owner).is_some());
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    let changed = match world.get::<Layer>(layer) {
        Some(level) => {
            super::beds::decide(&level.world, &mut claimed, &mut trader, (entity, layer, feet, now));
            decide(&level.world, &mut claimed, &mut trader, (entity, layer, feet, now), &mut dice)
        }
        None => Vec::new(),
    };
    trader.next_look = now + LOOK_EVERY + u64::from(dice.roll(LOOK_SPREAD));
    world.insert_resource(dice);
    world.insert_resource(claimed);
    world.entity_mut(entity).insert(trader);
    if !changed.is_empty() {
        tell(world, entity, changed);
    }
}

/// What a look changes of the trader `entity` at `feet` of `layer` at `now`: the metadata to tell.
fn decide(level: &Level, claimed: &mut Claimed, trader: &mut Trader, (entity, layer, feet, now): (Entity, Entity, [i32; 3], u64), dice: &mut Dice) -> Vec<MetadataDictionaryItem> {
    match trader.job {
        Some(site) if block_at(level, site) != of(trader.profession).map(|p| p.site) => {
            claimed.sites.remove(&(layer, site));
            trader.job = None;
            // TODO: BDS was not seen dropping a profession within 20 s of its site going (bds-job2.txt).
            if trader.experience == 0 && trader.tier == 0 {
                trader.profession = UNSKILLED;
                (trader.deals, trader.tiers) = (Vec::new(), Vec::new());
                return vec![int(Key::Variant, UNSKILLED), int(Key::MaxTradeTier, 0)];
            }
        }
        None if trader.profession != NITWIT && claimed.may_search(now, FOR_JOBS) => {
            let wanted = (trader.profession != UNSKILLED).then_some(trader.profession);
            let fits = |_, name: &str| PROFESSIONS.iter().find(|p| p.site == name && wanted.is_none_or(|wanted| wanted == p.variant)).map(|p| p.variant);
            let Some((site, variant)) = nearest(level, claimed, layer, feet, fits) else { return Vec::new() };
            claimed.sites.insert((layer, site), entity);
            trader.job = Some(site);
            if trader.profession == UNSKILLED {
                take_up(trader, variant, dice);
                return vec![int(Key::Variant, variant), int(Key::MaxTradeTier, trader.max_tier())];
            }
        }
        _ => {}
    }
    Vec::new()
}

/// The work period `time` falls in: BDS's work schedule works from 0 to 8000 and 10000 to 11000
/// each day; `None` outside them.
fn work_period(time: i64) -> Option<i64> {
    let (day, hour) = (time.div_euclid(DAY), time.rem_euclid(DAY));
    match hour {
        0..8_000 => Some(day * 2),
        10_000..11_000 => Some(day * 2 + 1),
        _ => None,
    }
}

/// In working hours a villager goes to its job site, and restocks there once a work period.
/// False when it does not work now.
pub(super) fn work(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let (Some(trader), Some(body)) = (world.get::<Trader>(entity), world.get::<Body>(entity).copied()) else { return false };
    let Some(site) = trader.job else { return false };
    let due = |period: i64| trader.restocked != Some(period) && trader.deals.iter().any(|deal| deal.uses > 0);
    let Some(period) = world.get::<Layer>(layer).and_then(|layer| work_period(layer.world.level().time)) else { return false };
    let restocks = due(period);
    let [x, z] = [site[0] as f32 + 0.5, site[2] as f32 + 0.5];
    if (x - body.at[0]).hypot(z - body.at[2]) > AT_WORK + 0.5 {
        super::super::legs::walk(world, entity, layer, &body, [x, z], WALK, AT_WORK);
        return true;
    }
    super::super::legs::rest(world, entity);
    if restocks {
        restock(world, entity, period);
    }
    true
}

/// Every offer of the trader restocked (BDS's `minecraft:resupply_trades` on arriving at work).
pub(super) fn restock(world: &mut World, entity: Entity, period: i64) {
    let Some(mut trader) = world.get_mut::<Trader>(entity) else { return };
    trader.deals.iter_mut().for_each(super::Deal::restock);
    trader.restocked = Some(period);
    super::storage::keep(world, entity);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_has_two_work_periods() {
        assert_eq!([0, 7_999, 8_000, 10_000, 11_000, 24_000 + 10_500].map(work_period), [Some(0), Some(0), None, Some(1), None, Some(3)]);
    }

    #[test]
    fn become_events_name_the_pack_groups() {
        let mut dice = Dice::default();
        assert_eq!(["minecraft:become_armorer", "minecraft:become_weaponsmith", "minecraft:become_mason", "minecraft:become_unskilled"].map(|e| first_profession(Some(e), &mut dice)), [8, 9, 13, 0]);
    }
}
