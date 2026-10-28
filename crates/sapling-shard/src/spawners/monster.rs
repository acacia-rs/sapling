//! Monster spawners (BDS `BaseMobSpawner::tick`, docs/research/spawners.md "Monster spawner"): with
//! a kind set and a player within `RequiredPlayerRange`, `Delay` counts down; at 0 up to
//! `SpawnCount` mobs appear within `SpawnRange`, each where the kind's own checks pass, until
//! `MaxNearbyEntities` of the kind are about. A spawn egg sets the kind.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{BlockEvent, BlockEventType, LevelEvent, LevelEventEvent};
use acacia_proto::types::{BlockCoordinates, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty, SpawnReason, Weather};
use sapling_world::{World as Level, new_block_entity};

use super::fields::Fields;
use super::{MOB_SPAWNER, Near, centre, dice};
use crate::body::Body;
use crate::components::{InLayer, Layer, Settings, Tick};
use crate::entities::{self, Kind};
use crate::spawning::Natural;

const DELAY: i64 = 20;
const MIN_DELAY: i64 = 200;
const MAX_DELAY: i64 = 800;
const COUNT: i64 = 4;
const MAX_NEARBY: i64 = 6;
const PLAYER_RANGE: i64 = 16;
const SPAWN_RANGE: i64 = 4;
/// The nearby count's box reaches this far up and down from the spawner.
const COUNTED_UP: f32 = 4.0;
/// Monsters need raw brightness below this (`Monster::isDarkEnoughToSpawn`).
const DARK: u8 = 8;
/// The sky's dimming brightness is reckoned by in a thunderstorm.
const STORM_DARKEN: u8 = 10;
const NETHER: i32 = 1;

/// A spawner's entity as BDS's constructor fills it, of `kind` (empty: none).
fn fresh(pos: BlockPos, kind: &str) -> Fields {
    let Value::Compound(base) = new_block_entity(MOB_SPAWNER, pos) else { unreachable!("a block entity is a compound") };
    let mut fields = Fields(base);
    fields.set("EntityIdentifier", Value::String(kind.into()));
    for (key, value) in [("Delay", DELAY), ("MinSpawnDelay", MIN_DELAY), ("MaxSpawnDelay", MAX_DELAY), ("SpawnCount", COUNT), ("MaxNearbyEntities", MAX_NEARBY), ("RequiredPlayerRange", PLAYER_RANGE), ("SpawnRange", SPAWN_RANGE)] {
        fields.set(key, Value::Short(value as i16));
    }
    for (key, value) in [("DisplayEntityWidth", 0.8), ("DisplayEntityHeight", 1.8), ("DisplayEntityScale", 1.0)] {
        fields.set(key, Value::Float(value));
    }
    fields
}

/// The kind the next spawn is of: `SpawnData`'s, else `EntityIdentifier`; `None` when unset.
fn kind(fields: &Fields) -> Option<String> {
    let next = fields.get("SpawnData").and_then(|data| match data.get("TypeId") {
        Some(Value::String(kind)) => Some(kind.to_string()),
        _ => None,
    });
    next.or_else(|| fields.string("EntityIdentifier").map(str::to_owned)).filter(|kind| !kind.is_empty())
}

pub(super) fn tick(world: &mut World, layer: Entity, pos: BlockPos, mut fields: Fields, near: &[Near]) {
    let Some(kind) = kind(&fields) else { return };
    let range = fields.int("RequiredPlayerRange", PLAYER_RANGE) as f32;
    let middle = centre(pos);
    if range >= 0.0 && !near.iter().any(|player| super::distance(player.eyes(), middle) < range) {
        return;
    }
    match fields.int("Delay", 0) {
        delay if delay > 0 => fields.set("Delay", Value::Short((delay - 1) as i16)),
        -1 => reset(world, layer, pos, &mut fields),
        _ => {
            if spawn_all(world, layer, pos, &kind, &fields) {
                reset(world, layer, pos, &mut fields);
            }
        }
    }
    // Only a reset is told, by its block event; the countdown is the server's.
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, &fields.into_value());
    }
}

