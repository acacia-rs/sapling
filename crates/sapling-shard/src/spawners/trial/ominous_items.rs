//! The ominous item spawners an active ominous trial spawner drops over its players or its wave's
//! mobs (docs/research/spawners.md "Ominous item spawners", research/captures/bds-u1omen.txt): every
//! 160 world ticks one, carrying an item of the spawner's kept ominous table, which it lets go 60 to
//! 120 ticks on, shot straight down (shots/dispensed.rs) or dropped.

use std::collections::HashMap;

use acacia_proto::types::MetadataDictionaryItemValueDefault as Plain;
use acacia_proto::types::MetadataDictionaryItemKey as Key;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ItemStack;

use super::super::{Near, centre, dice, distance};
use super::{Trial, event, sound};
use acacia_proto::packets::LevelEventEvent;
use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::containers::loot;
use crate::entities;
use crate::tags::UniqueId;

const KIND: &str = "minecraft:ominous_item_spawner";
const TABLE: &str = "loot_tables/spawners/trial_chamber/items_to_drop_when_ominous.json";
const EVERY: i64 = 160;
const SIZE: [f32; 2] = [0.6, 1.8];
/// Ticks it carries its item: the first and how many more it may.
const DELAY: [u32; 2] = [60, 61];
const WARNS_AT: u32 = 35;
/// Blocks over the target's head attach point: the first and how many more it may.
const ABOVE: [u32; 2] = [2, 4];
/// A player's head attach point over its feet; fits the first spot of bds-u1omen.txt.
const HEAD: f32 = 1.52;
/// A mob's head attach point over its feet, a share of its height. [U] BDS's per-mob points.
const MOB_HEAD: f32 = 0.85;
/// The side of the box a spot is tested with against blocks' collision boxes: a point, near enough.
const POINT: f32 = 0.001;

/// Each ominous trial spawner's table, rolled once and kept (BDS keeps it unsaved, at +0x80).
#[derive(Resource, Default)]
struct Kept(HashMap<(Entity, BlockPos), Vec<ItemStack>>);

/// An ominous item spawner: what it lets go and in how many ticks, and the trial spawner it came from.
#[derive(Component)]
pub(crate) struct Carrying {
    item: ItemStack,
    left: u32,
    spawner: BlockPos,
}

/// The MT19937 seed BDS rolls the table of the spawner at `pos` with (1411990f0).
fn table_seed([x, y, z]: BlockPos, level_seed: i64) -> u32 {
    let u = (z.div_euclid(30).wrapping_mul(0x6eb_fff5) ^ y.div_euclid(20) ^ x.div_euclid(30).wrapping_mul(0x2f_c20f)) as u32;
    u.wrapping_mul(0x285_b825).wrapping_add(0xb).wrapping_mul(u).wrapping_add(level_seed as u32)
}

/// The table rolled for the spawner at `pos`, kept for it.
fn roll(world: &mut World, layer: Entity, pos: BlockPos) -> Vec<ItemStack> {
    let home = crate::dimensions::home(world, layer);
    let seed = world.get::<Layer>(home).map_or(0, |level| level.world.level().seed);
    let Some(level) = world.get::<Layer>(layer) else { return Vec::new() };
    let registry = level.world.registry();
    let kept: Vec<ItemStack> = loot::drawn(TABLE, table_seed(pos, seed)).into_iter().filter_map(|looted| loot::kept_stack(looted, registry)).collect();
    world.get_resource_or_insert_with(Kept::default).0.insert((layer, pos), kept.clone());
    kept
}

/// One of the spawner's kept stacks by the blocks' dice, weighted by count, whole: an item spawner
/// shows a kept stack of charges as it is and throws one (bds-au3charge.txt).
fn pick(world: &mut World, layer: Entity, pos: BlockPos) -> Option<ItemStack> {
    let kept = world.get_resource_or_insert_with(Kept::default).0.get(&(layer, pos)).cloned().unwrap_or_default();
    let kept = if kept.is_empty() { roll(world, layer, pos) } else { kept };
    let total: u32 = kept.iter().map(|stack| u32::from(stack.count)).sum();
    if total == 0 {
        return None;
    }
    let mut left = dice(world).roll(total);
    let stack = kept.iter().find(|stack| {
        let found = left < u32::from(stack.count);
        left = left.saturating_sub(stack.count.into());
        found
    })?;
    Some(stack.clone())
}

