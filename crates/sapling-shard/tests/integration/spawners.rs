//! Monster spawners, trial spawners and vaults (docs/research/spawners.md).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use common::{Harness, click};
use sapling_api::{Block, BlockEntityData, Difficulty, EntityInfo, GameMode, Item, PlayerId, ShardCtx, Slot};
use sapling_world::World;

const ALEX: PlayerId = PlayerId(1);
const MIDNIGHT: i64 = 18000;

/// A void world with a floor of `floor` at y 64 over the chunks about the origin.
fn floored(floor: &str) -> World {
    let mut level = World::void();
    let id = level.block_id(floor).unwrap();
    level.fill([-32, 64, -32], [31, 64, 31], id);
    level
}

/// Alex in `mode` at the origin over `floor`, at midnight on normal difficulty, with
/// `doMobSpawning` as `spawning`.
fn alex_on(mode: GameMode, floor: &str, spawning: bool) -> Harness {
    let mut h = Harness::with(common::config(), floored(floor));
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, mode);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(MIDNIGHT);
        assert!(ctx.set_game_rule("doMobSpawning", sapling_api::RuleValue::Bool(spawning)));
    });
    h.tick_all();
    h
}

/// Alex over a dark stone floor where nothing spawns by itself: monster spawners ignore the rule.
fn alex(mode: GameMode) -> Harness {
    alex_on(mode, "minecraft:stone", false)
}

fn block(h: &mut Harness, pos: [i32; 3]) -> Block {
    ask(h, move |ctx| ctx.block(pos)).expect("a block")
}

fn sounds(sent: &common::Sent) -> Vec<String> {
    common::all::<acacia_proto::packets::LevelSoundEvent>(sent.to(1)).into_iter().map(|sound| sound.sound_id).collect()
}

pub(crate) fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = Some(ask(ctx)));
    h.tick_all();
    found.lock().unwrap().take().expect("the handler ran")
}

fn run(h: &mut Harness, ticks: usize) {
    (0..ticks).for_each(|_| drop(h.tick_all()));
}

fn of_kind(h: &mut Harness, kind: &'static str) -> Vec<EntityInfo> {
    ask(h, move |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == kind).collect())
}

fn compound<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect())
}

/// Puts `block` at `pos` holding `entity`.
fn put(h: &mut Harness, block: Block, pos: [i32; 3], entity: Value) {
    let Value::Compound(mut fields) = entity else { panic!("a compound") };
    fields.extend(["x", "y", "z"].into_iter().zip(pos).map(|(axis, at)| (axis.into(), Value::Int(at))));
    let entity = Value::Compound(fields);
    let done = ask(h, move |ctx| (ctx.set_block(pos, &block), ctx.set_block_entity(pos, &BlockEntityData::new(entity))));
    assert_eq!(done, (true, true), "{pos:?}");
}

fn held(h: &mut Harness, item: &'static str, count: u8) {
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(item, count)))));
    h.tick_all();
}

fn entity_at(h: &mut Harness, pos: [i32; 3]) -> Value {
    ask(h, move |ctx| ctx.block_entity(pos).and_then(|data| data.get::<Value>().cloned())).expect("a block entity")
}

fn int(entity: &Value, key: &str) -> i64 {
    match entity.get(key) {
        Some(Value::Byte(v)) => (*v).into(),
        Some(Value::Short(v)) => (*v).into(),
        Some(Value::Int(v)) => (*v).into(),
        Some(Value::Long(v)) => *v,
        other => panic!("{key}: {other:?}"),
    }
}

mod monster;
mod trial;
mod vault;
