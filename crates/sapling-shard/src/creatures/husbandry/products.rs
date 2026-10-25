//! What animals give, as their packs' `minecraft:interact`, `spawn_entity` and `behavior.eat_block`
//! say: a grown hen lays an egg every 5 to 10 minutes; shears take a sheep's wool (1 to 3 of its
//! colour) and a mooshroom's mushrooms, which leave it a cow; a sheep grows its wool back eating
//! grass; a bucket milks a cow, mooshroom or grown goat; a bowl takes a mooshroom's stew, suspicious
//! where a brown one ate a flower; a dye colours a sheep; a brush takes an armadillo's scute.
// TODO: sheep spawned by the shard are all white; the grass a sheep eats is only the block it stands on or in.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ItemStack;

use super::{Raised, regroup, shown};
use crate::blocks::{self, Dice};
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode};
use crate::creatures::Creature;
use crate::entities::Kind;
use crate::inventory;

const SHEEP: &str = "minecraft:sheep";
const MOOSHROOM: &str = "minecraft:mooshroom";
const ARMADILLO: &str = "minecraft:armadillo";
const SHEARS: &str = "minecraft:shears";
const BRUSH: &str = "minecraft:brush";
const SHEARS_LAST: u16 = 238;
const BRUSH_LAST: u16 = 64;
/// A brush wears this much a scute (the armadillo's `hurt_item`).
const BRUSH_WEAR: i32 = 16;
/// A sheep's colours by `Color`, as wool and dye names have them.
const COLOURS: [&str; 16] = ["white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black"];
/// What a brown mooshroom fed each flower marks its next stew with (`mooshroom_brown_fed_*`).
const FLOWERS: [(&str, i32); 16] = [
    ("poppy", 0), ("cornflower", 1), ("red_tulip", 2), ("orange_tulip", 2), ("white_tulip", 2), ("pink_tulip", 2), ("azure_bluet", 3), ("lily_of_the_valley", 4),
    ("dandelion", 5), ("blue_orchid", 6), ("allium", 7), ("oxeye_daisy", 8), ("wither_rose", 9), ("torchflower", 10), ("open_eyeblossom", 11), ("closed_eyeblossom", 12),
];
/// A mooshroom's `mark_variant` before it eats a flower.
pub(super) const UNFED: i32 = -1;
/// The chance a tick a sheep grazes (its `behavior.eat_block`), a lamb's and a grown one's.
const GRAZES: [f32; 2] = [0.02, 0.001];
const MILKS: [&str; 3] = ["minecraft:cow", MOOSHROOM, "minecraft:goat"];

fn stack(name: String, count: u8) -> ItemStack {
    ItemStack { name, count, damage: 0, block: None, tag: None }
}

/// Tosses `count` of `item` from the middle of the mob.
fn give(world: &mut World, entity: Entity, item: String, count: u8) {
    let Some((layer, _, _, body)) = shown::of(world, entity) else { return };
    crate::drops::toss(world, layer, stack(item, count), [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]]);
}

fn is_creative(world: &World, player: Entity) -> bool {
    world.get::<Mode>(player).is_some_and(|mode| mode.0 == sapling_api::GameMode::Creative)
}

/// The player's held item is used up, but in creative.
pub(super) fn spend(world: &mut World, player: Entity) {
    if !is_creative(world, player) {
        inventory::use_up_held(world, player);
    }
}

/// A tick of a grown mob that lays: it drops its egg or scute once its time comes, and rolls the next.
pub(super) fn lay(world: &mut World, entity: Entity, kind: &str, now: u64) {
    let Some((item, least, most)) = sapling_data::husbandry(kind).and_then(|rules| rules.lays) else { return };
    let Some(raised) = world.get::<Raised>(entity).filter(|raised| !raised.is_baby()) else { return };
    match raised.lays_at {
        Some(due) if now < due => return,
        Some(_) => {
            give(world, entity, item.to_owned(), 1);
            shown::sound(world, entity, "plop");
        }
        None => {}
    }
    let wait = least as u64 + world.get_resource_or_insert_with(Dice::default).roll(most - least + 1) as u64;
    world.get_mut::<Raised>(entity).expect("read above").lays_at = Some(now + wait);
}