/// The head attach points of the wave's `mobs` that are alive here, in the wave's order.
fn mob_heads(world: &mut World, layer: Entity, mobs: &[i64]) -> Vec<[f32; 3]> {
    let mut found = world.query::<(Entity, &UniqueId, &InLayer, &Body)>();
    let bodies: Vec<(i64, Entity, [f32; 3])> = found
        .iter(world)
        .filter(|(_, id, in_layer, _)| in_layer.0 == layer && mobs.contains(&id.0))
        .map(|(entity, id, _, body)| (id.0, entity, [body.at[0], body.at[1] + body.size[1] * MOB_HEAD, body.at[2]]))
        .collect();
    let alive: Vec<_> = bodies.into_iter().filter(|(_, entity, _)| crate::creatures::is_alive(world, *entity)).collect();
    mobs.iter().filter_map(|id| alive.iter().find(|(of, ..)| of == id).map(|(.., head)| *head)).collect()
}

/// Drops one over a detected player in `near` or, on a coin, one of the wave's mobs, each within
/// range of the spawner at `pos`, on its tick; whether it did.
pub(super) fn drop_over(world: &mut World, layer: Entity, pos: BlockPos, near: &[Near], trial: &Trial, now: i64) -> bool {
    if now % EVERY != 0 {
        return false;
    }
    let middle = centre(pos);
    let in_range = |head: &[f32; 3]| distance(*head, middle) < trial.range;
    let players: Vec<[f32; 3]> = near
        .iter()
        .filter(|player| trial.registered.contains(&player.unique_id) && crate::vitals::is_mortal(world, player.entity))
        .map(|player| [player.feet[0], player.feet[1] + HEAD, player.feet[2]])
        .filter(in_range)
        .collect();
    if players.is_empty() {
        return false;
    }
    let on_mobs = dice(world).roll(2) == 1 && !trial.mobs.is_empty();
    let heads = if on_mobs { mob_heads(world, layer, &trial.mobs).into_iter().filter(in_range).collect() } else { players };
    if heads.is_empty() {
        return false;
    }
    let (head, above) = {
        let mut dice = dice(world);
        // BDS draws no index for a lone target.
        let chosen = if heads.len() == 1 { 0 } else { dice.roll(heads.len() as u32) as usize };
        (heads[chosen], ABOVE[0] + dice.roll(ABOVE[1]))
    };
    let Some(at) = spot(world, layer, head, above) else { return false };
    let Some(item) = pick(world, layer, pos) else { return false };
    let left = DELAY[0] + dice(world).roll(DELAY[1]);
    // BDS's compound: its fields in name order and `WasPickedUp` 0 (bds-au3charge.txt).
    let shown = crate::fireworks::display_fields(world, layer, &item, |fields| {
        fields.push(("WasPickedUp".into(), acacia_proto::nbt::Value::Byte(0)));
        fields.sort_by(|a, b| a.0.cmp(&b.0));
    });
    let spawned = entities::spawn(world, layer, KIND, Body::new(at, [0.0; 3], SIZE), at, |id| {
        let options = sapling_data::AddEntityOptions { id, identifier: KIND, feet: at, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: acacia_proto::types::MetadataFlags1(0), health: None, name: "" };
        let mut added = sapling_data::add_entity(&options);
        let offset = crate::dragon::cloud::item(Key::MinecartDisplayOffset, Plain::Int(left as i32));
        added.metadata.retain(|old| old.key != Key::MinecartDisplayBlock && old.key != Key::MinecartDisplayOffset);
        added.metadata.extend(shown.into_iter().chain([offset]));
        Some(vec![sapling_net::encode(&added)])
    });
    let Some(spawned) = spawned else { return false };
    world.entity_mut(spawned).insert(Carrying { item, left, spawner: pos });
    sound(world, layer, pos, "ominous_item_spawner.spawn_item_begin", at, -1);
    event(world, layer, pos, LevelEventEvent::ParticleTrialSpawnerSpawning, pos.map(|c| c as f32), 1);
    true
}

