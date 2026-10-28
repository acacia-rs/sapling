//! Raids (docs/research/effects-raids.md "Raids"): a player's raid omen run out in a village
//! (omens.rs, village.rs) starts one. After a 300-tick wait each wave of BDS's raid groups comes
//! (waves.rs), the next once the last is dead; with the last dead the raid is won and its heroes
//! take Hero of the Village, and it is lost once no villager is left. Shown on a boss bar (bar.rs),
//! which stays 30 s past the end. Its raiders make for the village and celebrate its fall
//! (raiders.rs); they hunt villagers and golems as their kind does (hunting/prey.rs).
// TODO: raiders do not join a raid they pass (BDS's `RaidRecruitSystem`, 0x141959650); a raid is not
// kept with the world.

mod bar;
mod omens;
mod raiders;
mod village;
mod waves;

pub(super) use raiders::steer;

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Health, is_alive};
use crate::components::{InLayer, Position, Settings, Tick, in_world};
use crate::vitals;

/// Ticks before each wave; ticks the bar stays once it is over.
const GATHERS: u64 = 300;
const ENDS: u64 = 600;
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
    /// The wave's health when it came.
    most: f32,
    heroes: Vec<Entity>,
}

impl Raid {
    fn reaches(&self, layer: Entity, at: [f32; 3]) -> bool {
        layer == self.layer && (0..3).map(|axis| (at[axis] - self.middle[axis]).powi(2)).sum::<f32>() <= REACH * REACH
    }

    fn is_over(&self) -> bool {
        matches!(self.stage, Stage::Won(_) | Stage::Lost(_))
    }
}

#[derive(Resource, Default)]
struct Raids(Vec<Raid>);

/// What is left of the health of `raiders`.
fn health(world: &World, raiders: &[Entity]) -> f32 {
    raiders.iter().filter_map(|raider| world.get::<Health>(*raider)).map(Health::left).sum()
}

fn start(world: &mut World, raids: &mut Vec<Raid>, trigger: omens::Trigger, now: u64) {
    let Some(middle) = village::centre(world, trigger.layer, trigger.at) else { return };
    if let Some(raid) = raids.iter_mut().find(|raid| !raid.is_over() && raid.reaches(trigger.layer, middle)) {
        raid.level = raid.level.saturating_add(trigger.level).min(MOST);
        raid.heroes.push(trigger.player);
        return;
    }
    let waves = waves::count(world.resource::<Settings>().0.difficulty);
    if waves > 0 {
        let level = trigger.level.min(MOST);
        raids.push(Raid { layer: trigger.layer, middle, level, waves, spawned: 0, stage: Stage::Gathering(now + GATHERS), raiders: Vec::new(), most: 1.0, heroes: vec![trigger.player] });
    }
}

/// The raid is won: its heroes, and the players in it now, take Hero of the Village.
fn win(world: &mut World, raid: &mut Raid, now: u64) {
    raid.stage = Stage::Won(now);
    let near: Vec<Entity> = in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).zip(world.get::<Position>(*player)).is_some_and(|(on, at)| raid.reaches(on.0, at.feet)))
        .collect();
    raid.heroes.extend(near);
    raid.heroes.sort();
    raid.heroes.dedup();
    for hero in raid.heroes.clone() {
        if vitals::is_mortal(world, hero) {
            vitals::give_effect(world, hero, Effect::HeroOfTheVillage, HEROES_FOR, raid.level - 1, false);
        }
    }
}

fn step(world: &mut World, raid: &mut Raid, now: u64) {
    match raid.stage {
        Stage::Gathering(until) if now >= until => {
            raid.raiders = waves::spawn(world, raid.layer, raid.middle, raid.spawned);
            for &raider in &raid.raiders {
                world.entity_mut(raider).insert(raiders::Raider { middle: raid.middle });
            }
            raid.spawned += 1;
            raid.most = health(world, &raid.raiders).max(1.0);
            raid.stage = Stage::Fighting;
        }
        Stage::Fighting => {
            raid.raiders.retain(|raider| is_alive(world, *raider));
            if raid.raiders.is_empty() {
                if raid.spawned >= raid.waves {
                    return win(world, raid, now);
                }
                raid.stage = Stage::Gathering(now + GATHERS);
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
    for trigger in triggers {
        start(world, &mut raids, trigger, now);
    }
    for raid in &mut raids {
        step(world, raid, now);
    }
    let (gone, kept): (Vec<Raid>, Vec<Raid>) = raids.into_iter().partition(|raid| matches!(raid.stage, Stage::Won(since) | Stage::Lost(since) if now >= since + ENDS));
    let raids = kept;
    gone.iter().for_each(|raid| raiders::disband(world, &raid.raiders));
    bar::show(world, &raids, now);
    world.resource_mut::<Raids>().0 = raids;
}