/// A tick of a sheep: now and then it eats the grass it stands on, or in, growing back its wool.
pub(super) fn graze(world: &mut World, entity: Entity, kind: &str) {
    if kind != SHEEP {
        return;
    }
    let Some(baby) = world.get::<Raised>(entity).map(Raised::is_baby) else { return };
    if world.get_resource_or_insert_with(Dice::default).fraction() >= GRAZES[usize::from(!baby)] {
        return;
    }
    let Some((layer, _, _, body)) = shown::of(world, entity) else { return };
    let [x, y, z] = body.at.map(|c| c.floor() as i32);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let named = |y: i32| level.registry().get(level.block(x, y, z)).map(|state| state.name);
    let eaten = match (named(y), named(y - 1)) {
        (Some("minecraft:short_grass" | "minecraft:tall_grass"), _) => level.registry().find("minecraft:air", "").map(|air| ([x, y, z], air)),
        (_, Some("minecraft:grass_block")) => level.registry().find("minecraft:dirt", "").map(|dirt| ([x, y - 1, z], dirt)),
        _ => None,
    };
    let griefs = level.level().rules.on("mobGriefing");
    let Some((pos, id)) = eaten.filter(|_| griefs) else { return };
    blocks::set(world, layer, pos, id);
    shown::event(world, entity, acacia_proto::packets::EntityEventEventId::EatGrassAnimation, 0);
    let Some(mut raised) = world.get_mut::<Raised>(entity).filter(|raised| raised.kept.sheared) else { return };
    raised.kept.sheared = false;
    if let Some(rules) = sapling_data::husbandry(kind) {
        regroup(world, entity, &rules.on_regrow);
    }
    shown::wool(world, entity);
}

/// Shears, a player's or a dispenser's, on a grown sheep with its wool, or a grown mooshroom.
/// False where they do nothing.
fn shear(world: &mut World, entity: Entity, kind: &str, player: Option<Entity>, variant: i32) -> bool {
    let Some(raised) = world.get::<Raised>(entity).filter(|raised| !raised.is_baby()) else { return false };
    let colour = COLOURS[raised.kept.colour.clamp(0, 15) as usize];
    match kind {
        SHEEP if !raised.kept.sheared => {
            let count = 1 + world.get_resource_or_insert_with(Dice::default).roll(3) as u8;
            world.get_mut::<Raised>(entity).expect("read above").kept.sheared = true;
            if let Some(rules) = sapling_data::husbandry(kind) {
                regroup(world, entity, &rules.on_sheared);
            }
            shown::wool(world, entity);
            give(world, entity, format!("minecraft:{colour}_wool"), count);
        }
        MOOSHROOM => {
            let mushroom = if variant == 1 { "minecraft:brown_mushroom" } else { "minecraft:red_mushroom" };
            give(world, entity, mushroom.to_owned(), 5);
            super::become_cow(world, entity);
        }
        _ => return false,
    }
    shown::sound(world, entity, "shear");
    if let Some(player) = player.filter(|player| !is_creative(world, *player)) {
        inventory::wear_hand_by(world, player, 1, SHEARS_LAST);
    }
    true
}

/// Shears from a dispenser on the mobs whose box is in `cell`: whether one was sheared.
pub(crate) fn shear_in(world: &mut World, layer: Entity, cell: BlockPos) -> bool {
    let [x, y, z] = cell.map(|c| c as f32);
    let inside = |body: &Body| {
        let (half, tall) = (body.size[0] / 2.0, body.size[1]);
        body.at[0] + half > x && body.at[0] - half < x + 1.0 && body.at[1] + tall > y && body.at[1] < y + 1.0 && body.at[2] + half > z && body.at[2] - half < z + 1.0
    };
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body, &Kind, &Creature), With<Raised>>();
    let found: Vec<(Entity, String, i32)> = mobs.iter(world).filter(|(_, on, body, ..)| on.0 == layer && inside(body)).map(|(mob, _, _, kind, creature)| (mob, kind.0.clone(), creature.look.as_ref().map_or(0, |look| look.variant))).collect();
    found.into_iter().any(|(mob, kind, variant)| shear(world, mob, &kind, None, variant))
}