/// Where over `head` it comes: `above` blocks up, or in the middle of the cell under the first
/// block in the way; none in `head`'s own cell or in a block's collision box.
fn spot(world: &World, layer: Entity, head: [f32; 3], above: u32) -> Option<[f32; 3]> {
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = head.map(|c| c.floor() as i32);
    let top = (head[1] + above as f32).floor() as i32;
    let solid = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid());
    let at = match (y + 1..=top).find(|&up| solid(up)) {
        Some(hit) => [x as f32 + 0.5, hit as f32 - 0.5, z as f32 + 0.5],
        None => [head[0], head[1] + above as f32, head[2]],
    };
    let free = Body::new(at, [0.0; 3], [POINT; 2]).is_free(level);
    (free && at.map(|c| c.floor() as i32) != [x, y, z]).then_some(at)
}

/// Counts every ominous item spawner down a tick: it warns at 35 left and lets its item go at 0.
pub(crate) fn tick(world: &mut World) {
    let mut carriers = world.query::<(Entity, &InLayer, &Body, &mut Carrying)>();
    let due: Vec<(Entity, Entity, [f32; 3], u32, BlockPos)> = carriers
        .iter_mut(world)
        .map(|(entity, layer, body, mut carrying)| {
            carrying.left = carrying.left.saturating_sub(1);
            (entity, layer.0, body.at, carrying.left, carrying.spawner)
        })
        .filter(|(.., left, _)| *left == 0 || *left == WARNS_AT)
        .collect();
    for (entity, layer, at, left, spawner) in due {
        if left == WARNS_AT {
            sound(world, layer, spawner, "ominous_item_spawner.about_to_spawn_item", at, -1);
            continue;
        }
        let Some(item) = world.entity_mut(entity).take::<Carrying>().map(|carrying| ItemStack { count: 1, ..carrying.item }) else { continue };
        if !crate::shots::dispensed::shoot_unowned(world, layer, at, [0.0, -1.0, 0.0], &item) {
            crate::drops::eject(world, layer, item, at, [0.0; 3]);
        }
        crate::mobs::remove(world, entity);
        sound(world, layer, spawner, "ominous_item_spawner.spawn_item", at, -1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kept_table_is_one_lingering_potion_and_one_shot_by_region() {
        assert_eq!(table_seed([0, 0, 0], 0x1_0000_0007), 7);
        assert_eq!(table_seed([29, 19, 29], 7), table_seed([0, 0, 0], 7));
        assert_ne!(table_seed([30, 0, 0], 7), table_seed([0, 0, 0], 7));
        for seed in 0..50 {
            let [potion, shot]: [sapling_data::Looted; 2] = loot::drawn(TABLE, seed).try_into().expect("two pools");
            assert!(potion.name == "minecraft:lingering_potion" && [31, 14, 40, 43, 44, 45, 46].contains(&potion.data), "{potion:?}");
            let shots = [("minecraft:arrow", 0), ("minecraft:arrow", 26), ("minecraft:arrow", 43), ("minecraft:fire_charge", 0), ("minecraft:wind_charge", 0)];
            assert!(shots.contains(&(shot.name.as_str(), shot.data)) && (1..=3).contains(&shot.count), "{shot:?}");
        }
    }

    #[test]
    fn a_kept_stack_of_charges_is_carried_whole() {
        let mut world = World::new();
        let (layer, pos) = (Entity::PLACEHOLDER, [0, 0, 0]);
        let charges = ItemStack { name: "minecraft:fire_charge".into(), count: 3, damage: 0, block: None, tag: None };
        world.insert_resource(Kept(HashMap::from([((layer, pos), vec![charges.clone()])])));
        assert_eq!(pick(&mut world, layer, pos), Some(charges));
    }
}
