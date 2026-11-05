//! Raids (docs/research/effects-raids.md "Raids"): a player's raid omen run out in a village
//! (omens.rs, village.rs) starts one. After a 300-tick wait each wave of BDS's raid groups comes
//! (waves.rs), the next once the last is dead; with the last dead the raid is won and the players
//! who slew a raider take Hero of the Village (none for raiders `/kill`ed: bds-raid.txt, bds-raidhero.txt), and it is lost once no villager is left. Shown on a boss bar (bar.rs),
//! which stays 30 s past the end. Its raiders make for the village and celebrate its fall
//! (raiders.rs); they hunt villagers and golems as their kind does (hunting/prey.rs). Raids are
//! kept with the world (kept.rs). Raiders of no raid that come near its raiders in its village
//! join it (raiders.rs `recruits`).

mod bar;
mod kept;
mod omens;
mod patrols;
mod raiders;
mod village;
mod waves;

pub(super) use patrols::{adopt as adopt_patrol, enlist as join_patrol, keep as keep_patrols};
pub(super) use kept::{village as village_uuid, village_middle};
pub(crate) use village::{centre as village_centre, within_bounds as within_village_bounds};

/// Before a raider's or a patrolling pillager's move, when it hunts nobody: false for a mob in neither.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    raiders::steer(world, entity, layer, kind) || patrols::steer(world, entity, layer)
}

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Health, is_alive};
use crate::components::{Settings, Tick};
use crate::vitals;

/// Ticks before each wave; ticks the bar stays once it is over.
const GATHERS: u64 = 300;
const ENDS: u64 = 600;
/// Ticks between a wave's last death and the next wait (or the win), the bar showing no raiders
/// left (research/captures/bds-raid.txt).
const CLEARS: u64 = 40;
/// Players this near the village's middle are in its raid.
const REACH: f32 = 96.0;
/// Hero of the Village's ticks, at the raid's level less one.
const HEROES_FOR: u32 = 48000;
/// The highest level an omen takes a raid to.
const MOST: u8 = 5;
/// Ticks between looks for the village's villagers.
const LOOKS: u64 = 20;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Gathering(u64),
    Fighting,
    Won(u64),
    Lost(u64),
}

struct Raid {
    layer: Entity,
    middle: [f32; 3],
    level: u8,
    waves: u8,
    spawned: u8,
    stage: Stage,
    raiders: Vec<Entity>,
    /// The `UniqueID`s of a kept raid's raiders not loaded yet.
    unloaded: Vec<i64>,
    /// The wave's health when it came.
    most: f32,
    heroes: Vec<Entity>,
    /// Where the last wave was spawned about (BDS's `SpawnX/Y/Z`, 0 before the first).
    spawn_at: [f32; 3],
}

impl Raid {
    fn reaches(&self, layer: Entity, at: [f32; 3]) -> bool {
        layer == self.layer && (0..3).map(|axis| (at[axis] - self.middle[axis]).powi(2)).sum::<f32>() <= REACH * REACH
    }

    fn is_over(&self) -> bool {
        matches!(self.stage, Stage::Won(_) | Stage::Lost(_))
    }
}

/// The raids under way, and whether the world folders' were read.
#[derive(Resource, Default)]
struct Raids(Vec<Raid>, bool);

/// What is left of the health of `raiders`.
fn health(world: &World, raiders: &[Entity]) -> f32 {
    raiders.iter().filter_map(|raider| world.get::<Health>(*raider)).map(Health::left).sum()
}

fn start(world: &mut World, raids: &mut Vec<Raid>, trigger: omens::Trigger, now: u64) {
    let Some(middle) = village::centre(world, trigger.layer, trigger.at) else { return };
    if let Some(raid) = raids.iter_mut().find(|raid| !raid.is_over() && raid.reaches(trigger.layer, middle)) {
        raid.level = raid.level.saturating_add(trigger.level).min(MOST);
        return;
    }
    let waves = waves::count(world.resource::<Settings>().0.difficulty);
    if waves > 0 {
        let level = trigger.level.min(MOST);
        raids.push(Raid { layer: trigger.layer, middle, level, waves, spawned: 0, stage: Stage::Gathering(now + GATHERS), raiders: Vec::new(), unloaded: Vec::new(), most: 1.0, heroes: Vec::new(), spawn_at: [0.0; 3] });
    }
}

