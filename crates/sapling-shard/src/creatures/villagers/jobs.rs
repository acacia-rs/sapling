//! Professions and job sites: each profession's block, table and screen title (BDS's
//! `villager_v2` behaviour pack); a villager's type by the biome it was born in; a villager
//! without a job taking the nearest free job site and the profession it stands for, and keeping
//! the profession when the site goes; going to its site in working hours and restocking at its
//! first start of work in each work period (docs/research/villagers.md, "Professions").

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::{Trader, int, tell};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};
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
/// How far a villager looks for a job site, across and up or down (bds-s3poi*.txt: BDS's walking
/// villagers took sites 14 off, 15 off once of twice, never 16 or more).
// TODO: BDS's POI search range was not decompiled; 16 across is a fit, 4 up and down a guess.
const REACH: i32 = 16;
const REACH_UP: i32 = 4;
/// Job site searches per tick, shard-wide: each looks at about 10000 blocks.
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
/// A baby looks for a bed only (research/captures/bds-village.txt: a baby slept in one).
pub(super) fn look(world: &mut World, entity: Entity, layer: Entity, now: u64, baby: bool) {
    let Some(mut trader) = world.get::<Trader>(entity).cloned() else { return };
    let feet = world.get::<Body>(entity).map_or([0; 3], |body| body.at.map(|c| c.floor() as i32));
    let mut claimed = world.remove_resource::<Claimed>().unwrap_or_default();
    // A villager that went leaves its site free.
    claimed.sites.retain(|_, owner| world.get::<Trader>(*owner).is_some());
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    let changed = match world.get::<Layer>(layer) {
        Some(level) => {
            super::beds::decide(&level.world, &mut claimed, &mut trader, (entity, layer, feet, now));
            match baby {
                true => Vec::new(),
                false => decide(&level.world, &mut claimed, &mut trader, (entity, layer, feet, now), &mut dice),
            }
        }
        None => Vec::new(),
    };
    trader.next_look = now + LOOK_EVERY + u64::from(dice.roll(LOOK_SPREAD));
    world.insert_resource(dice);
    world.insert_resource(claimed);
    world.entity_mut(entity).insert(trader);
    super::deals::chart(world, entity, layer);
    if !changed.is_empty() {
        tell(world, entity, changed);
    }
}

/// What a look changes of the trader `entity` at `feet` of `layer` at `now`: the metadata to tell.
fn decide(level: &Level, claimed: &mut Claimed, trader: &mut Trader, (entity, layer, feet, now): (Entity, Entity, [i32; 3], u64), dice: &mut Dice) -> Vec<MetadataDictionaryItem> {
    match trader.job {
        Some(site) if block_at(level, site) != of(trader.profession).map(|p| p.site) => {
            // It keeps its profession (bds-n1drop.txt: 90 s, traded or not; bds-job2.txt: 20 s).
            claimed.sites.remove(&(layer, site));
            trader.job = None;
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

/// In working hours a villager keeps its pack's `behavior.work`: it goes to its job site and works
/// there 250 ticks, then is free 200 (`active_time`, `goal_cooldown`). Its first arrival in a work
/// period restocks: `on_arrival`'s `minecraft:resupply_trades` adds `trade_resupply_component_group`,
/// which only the next schedule change takes off (bds-n1restock.txt: a trade after that arrival was
/// not restocked in 118 s). False when it does not work now.
pub(super) fn work(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let now = world.resource::<Tick>().0;
    let (Some(trader), Some(body)) = (world.get::<Trader>(entity), world.get::<Body>(entity).copied()) else { return false };
    let Some(site) = trader.job else { return false };
    let Some(period) = world.get::<Layer>(layer).and_then(|layer| work_period(layer.world.level().time)) else { return false };
    if now < trader.work.0 {
        super::super::legs::rest(world, entity);
        return true;
    }
    if now < trader.work.1 {
        return false;
    }
    let [x, z] = [site[0] as f32 + 0.5, site[2] as f32 + 0.5];
    if (x - body.at[0]).hypot(z - body.at[2]) > AT_WORK + 0.5 {
        super::super::legs::walk(world, entity, layer, &body, [x, z], WALK, AT_WORK);
        return true;
    }
    super::super::legs::rest(world, entity);
    start_work(world, entity, now, period);
    true
}

/// Ticks of `behavior.work`'s work at the site, and of its rest after.
const WORKS_FOR: u64 = 250;
const RESTS_FOR: u64 = 200;

/// The trader starts work at `now`, restocking every offer if it is its first start in `period`.
fn start_work(world: &mut World, entity: Entity, now: u64, period: i64) {
    let Some(mut trader) = world.get_mut::<Trader>(entity) else { return };
    trader.work = (now + WORKS_FOR, now + WORKS_FOR + RESTS_FOR);
    trader.worked = Some(now);
    if trader.restocked == Some(period) {
        return;
    }
    trader.restocked = Some(period);
    trader.deals.iter_mut().for_each(super::Deal::restock);
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
