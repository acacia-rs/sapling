//! Spawner blocks: monster spawners (monster.rs), trial spawners (trial.rs) and vaults (vault.rs).
//! Each ticks while its column is seen (ticking.rs) and keeps its state in its block entity and
//! block state, so a saved world holds it. BDS's numbers: docs/research/spawners.md.

mod eject;
mod fields;
mod monster;
mod trial;
mod vault;

use std::collections::HashSet;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};

use crate::blocks::Dice;
use crate::components::{InLayer, Layer, Mode, Position, Stage, Tick};
use crate::tags::Marks;
use fields::Fields;

const MOB_SPAWNER: &str = "MobSpawner";
const TRIAL_SPAWNER: &str = "TrialSpawner";
const VAULT: &str = "Vault";
/// Steps a line of sight is tested in, per block of its length.
const SIGHT_STEPS: f32 = 4.0;

/// The spawner blocks that tick.
#[derive(Resource, Default)]
pub(crate) struct Watched(HashSet<(Entity, BlockPos)>);

/// The spawner blocks' own dice, so that they leave the rolls of the rest of the shard as they were.
#[derive(Resource, Default)]
struct SpawnerDice(Dice);

fn dice(world: &mut World) -> Mut<'_, Dice> {
    world.get_resource_or_insert_with(SpawnerDice::default).map_unchanged(|dice| &mut dice.0)
}

fn now(world: &World) -> i64 {
    world.resource::<Tick>().0 as i64
}

/// A player a spawner block may see: in its world and not a spectator.
#[derive(Clone, Copy, Debug)]
struct Near {
    entity: Entity,
    feet: [f32; 3],
    /// Its `UniqueID`, kept with it between visits: what trial spawners and vaults know it by.
    unique_id: i64,
}

impl Near {
    /// Where BDS has a player: its eyes.
    fn eyes(&self) -> [f32; 3] {
        [self.feet[0], self.feet[1] + sapling_data::EYE_HEIGHT, self.feet[2]]
    }
}

fn players(world: &mut World, layer: Entity) -> Vec<Near> {
    let mut players = world.query::<(Entity, &InLayer, &Position, &Stage, &Mode, Option<&Marks>)>();
    players
        .iter(world)
        .filter(|(_, in_layer, _, stage, mode, _)| in_layer.0 == layer && **stage == Stage::InWorld && mode.0 != GameMode::Spectator)
        .map(|(entity, _, position, _, _, marks)| Near { entity, feet: position.feet, unique_id: marks.map_or(0, |marks| marks.unique_id) })
        .collect()
}

fn centre([x, y, z]: BlockPos) -> [f32; 3] {
    [x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// Whether no block with collision but the cells `from` and `to` are in lies between them.
fn in_sight(world: &World, layer: Entity, from: [f32; 3], to: [f32; 3]) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let ends = [from, to].map(|end| end.map(|c| c.floor() as i32));
    let steps = (distance(from, to) * SIGHT_STEPS).ceil().max(1.0) as usize;
    (0..=steps).map(|step| std::array::from_fn::<i32, 3, _>(|axis| (from[axis] + (to[axis] - from[axis]) * step as f32 / steps as f32).floor() as i32)).filter(|cell| !ends.contains(cell)).all(|[x, y, z]| {
        !level.in_height(y) || level.registry().get(level.block(x, y, z)).is_none_or(|state| !state.is_solid())
    })
}

/// Sets the property `key` of the block at `pos` and shows it; false where it has none.
fn set_state(world: &mut World, layer: Entity, pos: BlockPos, key: &str, value: &str) -> bool {
    let Some(id) = world.get::<Layer>(layer).and_then(|level| crate::blocks::with_property(level.world.registry(), level.world.block(pos[0], pos[1], pos[2]), key, value)) else { return false };
    crate::blocks::set(world, layer, pos, id);
    true
}

fn property(world: &World, layer: Entity, [x, y, z]: BlockPos, key: &str) -> Option<&'static str> {
    let level = &world.get::<Layer>(layer)?.world;
    level.registry().get(level.block(x, y, z))?.property(key)
}

/// Whether a block's entity is one of a spawner block, which ticks while its column is seen.
pub(crate) fn is_spawner(entity: &Value) -> bool {
    matches!(entity.get("id"), Some(Value::String(id)) if [MOB_SPAWNER, TRIAL_SPAWNER, VAULT].contains(&id.as_str()))
}

pub(crate) fn watch(world: &mut World, layer: Entity, pos: BlockPos) {
    world.get_resource_or_insert_with(Watched::default).0.insert((layer, pos));
}

pub(crate) fn forget(world: &mut World, layer: Entity, pos: BlockPos) {
    if let Some(mut watched) = world.get_resource_mut::<Watched>() {
        watched.0.remove(&(layer, pos));
    }
}

pub(crate) fn watched(world: &World) -> Vec<(Entity, BlockPos)> {
    world.get_resource::<Watched>().map(|watched| watched.0.iter().copied().collect()).unwrap_or_default()
}

/// Ticks every watched spawner block; one whose entity is gone is let go.
pub(crate) fn tick(world: &mut World) {
    let Some(watched) = world.get_resource::<Watched>().map(|watched| watched.0.clone()) else { return };
    for (layer, pos) in watched {
        let Some(fields) = Fields::at(world, layer, pos) else {
            forget(world, layer, pos);
            continue;
        };
        let near = players(world, layer);
        match fields.string("id") {
            Some(MOB_SPAWNER) => monster::tick(world, layer, pos, fields, &near),
            Some(TRIAL_SPAWNER) => trial::tick(world, layer, pos, fields, &near),
            Some(VAULT) => vault::tick(world, layer, pos, fields, &near),
            _ => forget(world, layer, pos),
        }
    }
}

/// A click with `held` on the block at `pos` by `player`: a spawn egg sets a spawner's kind, a key
/// opens a vault. `Some(spent)` when it did, `spent` when the item is used up outside creative.
pub(crate) fn use_item(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, held: &str) -> Option<bool> {
    let level = &world.get::<Layer>(layer)?.world;
    let name = level.registry().get(level.block(pos[0], pos[1], pos[2]))?.name;
    match name {
        "minecraft:mob_spawner" => monster::use_egg(world, layer, pos, held),
        "minecraft:trial_spawner" => trial::use_egg(world, layer, pos, held),
        "minecraft:vault" => vault::use_key(world, player, layer, pos, held),
        _ => None,
    }
}
