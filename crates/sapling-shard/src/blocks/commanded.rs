//! What BDS's `/setblock` (and so `ShardCtx::update_block`) writes beyond the block named
//! (research/captures/bds-fixset.txt, bds-bkticks.txt): a bed is laid whole with its head in the
//! named cell; nether vines, cave vines and kelp under water take a random age; a live coral fan
//! or kelp keeps a water source beside itself, which then runs.

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Face};

use super::{Dice, doors, level, liquid, send};

/// Blocks given a random age, by their age property, when the command names none.
const AGED: [(&str, &str); 4] = [
    ("minecraft:weeping_vines", "weeping_vines_age"),
    ("minecraft:twisting_vines", "twisting_vines_age"),
    ("minecraft:cave_vines", "growing_plant_age"),
    ("minecraft:kelp", "kelp_age"),
];
/// The ages drawn: the Java game's `nextInt(25)`; BDS's nine seen were 2 to 17.
const AGES: u32 = 25;

/// A bed `id` laid with its head at `pos` and its foot behind it, where that cell is air, both
/// shown in one UpdateSubchunkBlocks. `None` for any other block or a bed with no room.
pub(super) fn put_bed(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> Option<bool> {
    let level = level(world, layer)?;
    let registry = level.registry();
    let head = doors::with(registry, id, "head_piece_bit", "1")?;
    let foot_cell = doors::ahead(registry, id, pos, -1)?;
    let foot = doors::with(registry, id, "head_piece_bit", "0")?;
    let [x, y, z] = foot_cell;
    let is_free = level.in_height(y) && level.block(x, y, z) == registry.air_id();
    if !is_free {
        return None;
    }
    if level.block(pos[0], pos[1], pos[2]) == head {
        return Some(false);
    }
    if !send::set_quietly(world, layer, pos, head) || !send::set_quietly(world, layer, foot_cell, foot) {
        return None;
    }
    send::show_together(world, layer, &[(pos, send::FLAGS), (foot_cell, send::FLAGS)]);
    Some(true)
}

/// Writes `block` at `pos` as [`send::put_block`] does, aged or wetted as BDS's command does.
pub(super) fn put(world: &mut World, layer: Entity, pos: BlockPos, block: &Block) -> Option<bool> {
    let block = aged(world, layer, pos, block);
    let put = send::put_block(world, layer, pos, &block)?;
    // Kelp set in air got a source beside itself too (research/captures/bds-plantpop.txt).
    if put && (is_live_fan(&block.name) || block.name == "minecraft:kelp") {
        liquid::pour(world, layer, pos, liquid::Liquid::Water, true);
    }
    Some(put)
}

fn is_live_fan(name: &str) -> bool {
    !name.contains("dead_") && (name.ends_with("_coral_fan") || name.ends_with("_coral_wall_fan"))
}

/// `block` with a random age where it is one BDS ages and none was named. Kelp only under water:
/// a kelp set below glass kept age 0 (bds-fixset.txt), one in a pool two deep was aged (bds-bkticks.txt).
fn aged(world: &mut World, layer: Entity, [x, y, z]: BlockPos, block: &Block) -> Block {
    let Some((name, key)) = AGED.iter().find(|(name, _)| *name == block.name) else { return block.clone() };
    if block.properties.split(',').any(|pair| pair.split_once('=').is_some_and(|(named, _)| named == *key)) {
        return block.clone();
    }
    let wet_above = level(world, layer).is_some_and(|level| {
        let above = Face::Up.neighbour([x, y, z]);
        level.in_height(above[1]) && level.registry().get(level.block(above[0], above[1], above[2])).is_some_and(|state| state.name == "minecraft:water")
    });
    if *name == "minecraft:kelp" && !wet_above {
        return block.clone();
    }
    let age = world.get_resource_or_insert_with(Dice::default).roll(AGES);
    let states = [block.properties.as_str(), &format!("{key}={age}")].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(",");
    Block::new(*name).with(states)
}
