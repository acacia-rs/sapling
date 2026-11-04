//! What a block a command or plugin sets holds from the start and shows, as BDS's `/setblock`
//! did (research/captures/bds-setshown.txt, bds-s7ents.txt): the entity a player's placing gives
//! it, or for blocks only a set gives one here, the one BDS made; some are shown again, whole, a
//! tick later. Containers, pistons, conduits and command blocks furnish their own.

use acacia_proto::nbt::{Str, Value};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::new_block_entity;

use super::entities::{BANNERS, CAMPFIRES, Placing, SKULLS, placed};
use super::jukebox::FINISHED;
use crate::components::Layer;

/// Blocks a set gives an entity and shows it, beyond those [`placed`] makes (signs, skulls,
/// banners, campfires and the rest of [`placed`]'s). A raid's village is found by its bells and
/// beds (creatures/raids/village.rs).
const SHOWN: [&str; 18] = [
    "minecraft:bell", "minecraft:bed", "minecraft:flower_pot", "minecraft:cauldron", "minecraft:chiseled_bookshelf", "minecraft:beacon", "minecraft:frame", "minecraft:glow_frame",
    "minecraft:lectern", "minecraft:jukebox", "minecraft:lodestone", "minecraft:mob_spawner", "minecraft:trial_spawner", "minecraft:vault", "minecraft:enchanting_table",
    "minecraft:decorated_pot", "minecraft:suspicious_sand", "minecraft:suspicious_gravel",
];
/// Of those, the blocks shown whole again a tick later, with skulls, signs and campfires.
const AGAIN: [&str; 5] = ["minecraft:cauldron", "minecraft:frame", "minecraft:glow_frame", "minecraft:lectern", "minecraft:jukebox"];
/// A commanded bed's colour: BDS's is red (bds-setshown.txt).
const RED: i16 = 14;
const JUKEBOX: &str = "Jukebox";

fn is_shown(block: &str) -> bool {
    SHOWN.contains(&block) || is_shown_again(block) || BANNERS.contains(&block)
}

fn is_shown_again(block: &str) -> bool {
    AGAIN.contains(&block) || SKULLS.contains(&block) || CAMPFIRES.contains(&block) || block.ends_with("_sign")
}

fn empty_stack() -> Value {
    let fields = [("Count", Value::Byte(0)), ("Damage", Value::Short(0)), ("Name", Value::String(Str::default())), ("WasPickedUp", Value::Byte(0))];
    Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect())
}

/// The entity BDS's set gave a block that only a set gives one here; `None` for any other.
fn set_only(block: &str, pos: BlockPos) -> Option<Value> {
    let (id, fields): (&str, Vec<(&str, Value)>) = match block {
        "minecraft:frame" => ("ItemFrame", Vec::new()),
        "minecraft:glow_frame" => ("GlowItemFrame", Vec::new()),
        "minecraft:lectern" => ("Lectern", Vec::new()),
        "minecraft:jukebox" => (JUKEBOX, vec![(FINISHED, Value::Byte(0)), ("TicksPlaying", Value::Long(0))]),
        "minecraft:lodestone" => ("Lodestone", Vec::new()),
        "minecraft:enchanting_table" => ("EnchantTable", vec![("rott", Value::Float(0.0))]),
        "minecraft:decorated_pot" => ("DecoratedPot", vec![("animation", Value::Byte(0)), (super::holders::POTTED, empty_stack())]),
        "minecraft:mob_spawner" => {
            let shorts = [("Delay", 20), ("MaxNearbyEntities", 6), ("MaxSpawnDelay", 800), ("MinSpawnDelay", 200), ("RequiredPlayerRange", 16), ("SpawnCount", 4), ("SpawnRange", 4)];
            let mut fields: Vec<_> = shorts.into_iter().map(|(key, value)| (key, Value::Short(value))).collect();
            fields.extend([("DisplayEntityHeight", Value::Float(1.8)), ("DisplayEntityScale", Value::Float(1.0)), ("DisplayEntityWidth", Value::Float(0.8)), ("EntityIdentifier", Value::String(Str::default()))]);
            ("MobSpawner", fields)
        }
        "minecraft:suspicious_sand" | "minecraft:suspicious_gravel" => {
            let table = Value::String("loot_tables/entities/empty_brushable_block.json".into());
            let fields = vec![("LootTable", table), ("LootTableSeed", Value::Int(0)), ("brush_count", Value::Int(0)), ("brush_direction", Value::Byte(6)), ("type", Value::String(block.into()))];
            ("BrushableBlock", fields)
        }
        _ => return None,
    };
    let mut entity = new_block_entity(id, pos);
    if let Value::Compound(entries) = &mut entity {
        entries.extend(fields.into_iter().map(|(key, value)| (key.into(), value)));
    }
    Some(entity)
}

/// Whether the block at `pos` is a frame with nothing to hang on: BDS's set one shows its break
/// at once and is gone, its item dropped, a tick later (research/captures/bds-e3sets.txt).
pub(super) fn is_unheld_frame(world: &World, layer: Entity, pos: BlockPos) -> bool {
    let Some(level) = super::level(world, layer) else { return false };
    let id = level.block(pos[0], pos[1], pos[2]);
    level.registry().get(id).is_some_and(|state| matches!(state.name, "minecraft:frame" | "minecraft:glow_frame")) && super::updates::lacks_support(world, layer, pos, id)
}

/// Gives a block set at `pos` without an entity the one BDS's command makes, shows it, and has
/// it shown again on the next tick where BDS did (shown_again.rs).
pub(super) fn furnish(world: &mut World, layer: Entity, pos: BlockPos, block: &str) {
    let placing = Placing { damage: if block == "minecraft:bed" { RED } else { 0 }, tag: None, yaw: 0.0, properties: "" };
    if !is_shown(block) {
        return;
    }
    let Some(made) = set_only(block, pos).or_else(|| placed(block, pos, &placing)) else { return };
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    if level.world.block_entity(pos).is_none() {
        _ = level.world.set_block_entity(pos, &made);
    }
    let Some(entity) = level.world.block_entity(pos).map(|entity| super::send::block_entity_data(pos, entity)) else { return };
    super::send::to_viewers(world, layer, pos, &[entity]);
    if is_unheld_frame(world, layer, pos) {
        let Some(level) = super::level(world, layer) else { return };
        let particles = super::send::destroy_particles(level.registry(), pos, level.block(pos[0], pos[1], pos[2]));
        super::send::to_viewers(world, layer, pos, &[particles]);
    }
    match block {
        "minecraft:bed" => super::shown_again::bed_next_tick(world, layer, pos),
        _ if is_shown_again(block) => super::shown_again::next_tick(world, layer, pos),
        _ => {}
    }
}

/// What a tick has done to a set block's entity by the time BDS showed it again: a jukebox with
/// no disc has finished its record.
pub(super) fn settle(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    let Some(mut entity) = level.world.block_entity(pos).map(|entity| entity.nbt.clone()) else { return };
    let unplayed = entity.get("id") == Some(&Value::String(JUKEBOX.into())) && entity.get(super::jukebox::RECORD).is_none() && entity.get(FINISHED) == Some(&Value::Byte(0));
    if let (true, Value::Compound(fields)) = (unplayed, &mut entity) {
        fields.retain(|(key, _)| key != FINISHED);
        fields.push((FINISHED.into(), Value::Byte(1)));
        _ = level.world.set_block_entity(pos, &entity);
    }
}