/// A bucket on a cow, a mooshroom or a grown goat fills with milk. False for anything else.
pub(super) fn milk(world: &mut World, entity: Entity, kind: &str, player: Entity) -> bool {
    let baby = world.get::<Raised>(entity).is_some_and(Raised::is_baby);
    if !MILKS.contains(&kind) || baby && kind == "minecraft:goat" {
        return false;
    }
    inventory::swap_held(world, player, "minecraft:milk_bucket");
    shown::sound(world, entity, "milk");
    true
}

/// A bowl on a grown mooshroom fills with stew: suspicious, of the flower its brown one last ate,
/// where it ate one. False for anything else.
pub(super) fn stew(world: &mut World, entity: Entity, kind: &str, player: Entity) -> bool {
    let Some(mut raised) = world.get_mut::<Raised>(entity).filter(|raised| kind == MOOSHROOM && !raised.is_baby()) else { return false };
    let mark = std::mem::replace(&mut raised.kept.mark_variant, UNFED);
    match mark {
        UNFED => inventory::swap_held(world, player, "minecraft:mushroom_stew"),
        _ => {
            let held = stack("minecraft:suspicious_stew".to_owned(), 1);
            spend(world, player);
            if inventory::give_and_pop(world, player, &ItemStack { damage: mark as i16, ..held.clone() }).is_some_and(|left| left > 0) {
                crate::drops::throw(world, player, ItemStack { damage: mark as i16, ..held });
            }
        }
    }
    shown::sound(world, entity, "milk_suspiciously");
    true
}

/// A flower fed to a brown mooshroom marks its next stew. False for anything else.
pub(super) fn feed_flower(world: &mut World, entity: Entity, kind: &str, item: &str, player: Entity, variant: i32) -> bool {
    let Some(mark) = FLOWERS.iter().find(|(flower, _)| item.strip_prefix("minecraft:") == Some(flower)).map(|(_, mark)| *mark) else { return false };
    let Some(mut raised) = world.get_mut::<Raised>(entity).filter(|raised| kind == MOOSHROOM && variant == 1 && raised.kept.mark_variant != mark) else { return false };
    raised.kept.mark_variant = mark;
    spend(world, player);
    shown::sound(world, entity, "eat");
    true
}

/// A dye on a sheep of another colour colours its wool. False for anything else.
pub(super) fn dye(world: &mut World, entity: Entity, kind: &str, item: &str, player: Entity) -> bool {
    let Some(colour) = item.strip_prefix("minecraft:").and_then(|name| name.strip_suffix("_dye")).and_then(|colour| COLOURS.iter().position(|known| *known == colour)) else { return false };
    let Some(mut raised) = world.get_mut::<Raised>(entity).filter(|raised| kind == SHEEP && raised.kept.colour != colour as i8) else { return false };
    raised.kept.colour = colour as i8;
    shown::wool(world, entity);
    spend(world, player);
    true
}

/// A brush on a grown armadillo takes a scute off it. False for anything else.
pub(super) fn brush(world: &mut World, entity: Entity, kind: &str, player: Entity) -> bool {
    if kind != ARMADILLO || world.get::<Raised>(entity).is_none_or(Raised::is_baby) {
        return false;
    }
    give(world, entity, "minecraft:armadillo_scute".to_owned(), 1);
    shown::sound(world, entity, "mob.armadillo.brush");
    if !is_creative(world, player) {
        inventory::wear_hand_by(world, player, BRUSH_WEAR, BRUSH_LAST);
    }
    true
}

/// What the item `item` in the player's hand takes from the mob, if it is a tool or container for it.
pub(super) fn take(world: &mut World, entity: Entity, kind: &str, item: &str, player: Entity, variant: i32) -> bool {
    match item {
        SHEARS => shear(world, entity, kind, Some(player), variant),
        BRUSH => brush(world, entity, kind, player),
        "minecraft:bucket" => milk(world, entity, kind, player),
        "minecraft:bowl" => stew(world, entity, kind, player),
        _ => feed_flower(world, entity, kind, item, player, variant) || dye(world, entity, kind, item, player),
    }
}
