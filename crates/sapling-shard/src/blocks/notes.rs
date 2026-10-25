//! Note blocks: a click raises the note by one and plays it, a rising signal plays it as it is
//! (docs/research/block-uses.md, docs/research/redstone-blocks.md "Note block"). The note is kept
//! in the block's entity (`Music`, `note`), so it outlasts a restart.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{BlockEvent, BlockEventType};
use acacia_proto::types::{BlockCoordinates, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{fields, level, send};

const NOTES: u8 = 25;
const ENTITY: &str = "Music";
const NOTE: &str = "note";

/// The instruments of single blocks under a note block, by BDS's numbers (docs/research/redstone-blocks.md, B7-B8).
const BY_BLOCK: &[(&str, i32)] = &[
    ("minecraft:clay", 5), ("minecraft:honeycomb_block", 5), ("minecraft:gold_block", 6), ("minecraft:packed_ice", 8), ("minecraft:bone_block", 9),
    ("minecraft:iron_block", 10), ("minecraft:soul_sand", 11), ("minecraft:pumpkin", 12), ("minecraft:emerald_block", 13), ("minecraft:hay_block", 14),
    ("minecraft:glowstone", 15),
];
/// Copper's trumpet, one for each stage of weathering.
const TRUMPET: i32 = 16;
/// Mob heads on top of a note block play their mob instead, at one pitch, and do not stifle it.
const HEADS: &[(&str, i32)] = &[
    ("minecraft:zombie_head", 20), ("minecraft:skeleton_skull", 21), ("minecraft:creeper_head", 22), ("minecraft:dragon_head", 23),
    ("minecraft:wither_skeleton_skull", 24), ("minecraft:piglin_head", 25),
];
const HEAD_NOTE: i32 = 9;

/// The instrument a note block over the block `under` plays: BDS played 1 over stone.
fn instrument(under: &str, is_solid: bool) -> i32 {
    let has = |parts: &[&str]| parts.iter().any(|part| under.contains(part));
    if let Some((_, instrument)) = BY_BLOCK.iter().find(|(name, _)| *name == under) {
        return *instrument;
    }
    if let Some(copper) = super::copper::Copper::of(under).filter(|copper| copper.base == "copper") {
        return TRUMPET + copper.stage as i32;
    }
    match () {
        () if under.ends_with("_wool") => 7,
        () if has(&["infested_", "resin_"]) => 5,
        () if has(&["sand", "gravel", "concrete_powder"]) => 2,
        () if has(&["glass", "sea_lantern", "beacon"]) => 3,
        () if has(&["planks", "_log", "_wood", "_stem", "bookshelf", "chest", "crafting_table", "noteblock"]) => 4,
        () if is_solid && has(&["stone", "_ore", "brick", "obsidian", "netherrack", "deepslate", "andesite", "diorite", "granite", "tuff", "basalt", "terracotta", "concrete"]) => 1,
        () => 0,
    }
}

fn note(world: &World, layer: Entity, pos: BlockPos) -> u8 {
    fields::byte(world, layer, pos, NOTE).map_or(0, |note| (note as u8) % NOTES)
}

/// Plays the note block at `pos` its `note`, unless a block over it stifles it.
fn play(world: &mut World, layer: Entity, pos: BlockPos, note: u8) {
    let [x, y, z] = pos;
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let above = registry.get(level.block(x, y + 1, z));
    let head = above.and_then(|above| HEADS.iter().find(|(name, _)| *name == above.name)).map(|(_, instrument)| *instrument);
    if head.is_none() && !above.is_some_and(|above| above.is_air()) {
        return;
    }
    let instrument = head.unwrap_or_else(|| registry.get(level.block(x, y - 1, z)).map_or(0, |under| instrument(under.name, under.is_solid())));
    let note = if head.is_some() { HEAD_NOTE } else { i32::from(note) };
    // The sound comes from the cell's corner, as BDS sent it; the event's type is the instrument.
    let told = [
        sapling_net::encode(&BlockEvent { position: BlockCoordinates { x, y, z }, r#type: BlockEventType::from_raw(instrument.into()), data: note }),
        send::noise("note", Vec3f { x: x as f32, y: y as f32, z: z as f32 }, instrument << 8 | note),
    ];
    send::to_viewers(world, layer, pos, &told);
    super::vibrate_at(world, layer, pos, super::GameEvent::NotePlay, false);
}

/// A click on the note block at `pos`: silent, and still a use, with a block over it.
pub(super) fn tuned(world: &mut World, layer: Entity, pos: BlockPos) {
    let next = (note(world, layer, pos) + 1) % NOTES;
    fields::set(world, layer, pos, ENTITY, NOTE, Some(Value::Byte(next as i8)));
    play(world, layer, pos, next);
}

/// The signal into the note block at `pos` rose.
pub(crate) fn sounded(world: &mut World, layer: Entity, pos: BlockPos) {
    let note = note(world, layer, pos);
    play(world, layer, pos, note);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_under_a_note_block_names_its_instrument() {
        let over = ["minecraft:stone", "minecraft:sand", "minecraft:glass", "minecraft:oak_planks", "minecraft:dirt", "minecraft:gold_block", "minecraft:white_wool", "minecraft:glowstone"].map(|under| instrument(under, true));
        assert_eq!(over, [1, 2, 3, 4, 0, 6, 7, 15]);
        assert_eq!([instrument("minecraft:weathered_copper", true), instrument("minecraft:waxed_copper", true)], [18, 16]);
    }
}
