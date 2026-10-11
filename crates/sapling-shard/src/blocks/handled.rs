//! What a click does to a block that needs nothing in the hand: a cake is bitten, redstone ore
//! lights, a berry bush is picked (docs/research/block-uses.md).
// TODO: lit redstone ore never goes out, and a note block keeps no note.

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};

use super::{level, send, with_property};
use crate::components::Position;
use crate::{drops, inventory, vitals};

const CAKE_BITES: u8 = 6;
/// What a slice feeds: hunger and saturation.
const SLICE: [f32; 2] = [2.0, 0.4];
/// The lowest growth with berries on it, and the growth a picked bush is left at.
const RIPE: u8 = 2;
const PICKED: &str = "1";
/// Below the eyes, where BDS put the burp.
const MOUTH: f32 = sapling_data::EYE_HEIGHT - 0.1;

fn property(properties: &str, key: &str) -> Option<u8> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).and_then(|(_, value)| value.parse().ok())
}

/// A click on the block `name` at `pos`: false when it is none of these.
pub(super) fn use_by_hand(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let [x, y, z] = pos;
    let Some(level) = level(world, layer) else { return false };
    let (registry, id) = (level.registry(), level.block(x, y, z));
    let Some(properties) = registry.get(id).map(|state| state.properties) else { return false };
    match name {
        "minecraft:cake" => {
            let Some(bitten) = property(properties, "bite_counter") else { return false };
            let left = if bitten >= CAKE_BITES { Some(registry.air_id()) } else { with_property(registry, id, "bite_counter", &(bitten + 1).to_string()) };
            if let (Some(left), true) = (left, vitals::bite(world, player, SLICE)) {
                send::set(world, layer, pos, left);
                if let Some([x, y, z]) = world.get::<Position>(player).map(|position| position.feet) {
                    send::to_viewers(world, layer, pos, &[send::noise("burp", Vec3f { x, y: y + MOUTH, z }, -1)]);
                }
            }
            true
        }
        "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore" => {
            if let Some(lit) = registry.find(&name.replace("minecraft:", "minecraft:lit_"), properties) {
                send::set(world, layer, pos, lit);
            }
            // Not a use: the hand's block is still put against it.
            false
        }
        "minecraft:sweet_berry_bush" => {
            let Some(growth) = property(properties, "growth").filter(|growth| *growth >= RIPE) else { return false };
            let (Some(picked), Some(berry)) = (with_property(registry, id, "growth", PICKED), inventory::stored(&Item::new("minecraft:sweet_berries".to_owned(), 1), registry)) else { return false };
            send::set(world, layer, pos, picked);
            // BDS gave two off a full bush; one off a half-grown one is the Java game's least.
            for _ in 0..growth - 1 {
                drops::spill(world, layer, berry.clone(), pos);
            }
            let at = Vec3f { x: x as f32 + 0.5, y: y as f32, z: z as f32 + 0.5 };
            send::to_viewers(world, layer, pos, &[send::noise("block.sweet_berry_bush.pick", at, -1)]);
            true
        }
        _ => false,
    }
}
