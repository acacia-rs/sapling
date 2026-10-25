//! Trial spawners (BDS `TrialSpawner::tick`, docs/research/spawners.md "Trial spawner"): its
//! `trial_spawner_state` goes waiting for players, active (a wave sized by the players it took on,
//! waves.rs), waiting to eject, ejecting a reward roll per player, and cooling down for its
//! config's length (configs.rs). An `ominous` one, as a world may hold, uses its ominous config.

mod configs;
mod waves;

use std::collections::HashSet;

use acacia_proto::nbt::Value;
use acacia_proto::packets::LevelEventEvent;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty};
use sapling_world::new_block_entity;

use super::eject::{event, gaussian, item, roll, sound};
use super::fields::Fields;
use super::{Near, TRIAL_SPAWNER, centre, dice, now, property, set_state};
use crate::components::{Layer, Settings};
use configs::Config;

const INACTIVE: u8 = 0;
const WAITING_FOR_PLAYERS: u8 = 1;
const ACTIVE: u8 = 2;
const WAITING_FOR_REWARD: u8 = 3;
const EJECTING: u8 = 4;
const COOLDOWN: u8 = 5;
/// `required_player_range`'s default.
const RANGE: i64 = 14;
/// Ticks from a wave's end to its first reward, and between rewards.
const REWARD_DELAY: i64 = 40;
const REWARD_GAP: i64 = 30;
/// Rewards come out this far over the block's corner, thrown up at [`EJECT_UP`] and by a normal
/// spread of [`EJECT_SPREAD`] every way.
const OUT: [f32; 3] = [0.5, 0.875, 0.5];
const EJECT_UP: f32 = 0.2;
const EJECT_SPREAD: f32 = 0.045;

/// The spawners that have ticked since they were taken up: BDS leaves a loaded one's mobs
/// counted for its first tick, while they may still be on their way in with their columns.
#[derive(Resource, Default)]
struct Ticked(HashSet<(Entity, BlockPos)>);

/// A trial spawner's entity, read.
pub(super) struct Trial {
    fields: Fields,
    config: Config,
    pub ominous: bool,
    pub range: f32,
    pub registered: Vec<i64>,
    mobs: Vec<i64>,
    total_spawned: i64,
    pub next_spawn: i64,
    cooldown_end: i64,
    /// Whether clients are to be shown the entity again.
    told: bool,
}

impl Trial {
    fn load(fields: Fields, ominous: bool, now: i64) -> Self {
        let config = configs::of(&fields, ominous);
        // A world BDS saved counts its own ticks, not this shard's.
        let cooldown_end = fields.int("cooldown_end_at", 0).min(now + config.cooldown);
        let next_spawn = fields.int("next_mob_spawns_at", 0).min(now + config.ticks_between.max(REWARD_DELAY));
        Self {
            ominous,
            range: fields.int("required_player_range", RANGE) as f32,
            registered: waves::from_list(&fields.list("registered_players")),
            mobs: waves::from_list(&fields.list("current_mobs")),
            total_spawned: fields.int("total_mobs_spawned", 0),
            next_spawn,
            cooldown_end,
            config,
            fields,
            told: false,
        }
    }

    fn into_value(mut self) -> Value {
        self.fields.set("registered_players", waves::to_list(&self.registered));
        self.fields.set("current_mobs", waves::to_list(&self.mobs));
        self.fields.set("total_mobs_spawned", Value::Int(self.total_spawned as i32));
        self.fields.set("next_mob_spawns_at", Value::Long(self.next_spawn));
        self.fields.set("cooldown_end_at", Value::Long(self.cooldown_end));
        self.fields.into_value()
    }

