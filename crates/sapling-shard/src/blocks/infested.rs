//! An infested block that breaks lets its silverfish out, as BDS's infested block (142fd3ec0:
//! none for a silk touch tool, else 142fd3cf0 spawns one at the cell's corner); docs/research/blocks.md.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, SpawnReason};

const SILVERFISH: &str = "minecraft:silverfish";
const TILE_DROPS: &str = "doTileDrops";

/// The block `name` at `pos` broke, by a silk touch tool if `silk_touch`: an infested one lets a
/// silverfish out where it was, unless the world's `doTileDrops` is off.
pub(super) fn release(world: &mut World, layer: Entity, pos: BlockPos, name: &str, silk_touch: bool) {
    if silk_touch || !name.starts_with("minecraft:infested_") || !crate::rules::on(world, layer, TILE_DROPS) {
        return;
    }
    crate::creatures::spawn(world, layer, SILVERFISH, pos.map(|c| c as f32), SpawnReason::Effect);
}
