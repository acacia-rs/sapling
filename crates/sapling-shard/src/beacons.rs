//! Beacons: a payment at the screen (`BeaconPayment`, inventory/requests.rs) picks the effects,
//! kept in the block entity's `primary` and `secondary` without telling anyone, as BDS did; every
//! 80 ticks a lit beacon gives them to the players in its range. Its level is how many full
//! layers of iron, gold, emerald, diamond or netherite blocks lie under it (Java's
//! `updateBase`); the range is 10 more than ten a level, all the way up, and the effect lasts
//! 9 + 2 a level seconds (BDS gave speed 220 ticks on a one-layer pyramid, resent as an Update each
//! 80 ticks; research/captures/bds-stations.txt). A secondary equal to the primary raises it a level;
//! otherwise it is regeneration. Which effect each level allows: docs/research/workstations.md.
// TODO: the beam's way up is not checked, and BDS's `ambient` flag is not told.

use std::collections::HashSet;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Effect};
use sapling_world::new_block_entity;

use crate::components::{InLayer, Layer, Outgoing, Position, Tick};

const PERIOD: u64 = 80;
const BASES: [&str; 5] = ["minecraft:iron_block", "minecraft:gold_block", "minecraft:emerald_block", "minecraft:diamond_block", "minecraft:netherite_block"];
const PAYMENTS: [&str; 5] = ["minecraft:iron_ingot", "minecraft:gold_ingot", "minecraft:emerald", "minecraft:diamond", "minecraft:netherite_ingot"];
/// The effects (Bedrock ids) a beacon of at least the level of each row's index + 1 offers first.
const OFFERED: [&[i32]; 3] = [&[1, 3], &[11, 8], &[5]];
const REGENERATION: i32 = 10;
const MAX_LEVEL: i32 = 4;

/// The beacons that give effects.
#[derive(Resource, Default)]
struct Beacons(HashSet<(Entity, BlockPos)>);

fn int(entity: &Value, key: &str) -> i32 {
    if let Some(Value::Int(value)) = entity.get(key) { *value } else { 0 }
}

/// Whether a beacon's entity has an effect picked.
pub(crate) fn is_lit(entity: &Value) -> bool {
    int(entity, "primary") > 0
}

pub(crate) fn is_payment(name: &str) -> bool {
    PAYMENTS.contains(&name)
}

/// Whether a beacon of `level` offers `primary` and `secondary` (0 for none).
pub(crate) fn offers(level: i32, primary: i32, secondary: i32) -> bool {
    let offered = |effect: i32| OFFERED.iter().take(level.clamp(0, 3) as usize).any(|row| row.contains(&effect));
    offered(primary) && (secondary == 0 || (level >= MAX_LEVEL && (secondary == primary || secondary == REGENERATION)))
}

pub(crate) fn watch(world: &mut World, layer: Entity, pos: BlockPos) {
    world.get_resource_or_insert_with(Beacons::default).0.insert((layer, pos));
}

pub(crate) fn watched(world: &World) -> Vec<(Entity, BlockPos)> {
    world.get_resource::<Beacons>().map(|beacons| beacons.0.iter().copied().collect()).unwrap_or_default()
}

pub(crate) fn forget(world: &mut World, layer: Entity, pos: BlockPos) {
    if let Some(mut beacons) = world.get_resource_mut::<Beacons>() {
        beacons.0.remove(&(layer, pos));
    }
}

/// How many full layers of base blocks lie under the beacon at `pos`.
pub(crate) fn level(world: &World, layer: Entity, [x, y, z]: BlockPos) -> i32 {
    let is_base = |pos: BlockPos| crate::blocks::block_at(world, layer, pos).is_some_and(|block| BASES.contains(&block.name.as_str()));
    (1..=MAX_LEVEL).take_while(|&l| (-l..=l).all(|dx| (-l..=l).all(|dz| is_base([x + dx, y - l, z + dz])))).count() as i32
}

/// Keeps the effects paid for at the beacon at `pos` (the request checked them, forge.rs).
pub(crate) fn pay(world: &mut World, layer: Entity, pos: BlockPos, [primary, secondary]: [i32; 2]) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    let Value::Compound(mut fields) = level.world.block_entity(pos).map_or_else(|| new_block_entity("Beacon", pos), |entity| entity.nbt.clone()) else { return };
    for (key, value) in [("primary", primary), ("secondary", secondary)] {
        match fields.iter_mut().find(|(name, _)| name == key) {
            Some((_, field)) => *field = Value::Int(value),
            None => fields.push((key.into(), Value::Int(value))),
        }
    }
    level.world.set_block_entity(pos, &Value::Compound(fields));
    watch(world, layer, pos);
}

/// Every [`PERIOD`] ticks, each lit beacon's effects to the players in its range.
pub(crate) fn tick(world: &mut World) {
    if world.resource::<Tick>().0 % PERIOD != 0 {
        return;
    }
    for (layer, pos) in watched(world) {
        let Some(entity) = world.get::<Layer>(layer).and_then(|level| level.world.block_entity(pos)).map(|entity| entity.nbt.clone()) else {
            forget(world, layer, pos);
            continue;
        };
        let (primary, secondary, level) = (int(&entity, "primary"), int(&entity, "secondary"), level(world, layer, pos));
        if level == 0 || !offers(level, primary, secondary) {
            continue;
        }
        let range = (level * 10 + 10) as f32;
        let ticks = (9 + 2 * level as u32) * 20;
        let [x, y, z] = pos.map(|c| c as f32);
        let near = |feet: [f32; 3]| feet[0] >= x - range && feet[0] <= x + 1.0 + range && feet[2] >= z - range && feet[2] <= z + 1.0 + range && feet[1] >= y - range;
        let players: Vec<Entity> =
            world.query_filtered::<(Entity, &InLayer, &Position), With<Outgoing>>().iter(world).filter(|(_, in_layer, at)| in_layer.0 == layer && near(at.feet)).map(|(player, ..)| player).collect();
        let effect = |id: i32| Effect::ALL.into_iter().find(|effect| effect.id() == id);
        for player in players {
            if let Some(effect) = effect(primary) {
                crate::vitals::give_effect(world, player, effect, ticks, u8::from(secondary == primary), true);
            }
            if let Some(effect) = effect(secondary).filter(|_| secondary != primary) {
                crate::vitals::give_effect(world, player, effect, ticks, 0, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_level_offers_its_own_and_the_lower_effects() {
        assert!(offers(1, 1, 0) && !offers(1, 11, 0) && offers(2, 8, 0) && offers(3, 5, 0));
        assert!(!offers(3, 5, 5) && offers(4, 5, 5) && offers(4, 1, REGENERATION) && !offers(4, 1, 3));
    }
}
