//! Fish, axolotls and tadpoles taken up in a water bucket and let out of it. BDS removed the mob
//! at the click, played `bucket.fill.fish` at the corner of its cell and kept its whole actor
//! compound as the bucket's tag (research/captures/bds-buckets.txt); emptied, the water is poured
//! as from a water bucket and the mob made again from that compound in the water's cell.

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use bevy_ecs::prelude::*;
use bytes::BytesMut;
use sapling_api::{BlockPos, GameMode};
use sapling_world::{ChunkPos, ItemStack, placed_actor, stored_actors};

use crate::body::Body;
use crate::components::{InLayer, Mode, Players, Tick};
use crate::entities::{Kind, Stored};
use crate::inventory::{self, Inventory};

const WATER_BUCKET: &str = "minecraft:water_bucket";
/// Each mob a water bucket takes up, and the bucket it fills.
const SCOOPED: [(&str, &str); 6] = [
    ("minecraft:cod", "minecraft:cod_bucket"), ("minecraft:salmon", "minecraft:salmon_bucket"), ("minecraft:pufferfish", "minecraft:pufferfish_bucket"),
    ("minecraft:tropicalfish", "minecraft:tropical_fish_bucket"), ("minecraft:axolotl", "minecraft:axolotl_bucket"), ("minecraft:tadpole", "minecraft:tadpole_bucket"),
];

/// The mob a bucket holds, if it is one that holds one.
pub(crate) fn held_kind(bucket: &str) -> Option<&'static str> {
    SCOOPED.iter().find(|(_, filled)| *filled == bucket).map(|(kind, _)| *kind)
}

/// The player's click on `entity` with a water bucket takes it up; false where it is none of
/// [`SCOOPED`] or the hand holds no water bucket.
pub(crate) fn scoop(world: &mut World, player: Entity, entity: Entity) -> bool {
    let holds_water = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).is_some_and(|held| held.stack.name == WATER_BUCKET);
    let Some(kind) = world.get::<Kind>(entity).map(|kind| kind.0.clone()) else { return false };
    let Some(&(_, filled)) = SCOOPED.iter().find(|(scooped, _)| *scooped == kind).filter(|_| holds_water && super::is_alive(world, entity)) else { return false };
    let (Some(layer), Some(at)) = (world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).map(|body| body.at)) else { return false };
    let tag = world.get::<Stored>(entity).and_then(|stored| nbt::read::<LittleEndian>(&mut &stored.compound[..]).ok()).map(|nbt| nbt.value);
    crate::mobs::remove(world, entity);
    let cell = at.map(|c| c.floor() as i32);
    crate::view::to_viewers(world, layer, cell, &[super::sound("bucket.fill.fish", cell.map(|c| c as f32), "")]);
    let bucket = ItemStack { name: filled.to_owned(), count: 1, damage: 0, block: None, tag };
    if world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative) {
        let _ = inventory::change(world, player, |inventory| inventory.add(&bucket));
    } else {
        inventory::swap_held_for(world, player, bucket);
    }
    true
}

/// Lets the mob kept in `bucket` out into the middle of `cell` of `layer`, as it was taken up.
pub(crate) fn release(world: &mut World, layer: Entity, bucket: &ItemStack, cell: BlockPos) {
    let Some(kind) = held_kind(&bucket.name) else { return };
    let feet = [cell[0] as f32 + 0.5, cell[1] as f32, cell[2] as f32 + 0.5];
    let unique_id = ((world.resource::<Tick>().0 as i64) << 32) | (world.resource::<Players>().next_net_id as i64 & 0xffff_ffff);
    let kept = match &bucket.tag {
        Some(tag @ Value::Compound(_)) => {
            let mut bytes = BytesMut::new();
            nbt::write::<LittleEndian>(&mut bytes, &Nbt { name: String::new(), value: tag.clone() });
            bytes.to_vec()
        }
        _ => sapling_world::new_actor(unique_id, kind, feet, [0.0; 3]),
    };
    let Ok(compound) = placed_actor(&kept, unique_id, feet, [0.0, 0.0]) else { return };
    let Some((actor, unique_id, compound)) = stored_actors(&compound).ok().and_then(|mut read| read.pop()) else { return };
    crate::actors::bring_in(world, layer, ChunkPos::of_block(cell[0], cell[2]), actor, unique_id, compound);
}
