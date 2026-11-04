//! Villagers a zombie or husk kills become zombie villagers, and zombie villagers cured become
//! villagers again (`villager_v2.json` `become_zombie`, `zombie_villager_v2.json` `to_villager`).
//! research/captures/bds-f3convert.txt: a killing hit on hard turned every villager, on normal 4 of 6,
//! the zombie villager coming in the hit's tick where the villager stood, thrown as the hit threw
//! it, with its `Variant`, `MarkVariant`, `SkinID`, `TradeTier` and `MaxTradeTier` (`keep_level`).

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, Effect, GameMode, SpawnReason};

use super::super::turning::Turning;
use super::{Trader, VILLAGER, OLD_VILLAGER, shown};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, NetId, Settings, Tick};
use crate::entities::{self, Kind};
use crate::inventory::{self, Inventory};
use crate::mobs;

const ZOMBIE_VILLAGER: &str = "minecraft:zombie_villager_v2";
const APPLE: &str = "minecraft:golden_apple";
const REMEDY: &str = "remedy";
const UNFECT: &str = "unfect";
/// `to_villager`'s strength, 300 s.
const STRENGTH: u32 = 6000;
/// Its delay: `value` 100 s plus `range_min` 80 s, then up to 120 s more (`range_max` 200 s less `range_min`).
const CURE_LEAST: u64 = 3600;
const CURE_SPREAD: u32 = 2400;
const ASSISTED: f32 = 0.01;
const HELPS: f32 = 0.3;
const NEAR: i32 = 4;
const HELPERS: [&str; 2] = ["minecraft:bed", "minecraft:iron_bars"];
/// The families whose killing hit turns a villager.
const INFECTS: [&str; 2] = ["zombie", "husk"];

/// What a zombie villager was as a villager, for its cure.
#[derive(Component, Clone)]
pub(in crate::creatures) struct Infected(pub(super) Trader);

fn turns(world: &mut World) -> bool {
    match world.resource::<Settings>().0.difficulty {
        Difficulty::Hard => true,
        Difficulty::Normal => world.get_resource_or_insert_with(Dice::default).roll(2) == 0,
        _ => false,
    }
}

/// A hit of `harm` by a mob of `by` on `mob`: a villager it would kill that turns becomes a zombie
/// villager instead. True when it did, and the hit is spent.
pub(in crate::creatures) fn infect(world: &mut World, mob: Entity, layer: Entity, by: &str, harm: f32) -> bool {
    let villager = world.get::<Kind>(mob).is_some_and(|kind| kind.0 == VILLAGER || kind.0 == OLD_VILLAGER);
    let fatal = world.get::<super::super::health::Health>(mob).is_some_and(|health| health.left() <= harm);
    if !villager || !fatal || !sapling_data::mob_families(by).iter().any(|family| INFECTS.contains(family)) || !turns(world) {
        return false;
    }
    let (Some(body), Some(trader)) = (world.get::<Body>(mob).copied(), world.get::<Trader>(mob).cloned()) else { return false };
    let yaw = world.get::<super::super::legs::Legs>(mob).map_or(0.0, |legs| legs.yaw);
    let event = if super::super::shown::is_baby(world, mob) { "minecraft:as_baby" } else { super::super::turning::TRANSFORMED };
    let turned = super::super::spawn_as(world, layer, ZOMBIE_VILLAGER, body.at, SpawnReason::Transformed, yaw, Some(event)).and_then(|id| entities::find(world, id));
    mobs::remove(world, mob);
    let Some(turned) = turned else { return true };
    if let Some(mut thrown) = world.get_mut::<Body>(turned) {
        thrown.motion = body.motion;
    }
    world.entity_mut(turned).insert(Infected(trader));
    dress(world, turned);
    true
}

/// The player clicked `zombie` with a golden apple: a weakened zombie villager starts its cure
/// (the pack's `villager_converted`, its `to_villager` group). False when it is none of that.
pub(in crate::creatures) fn cure(world: &mut World, player: Entity, zombie: Entity) -> bool {
    let is_zombie = world.get::<Kind>(zombie).is_some_and(|kind| kind.0 == ZOMBIE_VILLAGER) && !super::super::is_dying(world, zombie);
    let apple = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name == APPLE)).unwrap_or(false);
    let weak = super::super::afflictions::level(world, zombie, Effect::Weakness).is_some();
    if !is_zombie || !apple || !weak || world.get::<Turning>(zombie).is_some() {
        return false;
    }
    let (Some(layer), Some(body), Some(id)) = (world.get::<InLayer>(zombie).map(|layer| layer.0), world.get::<Body>(zombie).copied(), world.get::<NetId>(zombie).map(|id| id.0)) else { return false };
    if world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative) {
        inventory::use_up_held(world, player);
    }
    super::super::afflictions::take(world, zombie, Some(Effect::Weakness));
    super::super::afflictions::give(world, zombie, Effect::Strength, STRENGTH, 0);
    let flags = entities::mob_flags(world, zombie, ZOMBIE_VILLAGER, MetadataFlags1::VIBRATING, true);
    let cell = body.at.map(|c| c.floor() as i32);
    let shaking = sapling_net::encode(&sapling_data::mob_flags_set(id, flags));
    let remedy = super::super::hunting::sound(REMEDY, id, ZOMBIE_VILLAGER, &body, 0.5);
    entities::tell_viewers(world, zombie, layer, cell, vec![shaking, remedy]);
    let at = world.resource::<Tick>().0 + cure_delay(world);
    world.entity_mut(zombie).insert((Curing, Turning { into: VILLAGER, sound: Some(UNFECT), at, keeps_gear: false, reason: SpawnReason::Transformed }));
    true
}

