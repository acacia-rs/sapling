//! Note blocks: a click raises the note by one and plays it (docs/research/block-uses.md).
// TODO: the note is kept in memory, not in the block's entity, so a restart forgets it; redstone
// plays none; only the four oldest instruments are told apart, by the block's name.

use std::collections::HashMap;

use acacia_proto::packets::{BlockEvent, BlockEventType};
use acacia_proto::types::{BlockCoordinates, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{level, send};

const NOTES: u8 = 25;

/// The note of each note block clicked, by world.
#[derive(Resource, Default)]
pub(crate) struct Notes(HashMap<(Entity, BlockPos), u8>);

/// The instrument a note block over the block `under` plays: BDS played 1 over stone.
fn instrument(under: &str, is_solid: bool) -> i32 {
    let has = |parts: &[&str]| parts.iter().any(|part| under.contains(part));
    match () {
        () if has(&["sand", "gravel", "concrete_powder"]) => 2,
        () if has(&["glass", "sea_lantern", "beacon"]) => 3,
        () if has(&["planks", "_log", "_wood", "_stem", "bookshelf", "chest", "crafting_table", "noteblock"]) => 4,
        () if is_solid && has(&["stone", "_ore", "brick", "obsidian", "netherrack", "deepslate", "andesite", "diorite", "granite", "tuff", "basalt", "terracotta", "concrete"]) => 1,
        () => 0,
    }
}

/// A click on the note block at `pos`: silent, and still a use, with a block over it.
pub(super) fn play(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos;
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let is_covered = !registry.get(level.block(x, y + 1, z)).is_some_and(|above| above.is_air());
    let instrument = registry.get(level.block(x, y - 1, z)).map_or(0, |under| instrument(under.name, under.is_solid()));
    let mut notes = world.get_resource_or_insert_with(Notes::default);
    let note = notes.0.entry((layer, pos)).or_insert(0);
    *note = (*note + 1) % NOTES;
    let note = i32::from(*note);
    if is_covered {
        return;
    }
    // The sound comes from the cell's corner, as BDS sent it.
    let told = [
        sapling_net::encode(&BlockEvent { position: BlockCoordinates { x, y, z }, r#type: BlockEventType::ChangeState, data: note }),
        send::noise("note", Vec3f { x: x as f32, y: y as f32, z: z as f32 }, instrument << 8 | note),
    ];
    send::to_viewers(world, layer, pos, &told);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_under_a_note_block_names_its_instrument() {
        let over = ["minecraft:stone", "minecraft:sand", "minecraft:glass", "minecraft:oak_planks", "minecraft:dirt"].map(|under| instrument(under, true));
        assert_eq!(over, [1, 2, 3, 4, 0]);
    }
}
