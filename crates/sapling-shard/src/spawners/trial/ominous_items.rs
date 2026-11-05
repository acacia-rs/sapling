//! The ominous item spawners an active ominous trial spawner drops over its players
//! (docs/research/spawners.md "Ominous item spawners", research/captures/bds-u1omen.txt): every
//! 160 world ticks one over a player it may take on, carrying an item of its ominous table, which
//! it lets go 60 to 120 ticks on, shot straight down (shots/dispensed.rs) or dropped.

use acacia_proto::types::MetadataDictionaryItemValueDefault as Plain;
use acacia_proto::types::MetadataDictionaryItemKey as Key;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ItemStack;

use super::super::{Near, dice, distance};
use super::{Trial, event, sound};
use acacia_proto::packets::LevelEventEvent;
use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::entities;

const KIND: &str = "minecraft:ominous_item_spawner";
const EVERY: i64 = 160;
const SIZE: [f32; 2] = [0.6, 1.8];
/// Ticks it carries its item: the first and how many more it may.
const DELAY: [u32; 2] = [60, 61];
const WARNS_AT: u32 = 35;
/// Blocks over the player's head attach point: the first and how many more it may.
const ABOVE: [u32; 2] = [2, 4];
/// A player's head attach point over its feet; fits the first spot of bds-u1omen.txt.
const HEAD: f32 = 1.52;
/// `items_to_drop_when_ominous.json`'s two pools, each item one weight: a lingering potion of
/// these, and one of these (their data, a tipped arrow's its potion + 1).
// TODO: [U] BDS rolls the table once per spawner by an MT19937 seeded from its position and keeps it (spawners.md).
const POTIONS: [i16; 7] = [43, 45, 44, 46, 31, 14, 40];
const SHOTS: [(&str, i16); 5] = [("minecraft:arrow", 0), ("minecraft:arrow", 26), ("minecraft:arrow", 43), ("minecraft:fire_charge", 0), ("minecraft:wind_charge", 0)];

/// An ominous item spawner: what it lets go and in how many ticks, and the trial spawner it came from.
#[derive(Component)]
pub(crate) struct Carrying {
    item: ItemStack,
    left: u32,
    spawner: BlockPos,
}

fn stack(name: &str, damage: i16) -> ItemStack {
    ItemStack { name: name.to_owned(), count: 1, damage, block: None, tag: None }
}

/// One of the table's items, by the spawner blocks' dice.
fn pick(world: &mut World) -> ItemStack {
    let mut dice = dice(world);
    if dice.roll(2) == 0 {
        return stack("minecraft:lingering_potion", POTIONS[dice.roll(POTIONS.len() as u32) as usize]);
    }
    let (name, damage) = SHOTS[dice.roll(SHOTS.len() as u32) as usize];
    stack(name, damage)
}

/// Drops one over a player in `near` the spawner at `pos` may take on, on its tick; whether it did.
pub(super) fn drop_over(world: &mut World, layer: Entity, pos: BlockPos, near: &[Near], trial: &Trial, now: i64) -> bool {
    if now % EVERY != 0 {
        return false;
    }
    let corner = pos.map(|c| c as f32);
    let can: Vec<Near> = near.iter().copied().filter(|player| crate::vitals::is_mortal(world, player.entity) && distance(player.eyes(), corner) <= trial.range).collect();
    if can.is_empty() {
        return false;
    }
    // TODO: [U] BDS's coin between the wave's mobs and the players, and a mob's head attach point.
    let (above, chosen) = {
        let mut dice = dice(world);
        (ABOVE[0] + dice.roll(ABOVE[1]), can[dice.roll(can.len() as u32) as usize])
    };
    let head = [chosen.feet[0], chosen.feet[1] + HEAD, chosen.feet[2]];
    let Some(at) = spot(world, layer, head, above) else { return false };
    let item = pick(world);
    let left = DELAY[0] + dice(world).roll(DELAY[1]);
    let shown = crate::fireworks::display_item(world, layer, &item);
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
    event(world, layer, pos, LevelEventEvent::ParticleTrialSpawnerSpawning, corner, 1);
    true
}

/// Where over `head` it comes: `above` blocks up, or in the middle of the cell under the first
/// block in the way; none in `head`'s own cell.
fn spot(world: &World, layer: Entity, head: [f32; 3], above: u32) -> Option<[f32; 3]> {
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = head.map(|c| c.floor() as i32);
    let top = (head[1] + above as f32).floor() as i32;
    let solid = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid());
    let at = match (y + 1..=top).find(|&up| solid(up)) {
        Some(hit) => [x as f32 + 0.5, hit as f32 - 0.5, z as f32 + 0.5],
        None => [head[0], head[1] + above as f32, head[2]],
    };
    (at.map(|c| c.floor() as i32) != [x, y, z]).then_some(at)
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
        let Some(item) = world.entity_mut(entity).take::<Carrying>().map(|carrying| carrying.item) else { continue };
        if !crate::shots::dispensed::shoot_unowned(world, layer, at, [0.0, -1.0, 0.0], &item) {
            crate::drops::eject(world, layer, item, at, [0.0; 3]);
        }
        crate::mobs::remove(world, entity);
        sound(world, layer, spawner, "ominous_item_spawner.spawn_item", at, -1);
    }
}