/// The ticks a cure takes: the delay's `value` plus `range_min`, and up to `range_max - range_min`
/// more (BDS 1424459c0, each in seconds times 20).
fn cure_delay(world: &mut World) -> u64 {
    CURE_LEAST + u64::from(world.get_resource_or_insert_with(Dice::default).roll(CURE_SPREAD))
}

/// A zombie villager being cured, whose beds and iron bars near speed it.
#[derive(Component)]
pub(in crate::creatures) struct Curing;

/// A tick of each cure, as BDS's transformation tick (14194f610) has it: one tick in a hundred
/// (`block_assist_chance`) the cells within 4 are looked through for beds and iron bars, at most 4
/// counted, each taking a tick more off the cure three times in ten (`block_chance`).
pub(in crate::creatures) fn hasten_cures(world: &mut World) {
    let curing: Vec<(Entity, Entity, [i32; 3])> =
        world.query_filtered::<(Entity, &InLayer, &Body), With<Curing>>().iter(world).map(|(mob, layer, body)| (mob, layer.0, body.at.map(|c| c.floor() as i32))).collect();
    for (mob, layer, [x, y, z]) in curing {
        if world.get_resource_or_insert_with(Dice::default).fraction() >= ASSISTED {
            continue;
        }
        let found = world.get::<Layer>(layer).map_or(0, |level| {
            let level = &level.world;
            let helps = |cell: [i32; 3]| level.in_height(cell[1]) && level.registry().get(level.block(cell[0], cell[1], cell[2])).is_some_and(|state| HELPERS.contains(&state.name));
            let near = (-NEAR..=NEAR).flat_map(|dx| (-NEAR..=NEAR).flat_map(move |dy| (-NEAR..=NEAR).map(move |dz| [x + dx, y + dy, z + dz])));
            near.filter(|cell| helps(*cell)).take(NEAR as usize).count()
        });
        let dice = &mut *world.get_resource_or_insert_with(Dice::default);
        let sped = (0..found).filter(|_| dice.fraction() < HELPS).count() as u64;
        if let Some(mut turning) = world.get_mut::<Turning>(mob) {
            turning.at = turning.at.saturating_sub(sped);
        }
    }
}

/// What a turning mob was as a villager, if it was one.
pub(in crate::creatures) fn kept_through_turning(world: &World, mob: Entity) -> Option<Trader> {
    world.get::<Infected>(mob).map(|infected| infected.0.clone())
}

/// A cured zombie villager's villager is what it was before: its offers and how far it got.
pub(in crate::creatures) fn turned(world: &mut World, villager: Entity, kept: Option<Trader>) {
    let Some(was) = kept else { return };
    let Some(mut trader) = world.get::<Trader>(villager).cloned() else { return };
    (trader.profession, trader.mark, trader.skin, trader.tier, trader.experience) = (was.profession, was.mark, was.skin, was.tier, was.experience);
    (trader.deals, trader.tiers) = (was.deals, was.tiers);
    let Some(id) = world.get::<NetId>(villager).map(|id| id.0) else { return };
    let metadata = shown(id, &trader).metadata;
    world.entity_mut(villager).insert(trader);
    super::tell(world, villager, metadata);
}

/// The zombie villager's first data and announcement hold what it was.
fn dress(world: &mut World, zombie: Entity) {
    let (Some(id), Some(Infected(trader))) = (world.get::<NetId>(zombie).map(|id| id.0), world.get::<Infected>(zombie).cloned()) else { return };
    entities::show_first(world, zombie, shown(id, &trader).metadata);
    if let Some(mut announcement) = mobs::announced(world, zombie) {
        announcement.push(sapling_net::encode(&shown(id, &trader)));
        mobs::reannounce(world, zombie, announcement);
    }
}