    /// The kind its wave is of (`getOrCreateNextSpawnData`): kept in `spawn_data` for the whole
    /// cycle once drawn from its config.
    fn next_kind(&mut self, world: &mut World) -> Option<String> {
        if let Some(kind) = self.fields.get("spawn_data").and_then(configs::spawn_kind) {
            return Some(kind);
        }
        let total: u32 = self.config.potentials.iter().map(|(_, weight, _)| weight).sum();
        if total == 0 {
            return None;
        }
        let mut left = dice(world).roll(total);
        let (kind, weight, baby) = self.config.potentials.iter().find(|(_, weight, _)| {
            let found = left < *weight;
            left = left.saturating_sub(*weight);
            found
        })?.clone();
        let mut data = vec![("TypeId".into(), Value::String(kind.as_str().into())), ("Weight".into(), Value::Int(weight as i32))];
        if baby {
            data.push(("IsBaby".into(), Value::Byte(1)));
        }
        self.fields.set("spawn_data", Value::Compound(data));
        self.told = true;
        Some(kind)
    }

    fn reset(&mut self) {
        (self.cooldown_end, self.next_spawn, self.total_spawned) = (0, 0, 0);
        self.registered.clear();
    }

    /// Where the cooldown began: the wave's end.
    fn ended_at(&self) -> i64 {
        self.cooldown_end - self.config.cooldown
    }
}

/// Whether spawning stops here: peaceful ([U]: BDS's level query) or `doMobSpawning` off.
fn halted(world: &World, layer: Entity) -> bool {
    world.resource::<Settings>().0.difficulty == Difficulty::Peaceful || !crate::rules::on(world, layer, "doMobSpawning")
}

pub(super) fn tick(world: &mut World, layer: Entity, pos: BlockPos, fields: Fields, near: &[Near]) {
    let Some(state) = property(world, layer, pos, "trial_spawner_state").and_then(|state| state.parse::<u8>().ok()) else { return };
    let (ominous, now) = (property(world, layer, pos, "ominous") == Some("1"), now(world));
    let before = Value::Compound(fields.0.clone());
    let mut trial = Trial::load(fields, ominous, now);
    let first = world.get_resource_or_insert_with(Ticked::default).0.insert((layer, pos));
    let Some(kind) = trial.next_kind(world) else { return keep(world, layer, pos, trial, &before) };
    if !first {
        let alive = waves::alive(world, layer, pos, &trial.mobs);
        trial.told |= alive.len() != trial.mobs.len();
        trial.mobs = alive;
    }
    let to = match state {
        INACTIVE => Some(WAITING_FOR_PLAYERS),
        WAITING_FOR_PLAYERS | ACTIVE if halted(world, layer) => {
            trial.reset();
            Some(WAITING_FOR_PLAYERS).filter(|_| state == ACTIVE)
        }
        WAITING_FOR_PLAYERS => (waves::scans(pos, now) && waves::register(world, layer, pos, near, &mut trial, now)).then(|| {
            trial.total_spawned = 0;
            ACTIVE
        }),
        ACTIVE => wave(world, layer, pos, near, &mut trial, &kind, now),
        WAITING_FOR_REWARD => (now >= trial.ended_at() + REWARD_DELAY).then(|| {
            let table = pick_table(world, &trial.config);
            trial.fields.set("selected_loot_table", Value::String(table.as_str().into()));
            EJECTING
        }),
        EJECTING => ((now - trial.ended_at()) % REWARD_GAP == 0).then(|| reward(world, layer, pos, &mut trial)).flatten(),
        COOLDOWN => (now >= trial.cooldown_end).then(|| {
            trial.registered.clear();
            trial.mobs.clear();
            if !trial.config.potentials.is_empty() {
                trial.fields.0.retain(|(key, _)| key != "spawn_data");
            }
            set_state(world, layer, pos, "ominous", "0");
            WAITING_FOR_PLAYERS
        }),
        _ => None,
    };
    if let Some(to) = to {
        set_state(world, layer, pos, "trial_spawner_state", &to.to_string());
        trial.told = true;
    }
    keep(world, layer, pos, trial, &before);
}