/// The player slew `raider`: a hero of its raid.
pub(super) fn slain(world: &mut World, player: Entity, raider: Entity) {
    let Some(mut raids) = world.get_resource_mut::<Raids>() else { return };
    if let Some(raid) = raids.0.iter_mut().find(|raid| raid.raiders.contains(&raider)) {
        if !raid.heroes.contains(&player) {
            raid.heroes.push(player);
        }
    }
}

/// The raid is won: its heroes take Hero of the Village.
fn win(world: &mut World, raid: &mut Raid, now: u64) {
    raid.stage = Stage::Won(now);
    for hero in raid.heroes.clone() {
        if vitals::is_mortal(world, hero) {
            vitals::give_effect(world, hero, Effect::HeroOfTheVillage, HEROES_FOR, raid.level - 1, false);
        }
    }
}

/// Raiders of no raid near the raid's raiders and in its village join it, their health to its.
fn recruit(world: &mut World, raid: &mut Raid) {
    for (mob, at) in raiders::recruits(world, raid.layer, &raid.raiders) {
        if raid.reaches(raid.layer, at) {
            world.entity_mut(mob).insert(raiders::Raider { middle: raid.middle });
            raid.raiders.push(mob);
            raid.most += world.get::<Health>(mob).map_or(0.0, Health::max);
        }
    }
}

fn step(world: &mut World, raid: &mut Raid, now: u64) {
    match raid.stage {
        Stage::Gathering(until) if raid.spawned >= raid.waves && raid.spawned > 0 => {
            if now + GATHERS >= until {
                return win(world, raid, now);
            }
        }
        Stage::Gathering(until) if now >= until => {
            (raid.raiders, raid.spawn_at) = waves::spawn(world, raid.layer, raid.middle, raid.spawned).unwrap_or((Vec::new(), raid.spawn_at));
            for &raider in &raid.raiders {
                world.entity_mut(raider).insert(raiders::Raider { middle: raid.middle });
            }
            raid.spawned += 1;
            raid.most = health(world, &raid.raiders).max(1.0);
            raid.stage = Stage::Fighting;
        }
        Stage::Fighting => {
            if now.is_multiple_of(LOOKS) {
                kept::rejoin(world, raid);
                recruit(world, raid);
            }
            raid.raiders.retain(|raider| is_alive(world, *raider));
            if raid.raiders.is_empty() && raid.unloaded.is_empty() {
                raid.stage = Stage::Gathering(now + CLEARS + GATHERS);
            }
        }
        _ => {}
    }
    if !raid.is_over() && now.is_multiple_of(LOOKS) && !village::holds(world, raid.layer, raid.middle) {
        raid.stage = Stage::Lost(now);
        raid.raiders.retain(|raider| is_alive(world, *raider));
        raiders::celebrate(world, &raid.raiders, now);
    }
}

pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let triggers = omens::tick(world);
    let mut raids = std::mem::take(&mut world.get_resource_or_insert_with(Raids::default).0);
    if !std::mem::replace(&mut world.resource_mut::<Raids>().1, true) {
        raids.extend(kept::load(world, now));
    }
    let count = raids.len();
    for trigger in triggers {
        start(world, &mut raids, trigger, now);
    }
    for raid in &mut raids {
        step(world, raid, now);
    }
    let (gone, kept): (Vec<Raid>, Vec<Raid>) = raids.into_iter().partition(|raid| matches!(raid.stage, Stage::Won(since) | Stage::Lost(since) if now >= since + ENDS));
    let raids = kept;
    gone.iter().for_each(|raid| raiders::disband(world, &raid.raiders));
    // BDS re-sent the bar once a second, not as it changed (research/captures/bds-raidhero.txt).
    if now.is_multiple_of(bar::EVERY) {
        bar::show(world, &raids, now);
    }
    if now.is_multiple_of(LOOKS) || raids.len() != count {
        kept::keep(world, &raids, now);
    }
    world.resource_mut::<Raids>().0 = raids;
}