/// The next delay, from `MinSpawnDelay` up to `MaxSpawnDelay`, and the next of `SpawnPotentials`
/// by weight (`BaseMobSpawner::_delay`).
fn reset(world: &mut World, layer: Entity, [x, y, z]: BlockPos, fields: &mut Fields) {
    let (min, max) = (fields.int("MinSpawnDelay", MIN_DELAY), fields.int("MaxSpawnDelay", MAX_DELAY));
    let mut delay = min;
    if max > min {
        delay += i64::from(dice(world).roll((max - min) as u32));
    }
    fields.set("Delay", Value::Short(delay as i16));
    if let Some(Value::List(potentials)) = fields.get("SpawnPotentials") {
        let weight = |data: &Value| if let Some(Value::Int(weight)) = data.get("Weight") { i64::from(*weight) } else { 0 };
        let total: i64 = potentials.items.iter().map(weight).sum();
        if total > 0 {
            let mut left = i64::from(dice(world).roll(total as u32));
            let next = potentials.items.iter().find(|data| {
                left -= weight(data);
                left < 0
            });
            if let Some(next) = next.cloned() {
                fields.set("SpawnData", next);
            }
        }
    }
    let told = sapling_net::encode(&BlockEvent { position: BlockCoordinates { x, y, z }, r#type: BlockEventType::ChangeState, data: 0 });
    crate::blocks::to_viewers(world, layer, [x, y, z], &[told]);
}

/// Up to `SpawnCount` tries; whether any spawned. Too many of the kind about ends the tries
/// with a reset, as a spawn does.
fn spawn_all(world: &mut World, layer: Entity, pos: BlockPos, kind: &str, fields: &Fields) -> bool {
    let range = fields.int("SpawnRange", SPAWN_RANGE);
    let mut spawned = false;
    for _ in 0..fields.int("SpawnCount", COUNT) {
        let at = {
            let mut dice = dice(world);
            let x = range as f32 * (dice.fraction() - dice.fraction()) + pos[0] as f32 + 0.5;
            let y = pos[1] + dice.roll(3) as i32 - 1;
            let z = range as f32 * (dice.fraction() - dice.fraction()) + pos[2] as f32 + 0.5;
            [x, y as f32, z]
        };
        let cell = at.map(|c| c.floor() as i32);
        let off: i64 = (0..3).map(|axis| i64::from((cell[axis] - pos[axis]).abs())).sum();
        if off > range || world.get::<Layer>(layer).is_none_or(|level| is_full_solid(&level.world, cell)) {
            continue;
        }
        if nearby(world, layer, pos, kind, range as f32) >= fields.int("MaxNearbyEntities", MAX_NEARBY) as usize {
            return true;
        }
        if !may_spawn(world, layer, kind, at) {
            continue;
        }
        spawned = true;
        if crate::spawning::removed_in_peaceful(kind) && world.resource::<Settings>().0.difficulty == Difficulty::Peaceful {
            // BDS spawns it and the monster takes itself away on its first tick.
            continue;
        }
        spawn(world, layer, pos, kind, at);
    }
    spawned
}

fn spawn(world: &mut World, layer: Entity, [x, y, z]: BlockPos, kind: &str, at: [f32; 3]) {
    let Some(id) = crate::creatures::spawn(world, layer, kind, at, SpawnReason::Spawner) else { return };
    let now = world.resource::<Tick>().0;
    if let Some(entity) = entities::find(world, id) {
        // BDS spawns it as a natural spawn, which despawns as one.
        world.entity_mut(entity).insert(Natural { born: now });
    }
    let [width, height] = sapling_data::mob_size(kind).unwrap_or([1.0; 2]);
    let data = width.ceil() as i32 + ((height.ceil() as i32) << 8);
    let position = Vec3f { x: x as f32, y: y as f32, z: z as f32 };
    let told = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleSpawn, position, data });
    crate::blocks::to_viewers(world, layer, [x, y, z], &[told]);
}

fn is_full_solid(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid() && state.is_full_cube())
}

fn is_monster(kind: &str) -> bool {
    sapling_data::has_family(kind, "monster")
}