/// Writes the entity back where it changed, and shows it where clients see the change.
fn keep(world: &mut World, layer: Entity, pos: BlockPos, trial: Trial, before: &Value) {
    let told = trial.told;
    let entity = trial.into_value();
    if told {
        crate::blocks::keep_entity(world, layer, pos, &entity);
    } else if entity != *before && let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, &entity);
    }
}

/// An active spawner's tick: players taken on, a mob when the wave and the time allow, and the
/// wave's end once all have come and gone.
fn wave(world: &mut World, layer: Entity, pos: BlockPos, near: &[Near], trial: &mut Trial, kind: &str, now: i64) -> Option<u8> {
    if waves::scans(pos, now) && waves::register(world, layer, pos, near, trial, now) {
        trial.told = true;
    }
    let (total, at_once) = trial.config.caps(trial.registered.len());
    if (trial.total_spawned as f32) < total && (trial.mobs.len() as f32) < at_once && now >= trial.next_spawn {
        if let Some(id) = waves::spawn(world, layer, pos, kind, trial.config.spawn_range, trial.ominous) {
            trial.mobs.push(id);
            trial.total_spawned += 1;
            trial.next_spawn = now + trial.config.ticks_between;
            trial.told = true;
        }
    }
    if (trial.total_spawned as f32) < total || !trial.mobs.is_empty() {
        return None;
    }
    sound(world, layer, pos, "trial_spawner.open_shutter", centre(pos), -1);
    trial.cooldown_end = now + trial.config.cooldown;
    Some(WAITING_FOR_REWARD)
}

/// One of the config's loot tables by weight.
fn pick_table(world: &mut World, config: &Config) -> String {
    let total: u32 = config.loot.iter().map(|(_, weight)| weight).sum();
    let mut left = dice(world).roll(total);
    let found = config.loot.iter().find(|(_, weight)| {
        let found = left < *weight;
        left = left.saturating_sub(*weight);
        found
    });
    found.or(config.loot.first()).map(|(table, _)| table.clone()).unwrap_or_default()
}

/// A roll of the chosen table for one of the players it took on, until none is left.
fn reward(world: &mut World, layer: Entity, pos: BlockPos, trial: &mut Trial) -> Option<u8> {
    if trial.registered.pop().is_none() {
        sound(world, layer, pos, "trial_spawner.close_shutter", centre(pos), -1);
        return Some(COOLDOWN);
    }
    let table = trial.fields.string("selected_loot_table").unwrap_or_default().to_owned();
    let at: [f32; 3] = std::array::from_fn(|axis| pos[axis] as f32 + OUT[axis]);
    for stack in roll(world, &table) {
        let motion = [gaussian(world) * EJECT_SPREAD, EJECT_UP + gaussian(world) * EJECT_SPREAD, gaussian(world) * EJECT_SPREAD];
        item(world, layer, stack, at, motion);
    }
    let mouth = [pos[0] as f32 + 0.5, pos[1] as f32 + 1.0, pos[2] as f32 + 0.5];
    sound(world, layer, pos, "trial_spawner.eject_item", mouth, -1);
    event(world, layer, pos, LevelEventEvent::ParticleTrialSpawnerEjecting, mouth, trial.ominous.into());
    trial.told = true;
    None
}

/// A spawn egg on a trial spawner makes its wave of the egg's kind; spent outside creative.
// TODO: [U] what BDS's egg (`14488a3a0`) does to the configs; here the kind is kept in `spawn_data`.
pub(super) fn use_egg(world: &mut World, layer: Entity, pos: BlockPos, held: &str) -> Option<bool> {
    let kind = crate::spawning::egg_kind(held)?;
    let mut fields = Fields::at(world, layer, pos).unwrap_or_else(|| Fields(if let Value::Compound(base) = new_block_entity(TRIAL_SPAWNER, pos) { base } else { Vec::new() }));
    fields.set("spawn_data", Value::Compound(vec![("TypeId".into(), Value::String(kind.into())), ("Weight".into(), Value::Int(1))]));
    crate::blocks::keep_entity(world, layer, pos, &fields.into_value());
    super::watch(world, layer, pos);
    Some(true)
}
