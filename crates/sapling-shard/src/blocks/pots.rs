//! Flower pots: a click with a plant pots it, a click with an empty hand takes it out again (Java
//! `FlowerPotBlock.useItemOn`); the plant is kept as BDS keeps it, as the `PlantBlock` of the
//! pot's entity (docs/research/block-entities.md).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode, Item};
use sapling_world::{block_of_hash, new_block_entity};

use super::send;
use crate::components::{Layer, Mode};
use crate::inventory::{self, Inventory, Place};

const POT: &str = "minecraft:flower_pot";
const PLANT: &str = "PlantBlock";
/// What goes into a pot (the Java game's `potted_*` blocks), by the end of the item's name.
const POTTABLE: &[&str] = &[
    "_sapling", ":mangrove_propagule", ":poppy", ":dandelion", ":blue_orchid", ":allium", ":azure_bluet", "_tulip", ":oxeye_daisy", ":cornflower",
    ":lily_of_the_valley", ":wither_rose", ":torchflower", "_eyeblossom", "_mushroom", ":fern", ":deadbush", ":cactus", ":bamboo", "azalea", "_fungus", "_roots",
];

fn is_pottable(name: &str) -> bool {
    POTTABLE.iter().any(|end| name.ends_with(end))
}

/// The pot's entity with `plant` in it, or without one.
fn holding(kept: Option<Value>, pos: BlockPos, plant: Option<Value>) -> Value {
    let mut fields = match kept {
        Some(Value::Compound(fields)) => fields,
        _ => match new_block_entity("FlowerPot", pos) {
            Value::Compound(fields) => fields,
            _ => Vec::new(),
        },
    };
    fields.retain(|(key, _)| key != PLANT);
    fields.extend(plant.map(|plant| (PLANT.into(), plant)));
    Value::Compound(fields)
}

/// A click on the block `name` at `pos`: a pot takes the plant in the hand or gives its own
/// back. False for any other block.
pub(crate) fn use_pot(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if name != POT {
        return false;
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return true };
    let kept = level.block_entity(pos).map(|entity| entity.nbt.clone());
    let potted = kept.as_ref().and_then(|kept| kept.get(PLANT)).cloned();
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let spends = world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative);
    match (potted, held) {
        (None, Some(stack)) if is_pottable(&stack.name) => {
            let Some(plant) = stack.block.and_then(|hash| block_of_hash(level.registry(), hash)) else { return true };
            // BDS marks a pot that got a plant (research/captures/bds-clicks.txt).
            let marked = super::with_property(level.registry(), level.block(pos[0], pos[1], pos[2]), "update_bit", "1");
            send::keep_entity(world, layer, pos, &holding(kept, pos, Some(plant)));
            if let Some(marked) = marked {
                send::set(world, layer, pos, marked);
            }
            if spends {
                inventory::change(world, player, |inventory| inventory.take(Place::Main(inventory.selected.into()), 1).is_ok());
            }
        }
        (Some(plant), None) => {
            let Some(Value::String(name)) = plant.get("name") else { return true };
            let Some(stack) = inventory::stored(&Item::new(name.clone(), 1), level.registry()) else { return true };
            send::keep_entity(world, layer, pos, &holding(kept, pos, None));
            inventory::change(world, player, |inventory| inventory.put(Place::Main(inventory.selected.into()), Some(stack)));
        }
        _ => {}
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pottable_plants_are_items_and_blocks() {
        let registry = sapling_world::BlockRegistry::vanilla();
        for plant in ["minecraft:oak_sapling", "minecraft:poppy", "minecraft:red_tulip", "minecraft:cactus", "minecraft:crimson_fungus", "minecraft:warped_roots", "minecraft:fern"] {
            assert!(is_pottable(plant) && sapling_data::is_item(plant) && registry.states_of(plant).next().is_some(), "{plant}");
        }
        assert!(!is_pottable("minecraft:stone") && !is_pottable("minecraft:wheat_seeds"));
    }
}