/// The mobs of `kind` in the box `range` twice over across and [`COUNTED_UP`] up and down from
/// the spawner's corner.
fn nearby(world: &mut World, layer: Entity, pos: BlockPos, kind: &str, range: f32) -> usize {
    let [x, y, z] = pos.map(|c| c as f32);
    let (min, max) = ([x - 2.0 * range, y - COUNTED_UP, z - 2.0 * range], [x + 2.0 * range, y + COUNTED_UP, z + 2.0 * range]);
    let mut mobs = world.query::<(&InLayer, &Kind, &Body)>();
    mobs.iter(world).filter(|(in_layer, of, body)| in_layer.0 == layer && of.0 == kind && (0..3).all(|axis| (min[axis]..=max[axis]).contains(&body.at[axis]))).count()
}

/// The kind's own checks (`Mob::checkSpawnRules`): room for its box, and for a monster the dark.
fn may_spawn(world: &World, layer: Entity, kind: &str, at: [f32; 3]) -> bool {
    let (Some(level), Some([width, height])) = (world.get::<Layer>(layer).map(|layer| &layer.world), sapling_data::mob_size(kind)) else { return false };
    let half = width / 2.0;
    let (low, high) = ([at[0] - half, at[1], at[2] - half].map(|c| c.floor() as i32), [at[0] + half, at[1] + height, at[2] + half].map(|c| (c - 1e-3).floor() as i32));
    let registry = level.registry();
    for x in low[0]..=high[0] {
        for y in low[1]..=high[1] {
            for z in low[2]..=high[2] {
                if registry.get(level.block(x, y, z)).is_none_or(|state| state.is_solid()) {
                    return false;
                }
            }
        }
    }
    !is_monster(kind) || is_dark(world, layer, level, at.map(|c| c.floor() as i32))
}

/// `Monster::isDarkEnoughToSpawn` for mobs of today's versions: raw brightness under [`DARK`],
/// reckoned in a storm as the storm darkens, and outside the nether no block light at all.
fn is_dark(world: &World, layer: Entity, level: &Level, [x, y, z]: [i32; 3]) -> bool {
    let weather = crate::weather::of(world, layer);
    let darken = if weather == Weather::Thunder { STORM_DARKEN } else { crate::light::sky_darken(level.level().time, weather) };
    let lit = level.brightness_at(x, y, z, darken).is_none_or(|raw| raw >= DARK);
    let block_lit = level.dimension() != NETHER && level.light_at(x, y, z).is_none_or(|(_, block)| block != 0);
    !lit && !block_lit
}

/// A spawn egg on a monster spawner sets its kind (`MobSpawnerBlockActor::setMob`), in any game
/// mode; the delay and any `SpawnData` stay as they were. Spent outside creative.
pub(super) fn use_egg(world: &mut World, layer: Entity, pos: BlockPos, held: &str) -> Option<bool> {
    let kind = crate::spawning::egg_kind(held)?;
    // A spawner placed by hand holds a bare entity; BDS's holds every default.
    let mut fields = Fields::at(world, layer, pos).filter(|fields| fields.get("MinSpawnDelay").is_some()).unwrap_or_else(|| fresh(pos, ""));
    fields.set("EntityIdentifier", Value::String(kind.into()));
    crate::blocks::keep_entity(world, layer, pos, &fields.into_value());
    super::watch(world, layer, pos);
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_spawner_holds_bds_defaults_as_shorts() {
        let fields = fresh([1, 2, 3], "minecraft:zombie");
        assert_eq!(kind(&fields).as_deref(), Some("minecraft:zombie"));
        assert!(matches!(fields.get("MinSpawnDelay"), Some(Value::Short(200))));
        assert!(kind(&fresh([1, 2, 3], "")).is_none());
    }

    #[test]
    fn spawn_data_names_the_next_kind() {
        let mut fields = fresh([0; 3], "minecraft:zombie");
        fields.set("SpawnData", Value::Compound(vec![("TypeId".into(), Value::String("minecraft:skeleton".into())), ("Weight".into(), Value::Int(1))]));
        assert_eq!(kind(&fields).as_deref(), Some("minecraft:skeleton"));
    }
}
